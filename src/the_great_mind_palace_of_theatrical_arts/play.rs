use std::collections::HashMap;

use enum_dispatch::enum_dispatch;
use uuid::Uuid;

use self::scene::{definitions::linac_lab::LinacLabScene, CamInfo};

pub mod backstage;
pub mod definition;
pub mod orchestra;
pub mod scene;

pub struct Play {
    pub first_playable: Uuid,
    pub playables: HashMap<Uuid, Playables>,
    pub playable_names: HashMap<String, Uuid>,
}
#[enum_dispatch(Playable)]
pub enum Playables {
    LinacLabScene,
    LoadingScreen,
    MainMenu,
}
pub trait Playable {
    fn get_playable_uuid(&self) -> Uuid;
    fn get_playable_name(&self) -> &str;
    fn get_starting_cam_info(&self) -> CamInfo;
}
