use std::collections::HashMap;

use crate::theater::play::scene::{CamInfo, SceneDefinition, SceneImplementation, Scenic};
use uuid::{uuid, Uuid};
const PDP11_CAM_INFO: [f32; 5] = [-3.729838, 4.512105, -0.103016704, -0.4487015, 0.025398161];
const VT100_CAM_INFO: [f32; 5] = [-5.068789, 1.3310424, -3.6215494, -0.31070346, 6.262584];
const THERAC_CAM_INFO: [f32; 5] = [-2.580962, 2.8690546, 2.878742, -0.27470315, 5.620602];
const TOITOI_CAM_INFO: [f32; 5] = [-6.814362, 2.740766, 0.7109763, -0.17870337, 0.0073876693];
const OVERVIEW_CAM_INFO: [f32; 5] = [-6.217338, 3.8491437, 5.883971, -0.40870047, 5.76257];
#[derive(Default)]
pub struct LinacLabScene {
    pub scene_uuid: Uuid,
    pub scene_name: String,
    pub definition: SceneDefinition,
    pub implementation: Option<SceneImplementation>,
}
impl Scenic for LinacLabScene {
    fn define(&mut self) {
        self.scene_uuid = uuid!("517e70e9-9f6d-48fe-a685-e24482d6d409");
        self.definition = SceneDefinition {
            stage: ("LinacLab".to_owned(), "assets/gltf_scenes".to_owned()),
            actors: vec![("Midori".to_owned(), "assets/inochi2d-models".to_owned())],
            props: vec![("fried_egg".to_owned(), "lfs_scam/props".to_owned())],
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
        self.scene_name = "LinacLab".to_owned();
    }

    fn get_scene_definition(&mut self) -> &mut SceneDefinition {
        &mut self.definition
    }

    fn get_scene_implementation(&mut self) -> &mut Option<SceneImplementation> {
        &mut self.implementation
    }

    fn get_scene_uuid(&self) -> Uuid {
        self.scene_uuid
    }

    fn get_scene_name(&self) -> &str {
        &self.scene_name
    }

    fn implement(&mut self) {
        todo!() // ok fuck this macro time
    }

    fn get_starting_cam_info(&self) -> CamInfo {
        todo!()
    }
}
