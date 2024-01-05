use egui::Context;

use crate::GameProgrammeData;

pub trait Choral {
    fn implement_chorus_for_choral(&self, egui_ctx: &mut Context, data: &GameProgrammeData);
}
