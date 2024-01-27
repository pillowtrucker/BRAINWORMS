use std::{collections::HashMap, sync::Arc};

use brainworms_arson::egui;
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
    scene::{chorus::Choral, CamInfo, SceneDefinition, SceneImplementation, Scenic},
};

use super::basement::{cla::GameProgrammeSettings, input_handling::HandlesInputContexts};

pub mod backstage;
pub mod definition;
pub mod orchestra;
pub mod scene;
#[derive(Default)]
pub struct Play<PlayablesEnum> {
    pub first_playable: Uuid,
    pub playables: HashMap<Uuid, PlayablesEnum>,
    pub playable_names: HashMap<String, Uuid>,
}

#[enum_dispatch]
pub trait Playable<InputContextEnum: InputContext> {
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
impl<
        InputContextEnum: InputContext,
        T: Scenic + Choral + HandlesInputContexts<InputContextEnum>,
    > Playable<InputContextEnum> for T
{
    fn playable_uuid(&self) -> Uuid {
        self.scene_uuid()
    }

    fn playable_name(&self) -> &str {
        self.scene_name()
    }

    fn starting_cam_info(&self) -> CamInfo {
        self.scene_starting_cam_info()
    }

    fn implement_playable(
        &mut self,
        settings: &GameProgrammeSettings,
        event_loop: &EventLoop<MyEvent>,
        renderer: Arc<Renderer>,
        routines: Arc<DefaultRoutines>,
        rts: &Runtime,
    ) {
        self.implement_scene(settings, event_loop, renderer, routines, rts)
    }

    fn define_playable(&mut self) {
        self.define_scene()
    }
    fn implement_chorus_for_playable(&self, egui_ctx: Context) {
        self.implement_chorus_for_choral(egui_ctx);
    }

    fn playable_definition(&mut self) -> &mut Definitions {
        self.raw_definition()
    }

    fn playable_implementation(&mut self) -> &mut Option<Implementations> {
        self.raw_implementation()
    }

    fn handle_input_for_playable(
        &mut self,
        settings: &GameProgrammeSettings,
        state: &mut GameProgrammeState<InputContextEnum>,
        window: &Arc<Window>,
    ) {
        self.handle_input_for_context(settings, state, window)
    }
}
