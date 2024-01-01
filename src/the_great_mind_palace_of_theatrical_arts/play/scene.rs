/*
use core::hash::Hash;
use std::collections::HashMap;

use uuid::Uuid;

use self::{actors::Actor, props::Prop};

use super::{ActorUUID, Playable, PlayableUUID, PropUUID, Script};
*/
pub mod actors;
pub mod definitions;
pub mod props;
pub mod stage2d;
pub mod stage3d;
/*
pub struct Scene {
    pub uuid: Uuid,
    pub after_id: Option<PlayableUUID>,
    pub before_id: Option<PlayableUUID>,
    actors: HashMap<ActorUUID, Actor>,
    props: HashMap<PropUUID, Prop>,
    script: Script,
}
impl Playable for Scene {
    fn get_uuid(&self) -> &PlayableUUID {
        &self.uuid
    }

    fn before_id(&self) -> &Option<PlayableUUID> {
        &self.before_id
    }

    fn after_id(&self) -> &Option<PlayableUUID> {
        &self.after_id
    }

    fn play(&mut self) {
        todo!()
    }
}
*/

pub struct Camera {
    pub name: String,
    pub renderer_camera: rend3::types::Camera,
    pub cam_attributes: [f32; 5],
}
