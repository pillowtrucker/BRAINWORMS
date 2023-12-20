#![feature(variant_count)]
pub mod frame_rate;
pub mod kinetic_novel;
pub mod scene_viewer_utils;

use std::{
    fs::File,
    io::{BufRead, BufReader, Lines},
    path::Path,
    process::exit,
    sync::Arc,
};

use egui::{Color32, TextStyle, Visuals};
use frame_rate::FrameRate;
use kinetic_novel::{Gay, KineticEffect, KineticLabel, ShakeLetters};
use log::info;
use nanorand::{RandomGen, Rng};
use rend3_framework::UserResizeEvent;
use winit::{event::WindowEvent, event_loop::EventLoopWindowTarget};

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

const SAMPLE_COUNT: rend3::types::SampleCount = rend3::types::SampleCount::One;

#[derive(Default)]
struct GameProgramme {
    data: Option<GameProgrammeData>,
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
        _routines: &Arc<rend3_framework::DefaultRoutines>,
        surface_format: rend3::types::TextureFormat,
    ) {
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

        let Some((_test_text, test_lines)) = (match read_lines("assets/texts/PARADISE_LOST.txt") {
            Ok(test_text) => {
                let mut rng = nanorand::tls_rng();
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
        let mut rng = nanorand::tls_rng();
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
