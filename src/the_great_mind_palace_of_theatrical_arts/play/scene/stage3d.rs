// how to set the stage for a 3d scene
// for now I'm going to test and experiment in main() and then dump the results here
use std::{collections::HashMap, hash::BuildHasher, path::Path, sync::Arc};

use egui::TextBuffer;
use glam::UVec2;

use parking_lot::Mutex;
use parry3d::shape::TriMesh;
use rend3::{
    types::{Texture, TextureFormat},
    Renderer,
};
use rend3_gltf::{GltfLoadSettings, GltfSceneInstance};
use rend3_routine::skybox::SkyboxRoutine;
use uuid::Uuid;
use winit::event_loop::EventLoopProxy;

use std::time;

use crate::{
    theater::play::backstage::plumbing::asset_loader::{AssetError, AssetLoader, AssetPath},
    MyEvent, MyWinitEvent,
};

use super::AstinkScene;

pub fn lock<T>(lock: &parking_lot::Mutex<T>) -> parking_lot::MutexGuard<'_, T> {
    let guard = lock.lock();

    guard
}

//pub(crate) async fn

pub(crate) async fn load_skybox_image(loader: &AssetLoader, data: &mut Vec<u8>, path: &str) {
    let decoded = image::load_from_memory(
        &loader
            .get_asset(AssetPath::Internal(path))
            .await
            .unwrap_or_else(|e| panic!("Error {}: {}", path, e)),
    )
    .unwrap()
    .into_rgba8();

    data.extend_from_slice(decoded.as_raw());
}

pub(crate) async fn load_stage3d(
    name: String,
    directory: String,
    sc_id: Uuid,
    renderer: Arc<Renderer>,
    gltf_settings: GltfLoadSettings,
    event_loop_proxy: EventLoopProxy<MyEvent>,
) {
    let loader = AssetLoader::default();
    let path = format!("{}/{}.glb", directory, name);
    let ret = Box::new(
        load_gltf(
            &renderer,
            &loader,
            &gltf_settings,
            AssetPath::Internal(&path),
        )
        .await,
    );
    let _ = event_loop_proxy.send_event(MyWinitEvent::Stage3D(AstinkScene::Loaded((
        name,
        sc_id,
        ret.unwrap(),
    ))));
}

pub(crate) async fn load_skybox(
    renderer: &Arc<Renderer>,
    skybox_routine: &Mutex<SkyboxRoutine>,
) -> Result<(), Box<dyn std::error::Error>> {
    let loader = &AssetLoader::default();
    let mut data = Vec::new();
    load_skybox_image(loader, &mut data, "assets/skybox/right.jpg").await;
    load_skybox_image(loader, &mut data, "assets/skybox/left.jpg").await;
    load_skybox_image(loader, &mut data, "assets/skybox/top.jpg").await;
    load_skybox_image(loader, &mut data, "assets/skybox/bottom.jpg").await;
    load_skybox_image(loader, &mut data, "assets/skybox/front.jpg").await;
    load_skybox_image(loader, &mut data, "assets/skybox/back.jpg").await;

    let handle = renderer.add_texture_cube(Texture {
        format: TextureFormat::Bgra8UnormSrgb,
        size: UVec2::new(2048, 2048),
        data,
        label: Some("background".into()),
        mip_count: rend3::types::MipmapCount::ONE,
        mip_source: rend3::types::MipmapSource::Uploaded,
    })?;
    lock(skybox_routine).set_background_texture(Some(handle));
    Ok(())
}

pub(crate) async fn load_gltf(
    renderer: &Arc<Renderer>,
    loader: &AssetLoader,
    settings: &rend3_gltf::GltfLoadSettings,
    location: AssetPath<'_>,
) -> Option<(rend3_gltf::LoadedGltfScene, GltfSceneInstance)> {
    // profiling::scope!("loading gltf");
    let gltf_start = time::Instant::now();
    let path = loader.get_asset_path(location);
    let path = Path::new(&*path);
    let parent = path.parent().unwrap();

    let parent_str = parent.to_string_lossy();
    let path_str = path.as_os_str().to_string_lossy();
    log::info!("Reading gltf file: {}", path_str);
    let gltf_data_result = loader.get_asset(AssetPath::External(&path_str)).await;

    let gltf_data = match gltf_data_result {
        Ok(d) => d,
        Err(AssetError::FileError { path, error }) => {
            panic!("Error Loading gltf file {} E: {}", path, error)
        }
    };

    let gltf_elapsed = gltf_start.elapsed();
    let resources_start = time::Instant::now();
    //let colliders = load_colliders_from_gltf(collider_ids, &gltf_data, settings);
    let (scene, instance) = rend3_gltf::load_gltf(renderer, &gltf_data, settings, |uri| async {
        if let Some(base64) = rend3_gltf::try_load_base64(&uri) {
            Ok(base64)
        } else {
            log::info!("Loading resource {}", uri);
            let uri = uri;
            let full_uri = parent_str.clone() + "/" + uri.as_str();
            loader.get_asset(AssetPath::External(&full_uri)).await
        }
    })
    .await
    .unwrap();

    log::info!(
        "Loaded gltf in {:.3?}, resources loaded in {:.3?}",
        gltf_elapsed,
        resources_start.elapsed()
    );
    Some((scene, instance))
}
/*
pub(crate) fn load_colliders_from_gltf(
    collider_ids: Vec<String>,
    gltf_data: &Vec<u8>,
    settings: &GltfLoadSettings,
) -> Result<Vec<(String, TriMesh)>, _> {
    let file = gltf::Gltf::from_slice_without_validation(gltf_data)?;
    for n in file.document.nodes() {
        if n.name()
            .is_some_and(|n| collider_ids.contains(&n.to_owned()))
        {
            let mut c = n.children();

            while (!c.is_empty()) {
                for cc in c {
                    if cc.mesh().is_some() {
                        for p in cc.mesh().unwrap().primitives() {
                            p.
                        }
                    }
                }
            }

        }
    }
}
*/
pub(crate) fn button_pressed<Hash: BuildHasher>(map: &HashMap<u32, bool, Hash>, key: u32) -> bool {
    map.get(&key).map_or(false, |b| *b)
}
pub fn make_camera(
    (name, cam_attributes @ [x, y, z, pitch, yaw]): (String, [f32; 5]),
) -> super::Camera {
    let camera_location = glam::Vec3A::new(x, y, z);
    let view = glam::Mat4::from_euler(glam::EulerRot::XYZ, -pitch, -yaw, 0.0);
    let view = view * glam::Mat4::from_translation((-camera_location).into());

    // Set camera location data
    super::Camera {
        name,
        renderer_camera: rend3::types::Camera {
            projection: rend3::types::CameraProjection::Perspective {
                vfov: 60.0,
                near: 0.1,
            },
            view,
        },
        cam_attributes,
    }
}
