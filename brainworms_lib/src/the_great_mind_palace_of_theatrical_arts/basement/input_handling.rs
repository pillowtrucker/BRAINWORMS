use std::{collections::HashMap, sync::Arc};

use glam::DVec2;
use log::info;
use winit::{
    dpi::PhysicalPosition,
    event::{ElementState, MouseButton},
    keyboard::KeyCode,
    window::Window,
};

use crate::{GameProgrammeSettings, GameProgrammeState};

pub type KeyStates = HashMap<AcceptedInput, ElementState>;
pub type KeyBindings<TO> = HashMap<TO, AcceptedInput>;

#[derive(Debug, Hash, Eq, PartialEq)]
pub enum DebugInputContext {
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
    Marker,
}
pub trait InputContext:
    std::default::Default + std::fmt::Debug + std::hash::Hash + Eq + PartialEq
{
}

#[derive(Debug, Eq, PartialEq, Hash)]
pub enum AcceptedInput {
    KB(KeyCode),
    M(MouseButton),
}

// Ideally I want this to only mess with its own data but for now let's just reproduce the existing behaviour
//#[enum_dispatch(Playables)]
pub trait HandlesInputContexts<InputContextEnum: InputContext> {
    fn handle_input_for_context(
        &mut self,
        settings: &GameProgrammeSettings,
        state: &mut GameProgrammeState<InputContextEnum>,
        window: &Arc<Window>,
    );
    fn key_down(input_status: &KeyStates, the_input: &AcceptedInput) -> Option<bool> {
        Self::key_is_state(input_status, the_input, &ElementState::Pressed)
    }
    fn key_up(input_status: &KeyStates, the_input: &AcceptedInput) -> Option<bool> {
        Self::key_is_state(input_status, the_input, &ElementState::Released)
    }
    fn key_is_state(
        input_status: &KeyStates,
        the_input: &AcceptedInput,
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
    fn input_down(
        input_status: &KeyStates,
        keybindings: &KeyBindings<InputContextEnum>,
        binding: &InputContextEnum,
    ) -> Option<bool> {
        match keybindings.get(binding) {
            Some(the_key) => Self::key_down(input_status, the_key),
            None => {
                info!("No binding for {:?}", binding);
                None
            }
        }
    }
    fn input_up(
        input_status: &KeyStates,
        keybindings: &KeyBindings<InputContextEnum>,
        binding: &InputContextEnum,
    ) -> Option<bool> {
        match keybindings.get(binding) {
            Some(the_key) => Self::key_up(input_status, the_key),
            None => {
                info!("No binding for {:?}", binding);
                None
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
