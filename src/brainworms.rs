#![feature(variant_count)]
mod the_great_mind_palace_of_theatrical_arts;
use egui::{Color32, TextStyle, Visuals};

use glam::{DVec2, Mat3A, Mat4, Vec3};
use log::info;
use nanorand::{RandomGen, Rng};
use parking_lot::Mutex;
use rend3::types::{Camera, CameraProjection, DirectionalLight};

use uuid::Uuid;

use std::{collections::HashMap, path::Path, process::exit, sync::Arc, time};
use wgpu::TextureFormat;

use the_great_mind_palace_of_theatrical_arts as theater;
use theater::{
    basement::{
        cla::GameProgrammeSettings, frame_rate::FrameRate, grab::Grabber, logging::register_logger,
        platform_scancodes::Scancodes, text_files::read_lines,
    },
    play::{
        backstage::{
            plumbing::{start, DefaultRoutines, StoredSurfaceInfo},
            pyrotechnics::kinetic_narrative::KineticEffect,
        },
        definition::define_play,
        scene::{
            actors::{create_actor, AstinkSprite},
            stage3d::{button_pressed, load_skybox, load_stage3d, lock, make_camera},
            AstinkScene, SceneImplementation,
        },
        Play,
    },
};
use winit::{
    event::{DeviceEvent, ElementState, KeyEvent, MouseButton, WindowEvent},
    event_loop::{ControlFlow, EventLoopWindowTarget},
    platform::scancode::PhysicalKeyExtScancode,
    window::{Fullscreen, WindowBuilder},
};

use crate::theater::play::scene::actors::draw_actor;

pub struct GameProgrammeData {
    pub egui_routine: rend3_egui::EguiRenderRoutine,
    pub egui_ctx: egui::Context,
    pub platform: egui_winit::State,
    pub _test_text: String,
    pub test_lines: String,
    pub random_line_effects: Vec<KineticEffect>,
    pub _start_time: time::Instant,
    pub last_update: time::Instant,
    pub frame_rate: FrameRate,
    pub elapsed: f32,
    pub timestamp_start: time::Instant,
    pub play: Play,
    pub current_playable: Option<Uuid>,
}

pub struct GameProgramme {
    pub data: Option<GameProgrammeData>,
    pub settings: GameProgrammeSettings,
    pub rts: tokio::runtime::Runtime,
}
type MyEvent = MyWinitEvent<AstinkScene, AstinkSprite>;
pub type Event = winit::event::Event<MyEvent>;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum MyWinitEvent<TS, TA: 'static> {
    /// Custom user event types
    Stage3D(TS),
    Actress(TA),
}

impl GameProgramme {
    const HANDEDNESS: rend3::types::Handedness = rend3::types::Handedness::Right;

    pub async fn async_start(mut self, window_builder: WindowBuilder) {
        let iad = self.create_iad().await.unwrap();

        let (event_loop, window) = self
            .create_window(window_builder.with_visible(false))
            .unwrap();

        let window_size = window.inner_size();
        // The one line of unsafe needed. We just need to guarentee that the window
        // outlives the use of the surface.
        //
        // Android has to defer the surface until `Resumed` is fired. This doesn't fire
        // on other platforms though :|
        let mut surface = if cfg!(target_os = "android") {
            None
        } else {
            Some(Arc::new(
                unsafe { iad.instance.create_surface(&window) }.unwrap(),
            ))
        };
        let renderer = rend3::Renderer::new(
            iad.clone(),
            Self::HANDEDNESS,
            Some(window_size.width as f32 / window_size.height as f32),
        )
        .unwrap();

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

        let base_rendergraph = self.create_base_rendergraph(&renderer, &spp);
        let mut data_core = renderer.data_core.lock();
        let routines = Arc::new(DefaultRoutines {
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
        });
        drop(data_core); // initiate noocoolar explosion
                         // SETUP CALLED HERE
        self.setup(&event_loop, &window, &renderer, &routines, format);

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
                &window,
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
                &window,
                &renderer,
                &routines,
                &base_rendergraph,
                surface.as_ref(),
                stored_surface_info.size,
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
        window: &winit::window::Window,
        renderer: &Arc<rend3::Renderer>,
        routines: &Arc<DefaultRoutines>,
        base_rendergraph: &rend3_routine::base::BaseRenderGraph,
        surface: Option<&Arc<rend3::types::Surface>>,
        resolution: glam::UVec2,
        event: Event,
        control_flow: impl FnOnce(winit::event_loop::ControlFlow),
        event_loop_window_target: &EventLoopWindowTarget<MyEvent>,
    ) {
        let data = self.data.as_mut().unwrap();
        match event {
            Event::WindowEvent {
                window_id: _,
                event: WindowEvent::RedrawRequested,
            } => {
                let last_frame_duration = data.last_update.elapsed().as_secs_f32();
                data.elapsed += last_frame_duration;
                data.frame_rate.update(last_frame_duration);
                data.last_update = time::Instant::now();

                let view = Mat4::from_euler(
                    glam::EulerRot::XYZ,
                    -self.settings.camera_pitch,
                    -self.settings.camera_yaw,
                    0.0,
                );
                let view = view * Mat4::from_translation((-self.settings.camera_location).into());
                // Get a frame
                let frame = surface.unwrap().get_current_texture().unwrap();

                renderer.set_camera_data(Camera {
                    projection: CameraProjection::Perspective {
                        vfov: 60.0,
                        near: 0.1,
                    },
                    view,
                });

                data.egui_ctx
                    .begin_frame(data.platform.take_egui_input(window));

                // Insert egui commands here
                let ctx = &data.egui_ctx;
                let current_scene_id = data.current_scene.unwrap();
                let current_scene = data.play.scenes.get_mut(&current_scene_id).unwrap();
                current_scene.sing(ctx);
                // End the UI frame. Now let's draw the UI with our Backend, we could also
                // handle the output here
                let egui::FullOutput {
                    shapes,
                    textures_delta,
                    ..
                } = data.egui_ctx.end_frame();
                let paint_jobs = data
                    .egui_ctx
                    .tessellate(shapes, window.scale_factor() as f32);

                let input = rend3_egui::Input {
                    clipped_meshes: &paint_jobs,
                    textures_delta,
                    context: data.egui_ctx.clone(),
                };

                // Swap the instruction buffers so that our frame's changes can be processed.
                renderer.swap_instruction_buffers();
                // Evaluate our frame's world-change instructions
                let mut eval_output = renderer.evaluate_instructions();

                // Lock the routines
                let pbr_routine = lock(&routines.pbr);
                let mut skybox_routine = lock(&routines.skybox);
                let tonemapping_routine = lock(&routines.tonemapping);
                skybox_routine.evaluate(renderer);
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
                data.egui_routine
                    .add_to_graph(&mut graph, input, frame_handle);

                // Dispatch a render using the built up rendergraph!
                self.settings.previous_profiling_stats = graph.execute(renderer, &mut eval_output);

                let cs_implementation = current_scene.implementation.as_mut().unwrap();
                let t = data.timestamp_start.elapsed().as_secs_f32();
                let actresses = cs_implementation.actresses.values();
                for a in actresses {
                    let renderer = Arc::clone(renderer);
                    let a = Arc::clone(a);
                    // this kind of makes self.spawn at best useless and probably counter-productive
                    self.rts.spawn(async move {
                        draw_actor(a, renderer, t);
                    });
                }
                // Present the frame
                frame.present();
                // mark the end of the frame for tracy/other profilers
                profiling::finish_frame!();
                control_flow(winit::event_loop::ControlFlow::Poll);
            }
            Event::AboutToWait => {
                profiling::scope!("MainEventsCleared");

                let rotation = Mat3A::from_euler(
                    glam::EulerRot::XYZ,
                    -self.settings.camera_pitch,
                    -self.settings.camera_yaw,
                    0.0,
                )
                .transpose();
                let forward = -rotation.z_axis;
                let up = rotation.y_axis;
                let side = -rotation.x_axis;
                let velocity = if button_pressed(&self.settings.scancode_status, Scancodes::SHIFT) {
                    self.settings.run_speed
                } else {
                    self.settings.walk_speed
                };
                if button_pressed(&self.settings.scancode_status, Scancodes::W) {
                    self.settings.camera_location +=
                        forward * velocity * data.last_update.elapsed().as_secs_f32();
                }
                if button_pressed(&self.settings.scancode_status, Scancodes::S) {
                    self.settings.camera_location -=
                        forward * velocity * data.last_update.elapsed().as_secs_f32();
                }
                if button_pressed(&self.settings.scancode_status, Scancodes::A) {
                    self.settings.camera_location +=
                        side * velocity * data.last_update.elapsed().as_secs_f32();
                }
                if button_pressed(&self.settings.scancode_status, Scancodes::D) {
                    self.settings.camera_location -=
                        side * velocity * data.last_update.elapsed().as_secs_f32();
                }
                if button_pressed(&self.settings.scancode_status, Scancodes::Q) {
                    self.settings.camera_location +=
                        up * velocity * data.last_update.elapsed().as_secs_f32();
                }
                if button_pressed(&self.settings.scancode_status, Scancodes::PERIOD) {
                    println!(
                        "{x},{y},{z},{pitch},{yaw}",
                        x = self.settings.camera_location.x,
                        y = self.settings.camera_location.y,
                        z = self.settings.camera_location.z,
                        pitch = self.settings.camera_pitch,
                        yaw = self.settings.camera_yaw
                    );
                }

                if button_pressed(&self.settings.scancode_status, Scancodes::ESCAPE) {
                    self.settings
                        .grabber
                        .as_mut()
                        .unwrap()
                        .request_ungrab(window);
                }

                if button_pressed(&self.settings.scancode_status, Scancodes::P) {
                    // write out gpu side performance info into a trace readable by chrome://tracing
                    if let Some(ref stats) = self.settings.previous_profiling_stats {
                        println!("Outputing gpu timing chrome trace to profile.json");
                        wgpu_profiler::chrometrace::write_chrometrace(
                            Path::new("profile.json"),
                            stats,
                        )
                        .unwrap();
                    } else {
                        println!("No gpu timing trace available, either timestamp queries are unsupported or not enough frames have elapsed yet!");
                    }
                }
                window.request_redraw();
            }
            Event::WindowEvent { event, .. } => {
                // Pass the window events to the egui integration.
                if data.platform.on_window_event(window, &event).consumed {
                    return;
                }

                match event {
                    WindowEvent::CloseRequested => {
                        event_loop_window_target.exit();
                    }
                    winit::event::WindowEvent::Resized(size) => {
                        data.egui_routine.resize(
                            size.width,
                            size.height,
                            window.scale_factor() as f32,
                        );
                    }

                    WindowEvent::KeyboardInput {
                        event:
                            KeyEvent {
                                physical_key,
                                state,
                                ..
                            },
                        ..
                    } => {
                        let scancode = PhysicalKeyExtScancode::to_scancode(physical_key).unwrap();

                        log::info!("WE scancode {:x}", scancode);
                        self.settings.scancode_status.insert(
                            scancode,
                            match state {
                                ElementState::Pressed => true,
                                ElementState::Released => false,
                            },
                        );
                    }
                    WindowEvent::MouseInput {
                        button: MouseButton::Left,
                        state: ElementState::Pressed,
                        ..
                    } => {
                        let grabber = self.settings.grabber.as_mut().unwrap();

                        if !grabber.grabbed() {
                            grabber.request_grab(window);
                        }
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
                if !self.settings.grabber.as_ref().unwrap().grabbed() {
                    return;
                }

                const TAU: f32 = std::f32::consts::PI * 2.0;

                let mouse_delta = if self.settings.absolute_mouse {
                    let prev = self
                        .settings
                        .last_mouse_delta
                        .replace(DVec2::new(delta_x, delta_y));
                    if let Some(prev) = prev {
                        (DVec2::new(delta_x, delta_y) - prev) / 4.0
                    } else {
                        return;
                    }
                } else {
                    DVec2::new(delta_x, delta_y)
                };

                self.settings.camera_yaw -= (mouse_delta.x / 1000.0) as f32;
                self.settings.camera_pitch -= (mouse_delta.y / 1000.0) as f32;
                if self.settings.camera_yaw < 0.0 {
                    self.settings.camera_yaw += TAU;
                } else if self.settings.camera_yaw >= TAU {
                    self.settings.camera_yaw -= TAU;
                }
                self.settings.camera_pitch = self.settings.camera_pitch.clamp(
                    -std::f32::consts::FRAC_PI_2 + 0.0001,
                    std::f32::consts::FRAC_PI_2 - 0.0001,
                )
            }
            Event::UserEvent(MyWinitEvent::Stage3D(AstinkScene::Loaded((name, sc_id, scdata)))) => {
                info!(
                    "Actually caught the user event and assigned the stage3d data to current scene"
                );
                let sc_imp = data
                    .play
                    .scenes
                    .get_mut(&sc_id)
                    .unwrap()
                    .implementation
                    .as_mut()
                    .unwrap();
                sc_imp
                    .stage3d
                    .insert(name.clone(), AstinkScene::Loaded((name, sc_id, scdata)));
            }
            Event::UserEvent(MyWinitEvent::Actress(AstinkSprite::Loaded((
                name,
                sc_id,
                acdata,
            )))) => {
                info!("Actually caught the user event and assigned sprite data to {name}");
                let sc_imp = data
                    .play
                    .scenes
                    .get_mut(&sc_id)
                    .unwrap()
                    .implementation
                    .as_mut()
                    .unwrap();
                sc_imp.actresses.insert(
                    name.clone(),
                    Arc::new(Mutex::new(AstinkSprite::Loaded((name, sc_id, acdata)))),
                );
            }
            _ => {}
        }
    }

    fn setup(
        &mut self,
        event_loop: &winit::event_loop::EventLoop<MyEvent>,
        window: &winit::window::Window,
        renderer: &Arc<rend3::Renderer>,
        routines: &Arc<DefaultRoutines>,
        surface_format: rend3::types::TextureFormat,
    ) {
        self.settings.grabber = Some(Grabber::new(window));
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
        /*
        recursively load the play->scene->actor/etc definitions
        TODO: Read this from script/data file
        */
        let play = define_play();

        let window_size = window.inner_size();

        // Create the egui render routine
        let egui_routine = rend3_egui::EguiRenderRoutine::new(
            renderer,
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
        let mut rng = nanorand::tls_rng();
        let Some((_test_text, test_lines)) = (match read_lines("assets/texts/PARADISE_LOST.txt") {
            Ok(test_text) => {
                let the_body = test_text.fold("".to_owned(), |acc: String, l| {
                    if let Ok(l) = l {
                        format!("{}{}\n", acc, l) // this is probably quadratic but fuck rust's string concatenation options wholesale
                    } else {
                        acc
                    }
                });
                let good_number = rng.generate_range(0..(the_body.lines().count() - 66));
                let random_lines = the_body.lines().collect::<Vec<&str>>()
                    [good_number..good_number + 66]
                    .to_owned();
                Some((the_body.to_owned(), random_lines.to_owned().join("\n")))
            }
            Err(_) => None,
        }) else {
            info!("couldnt read text file");
            exit(1);
        };
        let mut random_line_effects = vec![];

        for _ in test_lines.lines() {
            random_line_effects.push(KineticEffect::random(&mut rng));
        }

        // Create the winit/egui integration.
        let platform = egui_winit::State::new(
            egui_ctx.clone(),
            egui::ViewportId::default(),
            &window,
            Some(window.scale_factor() as f32),
            None,
        );
        let timestamp_start = time::Instant::now();
        // Definitions for Play/Scene/etc go above
        self.data = Some(GameProgrammeData {
            _start_time: time::Instant::now(),
            last_update: time::Instant::now(),
            frame_rate: FrameRate::new(100),
            elapsed: 0.0,
            egui_routine,
            egui_ctx,
            platform,
            test_lines,
            _test_text,
            random_line_effects,
            timestamp_start,
            play,
            current_playable: None,
        });
        // Implementations for Play/Scene/etc go below
        let data = self.data.as_mut().unwrap();
        let play = &mut data.play;
        data.current_playable = Some(play.first_playable);
        let scene1 = play.scenes.get_mut(&play.first_scene).unwrap();
        // Set camera location data
        let scene1_starting_cam_info = *scene1
            .definition
            .cameras
            .get(&scene1.definition.start_cam)
            .unwrap();

        self.settings.camera_location = glam::Vec3A::new(
            scene1_starting_cam_info[0],
            scene1_starting_cam_info[1],
            scene1_starting_cam_info[2],
        );
        self.settings.camera_pitch = scene1_starting_cam_info[3];
        self.settings.camera_yaw = scene1_starting_cam_info[4];
        let scene1_starting_cam = make_camera((
            scene1.definition.start_cam.clone(),
            scene1_starting_cam_info,
        ));
        let mut scene1_cameras = HashMap::new();
        scene1_cameras.insert(scene1_starting_cam.name.clone(), scene1_starting_cam);
        let gltf_settings = self.settings.gltf_settings;
        let renderer = Arc::clone(renderer);
        let routines = Arc::clone(routines);
        let event_loop_proxy = event_loop.create_proxy();
        let scene1_uuid = scene1.scene_uuid;
        let scene1_stage_name = scene1.definition.stage.0.clone();
        let scene1_stage_directory = scene1.definition.stage.1.clone();
        let mut scene1_stage3d = HashMap::new();
        scene1_stage3d.insert(scene1_stage_name.clone(), AstinkScene::Loading);

        let mut scene1_actor_impls = HashMap::new();
        for (name, _) in scene1.definition.actors.clone() {
            scene1_actor_impls.insert(name.clone(), AstinkSprite::Loading);
        }
        let scene1_implementation = SceneImplementation {
            stage3d: scene1_stage3d,
            actresses: HashMap::new(),
            props: HashMap::new(), // todo!(),
            cameras: scene1_cameras,
        };

        scene1.implementation = Some(scene1_implementation);
        let scene1_actors = scene1.definition.actors.clone();
        for (name, directory) in scene1_actors {
            let renderer = Arc::clone(&renderer);
            let event_loop_proxy = event_loop.create_proxy();
            let name = name.to_owned();
            self.spawn(async move {
                create_actor(name, directory, renderer, event_loop_proxy, scene1_uuid).await
            });
        }
        let skybox_renderer_copy = Arc::clone(&renderer);
        let skybox_routines_copy = Arc::clone(&routines);
        self.spawn(async move {
            if let Err(e) = load_skybox(&skybox_renderer_copy, &skybox_routines_copy.skybox).await {
                info!("Failed to load skybox {}", e)
            };
        });

        self.spawn(async move {
            load_stage3d(
                scene1_stage_name,
                scene1_stage_directory,
                scene1_uuid,
                renderer,
                gltf_settings,
                event_loop_proxy,
            )
            .await;
        });
    }
}

#[cfg_attr(
    target_os = "android",
    ndk_glue::main(backtrace = "on", logger(level = "debug"))
)]
fn main() {
    let window_builder = WindowBuilder::new()
        .with_title("Therac3D")
        .with_maximized(true)
        .with_fullscreen(Some(Fullscreen::Borderless(None)))
        .with_decorations(false);
    register_logger();

    let the_game_programme = GameProgramme::new();
    start(the_game_programme, window_builder);
}
