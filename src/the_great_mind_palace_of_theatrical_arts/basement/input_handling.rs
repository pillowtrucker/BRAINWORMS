use std::{collections::HashMap, sync::Arc};

use enum_dispatch::enum_dispatch;
use glam::{vec3, vec4, DVec2};
use log::info;
use parry3d::query::{Ray, RayCast};
use winit::{
    dpi::PhysicalPosition,
    event::{ElementState, MouseButton},
    keyboard::KeyCode,
    window::Window,
};

use self::LogicalInputBinding as LIB;
use crate::{
    theater::play::{
        scene::{
            definitions::linac_lab::LinacLabScene,
            stage3d::{compute_projection_matrix, make_camera, update_camera_rotation},
            AstinkScene, Scenic,
        },
        Implementations, Playable, Playables,
    },
    GameProgrammeData, GameProgrammeSettings, GameProgrammeState,
};

pub type KeyStates = HashMap<AcceptedInputs, ElementState>;
pub type KeyBindings = HashMap<LIB, AcceptedInputs>;

pub fn key_down(input_status: &KeyStates, the_input: &AcceptedInputs) -> Option<bool> {
    key_is_state(input_status, the_input, &ElementState::Pressed)
}
pub fn key_up(input_status: &KeyStates, the_input: &AcceptedInputs) -> Option<bool> {
    key_is_state(input_status, the_input, &ElementState::Released)
}
pub fn key_is_state(
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
pub fn input_down(
    input_status: &KeyStates,
    keybindings: &KeyBindings,
    binding: &LIB,
) -> Option<bool> {
    match keybindings.get(binding) {
        Some(the_key) => key_down(input_status, the_key),
        None => {
            info!("No binding for {:?}", binding);
            None
        }
    }
}
pub fn input_up(
    input_status: &KeyStates,
    keybindings: &KeyBindings,
    binding: &LIB,
) -> Option<bool> {
    match keybindings.get(binding) {
        Some(the_key) => key_up(input_status, the_key),
        None => {
            info!("No binding for {:?}", binding);
            None
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

#[derive(Debug, Eq, PartialEq, Hash)]
pub enum AcceptedInputs {
    KB(KeyCode),
    M(MouseButton),
}

pub enum InputContexts {
    DebugCamera,
    SceneOverview,
    PauseScreen,
    MainMenu,
}

// Ideally I want this to only mess with its own data but for now let's just reproduce the existing behaviour
//#[enum_dispatch(Playables)]
pub trait InputContext {
    fn handle_input_for_context(
        &mut self,
        settings: &GameProgrammeSettings,
        state: &mut GameProgrammeState,
        window: &Arc<Window>,
    );
}
#[derive(Debug, Default)]
pub struct InputStatus {
    pub buttons: KeyStates,
    pub last_mouse_delta: Option<DVec2>,
    pub mouse_physical_poz: PhysicalPosition<f64>,
}
