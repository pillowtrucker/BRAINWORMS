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
pub enum MyPlayables {
    #[input_context_enum(MyInputContexts)]
    LinacLabScene(LinacLabScene),
    //    Curtain,   // loading screens
    //    TicketBox, // menus
}
/*
impl Playable<MyInputContexts> for MyPlayables {
    fn playable_uuid(&self) -> Uuid {
        match self {
            MyPlayables::LinacLabScene(inner) => inner.playable_uuid(),
        }
    }

    fn playable_name(&self) -> &str {
        match self {
            MyPlayables::LinacLabScene(inner) => inner.playable_name(),
        }
    }

    fn starting_cam_info(&self) -> CamInfo {
        match self {
            MyPlayables::LinacLabScene(inner) => inner.starting_cam_info(),
        }
    }

    fn implement_playable(
        &mut self,
        settings: &GameProgrammeSettings,
        event_loop: &EventLoop<MyEvent>,
        renderer: Arc<Renderer>,
        routines: Arc<DefaultRoutines>,
        rts: &Runtime,
    ) {
        match self {
            MyPlayables::LinacLabScene(inner) => {
                inner.implement_playable(settings, event_loop, renderer, routines, rts)
            }
        }
    }

    fn define_playable(&mut self) {
        match self {
            MyPlayables::LinacLabScene(inner) => inner.define_playable(),
        }
    }
    fn implement_chorus_for_playable(&self, egui_ctx: Context) {
        match self {
            MyPlayables::LinacLabScene(inner) => inner.implement_chorus_for_playable(egui_ctx),
        }
    }

    fn playable_definition(&mut self) -> &mut Definitions {
        match self {
            MyPlayables::LinacLabScene(inner) => inner.playable_definition(),
        }
    }

    fn playable_implementation(&mut self) -> &mut Option<Implementations> {
        match self {
            MyPlayables::LinacLabScene(inner) => inner.playable_implementation(),
        }
    }

    fn handle_input_for_playable(
        &mut self,
        settings: &GameProgrammeSettings,
        state: &mut GameProgrammeState<MyInputContexts>,
        window: &Arc<Window>,
    ) {
        match self {
            MyPlayables::LinacLabScene(inner) => {
                inner.handle_input_for_playable(settings, state, window)
            }
        }
    }
}
*/
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
