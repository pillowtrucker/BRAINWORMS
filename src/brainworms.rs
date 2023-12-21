#![feature(variant_count)]
mod the_great_mind_palace_of_theatrical_arts;
use egui::{Color32, TextStyle, Visuals};
use frame_rate::FrameRate;
use glam::{DVec2, Mat3A, Mat4, Vec3, Vec3A};
use log::info;
use nanorand::{RandomGen, Rng};
use pico_args::Arguments;
use rend3::types::{
    Camera, CameraProjection, DirectionalLight, DirectionalLightHandle, SampleCount,
};
use rend3::util::typedefs::FastHashMap;
use rend3::RendererProfile;
use rend3_framework::{lock, AssetPath, Event, UserResizeEvent};
use rend3_routine::pbr::NormalTextureYDirection;
use std::{
    fs::File,
    io::{BufRead, BufReader, Lines},
    path::Path,
    process::exit,
    sync::Arc,
};
use the_great_mind_palace_of_theatrical_arts as theater;
use theater::basement::frame_rate;
use theater::play::backstage::pyrotechnics::kinetic_narrative::{
    Gay, KineticEffect, KineticLabel, ShakeLetters,
};
use theater::play::scene::stage3d::{
    extract_array, extract_backend, extract_msaa, extract_profile, extract_vec3, extract_vsync,
    option_arg,
};
use wgpu::Backend;
use wgpu_profiler::GpuTimerScopeResult;
use winit::event::{DeviceEvent, ElementState, KeyEvent, MouseButton};
use winit::window::{Fullscreen, WindowBuilder};
use winit::{event::WindowEvent, event_loop::EventLoopWindowTarget};

#[cfg(not(wasm_platform))]
use std::time;
use theater::basement::platform_scancodes::Scancodes;
use theater::play::scene::stage3d::{button_pressed, load_gltf, load_skybox, spawn};
#[cfg(wasm_platform)]
use web_time as time;
#[cfg(target_arch = "wasm32")]
use winit::keyboard::PhysicalKey::Code;

#[cfg(not(target_arch = "wasm32"))]
use winit::platform::scancode::PhysicalKeyExtScancode;
struct GameProgrammeData {
    egui_routine: rend3_egui::EguiRenderRoutine,
    egui_ctx: egui::Context,
    platform: egui_winit::State,
    _test_text: String,
    test_lines: String,
    random_line_effects: Vec<KineticEffect>,
    _start_time: time::Instant,
    last_update: time::Instant,
    frame_rate: FrameRate,
    elapsed: f32,
}
const HELP: &str = "\
scene-viewer

gltf and glb scene viewer powered by the rend3 rendering library.

usage: scene-viewer --options ./path/to/gltf/file.gltf

Meta:
  --help            This menu.

Rendering:
  -b --backend                 Choose backend to run on ('vk', 'dx12', 'dx11', 'metal', 'gl').
  -d --device                  Choose device to run on (case insensitive device substring).
  -p --profile                 Choose rendering profile to use ('cpu', 'gpu').
  -v --vsync                   Choose vsync mode ('immediate' [no-vsync], 'fifo' [vsync], 'fifo_relaxed' [adaptive vsync], 'mailbox' [fast vsync])
  --msaa <level>               Level of antialiasing (either 1 or 4). Default 1.

Windowing:
  --absolute-mouse             Interpret the relative mouse coordinates as absolute. Useful when using things like VNC.
  --fullscreen                 Open the window in borderless fullscreen.

Assets:
  --normal-y-down                        Interpret all normals as having the DirectX convention of Y down. Defaults to Y up.
  --directional-light <x,y,z>            Create a directional light pointing towards the given coordinates.
  --directional-light-intensity <value>  All lights created by the above flag have this intensity. Defaults to 4.
  --gltf-disable-directional-lights      Disable all directional lights in the gltf
  --ambient <value>                      Set the value of the minimum ambient light. This will be treated as white light of this intensity. Defaults to 0.1.
  --scale <scale>                        Scale all objects loaded by this factor. Defaults to 1.0.
  --shadow-distance <value>              Distance from the camera there will be directional shadows. Lower values means higher quality shadows. Defaults to 100.
  --shadow-resolution <value>            Resolution of the shadow map. Higher values mean higher quality shadows with high performance cost. Defaults to 2048.

Controls:
  --walk <speed>               Walk speed (speed without holding shift) in units/second (typically meters). Default 10.
  --run  <speed>               Run speed (speed while holding shift) in units/second (typically meters). Default 50.
  --camera x,y,z,pitch,yaw     Spawns the camera at the given position. Press Period to get the current camera position.
";

struct GameProgrammeSettings {
    absolute_mouse: bool,
    desired_backend: Option<Backend>,
    desired_device_name: Option<String>,
    desired_profile: Option<RendererProfile>,
    file_to_load: Option<String>,
    walk_speed: f32,
    run_speed: f32,
    gltf_settings: rend3_gltf::GltfLoadSettings,
    directional_light_direction: Option<Vec3>,
    directional_light_intensity: f32,
    directional_light: Option<DirectionalLightHandle>,
    ambient_light_level: f32,
    present_mode: rend3::types::PresentMode,
    samples: SampleCount,
    fullscreen: bool,
    scancode_status: FastHashMap<u32, bool>,
    camera_pitch: f32,
    camera_yaw: f32,
    camera_location: Vec3A,
    previous_profiling_stats: Option<Vec<GpuTimerScopeResult>>,
    last_mouse_delta: Option<DVec2>,
    grabber: Option<rend3_framework::Grabber>,
}
impl GameProgrammeSettings {
    pub fn new() -> Self {
        #[cfg(feature = "tracy")]
        tracy_client::Client::start();

        let mut args = Arguments::from_vec(std::env::args_os().skip(1).collect());

        // Meta
        let help = args.contains(["-h", "--help"]);

        // Rendering
        let desired_backend = option_arg(
            args.opt_value_from_fn(["-b", "--backend"], extract_backend),
            HELP,
        );
        let desired_device_name: Option<String> =
            option_arg(args.opt_value_from_str(["-d", "--device"]), HELP)
                .map(|s: String| s.to_lowercase());
        let desired_mode = option_arg(
            args.opt_value_from_fn(["-p", "--profile"], extract_profile),
            HELP,
        );
        let samples = option_arg(args.opt_value_from_fn("--msaa", extract_msaa), HELP)
            .unwrap_or(SampleCount::Four);
        let present_mode = option_arg(
            args.opt_value_from_fn(["-v", "--vsync"], extract_vsync),
            HELP,
        )
        .unwrap_or(rend3::types::PresentMode::Immediate);

        // Windowing
        let absolute_mouse: bool = args.contains("--absolute-mouse");
        let fullscreen = args.contains("--fullscreen");

        // Assets
        let normal_direction = match args.contains("--normal-y-down") {
            true => NormalTextureYDirection::Down,
            false => NormalTextureYDirection::Up,
        };
        let directional_light_direction = option_arg(
            args.opt_value_from_fn("--directional-light", extract_vec3),
            HELP,
        );
        let directional_light_intensity: f32 = option_arg(
            args.opt_value_from_str("--directional-light-intensity"),
            HELP,
        )
        .unwrap_or(4.0);
        let ambient_light_level: f32 =
            option_arg(args.opt_value_from_str("--ambient"), HELP).unwrap_or(0.10);
        let scale: Option<f32> = option_arg(args.opt_value_from_str("--scale"), HELP);
        let shadow_distance: Option<f32> =
            option_arg(args.opt_value_from_str("--shadow-distance"), HELP);
        let shadow_resolution: u16 =
            option_arg(args.opt_value_from_str("--shadow-resolution"), HELP).unwrap_or(8192);
        let gltf_disable_directional_light: bool =
            args.contains("--gltf-disable-directional-lights");

        // Controls
        let walk_speed = args.value_from_str("--walk").unwrap_or(10.0_f32);
        let run_speed = args.value_from_str("--run").unwrap_or(50.0_f32);
        let camera_default = [
            3.0,
            3.0,
            3.0,
            -std::f32::consts::FRAC_PI_8,
            std::f32::consts::FRAC_PI_4,
        ];
        let camera_info = args
            .value_from_str("--camera")
            .map_or(camera_default, |s: String| {
                extract_array(&s, camera_default).unwrap()
            });

        // Free args
        let file_to_load: Option<String> = args.free_from_str().ok();

        let remaining = args.finish();

        if !remaining.is_empty() {
            eprint!("Unknown arguments:");
            for flag in remaining {
                eprint!(" '{}'", flag.to_string_lossy());
            }
            eprintln!("\n");

            eprintln!("{}", HELP);
            std::process::exit(1);
        }

        if help {
            eprintln!("{}", HELP);
            std::process::exit(1);
        }

        let mut gltf_settings = rend3_gltf::GltfLoadSettings {
            normal_direction,
            enable_directional: !gltf_disable_directional_light,
            ..Default::default()
        };
        if let Some(scale) = scale {
            gltf_settings.scale = scale
        }
        if let Some(shadow_distance) = shadow_distance {
            gltf_settings.directional_light_shadow_distance = shadow_distance;
        }

        gltf_settings.directional_light_resolution = shadow_resolution;

        Self {
            absolute_mouse,
            desired_backend,
            desired_device_name,
            desired_profile: desired_mode,
            file_to_load,
            walk_speed,
            run_speed,
            gltf_settings,
            directional_light_direction,
            directional_light_intensity,
            directional_light: None,
            ambient_light_level,
            present_mode,
            samples,

            fullscreen,

            scancode_status: FastHashMap::default(),
            camera_pitch: camera_info[3],
            camera_yaw: camera_info[4],
            camera_location: Vec3A::new(camera_info[0], camera_info[1], camera_info[2]),
            previous_profiling_stats: None,

            last_mouse_delta: None,

            grabber: None,
        }
    }
}

struct GameProgramme {
    data: Option<GameProgrammeData>,
    settings: GameProgrammeSettings,
}
impl GameProgramme {
    fn new() -> Self {
        Self {
            data: None,
            settings: GameProgrammeSettings::new(),
        }
    }
}
impl rend3_framework::App for GameProgramme {
    const HANDEDNESS: rend3::types::Handedness = rend3::types::Handedness::Right;

    fn present_mode(&self) -> rend3::types::PresentMode {
        self.settings.present_mode
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
                None,
            )
            .await?)
        })
    }
    fn sample_count(&self) -> rend3::types::SampleCount {
        self.settings.samples
    }

    fn setup(
        &mut self,
        _event_loop: &winit::event_loop::EventLoop<rend3_framework::UserResizeEvent<()>>,
        window: &winit::window::Window,
        renderer: &Arc<rend3::Renderer>,
        routines: &Arc<rend3_framework::DefaultRoutines>,
        surface_format: rend3::types::TextureFormat,
    ) {
        self.settings.grabber = Some(rend3_framework::Grabber::new(window));
        if let Some(direction) = self.settings.directional_light_direction {
            self.settings.directional_light = Some(
                renderer.add_directional_light(DirectionalLight {
                    color: Vec3::new(1., 0.9, 0.8),
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
            let loader = rend3_framework::AssetLoader::new_local(
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

    fn handle_event(
        &mut self,
        window: &winit::window::Window,
        renderer: &Arc<rend3::Renderer>,
        routines: &Arc<rend3_framework::DefaultRoutines>,
        base_rendergraph: &rend3_routine::base::BaseRenderGraph,
        surface: Option<&Arc<rend3::types::Surface>>,
        resolution: glam::UVec2,
        event: rend3_framework::Event<'_, ()>,
        control_flow: impl FnOnce(winit::event_loop::ControlFlow),
        event_loop_window_target: &EventLoopWindowTarget<UserResizeEvent<()>>,
    ) {
        let data = self.data.as_mut().unwrap();

        match event {
            rend3_framework::Event::WindowEvent {
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
                let pbr_routine = rend3_framework::lock(&routines.pbr);
                let mut skybox_routine = lock(&routines.skybox);
                let tonemapping_routine = rend3_framework::lock(&routines.tonemapping);
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

                // Add egui on top of all the other passes
                data.egui_routine
                    .add_to_graph(&mut graph, input, frame_handle);

                // Dispatch a render using the built up rendergraph!
                self.settings.previous_profiling_stats = graph.execute(renderer, &mut eval_output);

                // Present the frame
                frame.present();
                // mark the end of the frame for tracy/other profilers
                profiling::finish_frame!();
                control_flow(winit::event_loop::ControlFlow::Poll);
            }
            rend3_framework::Event::AboutToWait => {
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
            rend3_framework::Event::WindowEvent { event, .. } => {
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
}
#[cfg_attr(
    target_os = "android",
    ndk_glue::main(backtrace = "on", logger(level = "debug"))
)]
fn main() {
    let app = GameProgramme::new();
    let mut builder = WindowBuilder::new()
        .with_title("Therac3D")
        .with_maximized(true);
    if app.settings.fullscreen {
        builder = builder.with_fullscreen(Some(Fullscreen::Borderless(None)))
    }
    rend3_framework::start(app, builder)
}

pub fn read_lines<P>(filename: P) -> std::io::Result<Lines<BufReader<File>>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    Ok(BufReader::new(file).lines())
}
