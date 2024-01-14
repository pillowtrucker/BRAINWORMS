use std::{collections::HashMap, sync::Arc};

use egui::Context;
use enum_dispatch::enum_dispatch;
use rend3::Renderer;
use tokio::runtime::Runtime;
use uuid::Uuid;
use winit::event_loop::EventLoop;
use winit::window::Window;

use crate::{theater::basement::input_handling::InputContext, GameProgrammeState, MyEvent};

use self::{
    backstage::plumbing::DefaultRoutines,
    scene::{CamInfo, SceneDefinition, SceneImplementation},
};

use super::basement::cla::GameProgrammeSettings;

pub mod backstage;
pub mod definition;
pub mod scene;
#[derive(Default)]
pub struct Play<PlayablesEnum> {
    pub first_playable: Uuid,
    pub playables: HashMap<Uuid, PlayablesEnum>,
    pub playable_names: HashMap<String, Uuid>,
}

#[enum_dispatch]
pub trait Playable<InputContextEnum: InputContext> {
    //<TO: AmBindings> {
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
    fn implement_chorus_for_playable(&self, egui_ctx: Context);
    //    fn get_current_input_context(&self) -> &InputContext<TO>;
    fn handle_input_for_playable(
        &mut self,
        settings: &GameProgrammeSettings,
        state: &mut GameProgrammeState<InputContextEnum>,
        window: &Arc<Window>,
    );
}
#[derive(Debug)]
pub enum Definitions {
    SceneDefinition(SceneDefinition),
    BogusDefinition, // because fucking clippy that's why
}

impl Default for Definitions {
    fn default() -> Self {
        Definitions::SceneDefinition(SceneDefinition::default())
    }
}
#[allow(clippy::large_enum_variant)]
#[derive(Default)]
pub enum Implementations {
    SceneImplementation(SceneImplementation),
    #[default]
    BogusImplementation,
}
