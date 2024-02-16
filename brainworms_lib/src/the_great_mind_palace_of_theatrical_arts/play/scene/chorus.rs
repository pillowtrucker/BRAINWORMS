use std::sync::Arc;

use brainworms_arson::egui;
use egui::Context;
use parking_lot::Mutex;
use uuid::Uuid;

use crate::theater::{
    basement::cla::GameProgrammeSettings,
    play::{orchestra::Orchestra, Definitions, Implementations},
};

pub trait Choral<UserData> {
    fn implement_chorus_for_choral(
        &self,
        egui_ctx: Context,
        orchestra: Arc<Orchestra>,
        settings: &GameProgrammeSettings,
        user_data: Arc<Mutex<UserData>>,
    );
    fn chorus_uuid(&self) -> Uuid;
    fn chorus_name(&self) -> &str;
    fn chorus_definition(&mut self) -> &mut Definitions;
    fn chorus_implementation(&mut self) -> &mut Option<Implementations>;
    fn define_chorus(&mut self);
}
