use std::{collections::HashMap, sync::Arc};

use egui::Context;
use glam::{Mat3A, Vec3A};
use parking_lot::Mutex;
use rend3::Renderer;
use tokio::runtime::Runtime;
use uuid::Uuid;
use winit::{event_loop::EventLoop, window::Window};

use crate::{
    theater::basement::{
        cla::GameProgrammeSettings,
        input_handling::{HandlesInputContexts, InputContext},
    },
    GameProgrammeState, MyEvent,
};

use self::{actors::ActressDefinition, chorus::Choral, stage3d::Colliders};

use super::{backstage::plumbing::DefaultRoutines, Definitions, Implementations, Playable};

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
pub trait Scenic {
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
