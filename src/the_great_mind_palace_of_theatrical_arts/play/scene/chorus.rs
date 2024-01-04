use egui::{ahash::HashMap, Context};

use crate::{
    theater::play::backstage::pyrotechnics::kinetic_narrative::{
        Gay, KineticEffect, KineticLabel, ShakeLetters,
    },
    GameProgrammeData,
};

pub trait Choral {
    fn sing(&self, egui_ctx: &mut Context, data: &GameProgrammeData) {
        egui::Window::new("egui widget testing").show(egui_ctx, |ui| {
            ui.label(std::format!("framerate: {:.0}fps", data.frame_rate.get()));
            ui.horizontal(|ui| {
                ui.add(KineticLabel::new("blabla"));
                ui.add(KineticLabel::new("same").kinesis(vec![&KineticEffect::default()]));
                ui.add(
                    KineticLabel::new("line").kinesis(vec![&KineticEffect::ShakeLetters {
                        params: ShakeLetters::default(),
                    }]),
                );
                ui.add(
                    KineticLabel::new("still").kinesis(vec![&KineticEffect::Gay {
                        params: Gay::default(),
                    }]),
                );
            });
            for (i, line) in data.test_lines.lines().enumerate() {
                ui.add(KineticLabel::new(line).kinesis(vec![&data.random_line_effects[i]]));
            }
        });
    }
}
