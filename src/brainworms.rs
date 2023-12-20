#![feature(variant_count)]
pub mod backstage;
pub mod play;
use backstage::plumbing::frame_rate;
use backstage::pyrotechnics::kinetic_narrative::{Gay, KineticEffect, KineticLabel, ShakeLetters};
use egui::{Color32, TextStyle, Visuals};
use frame_rate::FrameRate;
use glam::{DVec2, Vec3, Vec3A};
use instant::Instant;
use log::info;
use nanorand::{RandomGen, Rng};
use pico_args::Arguments;
use play::stage3d::{
    extract_array, extract_backend, extract_msaa, extract_profile, extract_vec3, extract_vsync,
    option_arg,
};
use rend3::types::{DirectionalLight, DirectionalLightHandle, SampleCount};
use rend3::util::typedefs::FastHashMap;
use rend3::RendererProfile;
use rend3_framework::{AssetPath, UserResizeEvent};
use rend3_routine::pbr::NormalTextureYDirection;
use std::{
    fs::File,
    io::{BufRead, BufReader, Lines},
    path::Path,
    process::exit,
    sync::Arc,
};
use wgpu::Backend;
use wgpu_profiler::GpuTimerScopeResult;
use winit::{event::WindowEvent, event_loop::EventLoopWindowTarget};

use crate::play::stage3d::{load_gltf, load_skybox, spawn};

struct GameProgrammeData {
    _object_handle: rend3::types::ObjectHandle,
    material_handle: rend3::types::MaterialHandle,
    _directional_handle: rend3::types::DirectionalLightHandle,

    egui_routine: rend3_egui::EguiRenderRoutine,
    egui_ctx: egui::Context,
    platform: egui_winit::State,
    color: [f32; 4],
    _test_text: String,
    test_lines: String,
    random_line_effects: Vec<KineticEffect>,
    _start_time: instant::Instant,
    last_update: instant::Instant,
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
    timestamp_last_second: Instant,
    timestamp_last_frame: Instant,
    frame_times: histogram::Histogram,
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
        let desired_backend =
            option_arg(args.opt_value_from_fn(["-b", "--backend"], extract_backend));
        let desired_device_name: Option<String> =
            option_arg(args.opt_value_from_str(["-d", "--device"]))
                .map(|s: String| s.to_lowercase());
        let desired_mode = option_arg(args.opt_value_from_fn(["-p", "--profile"], extract_profile));
        let samples =
            option_arg(args.opt_value_from_fn("--msaa", extract_msaa)).unwrap_or(SampleCount::One);
        let present_mode = option_arg(args.opt_value_from_fn(["-v", "--vsync"], extract_vsync))
            .unwrap_or(rend3::types::PresentMode::Fifo);

        // Windowing
        let absolute_mouse: bool = args.contains("--absolute-mouse");
        let fullscreen = args.contains("--fullscreen");

        // Assets
        let normal_direction = match args.contains("--normal-y-down") {
            true => NormalTextureYDirection::Down,
            false => NormalTextureYDirection::Up,
        };
        let directional_light_direction =
            option_arg(args.opt_value_from_fn("--directional-light", extract_vec3));
        let directional_light_intensity: f32 =
            option_arg(args.opt_value_from_str("--directional-light-intensity")).unwrap_or(4.0);
        let ambient_light_level: f32 =
            option_arg(args.opt_value_from_str("--ambient")).unwrap_or(0.10);
        let scale: Option<f32> = option_arg(args.opt_value_from_str("--scale"));
        let shadow_distance: Option<f32> = option_arg(args.opt_value_from_str("--shadow-distance"));
        let shadow_resolution: Option<u16> =
            option_arg(args.opt_value_from_str("--shadow-resolution"));
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
        if let Some(shadow_resolution) = shadow_resolution {
            gltf_settings.directional_light_resolution = shadow_resolution;
        }

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
            timestamp_last_second: Instant::now(),
            timestamp_last_frame: Instant::now(),
            frame_times: histogram::Histogram::new(),
            last_mouse_delta: None,

            grabber: None,
        }
    }
}

const SAMPLE_COUNT: rend3::types::SampleCount = rend3::types::SampleCount::One;

#[derive(Default)]
struct GameProgramme {
    data: Option<GameProgrammeData>,
    settings: Option<GameProgrammeSettings>,
    rust_logo: egui::TextureId,
}
impl rend3_framework::App for GameProgramme {
    const HANDEDNESS: rend3::types::Handedness = rend3::types::Handedness::Left;

    fn sample_count(&self) -> rend3::types::SampleCount {
        SAMPLE_COUNT
    }

    fn setup(
        &mut self,
        _event_loop: &winit::event_loop::EventLoop<rend3_framework::UserResizeEvent<()>>,
        window: &winit::window::Window,
        renderer: &Arc<rend3::Renderer>,
        routines: &Arc<rend3_framework::DefaultRoutines>,
        surface_format: rend3::types::TextureFormat,
    ) {
        let mut _settings = GameProgrammeSettings::new();
        _settings.grabber = Some(rend3_framework::Grabber::new(window));
        if let Some(direction) = _settings.directional_light_direction {
            _settings.directional_light = Some(renderer.add_directional_light(DirectionalLight {
                color: Vec3::splat(1.0),
                intensity: _settings.directional_light_intensity,
                direction,
                distance: _settings.gltf_settings.directional_light_shadow_distance,
                resolution: 2048,
            }));
        }

        let window_size = window.inner_size();

        // Create the egui render routine
        let mut egui_routine = rend3_egui::EguiRenderRoutine::new(
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
        // Create mesh and calculate smooth normals based on vertices
        let mesh = create_mesh();

        // Add mesh to renderer's world.
        //
        // All handles are refcounted, so we only need to hang onto the handle until we
        // make an object.
        let mesh_handle = renderer.add_mesh(mesh).unwrap();

        // Add PBR material with all defaults except a single color.
        let material = rend3_routine::pbr::PbrMaterial {
            albedo: rend3_routine::pbr::AlbedoComponent::Value(glam::Vec4::new(0.0, 0.5, 0.5, 1.0)),
            transparency: rend3_routine::pbr::Transparency::Blend,
            ..rend3_routine::pbr::PbrMaterial::default()
        };
        let material_handle = renderer.add_material(material);

        // Combine the mesh and the material with a location to give an object.
        let object = rend3::types::Object {
            mesh_kind: rend3::types::ObjectMeshKind::Static(mesh_handle),
            material: material_handle.clone(),
            transform: glam::Mat4::IDENTITY,
        };

        // Creating an object will hold onto both the mesh and the material
        // even if they are deleted.
        //
        // We need to keep the object handle alive.
        let _object_handle = renderer.add_object(object);

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

        // Create a single directional light
        //
        // We need to keep the directional light handle alive.
        let _directional_handle = renderer.add_directional_light(rend3::types::DirectionalLight {
            color: glam::Vec3::ONE,
            intensity: 10.0,
            // Direction will be normalized
            direction: glam::Vec3::new(-1.0, -4.0, 2.0),
            distance: 400.0,
            resolution: 2048,
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
        let image_bytes = include_bytes!("../assets/rust-logo-128x128-blk.png");
        let image_image = image::load_from_memory(image_bytes).unwrap();
        let image_rgba = image_image.as_rgba8().unwrap().clone().into_raw();

        use image::GenericImageView;
        let dimensions = image_image.dimensions();

        let format = wgpu::TextureFormat::Rgba8UnormSrgb;

        self.rust_logo = rend3_egui::EguiRenderRoutine::create_egui_texture(
            &mut egui_routine.internal,
            renderer,
            format,
            &image_rgba,
            dimensions,
            Some("rust_logo_texture"),
        );

        let color: [f32; 4] = [0.0, 0.5, 0.5, 1.0];

        self.data = Some(GameProgrammeData {
            _object_handle,
            material_handle,
            _directional_handle,
            _start_time: instant::Instant::now(),
            last_update: instant::Instant::now(),
            frame_rate: FrameRate::new(100),
            elapsed: 0.0,
            egui_routine,
            egui_ctx,
            platform,
            color,
            test_lines,
            _test_text,
            random_line_effects,
        });

        let gltf_settings = _settings.gltf_settings;
        let file_to_load = _settings.file_to_load.take();
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
        self.settings = Some(_settings);
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
                data.last_update = instant::Instant::now();
                data.egui_ctx
                    .begin_frame(data.platform.take_egui_input(window));

                // Insert egui commands here
                let ctx = &data.egui_ctx;
                egui::Window::new("Change color")
                    .resizable(true)
                    .show(ctx, |ui| {
                        ui.label(std::format!("framerate: {:.0}fps", data.frame_rate.get()));
                        ui.label("Change the color of the cube");
                        if ui
                            .color_edit_button_rgba_unmultiplied(&mut data.color)
                            .changed()
                        {
                            renderer.update_material(
                                &data.material_handle.clone(),
                                rend3_routine::pbr::PbrMaterial {
                                    albedo: rend3_routine::pbr::AlbedoComponent::Value(
                                        glam::Vec4::from(data.color),
                                    ),
                                    transparency: rend3_routine::pbr::Transparency::Blend,
                                    ..rend3_routine::pbr::PbrMaterial::default()
                                },
                            );
                        }
                        ui.label("Want to get rusty?");
                        if ui
                            .add(egui::widgets::ImageButton::new((
                                self.rust_logo,
                                egui::Vec2::splat(64.0),
                            )))
                            .clicked()
                        {
                            webbrowser::open("https://www.rust-lang.org")
                                .expect("failed to open URL");
                        }
                    });

                egui::Window::new("egui widget testing").show(ctx, |ui| {
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
                renderer.swap_instruction_buffers();
                // Evaluate our frame's world-change instructions
                let mut eval_output = renderer.evaluate_instructions();

                // Lock the routines
                let pbr_routine = rend3_framework::lock(&routines.pbr);
                let tonemapping_routine = rend3_framework::lock(&routines.tonemapping);

                // Build a rendergraph
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
                    None,
                    &tonemapping_routine,
                    frame_handle,
                    resolution,
                    SAMPLE_COUNT,
                    glam::Vec4::ZERO,
                    glam::Vec4::new(0.10, 0.05, 0.10, 1.0), // Nice scene-referred purple
                );

                // Add egui on top of all the other passes
                data.egui_routine
                    .add_to_graph(&mut graph, input, frame_handle);

                // Dispatch a render using the built up rendergraph!
                graph.execute(renderer, &mut eval_output);

                // Present the frame
                frame.present();

                control_flow(winit::event_loop::ControlFlow::Poll);
            }
            rend3_framework::Event::AboutToWait => {
                window.request_redraw();
            }
            rend3_framework::Event::WindowEvent { event, .. } => {
                // Pass the window events to the egui integration.
                if data.platform.on_window_event(window, &event).consumed {
                    return;
                }

                match event {
                    winit::event::WindowEvent::Resized(size) => {
                        data.egui_routine.resize(
                            size.width,
                            size.height,
                            window.scale_factor() as f32,
                        );
                    }
                    winit::event::WindowEvent::CloseRequested => {
                        event_loop_window_target.exit();
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
}

fn main() {
    let app = GameProgramme::default();
    rend3_framework::start(
        app,
        winit::window::WindowBuilder::new()
            .with_title("egui")
            .with_maximized(true),
    )
}

fn vertex(pos: [f32; 3]) -> glam::Vec3 {
    glam::Vec3::from(pos)
}

fn create_mesh() -> rend3::types::Mesh {
    let vertex_positions = [
        // far side (0.0, 0.0, 1.0)
        vertex([-1.0, -1.0, 1.0]),
        vertex([1.0, -1.0, 1.0]),
        vertex([1.0, 1.0, 1.0]),
        vertex([-1.0, 1.0, 1.0]),
        // near side (0.0, 0.0, -1.0)
        vertex([-1.0, 1.0, -1.0]),
        vertex([1.0, 1.0, -1.0]),
        vertex([1.0, -1.0, -1.0]),
        vertex([-1.0, -1.0, -1.0]),
        // right side (1.0, 0.0, 0.0)
        vertex([1.0, -1.0, -1.0]),
        vertex([1.0, 1.0, -1.0]),
        vertex([1.0, 1.0, 1.0]),
        vertex([1.0, -1.0, 1.0]),
        // left side (-1.0, 0.0, 0.0)
        vertex([-1.0, -1.0, 1.0]),
        vertex([-1.0, 1.0, 1.0]),
        vertex([-1.0, 1.0, -1.0]),
        vertex([-1.0, -1.0, -1.0]),
        // top (0.0, 1.0, 0.0)
        vertex([1.0, 1.0, -1.0]),
        vertex([-1.0, 1.0, -1.0]),
        vertex([-1.0, 1.0, 1.0]),
        vertex([1.0, 1.0, 1.0]),
        // bottom (0.0, -1.0, 0.0)
        vertex([1.0, -1.0, 1.0]),
        vertex([-1.0, -1.0, 1.0]),
        vertex([-1.0, -1.0, -1.0]),
        vertex([1.0, -1.0, -1.0]),
    ];

    let index_data: &[u32] = &[
        0, 1, 2, 2, 3, 0, // far
        4, 5, 6, 6, 7, 4, // near
        8, 9, 10, 10, 11, 8, // right
        12, 13, 14, 14, 15, 12, // left
        16, 17, 18, 18, 19, 16, // top
        20, 21, 22, 22, 23, 20, // bottom
    ];

    rend3::types::MeshBuilder::new(vertex_positions.to_vec(), rend3::types::Handedness::Left)
        .with_indices(index_data.to_vec())
        .build()
        .unwrap()
}
pub fn read_lines<P>(filename: P) -> std::io::Result<Lines<BufReader<File>>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    Ok(BufReader::new(file).lines())
}
