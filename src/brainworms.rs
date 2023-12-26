#![feature(variant_count)]
mod the_great_mind_palace_of_theatrical_arts;
use egui::{viewport, Color32, TextStyle, Visuals};
use frame_rate::FrameRate;
use glam::{uvec2, vec2, DVec2, Mat3A, Mat4, UVec2, Vec2, Vec3};
use inox2d::{formats::inp::parse_inp, render::InoxRenderer};
use log::info;
use nanorand::{RandomGen, Rng};
use parking_lot::Mutex;
use rend3::{
    types::{Camera, CameraProjection, DirectionalLight, SampleCount},
    Renderer, ShaderPreProcessor,
};

use rend3_routine::base::BaseRenderGraph;

use std::{borrow::BorrowMut, path::Path, process::exit, sync::Arc};
use wgpu::{
    CommandEncoder, CommandEncoderDescriptor, Features, Instance, PresentMode, RenderPass, Surface,
    TextureFormat,
};

use the_great_mind_palace_of_theatrical_arts as theater;
use theater::play::backstage::pyrotechnics::kinetic_narrative::{
    Gay, KineticEffect, KineticLabel, ShakeLetters,
};
use theater::{
    basement::{
        cla::GameProgrammeSettings, grab::Grabber, logging::register_logger,
        logging::register_panic_hook,
    },
    play::backstage::plumbing::asset_loader::AssetLoader,
};
use theater::{
    basement::{frame_rate, text_files::read_lines},
    play::backstage::plumbing::asset_loader::AssetPath,
};

use winit::{
    dpi::PhysicalSize,
    error::EventLoopError,
    event::{DeviceEvent, ElementState, KeyEvent, MouseButton, WindowEvent},
    event_loop::{ControlFlow, EventLoop, EventLoopBuilder, EventLoopWindowTarget},
    window::{Fullscreen, Window, WindowBuilder, WindowId},
};

#[cfg(not(target_arch = "wasm32"))]
use std::time;
use theater::basement::platform_scancodes::Scancodes;
#[cfg(target_arch = "wasm32")]
use theater::basement::resize_observer::*;
use theater::play::scene::stage3d::{button_pressed, load_gltf, load_skybox, spawn};
#[cfg(target_arch = "wasm32")]
use web_time as time;
#[cfg(target_arch = "wasm32")]
use winit::keyboard::PhysicalKey::Code;
#[cfg(not(target_arch = "wasm32"))]
use winit::platform::scancode::PhysicalKeyExtScancode;

use crate::theater::play::scene::stage3d::lock;
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
}

pub struct GameProgramme {
    pub data: Option<GameProgrammeData>,
    pub settings: GameProgrammeSettings,
}
pub struct DefaultRoutines {
    pub pbr: Mutex<rend3_routine::pbr::PbrRoutine>,
    pub skybox: Mutex<rend3_routine::skybox::SkyboxRoutine>,
    pub tonemapping: Mutex<rend3_routine::tonemapping::TonemappingRoutine>,
}
struct StoredSurfaceInfo {
    size: UVec2,
    scale_factor: f32,
    sample_count: SampleCount,
    present_mode: PresentMode,
}
pub type Event = winit::event::Event<UserResizeEvent<()>>;
/// User event which the framework uses to resize on wasm.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum UserResizeEvent<T: 'static> {
    /// Used to fire off resizing on wasm
    Resize {
        window_id: WindowId,
        size: PhysicalSize<u32>,
    },
    /// Custom user event type
    Other(T),
}

pub fn start(gp: GameProgramme, window_builder: WindowBuilder) {
    #[cfg(target_arch = "wasm32")]
    {
        wasm_bindgen_futures::spawn_local(gp.async_start(window_builder));
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let Ok(wingman) = tokio::runtime::Runtime::new() else {
            panic!("no tokyo for you");
        };

        pollster::block_on(gp.async_start(window_builder));
    }
}
impl GameProgramme {
    const _PDP11_CAM_INFO: [f32; 5] = [-3.729838, 4.512105, -0.103016704, -0.4487015, 0.025398161];
    const _VT100_CAM_INFO: [f32; 5] = [-5.068789, 1.3310424, -3.6215494, -0.31070346, 6.262584];
    const _THERAC_CAM_INFO: [f32; 5] = [-2.580962, 2.8690546, 2.878742, -0.27470315, 5.620602];
    const _TOITOI_CAM_INFO: [f64; 5] = [-6.814362, 2.740766, 0.7109763, -0.17870337, 0.0073876693];
    const _OVERVIEW_CAM_INFO: [f64; 5] = [-6.217338, 3.8491437, 5.883971, -0.40870047, 5.76257];
    const HANDEDNESS: rend3::types::Handedness = rend3::types::Handedness::Right;
    fn new() -> Self {
        Self {
            data: None,
            settings: GameProgrammeSettings::new(),
        }
    }
    #[allow(clippy::too_many_arguments)]
    fn handle_surface(
        &self,
        window: &Window,
        event: &Event,
        instance: &Instance,
        surface: &mut Option<Arc<Surface>>,
        renderer: &Arc<Renderer>,
        format: rend3::types::TextureFormat,
        surface_info: &mut StoredSurfaceInfo,
    ) -> Option<bool> {
        match *event {
            Event::Resumed => {
                if surface.is_none() {
                    // uhh this is still the same one line of unsafe I guess but for android
                    *surface = Some(Arc::new(
                        unsafe { instance.create_surface(window) }.unwrap(),
                    ));
                }
                Some(false)
            }
            Event::Suspended => {
                *surface = None;
                Some(true)
            }
            Event::WindowEvent {
                event: winit::event::WindowEvent::Resized(size),
                ..
            } => {
                log::debug!("resize {:?}", size);
                let size = UVec2::new(size.width, size.height);

                if size.x == 0 || size.y == 0 {
                    return Some(false);
                }

                surface_info.size = size;
                surface_info.scale_factor = self.scale_factor();
                surface_info.sample_count = self.sample_count();
                surface_info.present_mode = self.present_mode();

                // Winit erroniously stomps on the canvas CSS when a scale factor
                // change happens, so we need to put it back to normal. We can't
                // do this in a scale factor changed event, as the override happens
                // after the event is sent.
                //
                // https://github.com/rust-windowing/winit/issues/3023
                #[cfg(target_arch = "wasm32")]
                {
                    use winit::platform::web::WindowExtWebSys;
                    let canvas = window.canvas().unwrap();
                    let style = canvas.style();

                    style.set_property("width", "100%").unwrap();
                    style.set_property("height", "100%").unwrap();
                }

                // Reconfigure the surface for the new size.
                rend3::configure_surface(
                    surface.as_ref().unwrap(),
                    &renderer.device,
                    format,
                    size,
                    surface_info.present_mode,
                );
                // Tell the renderer about the new aspect ratio.
                renderer.set_aspect_ratio(size.x as f32 / size.y as f32);
                Some(false)
            }
            _ => None,
        }
    }
    fn create_iad<'a>(
        &'a mut self,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = anyhow::Result<rend3::InstanceAdapterDevice>> + 'a>,
    > {
        Box::pin(async move {
            Ok(rend3::create_iad(
                self.settings.desired_backend,
                self.settings.desired_device_name.clone(),
                self.settings.desired_profile,
                Some(Features::ADDRESS_MODE_CLAMP_TO_BORDER),
            )
            .await?)
        })
    }
    fn create_window(
        &mut self,
        builder: WindowBuilder,
    ) -> Result<(EventLoop<UserResizeEvent<()>>, Window), EventLoopError> {
        profiling::scope!("creating window");

        let event_loop = EventLoopBuilder::with_user_event().build()?;
        let window = builder.build(&event_loop).expect("Could not build window");

        #[cfg(target_arch = "wasm32")]
        {
            use winit::platform::web::WindowExtWebSys;

            let canvas = window.canvas().unwrap();
            let style = canvas.style();
            style.set_property("width", "100%").unwrap();
            style.set_property("height", "100%").unwrap();

            web_sys::window()
                .and_then(|win| win.document())
                .and_then(|doc| doc.body())
                .and_then(|body| body.append_child(&canvas).ok())
                .expect("couldn't append canvas to document body");
        }

        Ok((event_loop, window))
    }
    fn create_base_rendergraph(
        &mut self,
        renderer: &Arc<Renderer>,
        spp: &ShaderPreProcessor,
    ) -> BaseRenderGraph {
        BaseRenderGraph::new(renderer, spp)
    }
    fn present_mode(&self) -> rend3::types::PresentMode {
        self.settings.present_mode
    }

    pub async fn async_start(mut self, window_builder: WindowBuilder) {
        let iad = self.create_iad().await.unwrap();

        let Ok((event_loop, window)) = self.create_window(window_builder.with_visible(false))
        else {
            exit(1)
        };
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
        let format = surface.as_ref().map_or(TextureFormat::Rgba8UnormSrgb, |s| {
            let caps = s.get_capabilities(&iad.adapter);
            let format = caps.formats[0];
            let alpha_modes = s.get_capabilities(&iad.adapter).alpha_modes;
            //            let alpha_mode = if alpha_modes.contains(&wgpu::CompositeAlphaMode::PreMultiplied) {
            //                wgpu::CompositeAlphaMode::PreMultiplied
            //            } else {
            //                alpha_modes[0]
            //            };
            /*
            let config = wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_DST,
                format: wgpu::TextureFormat::Bgra8Unorm,
                width: window.inner_size().width,
                height: window.inner_size().height,
                present_mode: wgpu::PresentMode::Fifo,
                alpha_mode,
                view_formats: Vec::new(),
            };
            */
            // Configure the surface to be ready for rendering.
            rend3::configure_surface(
                s,
                &iad.device,
                format,
                glam::UVec2::new(window_size.width, window_size.height),
                rend3::types::PresentMode::Fifo,
            );
            //            s.configure(&renderer.device, &config);
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

        #[cfg(target_arch = "wasm32")]
        let _observer = ResizeObserver::new(&window, event_loop.create_proxy());

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

        // On native this is a result, but on wasm it's a unit type.
        #[allow(clippy::let_unit_value)]
        let _ = Self::winit_run(event_loop, move |event, event_loop_window_target| {
            let event = match event {
                Event::UserEvent(UserResizeEvent::Resize { size, window_id }) => {
                    Event::WindowEvent {
                        window_id,
                        event: WindowEvent::Resized(size),
                    }
                }
                e => e,
            };
            let mut control_flow = event_loop_window_target.control_flow();
            if let Some(suspend) = Self::handle_surface(
                &self,
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
            /*
            let loader = AssetLoader::new_local(
                concat!(env!("CARGO_MANIFEST_DIR"), "/assets/"),
                "",
                "http://localhost:8000/assets/",
            );

            let pupper = pollster::block_on(async {
                loader
                    .get_asset(AssetPath::Internal(&self.settings.puppet_path))
                    .await
                    .unwrap()
            });
            */
            //            let mut inox_model = parse_inp(pupper.as_slice()).unwrap();
            /*            let mut inox_renderer = inox2d_wgpu::Renderer::new(
                &renderer.device,
                &renderer.queue,
                wgpu::TextureFormat::Bgra8Unorm,
                &inox_model,
                uvec2(window.inner_size().width, window.inner_size().height),
            );

            inox_renderer.camera.scale = Vec2::splat(0.15);
            */
            // event loop starts here
            self.handle_event(
                &window,
                &renderer,
                //&mut inox_renderer,
                //&mut inox_model,
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
        });
    }

    #[allow(clippy::too_many_arguments)]
    fn handle_event(
        &mut self,
        window: &winit::window::Window,
        renderer: &Arc<rend3::Renderer>,
        //        inox_renderer: &mut inox2d_wgpu::Renderer,
        //        inox_model: &mut inox2d::model::Model,
        routines: &Arc<DefaultRoutines>,
        base_rendergraph: &rend3_routine::base::BaseRenderGraph,
        surface: Option<&Arc<rend3::types::Surface>>,
        resolution: glam::UVec2,
        event: Event,
        control_flow: impl FnOnce(winit::event_loop::ControlFlow),
        event_loop_window_target: &EventLoopWindowTarget<UserResizeEvent<()>>,
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

                egui::Window::new("egui widget testing").show(ctx, |ui| {
                    ui.label(std::format!("framerate: {:.0}fps", data.frame_rate.get()));
                    ui.horizontal(|ui| {
                        ui.add(KineticLabel::new("blabla"));
                        ui.add(KineticLabel::new("same").kinesis(vec![&KineticEffect::default()]));
                        ui.add(KineticLabel::new("line").kinesis(vec![
                            &KineticEffect::ShakeLetters {
                                params: ShakeLetters::default(),
                            },
                        ]));
                        ui.add(
                            KineticLabel::new("still").kinesis(vec![&KineticEffect::Gay {
                                params: Gay::default(),
                            }]),
                        );
                    });
                    for (i, line) in data.test_lines.lines().enumerate() {
                        ui.add(KineticLabel::new(line).kinesis(vec![&data.random_line_effects[i]]));
                    }
                });
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

                // Get a frame
                let frame = surface.unwrap().get_current_texture().unwrap();

                // Swap the instruction buffers so that our frame's changes can be processed.

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
                // Add the default rendergraph without a skybox
                base_rendergraph.add_to_graph(
                    &mut graph,
                    &eval_output,
                    &pbr_routine,
                    Some(&skybox_routine),
                    &tonemapping_routine,
                    frame_handle,
                    resolution,
                    self.settings.samples,
                    Vec3::splat(self.settings.ambient_light_level).extend(1.0),
                    glam::Vec4::new(0.0, 0.0, 0.0, 1.0),
                );
                /*
                let texture_size = wgpu::Extent3d {
                    width: 1920,
                    height: 1080,
                    depth_or_array_layers: 1,
                };

                                let format = surface
                                    .unwrap()
                                    .get_current_texture()
                                    .unwrap()
                                    .texture
                                    .format();
                                let image_texture = renderer.device.create_texture(&wgpu::TextureDescriptor {
                                    size: texture_size,
                                    mip_level_count: 1,
                                    sample_count: 1,
                                    dimension: wgpu::TextureDimension::D2,
                                    format,
                                    usage: wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::RENDER_ATTACHMENT,
                                    label: Some("ok"),
                                    view_formats: &[format],
                                });
                */
                // Add egui on top of all the other passes
                data.egui_routine
                    .add_to_graph(&mut graph, input, frame_handle);
                /*
                let puppet = inox_model.puppet.borrow_mut();
                puppet.begin_set_params();
                let t = ctx.frame_nr() as f32;
                puppet.set_param("Head:: Yaw-Pitch", vec2(t.cos(), t.sin()));
                puppet.end_set_params();

                let output = image_texture; //.create_view(&wgpu::TextureViewDescriptor::default());
                let view = (output).create_view(&wgpu::TextureViewDescriptor::default());
                inox_renderer.render(&renderer.queue, &renderer.device, puppet, &view);


                let mut encoder =
                    renderer
                        .device
                        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                            label: Some("Part Render Encoder"),
                        });
                encoder.copy_texture_to_texture(
                    output.as_image_copy(),
                    surface
                        .unwrap()
                        .get_current_texture()
                        .unwrap()
                        .texture
                        .as_image_copy(),
                    surface
                        .unwrap()
                        .get_current_texture()
                        .unwrap()
                        .texture
                        .size(),
                );
                renderer.queue.submit(std::iter::once(encoder.finish()));
                */
                // Dispatch a render using the built up rendergraph!
                self.settings.previous_profiling_stats = graph.execute(renderer, &mut eval_output);

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
                        #[cfg(not(target_arch = "wasm32"))]
                        let scancode = PhysicalKeyExtScancode::to_scancode(physical_key).unwrap();
                        #[cfg(target_arch = "wasm32")]
                        let scancode = if let Code(kk) = physical_key {
                            kk as u32
                        } else {
                            0
                        };
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
            _ => {}
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn winit_run<F, T>(
        event_loop: winit::event_loop::EventLoop<T>,
        event_handler: F,
    ) -> Result<(), EventLoopError>
    where
        F: FnMut(winit::event::Event<T>, &EventLoopWindowTarget<T>) + 'static,
        T: 'static,
    {
        event_loop.run(event_handler)
    }

    #[cfg(target_arch = "wasm32")]
    fn winit_run<F, T>(event_loop: EventLoop<T>, event_handler: F)
    where
        F: FnMut(winit::event::Event<T>, &EventLoopWindowTarget<T>) + 'static,
        T: 'static,
    {
        use wasm_bindgen::prelude::*;

        let winit_closure =
            Closure::once_into_js(move || event_loop.run(event_handler).expect("Init failed"));

        // make sure to handle JS exceptions thrown inside start.
        // Otherwise wasm_bindgen_futures Queue would break and never handle any tasks
        // again. This is required, because winit uses JS exception for control flow
        // to escape from `run`.
        if let Err(error) = call_catch(&winit_closure) {
            let is_control_flow_exception = error.dyn_ref::<js_sys::Error>().map_or(false, |e| {
                e.message().includes("Using exceptions for control flow", 0)
            });

            if !is_control_flow_exception {
                web_sys::console::error_1(&error);
            }
        }

        #[wasm_bindgen]
        extern "C" {
            #[wasm_bindgen(catch, js_namespace = Function, js_name = "prototype.call.call")]
            fn call_catch(this: &JsValue) -> Result<(), JsValue>;
        }
    }

    fn scale_factor(&self) -> f32 {
        // Android has very low memory bandwidth, so lets run internal buffers at half
        // res by default
        cfg_if::cfg_if! {
            if #[cfg(target_os = "android")] {
                0.5
            } else {
                1.0
            }
        }
    }

    fn setup(
        &mut self,
        _event_loop: &winit::event_loop::EventLoop<UserResizeEvent<()>>,
        window: &winit::window::Window,
        renderer: &Arc<rend3::Renderer>,
        routines: &Arc<DefaultRoutines>,
        surface_format: rend3::types::TextureFormat,
    ) {
        self.settings.grabber = Some(Grabber::new(window));
        if let Some(direction) = self.settings.directional_light_direction {
            self.settings.directional_light = Some(
                renderer.add_directional_light(DirectionalLight {
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
        /*
        let loader = AssetLoader::new_local(
            concat!(env!("CARGO_MANIFEST_DIR"), "/assets/"),
            "",
            "http://localhost:8000/assets/",
        );

                let pp = self.settings.puppet_path.clone();
                let pupper = pollster::block_on(async move {
                    loader
                        .get_asset(AssetPath::Internal(pp.as_str()))
                        .await
                        .unwrap()
                });
                let model = parse_inp(pupper.as_slice()).unwrap();
                let mut inox_renderer = inox2d_wgpu::Renderer::new(
                    &renderer.device,
                    &renderer.queue,
                    surface_format,
                    &model,
                    uvec2(window.inner_size().width, window.inner_size().height),
                );
                inox_renderer.camera.scale = Vec2::splat(0.15);
                //        let mut scene_ctrl = ExampleSceneController::new(&inox_renderer.camera, 0.5);
        */
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
                        format!("{}{}\n", acc, l)
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
        //        let mut rng = nanorand::tls_rng();
        for _ in test_lines.lines() {
            random_line_effects.push(KineticEffect::random(&mut rng));
        }

        let camera_pitch = std::f32::consts::FRAC_PI_4;
        let camera_yaw = -std::f32::consts::FRAC_PI_4;
        // These values may seem arbitrary, but they center the camera on the cube in
        // the scene
        let camera_location = glam::Vec3A::new(5.0, 7.5, -5.0);
        let view = glam::Mat4::from_euler(glam::EulerRot::XYZ, -camera_pitch, -camera_yaw, 0.0);
        let view = view * glam::Mat4::from_translation((-camera_location).into());

        // Set camera location data
        renderer.set_camera_data(rend3::types::Camera {
            projection: rend3::types::CameraProjection::Perspective {
                vfov: 60.0,
                near: 0.1,
            },
            view,
        });

        // Create the winit/egui integration.
        let platform = egui_winit::State::new(
            egui_ctx.clone(),
            egui::ViewportId::default(),
            &window,
            Some(window.scale_factor() as f32),
            None,
        );

        //Images

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
        });

        let gltf_settings = self.settings.gltf_settings;
        let file_to_load = self.settings.file_to_load.take();
        let renderer = Arc::clone(renderer);
        let routines = Arc::clone(routines);
        spawn(async move {
            let loader = AssetLoader::new_local(
                concat!(env!("CARGO_MANIFEST_DIR"), "/assets/"),
                "",
                "http://localhost:8000/assets/",
            );
            if let Err(e) = load_skybox(&renderer, &loader, &routines.skybox).await {
                println!("Failed to load skybox {}", e)
            };
            Box::leak(Box::new(
                load_gltf(
                    &renderer,
                    &loader,
                    &gltf_settings,
                    file_to_load
                        .as_deref()
                        .map_or_else(|| AssetPath::Internal("LinacLab.glb"), AssetPath::External),
                )
                .await,
            ));
        });
    }
    fn sample_count(&self) -> rend3::types::SampleCount {
        self.settings.samples
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
    register_panic_hook();

    let the_game_programme = GameProgramme::new();
    start(the_game_programme, window_builder);
}
