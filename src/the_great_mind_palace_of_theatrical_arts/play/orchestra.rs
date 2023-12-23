/*
use uuid::Uuid;

use super::{Playable, PlayableUUID, Script};

//play elements outside of or between scenes
pub struct Intermezzo {
    pub uuid: Uuid,
    pub after_id: Option<PlayableUUID>,
    pub before_id: Option<PlayableUUID>,
    script: Script,
}
impl Playable for Intermezzo {
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
