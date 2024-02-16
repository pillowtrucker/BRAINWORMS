use std::sync::Arc;

use crate::{BrainwormsData, MyInputContexts};

use bl::egui::Context;

use bl::parking_lot::Mutex;
use bl::the_great_mind_palace_of_theatrical_arts::basement::cla::GameProgrammeSettings;
use bl::theater::play::orchestra::Orchestra;
use bl::theater::play::{Definitions, Implementations};
use bl::{
    macros::{Choral, Playable},
    uuid::Uuid,
};
use brainworms_lib as bl;
#[derive(Default, Choral, Playable)]
#[input_context_enum(MyInputContexts)]
#[user_data_struct(BrainwormsData)]
pub struct Curtain {
    pub uuid: Uuid,
    pub name: String,
    pub definition: Definitions,
    pub implementation: Option<Implementations>,
}
impl Curtain {
    fn implement_chorus(
        &self,
        egui_ctx: Context,
        orchestra: Arc<Orchestra>,
        settings: &GameProgrammeSettings,
        user_data: Arc<Mutex<BrainwormsData>>,
    ) {
        todo!()
        //        egui::Window::new("ok").fixed_size(size)
    }
    fn define(&mut self) {
        todo!()
    }
}
