use std::{collections::HashMap, sync::Arc};

use crate::{
    theater::{
        basement::cla::GameProgrammeSettings,
        play::{
            backstage::{
                plumbing::DefaultRoutines,
                pyrotechnics::kinetic_narrative::{Gay, KineticEffect, KineticLabel, ShakeLetters},
            },
            scene::{
                actors::{create_actor, AstinkSprite},
                chorus::Choral,
                stage3d::{load_stage3d, make_camera},
                AstinkScene, CamInfo, SceneDefinition, SceneImplementation, Scenic,
            },
            Definitions, Implementations,
        },
    },
    GameProgrammeData, MyEvent,
};
use egui::Context;
use proc_macros::{Choral, Scenic};
use rend3::Renderer;
use tokio::runtime::Runtime;
use uuid::{uuid, Uuid};
use winit::event_loop::EventLoop;
const PDP11_CAM_INFO: [f32; 5] = [-3.729838, 4.512105, -0.103016704, -0.4487015, 0.025398161];
const VT100_CAM_INFO: [f32; 5] = [-5.068789, 1.3310424, -3.6215494, -0.31070346, 6.262584];
const THERAC_CAM_INFO: [f32; 5] = [-2.580962, 2.8690546, 2.878742, -0.27470315, 5.620602];
const TOITOI_CAM_INFO: [f32; 5] = [-6.814362, 2.740766, 0.7109763, -0.17870337, 0.0073876693];
const OVERVIEW_CAM_INFO: [f32; 5] = [-6.217338, 3.8491437, 5.883971, -0.40870047, 5.76257];

//#[add_common_playable_fields] // this is not worth the stupid RA errors
#[derive(Default, Scenic, Choral)]
pub struct LinacLabScene {
    pub uuid: Uuid,
    pub name: String,
    pub definition: Definitions,
    pub implementation: Option<Implementations>,
}

impl LinacLabScene {
    fn define(&mut self) {
        self.uuid = uuid!("517e70e9-9f6d-48fe-a685-e24482d6d409");
        self.definition = Definitions::SceneDefinition(SceneDefinition {
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
        });
        self.name = "LinacLab".to_owned();
    }

    fn implement(
        &mut self,
        settings: &GameProgrammeSettings,
        event_loop: &EventLoop<MyEvent>,
        renderer: Arc<Renderer>,
        routines: Arc<DefaultRoutines>,
        rts: &Runtime,
    ) {
        let Definitions::SceneDefinition(definition) = self.definition else {
            panic!("scene has non-scene definition")
        };
        let scene1_starting_cam =
            make_camera((definition.start_cam.clone(), self.starting_cam_info()));
        let mut scene1_cameras = HashMap::new();
        scene1_cameras.insert(scene1_starting_cam.name.clone(), scene1_starting_cam);
        let gltf_settings = settings.gltf_settings;
        //        let renderer = Arc::clone(renderer);
        //        let routines = Arc::clone(routines);
        let event_loop_proxy = event_loop.create_proxy();
        let scene1_uuid = self.uuid;
        let scene1_stage_name = definition.stage.0.clone();
        let scene1_stage_directory = definition.stage.1.clone();
        let mut scene1_stage3d = HashMap::new();
        scene1_stage3d.insert(scene1_stage_name.clone(), AstinkScene::Loading);

        let mut scene1_actor_impls = HashMap::new();
        for (name, _) in definition.actors.clone() {
            scene1_actor_impls.insert(name.clone(), AstinkSprite::Loading);
        }
        let scene1_implementation = SceneImplementation {
            stage3d: scene1_stage3d,
            actresses: HashMap::new(),
            props: HashMap::new(), // todo!(),
            cameras: scene1_cameras,
        };

        self.implementation = Some(Implementations::SceneImplementation(scene1_implementation));
        let scene1_actors = definition.actors.clone();
        for (name, directory) in scene1_actors {
            let renderer = Arc::clone(&renderer);
            let event_loop_proxy = event_loop.create_proxy();
            let name = name.to_owned();
            rts.spawn(async move {
                create_actor(name, directory, renderer, event_loop_proxy, scene1_uuid).await
            });
        }

        rts.spawn(async move {
            load_stage3d(
                scene1_stage_name,
                scene1_stage_directory,
                scene1_uuid,
                renderer,
                gltf_settings,
                event_loop_proxy,
            )
            .await;
        });
    }

    fn starting_cam_info(&self) -> CamInfo {
        let Definitions::SceneDefinition(definition) = &self.definition else {
            panic!("scene has non-scene definition")
        };
        definition
            .cameras
            .get(&definition.start_cam)
            .unwrap()
            .clone()
    }

    fn implement_chorus(&self, egui_ctx: &mut Context, data: &GameProgrammeData) {
        egui::Window::new("egui widget testing").show(egui_ctx, |ui| {
            ui.label(std::format!("framerate: {:.0}fps", data.frame_rate.get()));
            ui.horizontal(|ui| {
                ui.add(KineticLabel::new("blabla"));
                ui.add(KineticLabel::new("same").kinesis(vec![&KineticEffect::default()]));
                ui.add(
                    KineticLabel::new("line").kinesis(vec![&KineticEffect::ShakeLetters {
                        params: ShakeLetters::default(),
                    }]),
                );
                ui.add(
                    KineticLabel::new("still").kinesis(vec![&KineticEffect::Gay {
                        params: Gay::default(),
                    }]),
                );
            });
            for (i, line) in data.test_lines.lines().enumerate() {
                ui.add(KineticLabel::new(line).kinesis(vec![&data.random_line_effects[i]]));
            }
        });
    }
}
