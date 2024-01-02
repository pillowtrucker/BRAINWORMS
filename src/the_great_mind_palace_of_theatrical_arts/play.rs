use std::collections::HashMap;

use uuid::Uuid;

use self::scene::Scene;

pub mod backstage;
pub mod definition;
pub mod orchestra;
pub mod scene;

pub struct Play {
    pub first_scene: Uuid,
    pub scenes: HashMap<Uuid, Scene>,
    pub scene_names: HashMap<String, Uuid>,
}
