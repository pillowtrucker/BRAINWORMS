use bl::brainworms_arson::{parse_fireworks, Gay, KineticEffect, KineticLabel, ShakeLetters};

use bl::brainworms_farting_noises::TicketedAudioRequestData as TARD;
use bl::brainworms_farting_noises::{AudioCommand, SoundGroup};
use bl::log::{debug, info};
use bl::nalgebra::distance;
use bl::nanorand::RandomGen;

use bl::parry3d::query::RayCast;

use bl::rend3::Renderer;
use bl::the_great_mind_palace_of_theatrical_arts::basement::input_handling::{
    AcceptedInput, DebugInputContext, HandlesInputContexts, KeyBindings,
};
use bl::the_great_mind_palace_of_theatrical_arts::play::orchestra::Orchestra;
use bl::the_great_mind_palace_of_theatrical_arts::play::scene::actors::create_actor;
use bl::the_great_mind_palace_of_theatrical_arts::play::scene::chorus::Choral;
use bl::the_great_mind_palace_of_theatrical_arts::play::scene::stage3d::{
    get_collisions_from_camera, load_stage3d, CollisionMap,
};
use bl::the_great_mind_palace_of_theatrical_arts::play::scene::Scenic;
use bl::tokio::runtime::Runtime;
use bl::uuid::{uuid, Uuid};
use bl::winit::event::MouseButton;
use bl::winit::event_loop::EventLoop;
use bl::winit::keyboard::KeyCode;
use bl::winit::window::Window;
use bl::{egui, glam, nalgebra, nanorand, MyEvent};
use brainworms_lib as bl;

use brainworms_lib::{
    theater::{
        basement::{cla::GameProgrammeSettings, text_files::read_lines},
        play::{
            backstage::plumbing::DefaultRoutines,
            scene::{
                actors::{ActressDefinition, AstinkSprite},
                stage3d::{make_camera, make_ray, update_camera_rotation},
                AstinkScene, CamInfo, SceneDefinition, SceneImplementation,
            },
            Definitions, Implementations,
        },
    },
    GameProgrammeState,
};
use egui::Context;
use nanorand::Rng;

use std::borrow::BorrowMut;
use std::path::PathBuf;
use std::thread::sleep;
use std::time::Duration;
use std::{collections::HashMap, f32::consts::PI, sync::Arc};

use DebugInputContext as DIC;
use MyInputContexts as MIC;

use crate::MyInputContexts;
//use MyInputContexts::DebugInputContext as DIC;

#[derive(Default, Hash, Eq, PartialEq, Debug, Copy, Clone)]
pub enum LinacLabIC {
    FocusObject,
    SwitchToDebug,
    Back,
    #[default]
    Marker,
}

//const PDP11_CAM_INFO: [f32; 5] = [-3.729838, 4.512105, -0.103016704, -0.4487015, 0.025398161];
const VT100_CAM_INFO: [f32; 5] = [-5.0835533, 1.3423158, -2.9590514, -0.26870108, 6.278548];
const THERAC_CAM_INFO: [f32; 5] = [-2.580962, 2.8690546, 2.878742, -0.27470315, 5.620602];
const TOITOI_CAM_INFO: [f32; 5] = [-6.814362, 2.740766, 0.7109763, -0.17870337, 0.0073876693];
const OVERVIEW_CAM_INFO: [f32; 5] = [-6.217338, 3.8491437, 5.883971, -0.40870047, 5.76257];
const PDP11_WITH_MIDORI_CAM_INFO: [f32; 5] =
    [-3.7894087, 3.8481617, 0.3033728, -0.29471007, 6.2545333];

//#[add_common_playable_fields] // this is not worth the stupid RA errors
#[derive(Default, bl::proc_macros::Scenic, bl::proc_macros::Choral)]
pub struct LinacLabScene {
    pub uuid: Uuid,
    pub name: String,
    pub definition: Definitions,
    pub implementation: Option<Implementations>,
    pub test_text: String,
    pub test_lines: String,
    pub test_markup: Vec<String>,
    pub parsed_test_markup: Vec<KineticLabel>,
    pub random_line_effects: Vec<KineticEffect>,
    pub keybindings: KeyBindings<MyInputContexts>,
}

impl LinacLabScene {
    pub fn define(&mut self) {
        // add default debug bindings (this can probably be derived TODO)
        let mut keybindings = KeyBindings::from(
            [
                (DIC::Sprint, KeyCode::ShiftLeft),
                (DIC::Forwards, KeyCode::KeyW),
                (DIC::Backwards, KeyCode::KeyS),
                (DebugInputContext::StrafeLeft, KeyCode::KeyA),
                (DebugInputContext::StrafeRight, KeyCode::KeyD),
                (DebugInputContext::LiftUp, KeyCode::KeyQ),
                (DebugInputContext::Interact, KeyCode::Period),
                (DebugInputContext::Back, KeyCode::Escape),
                (DIC::Profiling, KeyCode::KeyP),
                (DIC::SwitchToScene, KeyCode::F7),
            ]
            .map(|(lb, kc)| (MIC::DebugInputContext(lb), AcceptedInput::KB(kc))),
        );
        #[allow(clippy::single_element_loop)]
        for (lb, mb) in [(DebugInputContext::GrabWindow, MouseButton::Left)] {
            keybindings.insert(MIC::DebugInputContext(lb), AcceptedInput::M(mb));
        }
        // add keyboard bindings for scene1
        #[allow(clippy::single_element_loop)]
        for (lb, kc) in [(LinacLabIC::SwitchToDebug, KeyCode::F7)] {
            keybindings.insert(MIC::LinacLabIC(lb), AcceptedInput::KB(kc));
        }
        // add mouse bindings for scene1
        #[allow(clippy::single_element_loop)]
        for (lb, mb) in [
            (LinacLabIC::FocusObject, MouseButton::Left),
            (LinacLabIC::Back, MouseButton::Right),
        ] {
            keybindings.insert(MIC::LinacLabIC(lb), AcceptedInput::M(mb));
        }
        self.keybindings = keybindings;
        let next_to_pdp11 = glam::Mat4::from_scale_rotation_translation(
            glam::Vec3::new(1.0, 1.0, 1.0),
            glam::Quat::from_euler(glam::EulerRot::XYZ, 0., PI, 0.0),
            glam::Vec3::new(-2.0586073, 1.5, -4.085335),
        );
        self.uuid = uuid!("517e70e9-9f6d-48fe-a685-e24482d6d409");
        let midori = ActressDefinition {
            name: "Midori".to_owned(),
            directory: "assets/inochi2d-models".to_owned(),
            transform: next_to_pdp11,
            size: 5.0,
        };
        self.definition = Definitions::SceneDefinition(SceneDefinition {
            stage: ("LinacLab".to_owned(), "assets/gltf_scenes".to_owned()),
            actors: vec![midori],
            props: vec![("fried_egg".to_owned(), "lfs_scam/props".to_owned())],
            start_cam: "overview".to_owned(),
            cameras: vec![
                ("overview".to_owned(), OVERVIEW_CAM_INFO),
                //                ("pdp11".to_owned(), PDP11_CAM_INFO),
                ("pdp11".to_owned(), PDP11_WITH_MIDORI_CAM_INFO),
                ("vt100".to_owned(), VT100_CAM_INFO),
                ("Therac-25".to_owned(), THERAC_CAM_INFO),
                ("PortaPotty".to_owned(), TOITOI_CAM_INFO),
            ]
            .iter()
            .fold(HashMap::new(), |mut h, (k, v)| {
                h.insert(k.to_owned(), CamInfo::from_arr(v));
                h
            }),
        });
        self.name = "LinacLab".to_owned();

        let mut rng = nanorand::tls_rng();
        let Some((test_text, test_lines)) = (match read_lines("assets/texts/PARADISE_LOST.txt") {
            Ok(test_text) => {
                let the_body = test_text.map(|w| w.unwrap()).collect::<Vec<_>>();
                let good_number = rng.generate_range(0..(the_body.len() - 66));
                let random_lines = the_body[good_number..good_number + 66].to_owned();
                Some((the_body.join("\n"), random_lines.join("\n")))
            }
            Err(_) => None,
        }) else {
            panic!("couldnt read text file");
        };
        if let Ok(markup_lines) = read_lines("lfs_scam/test_markup.txt") {
            self.test_markup = markup_lines.map(Result::unwrap).collect();
        }
        if let Ok((_, parsed)) = parse_fireworks(&self.test_markup.join("")) {
            self.parsed_test_markup = parsed;
        }
        let mut random_line_effects = vec![];

        for _ in test_lines.lines() {
            random_line_effects.push(KineticEffect::random(&mut rng));
        }
        self.test_text = test_text;
        self.test_lines = test_lines;
        self.random_line_effects = random_line_effects;
    }

    pub fn implement(
        &mut self,
        settings: &GameProgrammeSettings,
        event_loop: &EventLoop<MyEvent>,
        renderer: Arc<Renderer>,
        _routines: Arc<DefaultRoutines>,
        rts: &Runtime,
        orchestra: Arc<Orchestra>,
    ) {
        let Definitions::SceneDefinition(definition) = &self.definition else {
            panic!("scene has non-scene definition")
        };

        let mut scene1_cameras = HashMap::new();

        for (c_n, cam_info) in &definition.cameras {
            let cam = make_camera((c_n.to_owned(), cam_info.as_arr()));
            scene1_cameras.insert(cam.name.clone(), cam);
        }

        let gltf_settings = settings.gltf_settings;

        let event_loop_proxy = event_loop.create_proxy();
        let scene1_uuid = self.uuid;
        let scene1_stage_name = definition.stage.0.clone();
        let scene1_stage_directory = definition.stage.1.clone();
        let scene1_stage3d = AstinkScene::Loading;

        let mut scene1_actor_impls = HashMap::new();
        for ActressDefinition { name, .. } in definition.actors.clone() {
            scene1_actor_impls.insert(name.clone(), AstinkSprite::Loading);
        }
        let scene1_implementation = SceneImplementation {
            stage3d: scene1_stage3d,
            actresses: HashMap::new(),
            props: HashMap::new(), // todo!(),
            cameras: scene1_cameras,
        };

        self.implementation = Some(Implementations::SceneImplementation(scene1_implementation));
        let scene1_actors = definition.actors.clone();
        for ActressDefinition {
            name,
            directory,
            transform,
            size,
        } in scene1_actors
        {
            let renderer = Arc::clone(&renderer);
            let event_loop_proxy = event_loop.create_proxy();
            let name = name.to_owned();
            rts.spawn(async move {
                create_actor(
                    name,
                    directory,
                    renderer,
                    event_loop_proxy,
                    transform,
                    size,
                    scene1_uuid,
                )
                .await
            });
        }
        let mut collider_ids = HashMap::new();
        [
            "Therac-25",
            "PortaPotty",
            "vt100",
            "pdp11",
            "Podloga",
            "Przedzialek",
            "Sciana1",
            "Sciana2",
            "Sciana3",
            "Sciana4",
        ]
        .iter()
        .for_each(|c| {
            let k = (*c).to_owned();
            let v = k.clone();
            collider_ids.insert(k, v.to_owned());
        });
        rts.spawn(async move {
            load_stage3d(
                scene1_stage_name,
                scene1_stage_directory,
                scene1_uuid,
                renderer,
                gltf_settings,
                event_loop_proxy,
                collider_ids,
            )
            .await;
        });
        let test_path =
            PathBuf::from("./brainworms_farting_noises/libymfm.wasm/docs/vgm/ym2612.vgm");
        let test_filename = Arc::new(test_path.file_name().unwrap().to_string_lossy().to_string());
        let mut rng = nanorand::tls_rng();
        let random_bytes = [0; 16];
        let random_bytes = random_bytes.map(|_| rng.generate::<u8>());

        info!("random bytes {random_bytes:?}");
        let test_uuid = bl::uuid::Builder::from_random_bytes(random_bytes).into_uuid();
        orchestra.send_cmd(AudioCommand::Prebake(
            TARD::ByPath(test_path),
            SoundGroup::BGM,
        ));
        let orchestra_player = Arc::clone(&orchestra);
        let cv_playback_started_send = Arc::new((bl::Mutex::new(false), bl::Condvar::new()));
        let cv_playback_started_recv = Arc::clone(&cv_playback_started_send);
        let fname = test_filename.clone();
        rts.spawn_blocking(move || {
            while !orchestra_player.is_registered(&fname) {
                println!("audio file not ready");
                sleep(Duration::from_secs(2));
            }
            println!("sending command from shitty closure to play");
            orchestra_player.send_cmd(AudioCommand::Play(TARD::Targeted(
                fname.to_string(),
                test_uuid,
            )));
            let (lock, cvar) = &*cv_playback_started_send;
            let mut playback_started = lock.lock();
            *playback_started = true;
            cvar.notify_one();
        });
        let cv_playback_paused_send = Arc::new((bl::Mutex::new(false), bl::Condvar::new()));
        let cv_playback_paused_recv = Arc::clone(&cv_playback_paused_send);
        let orchestra_pauser = Arc::clone(&orchestra);
        let fname = test_filename.clone();
        rts.spawn_blocking(move || {
            let (lock, cvar) = &*cv_playback_started_recv;
            cvar.wait_while(lock.lock().borrow_mut(), |&mut started| !started);
            sleep(Duration::from_secs(2));
            println!("sending command from shitty closure to pause");
            orchestra_pauser.send_cmd(AudioCommand::Pause(TARD::Targeted(
                fname.to_string(),
                test_uuid,
            )));
            let (lock, cvar) = &*cv_playback_paused_send;
            let mut playback_paused = lock.lock();
            *playback_paused = true;
            cvar.notify_one();
        });
        let orchestra_unpauser = Arc::clone(&orchestra);
        let cv_playback_unpaused_send = Arc::new((bl::Mutex::new(false), bl::Condvar::new()));
        let cv_playback_unpaused_recv = Arc::clone(&cv_playback_unpaused_send);
        let fname = test_filename.clone();
        rts.spawn_blocking(move || {
            let (lock, cvar) = &*cv_playback_paused_recv;
            cvar.wait_while(lock.lock().borrow_mut(), |&mut paused| !paused);
            sleep(Duration::from_secs(2));
            println!("sending command from shitty closure to unpause");
            orchestra_unpauser.send_cmd(AudioCommand::UnPause(TARD::Targeted(
                fname.to_string(),
                test_uuid,
            )));
            let (lock, cvar) = &*cv_playback_unpaused_send;
            let mut playback_unpaused = lock.lock();
            *playback_unpaused = true;
            cvar.notify_one();
        });
        let orchestra_stopper = Arc::clone(&orchestra);
        rts.spawn_blocking(move || {
            let (lock, cvar) = &*cv_playback_unpaused_recv;
            cvar.wait_while(lock.lock().borrow_mut(), |&mut unpaused| !unpaused);
            sleep(Duration::from_secs(2));
            println!("sending command from shitty closure to stop");
            orchestra_stopper.send_cmd(AudioCommand::Stop(TARD::Targeted(
                test_filename.to_string(),
                test_uuid,
            )));
        });
    }

    pub fn starting_cam_info(&self) -> CamInfo {
        let Definitions::SceneDefinition(definition) = &self.definition else {
            panic!("scene has non-scene definition")
        };
        definition
            .cameras
            .get(&definition.start_cam)
            .unwrap()
            .clone()
    }

    pub fn implement_chorus(&self, egui_ctx: Context, _orchestra: Arc<Orchestra>) {
        egui::Window::new("egui widget testing").show(&egui_ctx, |ui| {
            //
            ui.horizontal(|ui| {
                ui.add(KineticLabel::new("blabla"));
                ui.add(KineticLabel::new("same").kinesis(vec![KineticEffect::default()]));
                ui.add(
                    KineticLabel::new("line").kinesis(vec![KineticEffect::ShakeLetters {
                        params: ShakeLetters::default(),
                    }]),
                );
                ui.add(KineticLabel::new("still").kinesis(vec![KineticEffect::Gay {
                    params: Gay::default(),
                }]));
            });
            for (i, line) in self.test_lines.lines().enumerate() {
                ui.add(KineticLabel::new(line).kinesis(vec![self.random_line_effects[i].clone()]));
            }
        });

        egui::Window::new("markup test").show(&egui_ctx, |ui| {
            for pkl in &self.parsed_test_markup {
                ui.add(pkl.clone());
            }
        });
    }
}
impl HandlesInputContexts<MyInputContexts> for LinacLabScene {
    fn handle_input_for_context(
        &mut self,
        settings: &GameProgrammeSettings,
        state: &mut GameProgrammeState<MyInputContexts>,
        window: &Arc<Window>,
    ) {
        update_camera_rotation(state);
        let cur_camera = state.cur_camera.as_mut().unwrap();
        let rotation = cur_camera.rotation;
        let forward = -rotation.z_axis;
        let up = rotation.y_axis;
        let side = -rotation.x_axis;
        let buttons = &mut state.input_status.buttons;
        let cur_context = &state.cur_input_context;
        let mouse_physical_poz = &state.input_status.mouse_physical_poz;
        let handedness = settings.handedness;
        let really_pressed =
            |binding| Self::input_down(buttons, &self.keybindings, &binding).is_some_and(|k| k);
        let really_released =
            |binding| Self::input_up(buttons, &self.keybindings, &binding).is_some_and(|k| k);
        let interacted_with = |binding| really_pressed(binding) || really_released(binding);
        let wown: fn(LinacLabIC) -> MIC = MIC::LinacLabIC; // RA thinks this is an unused variable without the signature..
        let wdbg: fn(DebugInputContext) -> MIC = MIC::DebugInputContext;

        let (win_w, win_h) = window.inner_size().into();

        match cur_context {
            MyInputContexts::DebugInputContext(DIC::Marker) => {
                if really_released(wdbg(DIC::SwitchToScene)) {
                    if let Some(Implementations::SceneImplementation(ref si)) = self.implementation
                    {
                        if let Definitions::SceneDefinition(ref sd) = self.definition {
                            // reset camera to default from wherever we were in debug mode
                            state.cur_camera = Some(si.cameras[&sd.start_cam].clone()); // ehh this should have been a reference all along tbh TODO
                                                                                        // set input context to scene
                            state.cur_input_context = wown(LinacLabIC::Marker);
                            // ignore all of the other inputs since they mean something else in debug mode
                            state.input_status.buttons.clear();
                            return;
                        }
                    }
                }
                let velocity = if really_pressed(wdbg(DIC::Sprint)) {
                    settings.run_speed
                } else {
                    settings.walk_speed
                };
                let mut location = cur_camera.info.location();
                let last_update = state.last_update.unwrap();
                if really_pressed(wdbg(DIC::Forwards)) {
                    location += forward * velocity * last_update.elapsed().as_secs_f32();
                }
                if really_pressed(wdbg(DIC::Backwards)) {
                    location -= forward * velocity * last_update.elapsed().as_secs_f32();
                }
                if really_pressed(wdbg(DIC::StrafeLeft)) {
                    location += side * velocity * last_update.elapsed().as_secs_f32();
                }
                if really_pressed(wdbg(DIC::StrafeRight)) {
                    location -= side * velocity * last_update.elapsed().as_secs_f32();
                }
                if really_pressed(wdbg(DIC::LiftUp)) {
                    location += up * velocity * last_update.elapsed().as_secs_f32();
                }
                cur_camera
                    .info
                    .set_location(location.x, location.y, location.z);
                if really_released(wdbg(DIC::Interact)) {
                    let rayman = make_ray(
                        cur_camera,
                        &state.input_status.mouse_physical_poz,
                        win_w,
                        win_h,
                        settings.handedness,
                    );

                    if let Implementations::SceneImplementation(sc_imp) =
                        self.implementation.as_mut().unwrap()
                    {
                        const MAX_TOI: f32 = 100000.0;
                        if let AstinkScene::Loaded(stage3d) = &sc_imp.stage3d {
                            for (c_name, colliders) in stage3d.2 .2.col_map.iter() {
                                for c in colliders {
                                    if let Some(toi) = c.cast_local_ray(&rayman, MAX_TOI, true) {
                                        let intersection = rayman.point_at(toi);
                                        let cam_point = nalgebra::Point3::from([
                                            location.x, location.y, location.z,
                                        ]);
                                        if cam_point != intersection {
                                            println!(
                                                "{} intersects mouse ray at {}",
                                                c_name, intersection
                                            );
                                            #[cfg(feature = "extra_debugging")]
                                            {
                                                let renderer = state.renderer.clone().unwrap();
                                                crate::bl::theater::basement::debug_profiling_etc::draw_debug_mouse_picking_doodad(
                                                    crate::bl::theater::basement::debug_profiling_etc::DebugPickingDoodad::TheRay,
                                                    &cam_point,
                                                    &intersection,
                                                    &renderer,
                                                    settings.handedness,
                                                    c,
                                                );
                                                crate::bl::theater::basement::debug_profiling_etc::draw_debug_mouse_picking_doodad(
                                                    crate::bl::theater::basement::debug_profiling_etc::DebugPickingDoodad::TheColliderShape,
                                                    &cam_point,
                                                    &intersection,
                                                    &renderer,
                                                    settings.handedness,
                                                    c,
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                if really_released(wdbg(DIC::Back)) {
                    state.grabber.as_mut().unwrap().request_ungrab(window);
                }

                if really_released(wdbg(DIC::Profiling)) {
                    #[cfg(feature = "extra_debugging")]
                    crate::bl::theater::basement::debug_profiling_etc::write_profiling_json(
                        &state.previous_profiling_stats.as_ref(),
                    );
                }
                if interacted_with(wdbg(DIC::GrabWindow)) {
                    let grabber = state.grabber.as_mut().unwrap();

                    if !grabber.grabbed() {
                        grabber.request_grab(window);
                    }
                }
            }
            MyInputContexts::LinacLabIC(LinacLabIC::Marker) => {
                if really_released(wown(LinacLabIC::SwitchToDebug)) {
                    // set input context to debug
                    state.cur_input_context = wdbg(DIC::Marker);
                    // ignore all of the other inputs since they mean something else in debug mode
                    state.input_status.buttons.clear();
                    return;
                }
                if really_released(wown(LinacLabIC::FocusObject)) {
                    let Implementations::SceneImplementation(sc_imp) =
                        self.implementation.as_ref().unwrap()
                    else {
                        return;
                    };
                    let AstinkScene::Loaded(stage3d) = &sc_imp.stage3d else {
                        return;
                    };
                    let col_map = &stage3d.2 .2.col_map;
                    let collisions: CollisionMap = get_collisions_from_camera(
                        cur_camera,
                        mouse_physical_poz,
                        win_w,
                        win_h,
                        handedness,
                        col_map,
                    )
                    .iter()
                    .filter_map(|(k, v)| {
                        sc_imp
                            .cameras
                            .contains_key(k)
                            .then_some((k.to_owned(), v.to_owned()))
                    })
                    .collect();
                    debug!("collisions: {collisions:?}");
                    let Some((closest, _)) = collisions.iter().min_by(|c1, c2| {
                        let cam_point: nalgebra::Point3<f32> = cur_camera.info.location().into();
                        let min_c1 =
                            c1.1.iter()
                                .min_by(|cd1, cd2| {
                                    distance(cd1, &cam_point).total_cmp(&distance(cd2, &cam_point))
                                })
                                .unwrap();

                        debug!("min_c1 {min_c1}");
                        let min_c2 =
                            c2.1.iter()
                                .min_by(|cd1, cd2| {
                                    distance(cd1, &cam_point).total_cmp(&distance(cd2, &cam_point))
                                })
                                .unwrap();
                        debug!("min_c2 {min_c2}");
                        distance(min_c1, &cam_point).total_cmp(&distance(min_c2, &cam_point))
                    }) else {
                        buttons.retain(|_, v| v.is_pressed());
                        return;
                    };

                    state.cur_camera = Some(sc_imp.cameras[closest].clone());
                }
                if really_released(wown(LinacLabIC::Back)) {
                    let Implementations::SceneImplementation(sc_imp) =
                        self.implementation.as_ref().unwrap()
                    else {
                        return;
                    };
                    let Definitions::SceneDefinition(ref sd) = self.definition else {
                        return;
                    };
                    state.cur_camera = Some(sc_imp.cameras[&sd.start_cam].clone());
                }
            }
            _ => {}
        }
        buttons.retain(|_, v| v.is_pressed());
    }
}
