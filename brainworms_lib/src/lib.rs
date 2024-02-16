#![feature(variant_count, exact_size_is_empty, array_chunks, iter_array_chunks)]
pub mod the_great_mind_palace_of_theatrical_arts;
// the reexports suck but this stays until I can focus on wrapping this

pub use brainworms_arson::{self, anyhow, egui, egui_winit, nanorand};
pub use brainworms_farting_noises;

pub use cfg_if::cfg_if;
use egui::{Color32, TextStyle, Visuals};
pub use glam;
use glam::{Mat3A, Vec3};
pub use log;
use log::info;
pub use parking_lot::*;
pub use rend3;

pub use enum_dispatch;
pub use nalgebra;

pub use brainworms_demented_robotic_meat_grinder_machine as macros;
pub use parry3d;
use rend3::types::DirectionalLight;
pub use rend3_routine;
use rend3_routine::base::BaseRenderGraph;
use std::{
    sync::Arc,
    time::{self, Duration},
};
pub use the_great_mind_palace_of_theatrical_arts as theater;
use theater::{
    basement::{
        cla::GameProgrammeSettings,
        frame_rate::FrameRate,
        grab::Grabber,
        input_handling::{InputContext, InputStatus},
    },
    play::{
        backstage::plumbing::{create_base_rendergraph, DefaultRoutines, StoredSurfaceInfo},
        orchestra::Orchestra,
        scene::{
            actors::AstinkSprite,
            stage3d::{load_skybox, lock, update_camera_mouse_params},
            AstinkScene, Camera,
        },
        Definitions, Play, Playable,
    },
};
pub use tokio;
pub use uuid;
use uuid::Uuid;
pub use wgpu;
use wgpu::TextureFormat;
pub use winit;
use winit::{
    event::{DeviceEvent, KeyEvent, WindowEvent},
    event_loop::{ControlFlow, EventLoopWindowTarget},
    keyboard::PhysicalKey,
    window::{Window, WindowBuilder},
};

use crate::theater::{
    basement::{frame_rate::update_frame_stats, input_handling::AcceptedInput},
    play::scene::{actors::draw_actor, stage3d::do_update_camera},
};

pub struct GameProgrammeData<
    PlayablesEnum, //: Playable<InputContextEnum>,
                   //    InputContextEnum: InputContext + 'static,
> {
    pub timestamp_start: Arc<time::Instant>,
    pub play: Arc<Mutex<Play<PlayablesEnum>>>,
}

#[derive(Default)]
pub struct GameProgrammeState<InputContextEnum: InputContext> {
    #[cfg(feature = "extra_debugging")]
    pub previous_profiling_stats: Option<Vec<wgpu_profiler::GpuTimerScopeResult>>,
    pub egui_routine: Arc<Mutex<Option<rend3_egui::EguiRenderRoutine>>>,
    pub egui_ctx: Arc<Mutex<Option<egui::Context>>>,
    pub egui_platform: Arc<Mutex<Option<egui_winit::State>>>,
    pub last_update: Arc<Mutex<Option<time::Instant>>>,
    pub frame_rate: Arc<Mutex<FrameRate>>,
    pub current_playable: Arc<Mutex<Option<Uuid>>>,
    pub grabber: Arc<Mutex<Option<Grabber>>>,
    pub cur_camera: Arc<Mutex<Option<theater::play::scene::Camera>>>,
    pub input_status: Arc<Mutex<InputStatus>>,
    pub window: Arc<Mutex<Option<Window>>>,
    pub renderer: Arc<Mutex<Option<rend3::Renderer>>>,
    pub routines: Arc<Mutex<Option<DefaultRoutines>>>,
    pub base_rendergraph: Arc<Mutex<Option<BaseRenderGraph>>>,
    pub cur_input_context: Arc<Mutex<InputContextEnum>>,
    pub orchestra: Arc<Mutex<Option<Orchestra>>>,
}
pub struct GameProgramme<
    PlayablesEnum: Playable<InputContextEnum>,
    InputContextEnum: InputContext + 'static,
> {
    pub data: Arc<Mutex<GameProgrammeData<PlayablesEnum>>>,
    pub state: Arc<Mutex<GameProgrammeState<InputContextEnum>>>,
    pub settings: Arc<Mutex<GameProgrammeSettings>>,
    pub rts: Arc<Mutex<Option<tokio::runtime::Runtime>>>,
}
pub type MyEvent = MyWinitEvent<AstinkScene, AstinkSprite>;
pub type Event = winit::event::Event<MyEvent>;

#[derive(Debug, Clone, PartialEq)]
pub enum MyWinitEvent<TS, TA: 'static> {
    /// Custom user event types
    Stage3D(TS),
    Actress(TA),
}

impl<
        PlayablesEnum: Playable<InputContextEnum> + 'static,
        InputContextEnum: InputContext + 'static,
    > GameProgramme<PlayablesEnum, InputContextEnum>
{
    pub async fn async_start(mut self, window_builder: WindowBuilder) {
        let iad = self.create_iad().await.unwrap();

        let (event_loop, window) = self
            .create_window(window_builder.with_visible(false))
            .unwrap();

        let window_size = window.inner_size();
        let window = Arc::new(window);
        // The one line of unsafe needed. We just need to guarentee that the window
        // outlives the use of the surface.
        //
        // Android has to defer the surface until `Resumed` is fired. This doesn't fire
        // on other platforms though :|
        let mut surface = if cfg!(target_os = "android") {
            None
        } else {
            Some(Arc::new(
                iad.instance.create_surface(window.clone()).unwrap(),
            ))
        };
        let renderer = rend3::Renderer::new(
            iad.clone(),
            self.settings.handedness,
            Some(window_size.width as f32 / window_size.height as f32),
        )
        .unwrap();
        self.state.window = Some(window.clone());
        self.state.renderer = Some(renderer.clone());
        let window = self.state.window.clone().unwrap();
        // Get the preferred format for the surface.
        //
        // Assume android supports Rgba8Srgb, as it has 100% device coverage
        let format = surface.as_ref().map_or(TextureFormat::Bgra8Unorm, |s| {
            let format = wgpu::TextureFormat::Bgra8Unorm;

            /*
            Configure the surface to be ready for rendering.
            */
            rend3::configure_surface(
                s,
                &iad.device,
                format,
                glam::UVec2::new(window_size.width, window_size.height),
                rend3::types::PresentMode::Immediate,
            );
            format
        });

        let mut spp = rend3::ShaderPreProcessor::new();
        rend3_routine::builtin_shaders(&mut spp);

        let mut data_core = renderer.data_core.lock();
        let base_rendergraph = Arc::new(Mutex::new(create_base_rendergraph(&renderer, &spp)));
        {
            let base_rendergraph = base_rendergraph.lock();
            self.state.routines = Some(Arc::new(DefaultRoutines {
                pbr: Mutex::new(rend3_routine::pbr::PbrRoutine::new(
                    &renderer,
                    &mut data_core,
                    &spp,
                    &base_rendergraph.interfaces,
                    &base_rendergraph.gpu_culler.culling_buffer_map_handle,
                )),
                skybox: Mutex::new(rend3_routine::skybox::SkyboxRoutine::new(
                    &renderer,
                    &spp,
                    &base_rendergraph.interfaces,
                )),
                tonemapping: Mutex::new(rend3_routine::tonemapping::TonemappingRoutine::new(
                    &renderer,
                    &spp,
                    &base_rendergraph.interfaces,
                    format,
                )),
            }));
        }
        self.state.base_rendergraph = Some(base_rendergraph);
        drop(data_core); // initiate noocoolar explosion
                         // SETUP CALLED HERE
        self.setup(&event_loop, format);

        // We're ready, so lets make things visible
        window.set_visible(true);
        let mut suspended = cfg!(target_os = "android");
        let mut last_user_control_mode = ControlFlow::Poll;
        let mut stored_surface_info = StoredSurfaceInfo {
            size: glam::UVec2::new(window_size.width, window_size.height),
            scale_factor: self.scale_factor(),
            sample_count: self.sample_count(),
            present_mode: self.present_mode(),
        };

        // IMPORTANT this is where the loop actually starts you dumbass
        Self::winit_run(event_loop, move |event, event_loop_window_target| {
            let mut control_flow = event_loop_window_target.control_flow();
            if let Some(suspend) = Self::handle_surface(
                &mut self,
                window.clone(),
                &event,
                &iad.instance,
                &mut surface,
                &renderer,
                format,
                &mut stored_surface_info,
            ) {
                suspended = suspend;
            }

            // We move to Wait when we get suspended so we don't spin at 50k FPS.
            match event {
                Event::Suspended => {
                    control_flow = ControlFlow::Wait;
                }
                Event::Resumed => {
                    control_flow = last_user_control_mode;
                }
                _ => {}
            }

            // We need to block all updates
            if let Event::WindowEvent {
                window_id: _,
                event: winit::event::WindowEvent::RedrawRequested,
            } = event
            {
                if suspended {
                    return;
                }
            }

            self.handle_event(
                surface.as_ref(),
                event,
                |c: ControlFlow| {
                    control_flow = c;
                    last_user_control_mode = c;
                },
                event_loop_window_target,
            )
        })
        .unwrap();
    }

    #[allow(clippy::too_many_arguments)]
    fn handle_event(
        &mut self,
        surface: Option<&Arc<rend3::types::Surface>>,
        event: Event,
        control_flow: impl FnOnce(winit::event_loop::ControlFlow),
        event_loop_window_target: &EventLoopWindowTarget<MyEvent>,
    ) {
        let game_data = &mut self.data;
        let game_state = &mut self.state;
        let window = game_state.window.clone().unwrap();
        let window_size = window.inner_size();
        let renderer = game_state.renderer.clone().unwrap();
        let routines = game_state.routines.clone().unwrap();
        let resolution = glam::UVec2::new(window_size.width, window_size.height);
        let base_rendergraph = game_state.base_rendergraph.clone().unwrap();
        let base_rendergraph = base_rendergraph.lock();

        match event {
            Event::WindowEvent {
                window_id: _,
                event: WindowEvent::RedrawRequested,
            } => {
                update_frame_stats(game_state);
                do_update_camera(game_state);

                // Get a frame
                let frame = surface.unwrap().get_current_texture().unwrap();
                let egui_ctx = game_state.egui_ctx.clone().unwrap();
                let egui_platform = game_state.egui_platform.as_mut().unwrap();

                egui_ctx.begin_frame(egui_platform.take_egui_input(&window));

                // Insert egui commands here
                let current_scene_id = game_state.current_playable.unwrap();
                let current_scene = game_data.play.playables.get_mut(&current_scene_id).unwrap();

                current_scene.implement_chorus_for_playable(
                    egui_ctx.clone(),
                    game_state.orchestra.as_ref().unwrap().clone(),
                );
                egui::Window::new("FPS").show(&egui_ctx, |ui| {
                    ui.label(std::format!(
                        "framerate: {:.0}fps",
                        game_state.frame_rate.get()
                    ))
                });
                // End the UI frame. Now let's draw the UI with our Backend, we could also
                // handle the output here
                let egui::FullOutput {
                    shapes,
                    textures_delta,
                    ..
                } = egui_ctx.end_frame();
                let paint_jobs = egui_ctx.tessellate(shapes, window.scale_factor() as f32);

                let input = rend3_egui::Input {
                    clipped_meshes: &paint_jobs,
                    textures_delta,
                    context: egui_ctx.clone(),
                };

                // Swap the instruction buffers so that our frame's changes can be processed.
                renderer.swap_instruction_buffers();
                // Evaluate our frame's world-change instructions
                let mut eval_output = renderer.evaluate_instructions();

                // Lock the routines
                let pbr_routine = lock(&routines.pbr);
                let mut skybox_routine = lock(&routines.skybox);
                let tonemapping_routine = lock(&routines.tonemapping);
                skybox_routine.evaluate(&renderer);
                /*
                Build a rendergraph
                */
                let mut graph = rend3::graph::RenderGraph::new();

                // Import the surface texture into the render graph.
                let frame_handle = graph.add_imported_render_target(
                    &frame,
                    0..1,
                    0..1,
                    rend3::graph::ViewportRect::from_size(resolution),
                );

                base_rendergraph.add_to_graph(
                    &mut graph,
                    rend3_routine::base::BaseRenderGraphInputs {
                        eval_output: &eval_output,
                        routines: rend3_routine::base::BaseRenderGraphRoutines {
                            pbr: &pbr_routine,
                            skybox: Some(&skybox_routine),
                            tonemapping: &tonemapping_routine,
                        },
                        target: rend3_routine::base::OutputRenderTarget {
                            handle: frame_handle,
                            resolution,
                            samples: self.settings.samples,
                        },
                    },
                    rend3_routine::base::BaseRenderGraphSettings {
                        ambient_color: Vec3::splat(self.settings.ambient_light_level).extend(1.0),
                        clear_color: glam::Vec4::new(0.0, 0.0, 0.0, 1.0),
                    },
                );

                // Add egui on top of all the other passes
                let egui_routine = game_state.egui_routine.as_mut().unwrap();
                egui_routine.add_to_graph(&mut graph, input, frame_handle);

                // Dispatch a render using the built up rendergraph!
                cfg_if! {
                    if #[cfg(feature = "extra_debugging")] {
                        game_state.previous_profiling_stats = graph.execute(&renderer, &mut eval_output);
                    }
                    else {
                        graph.execute(&renderer, &mut eval_output);
                    }
                }
                if let theater::play::Implementations::SceneImplementation(
                    ref mut cs_implementation,
                ) = game_data
                    .play
                    .playables
                    .get_mut(&current_scene_id)
                    .unwrap()
                    .playable_implementation()
                    .as_mut()
                    .unwrap()
                {
                    let t = game_data.timestamp_start.elapsed().as_secs_f32();
                    let dt = game_state.last_update.unwrap().elapsed().as_secs_f32();
                    let actresses = cs_implementation.actresses.values();
                    for a in actresses {
                        let renderer = Arc::clone(&renderer);
                        let a = Arc::clone(a);
                        // this kind of makes self.spawn at best useless and probably counter-productive
                        self.rts.as_ref().map(|rts| {
                            rts.spawn(async move {
                                draw_actor(a, renderer, t, dt);
                            })
                        });
                    }
                }
                // Present the frame
                frame.present();
                #[cfg(feature = "extra_debugging")]
                // mark the end of the frame for tracy/other profilers
                profiling::finish_frame!();
                control_flow(winit::event_loop::ControlFlow::Poll);
            }
            Event::AboutToWait => {
                #[cfg(feature = "extra_debugging")]
                profiling::scope!("MainEventsCleared");

                let current_scene_id = game_state.current_playable.as_ref().unwrap();
                let current_scene = game_data.play.playables.get_mut(current_scene_id).unwrap();

                current_scene.handle_input_for_playable(&self.settings, game_state, &window);

                window.request_redraw();
            }
            Event::WindowEvent { event, .. } => {
                // Pass the window events to the egui integration.

                let egui_platform = game_state.egui_platform.as_mut().unwrap();
                if egui_platform.on_window_event(&window, &event).consumed {
                    return;
                }

                match event {
                    WindowEvent::CloseRequested => {
                        let the_rt = self.rts.take();
                        the_rt.unwrap().shutdown_timeout(Duration::from_secs(10));
                        event_loop_window_target.exit();
                    }
                    winit::event::WindowEvent::Resized(size) => {
                        let egui_routine = game_state.egui_routine.as_mut().unwrap();
                        egui_routine.resize(size.width, size.height, window.scale_factor() as f32);
                    }

                    WindowEvent::KeyboardInput {
                        event:
                            KeyEvent {
                                physical_key: PhysicalKey::Code(key_code),
                                state,
                                ..
                            },
                        ..
                    } => {
                        log::debug!("pressed {:?}", key_code);
                        game_state
                            .input_status
                            .buttons
                            .insert(AcceptedInput::KB(key_code), state);
                    }
                    WindowEvent::MouseInput { button, state, .. } => {
                        game_state
                            .input_status
                            .buttons
                            .insert(AcceptedInput::M(button), state);
                    }
                    WindowEvent::CursorMoved {
                        device_id: _,
                        position,
                    } => {
                        game_state.input_status.mouse_physical_poz = position;
                    }

                    _ => {}
                }
            }
            Event::DeviceEvent {
                event:
                    DeviceEvent::MouseMotion {
                        delta: (delta_x, delta_y),
                        ..
                    },
                ..
            } => {
                update_camera_mouse_params(
                    self.settings.absolute_mouse,
                    game_state,
                    delta_x,
                    delta_y,
                );
            }
            Event::UserEvent(MyWinitEvent::Stage3D(AstinkScene::Loaded((name, sc_id, scdata)))) => {
                info!(
                    "Actually caught the user event and assigned the stage3d data to current scene"
                );
                if let theater::play::Implementations::SceneImplementation(sc_imp) = game_data
                    .play
                    .playables
                    .get_mut(&sc_id)
                    .unwrap()
                    .playable_implementation()
                    .as_mut()
                    .unwrap()
                {
                    sc_imp.stage3d = AstinkScene::Loaded((name, sc_id, scdata));
                }
            }
            Event::UserEvent(MyWinitEvent::Actress(AstinkSprite::Loaded((
                name,
                sc_id,
                acdata,
            )))) => {
                info!("Actually caught the user event and assigned sprite data to {name}");

                if let theater::play::Implementations::SceneImplementation(sc_imp) = game_data
                    .play
                    .playables
                    .get_mut(&sc_id)
                    .unwrap()
                    .playable_implementation()
                    .as_mut()
                    .unwrap()
                {
                    sc_imp.actresses.insert(
                        name.clone(),
                        Arc::new(Mutex::new(AstinkSprite::Loaded((name, sc_id, acdata)))),
                    );
                }
            }
            _ => {}
        }
    }

    fn setup(
        &mut self,
        event_loop: &winit::event_loop::EventLoop<MyEvent>,
        surface_format: rend3::types::TextureFormat,
    ) {
        let renderer = self.state.renderer.clone().unwrap();
        let window = self.state.window.clone().unwrap();
        self.state.grabber = Some(Grabber::new(&window));
        if let Some(direction) = self.settings.directional_light_direction {
            self.settings.directional_light = Some(
                renderer.clone().add_directional_light(DirectionalLight {
                    color: Vec3::splat(1.), //Vec3::new(1., 0.9, 0.8),
                    intensity: self.settings.directional_light_intensity,
                    direction,
                    distance: self
                        .settings
                        .gltf_settings
                        .directional_light_shadow_distance,
                    resolution: self.settings.gltf_settings.directional_light_resolution,
                }),
            );
        }

        let window_size = window.inner_size();

        // Create the egui render routine
        let egui_routine = rend3_egui::EguiRenderRoutine::new(
            &renderer,
            surface_format,
            rend3::types::SampleCount::One,
            window_size.width,
            window_size.height,
            window.scale_factor() as f32,
        );

        // Create the egui context
        let egui_ctx = egui::Context::default();
        egui_ctx.set_visuals(Visuals {
            panel_fill: Color32::TRANSPARENT,
            window_fill: Color32::TRANSPARENT,
            extreme_bg_color: Color32::TRANSPARENT,
            code_bg_color: Color32::TRANSPARENT,
            faint_bg_color: Color32::TRANSPARENT,
            ..Default::default()
        });
        // increase font size
        egui_ctx.style_mut(|style| {
            if let Some(hum) = style.text_styles.get_mut(&TextStyle::Body) {
                hum.size = 24.;
            }
        });

        // Create the winit/egui integration.
        let egui_platform = egui_winit::State::new(
            egui_ctx.clone(),
            egui::ViewportId::default(),
            &window,
            Some(window.scale_factor() as f32),
            None,
        );

        // Definitions for Play/Scene/etc go above
        let state = &mut self.state;

        state.egui_routine = Some(egui_routine);
        state.egui_ctx = Some(egui_ctx);
        state.egui_platform = Some(egui_platform);
        state.last_update = Some(time::Instant::now());
        state.frame_rate = FrameRate::new(100);
        state.current_playable = None;
        state.cur_camera = None;
        state.input_status = InputStatus::default();
        state.orchestra = Some(Arc::new(Orchestra::new(
            self.rts.as_ref().unwrap().handle().clone(),
        )));
        //        state.cur_input_context = ;

        // Implementations for Play/Scene/etc go below
        let data = &mut self.data;
        let play = &mut data.play;
        let state = &mut self.state;
        state.current_playable = Some(play.first_playable);
        let scene1 = play.playables.get_mut(&play.first_playable).unwrap();

        // Set camera location data
        if let Definitions::SceneDefinition(definition) = scene1.playable_definition() {
            let scene1_starting_cam_info = definition.cameras.get(&definition.start_cam).unwrap();
            let cur_cam = Camera {
                name: definition.start_cam.clone(),
                renderer_camera: rend3::types::Camera::default(),
                info: scene1_starting_cam_info.clone(),
                rotation: Mat3A::default(),
            };
            state.cur_camera = Some(cur_cam);
        }
        let routines = state.routines.clone().unwrap();
        let playable_renderer_copy = Arc::clone(&renderer);
        let playable_routines_copy = Arc::clone(&routines);
        scene1.implement_playable(
            &self.settings,
            event_loop,
            playable_renderer_copy,
            playable_routines_copy,
            self.rts.as_ref().unwrap(),
            self.state.orchestra.as_ref().unwrap().clone(),
        );

        let skybox_renderer_copy = Arc::clone(&renderer);
        let skybox_routines_copy = Arc::clone(&routines);
        self.spawn(async move {
            if let Err(e) = load_skybox(&skybox_renderer_copy, &skybox_routines_copy.skybox).await {
                info!("Failed to load skybox {}", e)
            };
        });
    }
}
