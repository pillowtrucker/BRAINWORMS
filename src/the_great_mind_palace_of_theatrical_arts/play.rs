use std::{collections::HashMap, sync::Arc};

use egui::Context;
use enum_dispatch::enum_dispatch;
use rend3::Renderer;
use tokio::runtime::Runtime;
use uuid::Uuid;
use winit::event_loop::EventLoop;

use crate::{GameProgrammeData, MyEvent};

use self::{
    backstage::plumbing::DefaultRoutines,
    scene::{definitions::linac_lab::LinacLabScene, CamInfo, SceneDefinition, SceneImplementation},
};

use super::basement::cla::GameProgrammeSettings;

pub mod backstage;
pub mod definition;
pub mod orchestra;
pub mod scene;

pub struct Play {
    pub first_playable: Uuid,
    pub playables: HashMap<Uuid, Playables>,
    pub playable_names: HashMap<String, Uuid>,
}
#[enum_dispatch]
pub enum Playables {
    LinacLabScene,
    //    Curtain,   // loading screens
    //    TicketBox, // menus
}
#[enum_dispatch(Playables)]
pub trait Playable {
    fn playable_uuid(&self) -> Uuid;
    fn playable_name(&self) -> &str;
    fn playable_definition(&mut self) -> &mut Definitions;
    fn playable_implementation(&mut self) -> &mut Option<Implementations>;
    fn starting_cam_info(&self) -> CamInfo;
    fn implement_playable(
        &mut self,
        settings: &GameProgrammeSettings,
        event_loop: &EventLoop<MyEvent>,
        renderer: Arc<Renderer>,
        routines: Arc<DefaultRoutines>,
        rts: &Runtime,
    );
    fn define_playable(&mut self);
    fn implement_chorus_for_playable(&self, egui_ctx: &mut Context, data: &GameProgrammeData);
}
pub enum Definitions {
    SceneDefinition(SceneDefinition),
}

impl Default for Definitions {
    fn default() -> Self {
        Definitions::SceneDefinition(SceneDefinition::default())
    }
}
pub enum Implementations {
    SceneImplementation(SceneImplementation),
}
