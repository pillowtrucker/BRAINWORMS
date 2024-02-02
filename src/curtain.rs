use std::sync::Arc;

use crate::MyInputContexts;

use bl::egui::Context;

use bl::theater::play::orchestra::Orchestra;
use bl::theater::play::{Definitions, Implementations};
use bl::{
    macros::{Choral, Playable},
    uuid::Uuid,
};
use brainworms_lib as bl;
#[derive(Default, Choral, Playable)]
#[input_context_enum(MyInputContexts)]
pub struct Curtain {
    pub uuid: Uuid,
    pub name: String,
    pub definition: Definitions,
    pub implementation: Option<Implementations>,
}
impl Curtain {
    fn implement_chorus(&self, egui_ctx: Context, orchestra: Arc<Orchestra>) {
        todo!()
        //        egui::Window::new("ok").fixed_size(size)
    }
    fn define(&mut self) {
        todo!()
    }
}
