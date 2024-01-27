use std::{
    mem::{replace, swap},
    sync::Arc,
};

use baudio::{audio_router_thread, init, AudioCommand, JingleRegistry};
use brainworms_farting_noises as baudio;

use parking_lot::Mutex;
use tokio::{runtime::Handle, sync::mpsc::Sender};
type Generation = u64;
pub struct Orchestra {
    handler: (Generation, Option<Sender<AudioCommand>>),
    jingle_registry: Arc<Mutex<JingleRegistry>>,
    rth: Handle,
}
impl Orchestra {
    pub fn new(rth: Handle) -> Self {
        let jingle_registry = Arc::new(Mutex::new(JingleRegistry::new()));
        let mut me = Self {
            handler: (0, None),
            jingle_registry,
            rth,
        };
        me.replace_worker();
        me
    }
    fn replace_worker(&mut self) {
        let gen = self.handler.0 + 1;
        let new_ctx = Arc::new(Mutex::new(init(&format!("audio ctx gen {}", gen)).unwrap()));
        let (tx, rx) = tokio::sync::mpsc::channel(256);
        self.rth.spawn(audio_router_thread(
            rx,
            self.jingle_registry.clone(),
            new_ctx,
        ));
        self.handler = (gen, Some(tx));
    }
}
