use std::sync::Arc;

use brainworms_arson::egui;
use egui::Context;
use parking_lot::Mutex;
use tokio::runtime::Runtime;
use uuid::Uuid;

use crate::{
    theater::{
        basement::{cla::GameProgrammeSettings, input_handling::InputContext},
        play::{orchestra::Orchestra, Definitions, Implementations},
    },
    GameProgrammeData, GameProgrammeState,
};

pub trait Choral<InputContextEnum: InputContext, PlayablesEnum> {
    fn implement_chorus_for_choral(
        &mut self,
        game_settings: Arc<Mutex<GameProgrammeSettings>>,
        game_state: Arc<Mutex<GameProgrammeState<InputContextEnum>>>,
        game_data: Arc<Mutex<GameProgrammeData<PlayablesEnum>>>,
        rts: Arc<Mutex<Option<Runtime>>>,
    );
    fn chorus_uuid(&self) -> Uuid;
    fn chorus_name(&self) -> &str;
    fn chorus_definition(&mut self) -> &mut Definitions;
    fn chorus_implementation(&mut self) -> &mut Option<Implementations>;
    fn define_chorus(&mut self);
}
