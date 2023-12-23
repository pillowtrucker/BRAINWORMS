pub mod backstage;
pub mod orchestra;
pub mod scene;
//use std::collections::HashMap;

//use self::{orchestra::Intermezzo, scene::Scene};
/*
#[cfg(target_arch = "wasm32")]
pub(crate) use super::basement::resize_observer;
//pub use parking_lot::{Mutex, MutexGuard};
// I definitely want the minimal asset loader and the grabber from r3f
//pub use rend3_framework::{AssetError, AssetLoader, AssetPath, Grabber, UserResizeEvent};
use uuid::Uuid;

type PlayableUUID = Uuid;
type ActorUUID = Uuid;
type PropUUID = Uuid;
type Script = String;
pub trait Playable {
    fn get_uuid(&self) -> &PlayableUUID;
    fn before_id(&self) -> &Option<PlayableUUID>;
    fn after_id(&self) -> &Option<PlayableUUID>;
    fn play(&mut self);
}
*/
/*
pub struct Play {
    current_playable: Option<Uuid>,
    intermezzi: HashMap<PlayableUUID, Intermezzo>,
    scenes: HashMap<PlayableUUID, Scene>,
}
impl Play {
    fn get_first_pl_id(&mut self) -> &PlayableUUID {
        match self.intermezzi.iter().find(|(_, im)| im.after_id.is_none()) {
            Some((id, _)) => id,
            None => match self.scenes.iter().find(|(_, sc)| sc.after_id.is_none()) {
                Some((id, _intro)) => id,
                None => panic!("You don't have any scenes and no playable intro."),
            },
        }
    }
    fn run_playable(&mut self, id: PlayableUUID) {
        if let Some(sc) = self.scenes.get_mut(&id) {
            sc.play()
        } else if let Some(im) = self.intermezzi.get_mut(&id) {
            im.play()
        } else {
            panic!("asked to play unlinked playable")
        }
    }
    pub fn _showtime(&mut self) {
        self.current_playable = Some(self.get_first_pl_id().to_owned());
        self.run_playable(
            self.current_playable
                .expect("No playables found at start of play"),
        )
    }
}
*/
