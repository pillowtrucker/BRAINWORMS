// how to set the stage for a 3d scene
// for now I'm going to test and experiment in main() and then dump the results here
use std::{collections::HashMap, future::Future, hash::BuildHasher, path::Path, sync::Arc};

use glam::{UVec2, Vec3};
use rend3::{
    types::{Backend, SampleCount, Texture, TextureFormat},
    Renderer,
};
use rend3_framework::{lock, AssetPath, Mutex};
use rend3_gltf::GltfSceneInstance;
use rend3_routine::skybox::SkyboxRoutine;
#[cfg(not(wasm_platform))]
use std::time;
#[cfg(wasm_platform)]
use web_time as time;

pub(crate) async fn load_skybox_image(
    loader: &rend3_framework::AssetLoader,
    data: &mut Vec<u8>,
    path: &str,
) {
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
    loader: &rend3_framework::AssetLoader,
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
        format: TextureFormat::Rgba8UnormSrgb,
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
    loader: &rend3_framework::AssetLoader,
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
            let suffix = if cfg!(target_os = "windows") {
                ".exe"
            } else {
                ""
            };

            indoc::eprintdoc!("
                *** WARNING ***

                It appears you are running scene-viewer with no file to display.
                
                The default scene is no longer bundled into the repository. If you are running on git, use the following commands
                to download and unzip it into the right place. If you're running it through not-git, pass a custom folder to the -C argument
                to tar, then run scene-viewer path/to/scene.gltf.
                
                curl{0} https://cdn.cwfitz.com/scenes/rend3-default-scene.tar -o ./examples/scene-viewer/resources/rend3-default-scene.tar
                tar{0} xf ./examples/scene-viewer/resources/rend3-default-scene.tar -C ./examples/scene-viewer/resources

                ***************
            ", suffix);

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

pub(crate) fn extract_backend(value: &str) -> Result<Backend, &'static str> {
    Ok(match value.to_lowercase().as_str() {
        "vulkan" | "vk" => Backend::Vulkan,
        "dx12" | "12" => Backend::Dx12,
        "dx11" | "11" => Backend::Dx11,
        "metal" | "mtl" => Backend::Metal,
        "opengl" | "gl" => Backend::Gl,
        _ => return Err("unknown backend"),
    })
}

pub(crate) fn extract_profile(value: &str) -> Result<rend3::RendererProfile, &'static str> {
    Ok(match value.to_lowercase().as_str() {
        "legacy" | "c" | "cpu" => rend3::RendererProfile::CpuDriven,
        "modern" | "g" | "gpu" => rend3::RendererProfile::GpuDriven,
        _ => return Err("unknown rendermode"),
    })
}

pub(crate) fn extract_msaa(value: &str) -> Result<SampleCount, &'static str> {
    Ok(match value {
        "1" => SampleCount::One,
        "4" => SampleCount::Four,
        _ => return Err("invalid msaa count"),
    })
}

pub(crate) fn extract_vsync(value: &str) -> Result<rend3::types::PresentMode, &'static str> {
    Ok(match value.to_lowercase().as_str() {
        "immediate" => rend3::types::PresentMode::Immediate,
        "fifo" => rend3::types::PresentMode::Fifo,
        "mailbox" => rend3::types::PresentMode::Mailbox,
        _ => return Err("invalid msaa count"),
    })
}

pub(crate) fn extract_array<const N: usize>(
    value: &str,
    default: [f32; N],
) -> Result<[f32; N], &'static str> {
    let mut res = default;
    let split: Vec<_> = value.split(',').enumerate().collect();

    if split.len() != N {
        return Err("Mismatched argument count");
    }

    for (idx, inner) in split {
        let inner = inner.trim();

        res[idx] = inner.parse().map_err(|_| "Cannot parse argument number")?;
    }
    Ok(res)
}

pub(crate) fn extract_vec3(value: &str) -> Result<Vec3, &'static str> {
    let mut res = [0.0_f32, 0.0, 0.0];
    let split: Vec<_> = value.split(',').enumerate().collect();

    if split.len() != 3 {
        return Err("Directional lights are defined with 3 values");
    }

    for (idx, inner) in split {
        let inner = inner.trim();

        res[idx] = inner.parse().map_err(|_| "Cannot parse direction number")?;
    }
    Ok(Vec3::from(res))
}

pub(crate) fn option_arg<T>(result: Result<Option<T>, pico_args::Error>, usage: &str) -> Option<T> {
    match result {
        Ok(o) => o,
        Err(pico_args::Error::Utf8ArgumentParsingFailed { value, cause }) => {
            eprintln!("{}: '{}'\n\n{}", cause, value, usage);
            std::process::exit(1);
        }
        Err(pico_args::Error::OptionWithoutAValue(value)) => {
            eprintln!("{} flag needs an argument", value);
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("{:?}", e);
            std::process::exit(1);
        }
    }
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
