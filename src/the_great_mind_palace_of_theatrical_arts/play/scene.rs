use std::{collections::HashMap, sync::Arc};

use parking_lot::Mutex;
use uuid::Uuid;

use super::Playable;

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
    pub stage3d: HashMap<String, AstinkScene>,
    pub actresses: HashMap<String, Arc<Mutex<actors::AstinkSprite>>>,
    pub props: HashMap<String, props::Prop>,
    pub cameras: HashMap<String, Camera>,
    //    script: String, // I'm really kinda stuck on this chicken and egg problem with script <-> actual game logic
}
pub trait Scenic {
    fn get_scene_definition(&mut self) -> &mut SceneDefinition;
    fn get_scene_implementation(&mut self) -> &mut Option<SceneImplementation>;
    fn get_scene_uuid(&self) -> Uuid;
    fn get_scene_name(&self) -> &str;
    fn define(&mut self);
    fn implement(&mut self);
    fn get_starting_cam_info(&self) -> CamInfo;
}
/*
impl Playable for dyn Scenic {
    fn get_playable_uuid(&self) -> Uuid {
        self.get_scene_uuid()
    }

    fn get_playable_name(&self) -> &str {
        self.get_scene_name()
    }

    fn get_starting_cam_info(&self) -> CamInfo {
        self.get_starting_cam_info()
    }
}
*/
