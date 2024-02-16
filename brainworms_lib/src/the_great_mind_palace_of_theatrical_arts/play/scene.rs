use std::{collections::HashMap, sync::Arc};

use glam::{Mat3A, Vec3A};
use parking_lot::Mutex;
use rend3::Renderer;
use tokio::runtime::Runtime;
use uuid::Uuid;
use winit::event_loop::EventLoop;

use crate::{
    theater::basement::{cla::GameProgrammeSettings, input_handling::InputContext},
    GameProgrammeData, GameProgrammeState, MyEvent,
};

use self::{actors::ActressDefinition, stage3d::Colliders};

use super::{
    backstage::plumbing::DefaultRoutines, orchestra::Orchestra, Definitions, Implementations,
    Playable,
};

pub mod actors;
pub mod chorus;
pub mod definitions;
pub mod props;
pub mod stage3d;
#[derive(Debug, Default, Clone)]
pub struct CamInfo {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub pitch: f32,
    pub yaw: f32,
}
impl CamInfo {
    pub fn location(&self) -> Vec3A {
        Vec3A::new(self.x, self.y, self.z)
    }
    pub fn from_arr(arr: &[f32; 5]) -> Self {
        Self {
            x: arr[0],
            y: arr[1],
            z: arr[2],
            pitch: arr[3],
            yaw: arr[4],
        }
    }
    pub fn set_location(&mut self, x: f32, y: f32, z: f32) {
        self.x = x;
        self.y = y;
        self.z = z;
    }
    pub fn as_arr(&self) -> [f32; 5] {
        [self.x, self.y, self.z, self.pitch, self.yaw]
    }
}
#[derive(Debug, Default, Clone)]
pub struct Camera {
    pub name: String,
    pub renderer_camera: rend3::types::Camera,
    pub info: CamInfo,
    pub rotation: Mat3A,
}
#[derive(Default, Debug)]
pub struct SceneDefinition {
    pub stage: (String, String),
    pub actors: Vec<ActressDefinition>,
    pub props: Vec<(String, String)>,
    pub start_cam: String,
    pub cameras: HashMap<String, CamInfo>,
}

#[allow(clippy::large_enum_variant)]
#[derive(Default)]
pub enum AstinkScene {
    Loaded(
        (
            String,
            Uuid,
            (
                rend3_gltf::LoadedGltfScene,
                rend3_gltf::GltfSceneInstance,
                Colliders,
            ),
        ),
    ),
    #[default]
    Loading,
}
#[derive(Default)]
pub struct SceneImplementation {
    pub stage3d: AstinkScene,
    pub actresses: HashMap<String, Arc<Mutex<actors::AstinkSprite>>>,
    pub props: HashMap<String, props::Prop>,
    pub cameras: HashMap<String, Camera>,
    //    script: String, // I'm really kinda stuck on this chicken and egg problem with script <-> actual game logic
}
pub trait Scenic<
    InputContextEnum: InputContext,
    PlayablesEnum: Playable<InputContextEnum, PlayablesEnum>,
>
{
    fn scene_uuid(&self) -> Uuid;
    fn scene_name(&self) -> &str;
    fn define_scene(&mut self);
    fn implement_scene(
        &mut self,
        game_settings: Arc<Mutex<GameProgrammeSettings>>,
        game_state: Arc<Mutex<GameProgrammeState<InputContextEnum>>>,
        game_data: Arc<Mutex<GameProgrammeData<PlayablesEnum>>>,
        rts: Arc<Mutex<Option<Runtime>>>,
    );
    fn scene_starting_cam_info(&self) -> CamInfo;
    fn raw_definition(&mut self) -> &mut Definitions;
    fn raw_implementation(&mut self) -> &mut Option<Implementations>;
}
