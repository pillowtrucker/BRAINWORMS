pub mod linac_lab;

use std::{collections::HashMap, sync::Arc};

use bl::{
    egui::Context,
    proc_macros::Playable,
    rend3::Renderer,
    the_great_mind_palace_of_theatrical_arts::{
        basement::{
            cla::GameProgrammeSettings,
            input_handling::{DebugInputContext, InputContext},
        },
        play::{
            backstage::plumbing::DefaultRoutines, scene::CamInfo, scene::Scenic, Definitions,
            Implementations, Play, Playable,
        },
    },
    theater::basement::logging::register_logger,
    tokio::runtime::Runtime,
    uuid::Uuid,
    winit::{
        event_loop::EventLoop,
        window::{Fullscreen, Window, WindowBuilder},
    },
    GameProgramme, GameProgrammeState, MyEvent,
};
use brainworms_lib as bl;
use linac_lab::{LinacLabIC, LinacLabScene};

#[bl::enum_dispatch::enum_dispatch(Playable)] // this doesnt work across crates but it does generate at least the from and into stuff
#[derive(Playable)]
#[input_context_enum(MyInputContexts)]
pub enum MyPlayables {
    LinacLabScene(LinacLabScene),
    //    Curtain,   // loading screens
    //    TicketBox, // menus
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
    let mut linac_lab_scene = LinacLabScene::default();
    linac_lab_scene.define_scene();
    let mut playables = HashMap::new();
    let mut playable_names = HashMap::new();
    playable_names.insert(linac_lab_scene.name.clone(), linac_lab_scene.uuid);
    let first_playable = linac_lab_scene.uuid;
    playables.insert(
        linac_lab_scene.uuid,
        MyPlayables::LinacLabScene(linac_lab_scene),
    );

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
