use std::collections::HashMap;

use crate::{Scene, SceneDefinition};
use uuid::uuid;
const PDP11_CAM_INFO: [f32; 5] = [-3.729838, 4.512105, -0.103016704, -0.4487015, 0.025398161];
const VT100_CAM_INFO: [f32; 5] = [-5.068789, 1.3310424, -3.6215494, -0.31070346, 6.262584];
const THERAC_CAM_INFO: [f32; 5] = [-2.580962, 2.8690546, 2.878742, -0.27470315, 5.620602];
const TOITOI_CAM_INFO: [f32; 5] = [-6.814362, 2.740766, 0.7109763, -0.17870337, 0.0073876693];
const OVERVIEW_CAM_INFO: [f32; 5] = [-6.217338, 3.8491437, 5.883971, -0.40870047, 5.76257];
pub fn define_scene1() -> Scene {
    let scene1_uuid = uuid!("517e70e9-9f6d-48fe-a685-e24482d6d409");
    let scene1_definition = SceneDefinition {
        stage_name: "LinacLab".to_owned(),
        actors: vec![("Midori".to_owned(), "inochi2d-models/Midori.inp".to_owned())],
        props: vec![],
        start_cam: "overview".to_owned(),
        cameras: vec![
            ("overview".to_owned(), OVERVIEW_CAM_INFO),
            ("pdp11".to_owned(), PDP11_CAM_INFO),
            ("vt100".to_owned(), VT100_CAM_INFO),
            ("therac".to_owned(), THERAC_CAM_INFO),
            ("toitoi".to_owned(), TOITOI_CAM_INFO),
        ]
        .iter()
        .fold(HashMap::new(), |mut h, (k, v)| {
            h.insert(k.to_owned(), v.to_owned());
            h
        }),
    };
    Scene {
        scene_uuid: scene1_uuid,
        definition: scene1_definition,
        implementation: None,
    }
}
