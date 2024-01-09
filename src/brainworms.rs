#![feature(variant_count, exact_size_is_empty)]
mod the_great_mind_palace_of_theatrical_arts;
use egui::{Color32, TextStyle, Visuals};

use glam::{vec3, vec4, DVec2, Mat3A, Mat4, Vec3};
use log::info;
use nalgebra::Point3;
use parking_lot::Mutex;
use parry3d::query::{Ray, RayCast};
use rend3::types::{
    Camera, CameraProjection, DirectionalLight, Handedness, ObjectHandle, ObjectMeshKind,
    ResourceHandle, VertexAttributeId, VERTEX_ATTRIBUTE_POSITION,
};

use uuid::Uuid;

use std::{path::Path, sync::Arc, time};
use wgpu::{ComputePassDescriptor, TextureFormat};

use the_great_mind_palace_of_theatrical_arts as theater;
use theater::{
    basement::{
        cla::GameProgrammeSettings, frame_rate::FrameRate, grab::Grabber, logging::register_logger,
        platform_scancodes::Scancodes,
    },
    play::{
        backstage::plumbing::{start, DefaultRoutines, StoredSurfaceInfo},
        definition::define_play,
        scene::{
            actors::AstinkSprite,
            stage3d::{button_pressed, load_skybox, lock},
            AstinkScene,
        },
        Definitions, Play, Playable,
    },
};
use winit::{
    dpi::PhysicalPosition,
    event::{DeviceEvent, ElementState, KeyEvent, MouseButton, WindowEvent},
    event_loop::{ControlFlow, EventLoopWindowTarget},
    platform::scancode::PhysicalKeyExtScancode,
    window::{Fullscreen, WindowBuilder},
};

use crate::theater::play::{
    scene::{actors::draw_actor, stage3d::make_camera},
    Implementations,
};

pub struct GameProgrammeData {
    pub egui_routine: rend3_egui::EguiRenderRoutine,
    pub egui_ctx: egui::Context,
    pub platform: egui_winit::State,
    pub _start_time: time::Instant,
    pub last_update: time::Instant,
    pub frame_rate: FrameRate,
    pub elapsed: f32,
    pub timestamp_start: time::Instant,
    pub play: Play,
    pub current_playable: Option<Uuid>,
    pub mouse_physical_poz: PhysicalPosition<f64>,
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
                let current_scene_id = data.current_playable.unwrap();
                let current_scene = data.play.playables.get_mut(&current_scene_id).unwrap();

                current_scene.implement_chorus_for_playable(data.egui_ctx.clone());
                egui::Window::new("FPS").show(&data.egui_ctx, |ui| {
                    ui.label(std::format!("framerate: {:.0}fps", data.frame_rate.get()))
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

                if let theater::play::Implementations::SceneImplementation(
                    ref mut cs_implementation,
                ) = data
                    .play
                    .playables
                    .get_mut(&current_scene_id)
                    .unwrap()
                    .playable_implementation()
                    .as_mut()
                    .unwrap()
                {
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
                    let cam_x = self.settings.camera_location.x;
                    let cam_y = self.settings.camera_location.y;
                    let cam_z = self.settings.camera_location.z;
                    let cam_pitch = self.settings.camera_pitch;
                    let cam_yaw = self.settings.camera_yaw;
                    println!("{cam_x},{cam_y},{cam_z},{cam_pitch},{cam_yaw}",);
                    println!(
                        "mouse at {},{}",
                        data.mouse_physical_poz.x, data.mouse_physical_poz.y
                    );
                    let win_w = window.inner_size().width as f64;
                    let win_h = window.inner_size().height as f64;
                    let mouse_x = data.mouse_physical_poz.x;
                    let mouse_y = data.mouse_physical_poz.y;
                    let x = (2.0 * mouse_x) / win_w - 1.0;

                    let y = 1.0 - (2.0 * mouse_y) / win_h;
                    let z = 1.0;
                    let ray_nds = vec3(x as f32, y as f32, z as f32);
                    println!("ray_nds: {ray_nds}");
                    let ray_clip = vec4(ray_nds.x, ray_nds.y, -1.0, 1.0);
                    let cur_camera =
                        make_camera(("".to_owned(), [cam_x, cam_y, cam_z, cam_pitch, cam_yaw]));
                    let ray_eye = compute_projection_matrix(
                        cur_camera.renderer_camera,
                        Self::HANDEDNESS,
                        (win_w / win_h) as f32,
                    )
                    .inverse()
                        * ray_clip;
                    let ray_eye = vec4(ray_eye.x, ray_eye.y, -1.0, 0.0);
                    println!("ray_eye: {ray_eye}");
                    let ray_wor4 = cur_camera.renderer_camera.view.inverse() * ray_eye;
                    let ray_wor = vec3(ray_wor4.x, ray_wor4.y, ray_wor4.z);
                    let ray_wor = ray_wor.normalize();
                    println!("ray_world: {ray_wor}");
                    let rayman = Ray::new(Point3::new(cam_x, cam_y, cam_z), ray_wor.into());

                    if let Implementations::SceneImplementation(sc_imp) = data
                        .play
                        .playables
                        .get_mut(data.current_playable.as_ref().unwrap())
                        .as_mut()
                        .unwrap()
                        .playable_implementation()
                        .as_mut()
                        .unwrap()
                    {
                        const MAX_TOI: f32 = 100000.0;
                        if let AstinkScene::Loaded(stage3d) = &sc_imp.stage3d {
                            for (c_name, colliders) in stage3d.2 .2.iter() {
                                for c in colliders {
                                    if let Some(toi) = c.cast_local_ray(&rayman, MAX_TOI, true) {
                                        let intersection = rayman.point_at(toi);

                                        if Point3::from([cam_x, cam_y, cam_z]) != intersection {
                                            println!(
                                                "{} intersects mouse ray at {}",
                                                c_name, intersection
                                            );
                                            let line = draw_line(vec![
                                                [cam_x, cam_y, cam_z],
                                                [intersection.x, intersection.y, intersection.z],
                                            ]);
                                            let line_mesh_handle = renderer.add_mesh(line).unwrap();

                                            let line_mesh_material_handle = renderer.add_material(
                                                rend3_routine::pbr::PbrMaterial::default(),
                                            );
                                            let line_mesh_object = rend3::types::Object {
                                                mesh_kind: rend3::types::ObjectMeshKind::Static(
                                                    line_mesh_handle,
                                                ),
                                                material: line_mesh_material_handle,
                                                transform:
                                                    glam::Mat4::from_scale_rotation_translation(
                                                        glam::Vec3::new(1.0, 1.0, 1.0),
                                                        glam::Quat::from_euler(
                                                            glam::EulerRot::XYZ,
                                                            0.0,
                                                            0.0,
                                                            0.0,
                                                        ),
                                                        glam::Vec3::new(0.0, 0.0, 0.0),
                                                    ),
                                            };
                                            Box::leak(Box::new(
                                                renderer.add_object(line_mesh_object),
                                            ));
                                        }
                                    }
                                }
                            }
                        }
                    }
                    /* this wouldnt work
                            let scdata = &stage3d.2;

                            for humpf in &scdata.1.topological_order {
                                let n = scdata.1.nodes.get(*humpf).unwrap();
                                if let Some(l) = n.label {
                                    for p in n.inner.object.unwrap().inner.primitives {
                                        for obj in renderer
                                            .data_core
                                            .lock()
                                            .object_manager
                                            .enumerated_objects()
                                            .unwrap()
                                        {
                                            if let ObjectMeshKind::Static(m) = obj.1.mesh_kind {
                                                let mm = m.get_raw();

                                            }
                                        }
                                    }
                                }
                            }
                            //

                            for ok in &scdata.0.meshes {
                                if let Some(l) = &ok.label {
                                    //if l == "vt100" {
                                    //                                    println!("{:?}", ok);
                                    for wat in &ok.inner.primitives {
                                        let parosphere;
                                        {
                                            let mesh_mgr_internal =
                                                &renderer.mesh_manager.lock_internal_data();
                                            let hng = &mesh_mgr_internal[*wat.handle];

                                            let pos = hng
                                                .get_attribute(&VERTEX_ATTRIBUTE_POSITION)
                                                .unwrap();
                                            for wtf in pos {


                                            }
                                            parosphere =
                                                parry3d::bounding_volume::BoundingSphere::new(
                                                    hng.bounding_sphere.center.into(),
                                                    hng.bounding_sphere.radius,
                                                );
                                        }
                                        const MAX_TOI: f32 = 1000000.0;
                                        if parosphere.intersects_local_ray(&rayman, MAX_TOI) {
                                            let toi = parosphere
                                                .cast_local_ray(&rayman, MAX_TOI, true)
                                                .unwrap();
                                            let intersection = rayman.point_at(toi);

                                            if Point3::from([cam_x, cam_y, cam_z]) != intersection {
                                                println!(
                                                    "{} intersects mouse ray at {}",
                                                    l, intersection
                                                );
                                                let line = draw_line(vec![
                                                    [cam_x, cam_y, cam_z],
                                                    [
                                                        intersection.x,
                                                        intersection.y,
                                                        intersection.z,
                                                    ],
                                                ]);
                                                let line_mesh_handle =
                                                    renderer.add_mesh(line).unwrap();

                                                let line_mesh_material_handle = renderer
                                                    .add_material(
                                                        rend3_routine::pbr::PbrMaterial::default(),
                                                    );
                                                let line_mesh_object = rend3::types::Object {
                                                    mesh_kind: rend3::types::ObjectMeshKind::Static(
                                                        line_mesh_handle,
                                                    ),
                                                    material: line_mesh_material_handle,
                                                    transform:
                                                        glam::Mat4::from_scale_rotation_translation(
                                                            glam::Vec3::new(1.0, 1.0, 1.0),
                                                            glam::Quat::from_euler(
                                                                glam::EulerRot::XYZ,
                                                                0.0,
                                                                0.0,
                                                                0.0,
                                                            ),
                                                            glam::Vec3::new(0.0, 0.0, 0.0),
                                                        ),
                                                };
                                                Box::leak(Box::new(
                                                    renderer.add_object(line_mesh_object),
                                                ));
                                            }
                                        }
                                    }
                                    //                                    }
                                }
                            }

                        }

                    }
                    */
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
                    WindowEvent::CursorMoved {
                        device_id: _,
                        position,
                    } => {
                        data.mouse_physical_poz = position;
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
                if let theater::play::Implementations::SceneImplementation(sc_imp) = data
                    .play
                    .playables
                    .get_mut(&sc_id)
                    .unwrap()
                    .playable_implementation()
                    .as_mut()
                    .unwrap()
                {
                    /*
                    for hng in scdata.0.meshes {
                        for prims in hng.inner.primitives {
                            // this won't work
                        }
                    }
                    */
                    sc_imp.stage3d = AstinkScene::Loaded((name, sc_id, scdata));
                }
            }
            Event::UserEvent(MyWinitEvent::Actress(AstinkSprite::Loaded((
                name,
                sc_id,
                acdata,
            )))) => {
                info!("Actually caught the user event and assigned sprite data to {name}");

                if let theater::play::Implementations::SceneImplementation(sc_imp) = data
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
            timestamp_start,
            play,
            current_playable: None,
            mouse_physical_poz: PhysicalPosition::default(),
        });
        // Implementations for Play/Scene/etc go below
        let data = self.data.as_mut().unwrap();
        let play = &mut data.play;
        data.current_playable = Some(play.first_playable);
        let scene1 = play.playables.get_mut(&play.first_playable).unwrap();

        // Set camera location data
        if let Definitions::SceneDefinition(definition) = scene1.playable_definition() {
            let scene1_starting_cam_info = definition.cameras.get(&definition.start_cam).unwrap();

            self.settings.camera_location = glam::Vec3A::new(
                scene1_starting_cam_info[0],
                scene1_starting_cam_info[1],
                scene1_starting_cam_info[2],
            );
            self.settings.camera_pitch = scene1_starting_cam_info[3];
            self.settings.camera_yaw = scene1_starting_cam_info[4];
        }
        let playable_renderer_copy = Arc::clone(renderer);
        let playable_routines_copy = Arc::clone(routines);
        scene1.implement_playable(
            &self.settings,
            event_loop,
            playable_renderer_copy,
            playable_routines_copy,
            &self.rts,
        );

        let skybox_renderer_copy = Arc::clone(renderer);
        let skybox_routines_copy = Arc::clone(routines);
        self.spawn(async move {
            if let Err(e) = load_skybox(&skybox_renderer_copy, &skybox_routines_copy.skybox).await {
                info!("Failed to load skybox {}", e)
            };
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
pub(crate) fn compute_projection_matrix(
    data: Camera,
    handedness: Handedness,
    aspect_ratio: f32,
) -> Mat4 {
    match data.projection {
        CameraProjection::Orthographic { size } => {
            let half = size * 0.5;
            if handedness == Handedness::Left {
                Mat4::orthographic_lh(-half.x, half.x, -half.y, half.y, half.z, -half.z)
            } else {
                Mat4::orthographic_rh(-half.x, half.x, -half.y, half.y, half.z, -half.z)
            }
        }
        CameraProjection::Perspective { vfov, near } => {
            if handedness == Handedness::Left {
                Mat4::perspective_infinite_reverse_lh(vfov.to_radians(), aspect_ratio, near)
            } else {
                Mat4::perspective_infinite_reverse_rh(vfov.to_radians(), aspect_ratio, near)
            }
        }
        CameraProjection::Raw(proj) => proj,
    }
}

pub fn draw_line(points: Vec<[f32; 3]>) -> rend3::types::Mesh {
    const WIDTH: f32 = 0.5;
    let mut vertices: Vec<glam::Vec3> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    let w = WIDTH / 2.0;

    let x1 = points[0][0];
    let x2 = points[1][0];
    let y1 = points[0][1];
    let y2 = points[1][1];
    let z1 = points[0][2];
    let z2 = points[1][2];
    //    let color: [f32; 3] = [1.0, 1.0, 1.0];

    //let dx = x2 - x1;
    //let dy = y2 - y1;
    //let l = dx.hypot(dy);
    //let u = dx * WIDTH * 0.5 / l;
    //let v = dy * WIDTH * 0.5 / l;

    vertices.push(glam::Vec3::from([x1 + w, y1 - w, z1]));
    vertices.push(glam::Vec3::from([x1 - w, y1 + w, z1]));
    vertices.push(glam::Vec3::from([x2 - w, y2 + w, z2]));
    vertices.push(glam::Vec3::from([x2 + w, y2 - w, z2]));

    indices.push(2);
    indices.push(1);
    indices.push(0);
    indices.push(2);
    indices.push(0);
    indices.push(3);

    rend3::types::MeshBuilder::new(vertices, rend3::types::Handedness::Right)
        .with_indices(indices)
        .build()
        .unwrap()
}
