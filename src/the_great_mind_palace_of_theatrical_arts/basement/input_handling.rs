use std::collections::HashMap;

use glam::{vec3, vec4};
use log::info;
use parry3d::query::{Ray, RayCast};
use winit::{
    event::{ElementState, MouseButton},
    keyboard::KeyCode,
    window::Window,
};

use self::LogicalInputBinding as LIB;
use crate::{
    theater::play::{
        scene::{
            stage3d::{compute_projection_matrix, make_camera},
            AstinkScene,
        },
        Implementations, Playable,
    },
    GameProgrammeData, GameProgrammeSettings,
};

pub type KeyStates = HashMap<AcceptedInputs, ElementState>;
pub type KeyBindings = HashMap<LIB, AcceptedInputs>;

fn key_down(input_status: &KeyStates, the_input: &AcceptedInputs) -> Option<bool> {
    key_is_state(input_status, the_input, &ElementState::Pressed)
}
fn key_up(input_status: &KeyStates, the_input: &AcceptedInputs) -> Option<bool> {
    key_is_state(input_status, the_input, &ElementState::Released)
}
fn key_is_state(
    input_status: &KeyStates,
    the_input: &AcceptedInputs,
    state: &ElementState,
) -> Option<bool> {
    let want_pressed = match state {
        ElementState::Pressed => true,
        ElementState::Released => false,
    };
    input_status
        .get(the_input)
        .map(|k| k.is_pressed() && want_pressed || !k.is_pressed() && !want_pressed)
}
fn input_down(input_status: &KeyStates, keybindings: &KeyBindings, binding: &LIB) -> Option<bool> {
    match keybindings.get(binding) {
        Some(the_key) => key_down(input_status, the_key),
        None => {
            info!("No binding for {:?}", binding);
            None
        }
    }
}
fn input_up(input_status: &KeyStates, keybindings: &KeyBindings, binding: &LIB) -> Option<bool> {
    match keybindings.get(binding) {
        Some(the_key) => key_up(input_status, the_key),
        None => {
            info!("No binding for {:?}", binding);
            None
        }
    }
}
pub fn handle_input(
    settings: &mut GameProgrammeSettings,
    data: &mut GameProgrammeData,
    window: &Window,
) {
    let forward = -settings.rotation.z_axis;
    let up = settings.rotation.y_axis;
    let side = -settings.rotation.x_axis;
    let really_pressed = |binding| {
        input_down(&settings.input_status, &settings.keybindings, binding).is_some_and(|k| k)
    };
    let really_released = |binding| {
        input_up(&settings.input_status, &settings.keybindings, binding).is_some_and(|k| k)
    };
    let interacted_with = |binding| really_pressed(binding) || really_released(binding);
    let velocity = if really_pressed(&LIB::Sprint) {
        settings.run_speed
    } else {
        settings.walk_speed
    };
    if really_pressed(&LIB::Forwards) {
        settings.camera_location += forward * velocity * data.last_update.elapsed().as_secs_f32();
    }
    if really_pressed(&LIB::Backwards) {
        settings.camera_location -= forward * velocity * data.last_update.elapsed().as_secs_f32();
    }
    if really_pressed(&LIB::StrafeLeft) {
        settings.camera_location += side * velocity * data.last_update.elapsed().as_secs_f32();
    }
    if really_pressed(&LIB::StrafeRight) {
        settings.camera_location -= side * velocity * data.last_update.elapsed().as_secs_f32();
    }
    if really_pressed(&LIB::LiftUp) {
        settings.camera_location += up * velocity * data.last_update.elapsed().as_secs_f32();
    }
    if really_released(&LIB::Interact) {
        let rayman = make_ray(settings, data, window);

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
                for (c_name, colliders) in stage3d.2 .2.col_map.iter() {
                    for c in colliders {
                        if let Some(toi) = c.cast_local_ray(&rayman, MAX_TOI, true) {
                            let intersection = rayman.point_at(toi);
                            let cam_point = nalgebra::Point3::from([
                                settings.camera_location.x,
                                settings.camera_location.y,
                                settings.camera_location.z,
                            ]);
                            if cam_point != intersection {
                                println!("{} intersects mouse ray at {}", c_name, intersection);
                                #[cfg(extra_debugging)]
                                {
                                    theater::basement::debug_profiling_etc::draw_debug_mouse_picking_doodad(
                                                    theater::basement::debug_profiling_etc::DebugPickingDoodad::TheRay,
                                                    &cam_point,
                                                    &intersection,
                                                    &renderer,
                                                    Self::HANDEDNESS,
                                                    c,
                                                );
                                    theater::basement::debug_profiling_etc::draw_debug_mouse_picking_doodad(
                                                    theater::basement::debug_profiling_etc::DebugPickingDoodad::TheColliderShape,
                                                    &cam_point,
                                                    &intersection,
                                                    &renderer,
                                                    Self::HANDEDNESS,
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

    if really_released(&LIB::Back) {
        settings.grabber.as_mut().unwrap().request_ungrab(window);
    }

    if really_released(&LIB::DebugProfiling) {
        #[cfg(extra_debugging)]
        theater::basement::debug_profiling_etc::write_profiling_json(&self.settings);
    }
    if interacted_with(&LIB::GrabWindow) {
        let grabber = settings.grabber.as_mut().unwrap();

        if !grabber.grabbed() {
            grabber.request_grab(window);
        }
    }
    for (k, v) in settings.input_status.iter() {
        if !v.is_pressed() {
            settings.input_status.remove(k).unwrap();
        }
    }
}
#[derive(Debug, Hash, Eq, PartialEq)]
pub enum LogicalInputBinding {
    Sprint,
    Forwards,
    Backwards,
    StrafeLeft,
    StrafeRight,
    LiftUp,
    Interact,
    Back,
    DebugProfiling,
    GrabWindow,
}
fn make_ray(settings: &GameProgrammeSettings, data: &GameProgrammeData, window: &Window) -> Ray {
    let cam_x = settings.camera_location.x;
    let cam_y = settings.camera_location.y;
    let cam_z = settings.camera_location.z;
    let cam_pitch = settings.camera_pitch;
    let cam_yaw = settings.camera_yaw;
    info!("{cam_x},{cam_y},{cam_z},{cam_pitch},{cam_yaw}",);
    info!(
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
    info!("ray_nds: {ray_nds}");
    let ray_clip = vec4(ray_nds.x, ray_nds.y, -1.0, 1.0);
    let cur_camera = make_camera(("".to_owned(), [cam_x, cam_y, cam_z, cam_pitch, cam_yaw]));
    let ray_eye = compute_projection_matrix(
        cur_camera.renderer_camera,
        settings.handedness,
        (win_w / win_h) as f32,
    )
    .inverse()
        * ray_clip;
    let ray_eye = vec4(ray_eye.x, ray_eye.y, -1.0, 0.0);
    info!("ray_eye: {ray_eye}");
    let ray_wor4 = cur_camera.renderer_camera.view.inverse() * ray_eye;
    let ray_wor = vec3(ray_wor4.x, ray_wor4.y, ray_wor4.z);
    let ray_wor = ray_wor.normalize();
    info!("ray_world: {ray_wor}");
    Ray::new(nalgebra::Point3::new(cam_x, cam_y, cam_z), ray_wor.into())
}
#[derive(Debug, Eq, PartialEq, Hash)]
pub enum AcceptedInputs {
    KB(KeyCode),
    M(MouseButton),
}
