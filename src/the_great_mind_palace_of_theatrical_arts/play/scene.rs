use std::{collections::HashMap, sync::Arc};

use egui::Context;
use parking_lot::Mutex;
use rend3::Renderer;
use tokio::runtime::Runtime;
use uuid::Uuid;
use winit::event_loop::EventLoop;

use crate::{theater::basement::cla::GameProgrammeSettings, MyEvent};

use self::chorus::Choral;

use super::{backstage::plumbing::DefaultRoutines, Definitions, Implementations, Playable};

pub mod actors;
pub mod chorus;
pub mod definitions;
pub mod props;
pub mod stage2d;
pub mod stage3d;

pub type CamInfo = [f32; 5];
pub struct Camera {
    pub name: String,
    pub renderer_camera: rend3::types::Camera,
    pub cam_attributes: [f32; 5],
}
#[derive(Default)]
pub struct SceneDefinition {
    pub stage: (String, String),
    pub actors: Vec<(String, String)>,
    pub props: Vec<(String, String)>,
    pub start_cam: String,
    pub cameras: HashMap<String, CamInfo>,
}
/*
pub struct Scene {
    pub scene_uuid: Uuid,
    pub scene_name: String,
    pub definition: SceneDefinition,
    pub implementation: Option<SceneImplementation>,
}
*/
#[allow(clippy::large_enum_variant)]
pub enum AstinkScene {
    Loaded(
        (
            String,
            Uuid,
            (rend3_gltf::LoadedGltfScene, rend3_gltf::GltfSceneInstance),
        ),
    ),
    Loading,
}
pub struct SceneImplementation {
    pub stage3d: AstinkScene,
    pub actresses: HashMap<String, Arc<Mutex<actors::AstinkSprite>>>,
    pub props: HashMap<String, props::Prop>,
    pub cameras: HashMap<String, Camera>,
    //    script: String, // I'm really kinda stuck on this chicken and egg problem with script <-> actual game logic
}
pub trait Scenic {
    //    fn scene_definition(&mut self) -> &mut SceneDefinition;
    //    fn scene_implementation(&mut self) -> &mut Option<SceneImplementation>;
    fn scene_uuid(&self) -> Uuid;
    fn scene_name(&self) -> &str;
    fn define_scene(&mut self);
    fn implement_scene(
        &mut self,
        settings: &GameProgrammeSettings,
        event_loop: &EventLoop<MyEvent>,
        renderer: Arc<Renderer>,
        routines: Arc<DefaultRoutines>,
        rts: &Runtime,
    );
    fn scene_starting_cam_info(&self) -> CamInfo;
    fn raw_definition(&mut self) -> &mut Definitions;
    fn raw_implementation(&mut self) -> &mut Option<Implementations>;
}

impl<T: Scenic + Choral> Playable for T {
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
}
