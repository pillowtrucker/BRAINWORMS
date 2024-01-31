use std::{mem::take, sync::Arc};

use baudio::{audio_router_thread, AudioCommand, JingleRegistry};
use brainworms_farting_noises as baudio;

use parking_lot::Mutex;
use tokio::{runtime::Handle, sync::mpsc::UnboundedSender};
type Generation = u64;
pub struct Orchestra {
    handler: (Generation, Option<UnboundedSender<AudioCommand>>),
    jingle_registry: Arc<Mutex<JingleRegistry>>,
    rth: Handle,
}
impl Orchestra {
    pub fn new(rth: Handle) -> Self {
        let jingle_registry = Arc::new(Mutex::new(JingleRegistry::default()));
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
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let tx_theirs = tx.clone();
        self.rth.spawn(audio_router_thread(
            rx,
            tx_theirs,
            self.jingle_registry.clone(),
            gen,
        ));
        let old_tx = take(&mut self.handler.1);
        old_tx.map(|old_tx| old_tx.send(AudioCommand::Die));
        self.handler = (gen, Some(tx));
    }
    pub fn send_cmd(&self, cmd: AudioCommand) {
        let _ = self.handler.1.as_ref().unwrap().send(cmd);
    }
    pub fn is_registered(&self, name: &str) -> bool {
        let registry = self.jingle_registry.lock();
        registry.jingles.contains_key(name)
    }
}
