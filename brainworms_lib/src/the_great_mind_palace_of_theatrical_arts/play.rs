use std::{collections::HashMap, sync::Arc};

use brainworms_arson::egui;
use egui::Context;
use enum_dispatch::enum_dispatch;
use parking_lot::Mutex;
use rend3::Renderer;
use tokio::runtime::Runtime;
use uuid::Uuid;
use winit::event_loop::EventLoop;
use winit::window::Window;

use crate::{
    theater::basement::input_handling::InputContext, GameProgrammeData, GameProgrammeState, MyEvent,
};

use self::{
    backstage::plumbing::DefaultRoutines,
    orchestra::Orchestra,
    scene::{chorus::Choral, CamInfo, SceneDefinition, SceneImplementation, Scenic},
};

use super::basement::{cla::GameProgrammeSettings, input_handling::HandlesInputContexts};

pub mod backstage;
pub mod definition;
pub mod orchestra;
pub mod scene;
#[derive(Default)]
pub struct Play<PlayablesEnum> {
    pub first_playable: Uuid,
    pub playables: HashMap<Uuid, PlayablesEnum>,
    pub playable_names: HashMap<String, Uuid>,
}

#[enum_dispatch]
pub trait Playable<InputContextEnum: InputContext, PlayablesEnum> {
    fn playable_uuid(&self) -> Uuid;
    fn playable_name(&self) -> &str;
    fn playable_definition(&mut self) -> &mut Definitions;
    fn playable_implementation(&mut self) -> &mut Option<Implementations>;
    fn starting_cam_info(&self) -> CamInfo;
    fn implement_playable(
        &mut self,
        game_settings: Arc<Mutex<GameProgrammeSettings>>,
        game_state: Arc<Mutex<GameProgrammeState<InputContextEnum>>>,
        game_data: Arc<Mutex<GameProgrammeData<PlayablesEnum>>>,
        rts: Arc<Mutex<Option<Runtime>>>,
    );
    fn define_playable(&mut self);
    fn implement_chorus_for_playable(
        &mut self,
        game_settings: Arc<Mutex<GameProgrammeSettings>>,
        game_state: Arc<Mutex<GameProgrammeState<InputContextEnum>>>,
        game_data: Arc<Mutex<GameProgrammeData<PlayablesEnum>>>,
        rts: Arc<Mutex<Option<Runtime>>>,
    );
    //    fn get_current_input_context(&self) -> &InputContext<TO>;
    fn handle_input_for_playable(
        &mut self,
        game_settings: Arc<Mutex<GameProgrammeSettings>>,
        game_state: Arc<Mutex<GameProgrammeState<InputContextEnum>>>,
        game_data: Arc<Mutex<GameProgrammeData<PlayablesEnum>>>,
        rts: Arc<Mutex<Option<Runtime>>>,
    );
}
#[derive(Debug)]
pub enum Definitions {
    SceneDefinition(SceneDefinition),
    BogusDefinition, // because fucking clippy that's why
}

impl Default for Definitions {
    fn default() -> Self {
        Definitions::SceneDefinition(SceneDefinition::default())
    }
}

#[allow(clippy::large_enum_variant)]
#[derive(Default)]
pub enum Implementations {
    SceneImplementation(SceneImplementation),
    #[default]
    BogusImplementation,
}

impl<
        InputContextEnum: InputContext,
        PlayablesEnum: Playable<InputContextEnum, PlayablesEnum>,
        T: Scenic<InputContextEnum, PlayablesEnum>
            + Choral<InputContextEnum, PlayablesEnum>
            + HandlesInputContexts<InputContextEnum, PlayablesEnum>,
    > Playable<InputContextEnum, PlayablesEnum> for T
{
    fn playable_uuid(&self) -> Uuid {
        self.scene_uuid()
    }

    fn playable_name(&self) -> &str {
        self.scene_name()
    }

    fn starting_cam_info(&self) -> CamInfo {
        self.scene_starting_cam_info()
    }

    fn implement_playable(
        &mut self,
        game_settings: Arc<Mutex<GameProgrammeSettings>>,
        game_state: Arc<Mutex<GameProgrammeState<InputContextEnum>>>,
        game_data: Arc<Mutex<GameProgrammeData<PlayablesEnum>>>,
        rts: Arc<Mutex<Option<Runtime>>>,
    ) {
        self.implement_scene(game_settings, game_state, game_data, rts)
    }

    fn define_playable(&mut self) {
        self.define_scene()
    }
    fn implement_chorus_for_playable(
        &mut self,
        game_settings: Arc<Mutex<GameProgrammeSettings>>,
        game_state: Arc<Mutex<GameProgrammeState<InputContextEnum>>>,
        game_data: Arc<Mutex<GameProgrammeData<PlayablesEnum>>>,
        rts: Arc<Mutex<Option<Runtime>>>,
    ) {
        self.implement_chorus_for_choral(game_settings, game_state, game_data, rts);
    }

    fn playable_definition(&mut self) -> &mut Definitions {
        self.raw_definition()
    }

    fn playable_implementation(&mut self) -> &mut Option<Implementations> {
        self.raw_implementation()
    }

    fn handle_input_for_playable(
        &mut self,
        game_settings: Arc<Mutex<GameProgrammeSettings>>,
        game_state: Arc<Mutex<GameProgrammeState<InputContextEnum>>>,
        game_data: Arc<Mutex<GameProgrammeData<PlayablesEnum>>>,
        rts: Arc<Mutex<Option<Runtime>>>,
    ) {
        self.handle_input_for_context(game_settings, game_state, game_data, rts)
    }
}
