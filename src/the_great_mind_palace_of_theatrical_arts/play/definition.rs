use std::collections::HashMap;

use crate::Play;

use super::scene::{definitions::linac_lab::LinacLabScene, Scenic};

pub fn define_play() -> Play {
    let mut linac_lab_scene = LinacLabScene::default();
    linac_lab_scene.define_scene();
    let mut playables = HashMap::new();
    let mut playable_names = HashMap::new();
    playable_names.insert(linac_lab_scene.name.clone(), linac_lab_scene.uuid);
    let first_playable = linac_lab_scene.uuid;
    playables.insert(
        linac_lab_scene.uuid,
        super::Playables::LinacLabScene(linac_lab_scene),
    );

    Play {
        first_playable,
        playables,
        playable_names,
    }
}
