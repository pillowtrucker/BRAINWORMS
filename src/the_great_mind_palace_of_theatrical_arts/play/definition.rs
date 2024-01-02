use std::collections::HashMap;

use crate::Play;

use super::scene::definitions::linac_lab::define_scene1;

pub fn define_play() -> Play {
    let scene1 = define_scene1();
    let mut scenes = HashMap::new();
    let mut scene_names = HashMap::new();
    scene_names.insert(scene1.scene_name.clone(), scene1.scene_uuid);
    let first_scene = scene1.scene_uuid;
    scenes.insert(scene1.scene_uuid, scene1);

    Play {
        first_scene,
        scenes,
        scene_names,
    }
}
