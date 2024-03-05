#![feature(async_closure, variant_count)]
pub mod curtain;
pub mod linac_lab;

use std::{
    collections::HashMap,
    mem::{take, variant_count},
    ops::{Deref, DerefMut},
};

use bl::{
    macros::Playable,
    the_great_mind_palace_of_theatrical_arts::{
        basement::input_handling::{DebugInputContext, InputContext},
        play::{scene::Scenic, Play, Playable},
    },
    theater::basement::logging::register_logger,
    winit::window::{Fullscreen, WindowBuilder},
    GameProgramme,
};
use brainworms_lib as bl;
use curtain::Curtain;
use linac_lab::{LinacLabIC, LinacLabScene};
#[derive(Default, Debug)]
pub struct BrainwormsData;
use bl::into_variant::{self as into_variant, IntoVariant, VariantFrom};
#[derive(Playable, VariantFrom)]
#[input_context_enum(MyInputContexts)]
#[user_data_struct(BrainwormsData)]
pub enum MyPlayables {
    LinacLabScene(LinacLabScene),
    Curtain(Curtain), // loading screens and menus
}

#[derive(Default, Hash, Eq, PartialEq, Debug, Copy, Clone)]
pub enum MyInputContexts {
    DebugInputContext(DebugInputContext),
    LinacLabIC(LinacLabIC),
    #[default]
    Pause,
}
impl InputContext for MyInputContexts {}

pub fn define_play() -> Play<MyPlayables> {
    let mut all_playables: Vec<MyPlayables> = vec![
        Curtain::default().into_variant(),
        LinacLabScene::default().into_variant(),
    ];

    let mut playable_names = HashMap::new();
    let mut playables = HashMap::new();
    while let Some(mut p) = all_playables.pop() {
        p.define_playable();
        playable_names.insert(p.playable_name().to_owned(), p.playable_uuid());
        playables.insert(p.playable_uuid(), p);
    }

    //    let first_playable = playable_names["curtain"];
    let first_playable = playable_names["LinacLab"];
    Play {
        first_playable,
        playables,
        playable_names,
    }
}

#[cfg_attr(
    target_os = "android",
    ndk_glue::main(backtrace = "on", logger(level = "debug"))
)]
fn main() {
    let window_builder = WindowBuilder::new()
        .with_title("Therac3D")
        .with_maximized(true)
        .with_fullscreen(Some(Fullscreen::Borderless(None)))
        .with_decorations(false);
    register_logger();
    let play = define_play();
    let mut the_game_programme = GameProgramme::new(play);
    the_game_programme.state.cur_input_context =
        MyInputContexts::DebugInputContext(DebugInputContext::Marker);
    the_game_programme.start(window_builder);
}
