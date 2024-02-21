use std::{collections::HashMap, sync::Arc};

use brainworms_arson::egui;
use egui::Context;
use enum_dispatch::enum_dispatch;
use into_variant::VariantFrom;
use parking_lot::Mutex;
use rend3::Renderer;
use tokio::runtime::Runtime;
use uuid::Uuid;
use winit::event_loop::EventLoop;
use winit::window::Window;

use crate::{theater::basement::input_handling::InputContext, GameProgrammeState, MyEvent};

use self::{
    backstage::plumbing::DefaultRoutines,
    curtain::{CurtainDefinition, CurtainImplementation},
    orchestra::Orchestra,
    scene::{chorus::Choral, CamInfo, SceneDefinition, SceneImplementation, Scenic},
};

use super::basement::{cla::GameProgrammeSettings, input_handling::HandlesInputContexts};

pub mod backstage;
pub mod curtain;
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
pub trait Playable<InputContextEnum: InputContext, UserData> {
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
        orchestra: Arc<Orchestra>,
        user_data: Arc<Mutex<UserData>>,
    );
    fn define_playable(&mut self);
    fn implement_chorus_for_playable(
        &self,
        egui_ctx: Context,
        orchestra: Arc<Orchestra>,
        settings: &GameProgrammeSettings,
        user_data: Arc<Mutex<UserData>>,
    );

    fn handle_input_for_playable(
        &mut self,
        settings: &GameProgrammeSettings,
        state: &mut GameProgrammeState<InputContextEnum>,
        window: &Arc<Window>,
    );
}
#[derive(Debug, VariantFrom, Default)]
pub enum Definitions {
    SceneDefinition(SceneDefinition),
    CurtainDefinition(CurtainDefinition),
    #[default]
    BogusDefinition,
}

#[allow(clippy::large_enum_variant)]
#[derive(Default)]
pub enum Implementations {
    SceneImplementation(SceneImplementation),
    CurtainImplementation(CurtainImplementation),
    #[default]
    BogusImplementation,
}

impl<
        InputContextEnum: InputContext,
        T: Scenic<UserData> + Choral<UserData> + HandlesInputContexts<InputContextEnum>,
        UserData: Default,
    > Playable<InputContextEnum, UserData> for T
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
        orchestra: Arc<Orchestra>,
        user_data: Arc<Mutex<UserData>>,
    ) {
        self.implement_scene(
            settings, event_loop, renderer, routines, rts, orchestra, user_data,
        )
    }

    fn define_playable(&mut self) {
        self.define_scene()
    }
    fn implement_chorus_for_playable(
        &self,
        egui_ctx: Context,
        orchestra: Arc<Orchestra>,
        settings: &GameProgrammeSettings,
        user_data: Arc<Mutex<UserData>>,
    ) {
        self.implement_chorus_for_choral(egui_ctx, orchestra, settings, user_data);
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
