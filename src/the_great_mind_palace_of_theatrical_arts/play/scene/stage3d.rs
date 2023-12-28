// how to set the stage for a 3d scene
// for now I'm going to test and experiment in main() and then dump the results here
use std::{collections::HashMap, future::Future, hash::BuildHasher, path::Path, sync::Arc};

use glam::UVec2;

use parking_lot::Mutex;
use rend3::{
    types::{Texture, TextureFormat},
    Renderer,
};
use rend3_gltf::GltfSceneInstance;
use rend3_routine::skybox::SkyboxRoutine;
#[cfg(not(wasm_platform))]
use std::time;
#[cfg(wasm_platform)]
use web_time as time;

use crate::theater::play::backstage::plumbing::asset_loader::{AssetLoader, AssetPath};

pub fn lock<T>(lock: &parking_lot::Mutex<T>) -> parking_lot::MutexGuard<'_, T> {
    #[cfg(target_arch = "wasm32")]
    let guard = lock.try_lock().expect("Could not lock mutex on single-threaded wasm. Do not hold locks open while an .await causes you to yield execution.");
    #[cfg(not(target_arch = "wasm32"))]
    let guard = lock.lock();

    guard
}

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

pub(crate) async fn load_skybox(
    renderer: &Arc<Renderer>,
    loader: &AssetLoader,
    skybox_routine: &Mutex<SkyboxRoutine>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut data = Vec::new();
    load_skybox_image(loader, &mut data, "skybox/right.jpg").await;
    load_skybox_image(loader, &mut data, "skybox/left.jpg").await;
    load_skybox_image(loader, &mut data, "skybox/top.jpg").await;
    load_skybox_image(loader, &mut data, "skybox/bottom.jpg").await;
    load_skybox_image(loader, &mut data, "skybox/front.jpg").await;
    load_skybox_image(loader, &mut data, "skybox/back.jpg").await;

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
    let is_default_scene = matches!(location, AssetPath::Internal(_));
    let path = loader.get_asset_path(location);
    let path = Path::new(&*path);
    let parent = path.parent().unwrap();

    let parent_str = parent.to_string_lossy();
    let path_str = path.as_os_str().to_string_lossy();
    log::info!("Reading gltf file: {}", path_str);
    let gltf_data_result = loader.get_asset(AssetPath::External(&path_str)).await;

    let gltf_data = match gltf_data_result {
        Ok(d) => d,
        Err(_) if is_default_scene => {
            indoc::eprintdoc!(
                "
                *** WARNING ***

                It appears you are running scene-viewer with no file to display.

                ***************
            "
            );

            return None;
        }
        e => e.unwrap(),
    };

    let gltf_elapsed = gltf_start.elapsed();
    let resources_start = time::Instant::now();
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

pub(crate) fn button_pressed<Hash: BuildHasher>(map: &HashMap<u32, bool, Hash>, key: u32) -> bool {
    map.get(&key).map_or(false, |b| *b)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn spawn<Fut>(fut: Fut)
where
    Fut: Future + Send + 'static,
    Fut::Output: Send + 'static,
{
    std::thread::spawn(|| pollster::block_on(fut));
}

#[cfg(target_arch = "wasm32")]
pub fn spawn<Fut>(fut: Fut)
where
    Fut: Future + 'static,
    Fut::Output: 'static,
{
    wasm_bindgen_futures::spawn_local(async move {
        fut.await;
    });
}
