use egui::Context;

pub trait Choral {
    fn implement_chorus_for_choral(&self, egui_ctx: Context);
}
