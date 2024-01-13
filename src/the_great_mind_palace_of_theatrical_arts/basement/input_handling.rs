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
pub type DebugKeyBindings = HashMap<DebugCameraInputBinding, AcceptedInputs>;

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
pub fn input_down<TO: AmBindings>(
    input_status: &KeyStates,
    keybindings: &HashMap<TO, AcceptedInputs>,
    binding: &TO,
) -> Option<bool> {
    match keybindings.get(binding) {
        Some(the_key) => key_down(input_status, the_key),
        None => {
            info!("No binding for {:?}", binding);
            None
        }
    }
}
pub fn input_up<TO: AmBindings>(
    input_status: &KeyStates,
    keybindings: &HashMap<TO, AcceptedInputs>,
    binding: &TO,
) -> Option<bool> {
    match keybindings.get(binding) {
        Some(the_key) => key_up(input_status, the_key),
        None => {
            info!("No binding for {:?}", binding);
            None
        }
    }
}
/* maybe I'll add this later but it's just too much pointless indirection
pub enum PadlikeBinding {
    L1,
    L2,
    L3,
    R1,
    R2,
    R3,
    DUp,
    DDown,
    DLeft,
    DRight,
    Square,
    Triangle,
    Circle,
    Cross,
    Start,
    Select,
    GlobalMenu,
}
*/
#[derive(Debug, Hash, Eq, PartialEq)]
pub enum DebugCameraInputBinding {
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
pub trait AmBindings: std::fmt::Debug + std::hash::Hash + Eq + PartialEq {}
impl AmBindings for DebugCameraInputBinding {}
pub trait MapsBindings<T> {
    fn map_to_logical(padlike_binding: AcceptedInputs) -> T;
    fn map_from_logical(logical_binding: T) -> AcceptedInputs;
}

#[derive(Debug, Eq, PartialEq, Hash)]
pub enum AcceptedInputs {
    KB(KeyCode),
    M(MouseButton),
}

pub enum InputContexts<T: AmBindings> {
    Debug,
    Scene(T),
    //    Pause,
}

// Ideally I want this to only mess with its own data but for now let's just reproduce the existing behaviour
//#[enum_dispatch(Playables)]
pub trait HandlesInputContexts<TO> {
    fn handle_input_for_debug_context(
        &mut self,
        settings: &GameProgrammeSettings,
        state: &mut GameProgrammeState,
        window: &Arc<Window>,
    );
    fn handle_input_for_own_context(
        &mut self,
        settings: &GameProgrammeSettings,
        state: &mut GameProgrammeState,
        window: &Arc<Window>,
        own_logical_bindings: TO,
    );
}
pub(crate) trait WrappedHandlesInput<T: AmBindings> {
    fn handle_input_wrapped(
        &mut self,
        settings: &GameProgrammeSettings,
        state: &mut GameProgrammeState,
        window: &Arc<Window>,
        context: InputContexts<T>,
    );
}
impl<T: HandlesInputContexts<TO> + MapsBindings<TO>, TO: AmBindings> WrappedHandlesInput<TO> for T {
    fn handle_input_wrapped(
        &mut self,
        settings: &GameProgrammeSettings,
        state: &mut GameProgrammeState,
        window: &Arc<Window>,
        context: InputContexts<TO>,
    ) {
        match context {
            InputContexts::Debug => self.handle_input_for_debug_context(settings, state, window),
            InputContexts::Scene(own_local_bindings) => {
                self.handle_input_for_own_context(settings, state, window, own_local_bindings)
            }
        }
    }
}
#[derive(Debug, Default)]
pub struct InputStatus {
    pub buttons: KeyStates,
    pub last_mouse_delta: Option<DVec2>,
    pub mouse_physical_poz: PhysicalPosition<f64>,
}
