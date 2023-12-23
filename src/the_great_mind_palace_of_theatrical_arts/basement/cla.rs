use glam::{DVec2, Vec3, Vec3A};
use pico_args::Arguments;
use rend3::{
    types::{DirectionalLightHandle, SampleCount},
    util::typedefs::FastHashMap,
    RendererProfile,
};
use rend3_routine::pbr::NormalTextureYDirection;
use wgpu::Backend;
use wgpu_profiler::GpuTimerScopeResult;

use super::grab::Grabber;

const HELP: &str = "\
scene-viewer

gltf and glb scene viewer powered by the rend3 rendering library.

usage: scene-viewer --options ./path/to/gltf/file.gltf

Meta:
  --help            This menu.

Rendering:
  -b --backend                 Choose backend to run on ('vk', 'dx12', 'dx11', 'metal', 'gl').
  -d --device                  Choose device to run on (case insensitive device substring).
  -p --profile                 Choose rendering profile to use ('cpu', 'gpu').
  -v --vsync                   Choose vsync mode ('immediate' [no-vsync], 'fifo' [vsync], 'fifo_relaxed' [adaptive vsync], 'mailbox' [fast vsync])
  --msaa <level>               Level of antialiasing (either 1 or 4). Default 1.

Windowing:
  --absolute-mouse             Interpret the relative mouse coordinates as absolute. Useful when using things like VNC.
  --fullscreen                 Open the window in borderless fullscreen.

Assets:
  --normal-y-down                        Interpret all normals as having the DirectX convention of Y down. Defaults to Y up.
  --directional-light <x,y,z>            Create a directional light pointing towards the given coordinates.
  --directional-light-intensity <value>  All lights created by the above flag have this intensity. Defaults to 4.
  --gltf-disable-directional-lights      Disable all directional lights in the gltf
  --ambient <value>                      Set the value of the minimum ambient light. This will be treated as white light of this intensity. Defaults to 0.1.
  --scale <scale>                        Scale all objects loaded by this factor. Defaults to 1.0.
  --shadow-distance <value>              Distance from the camera there will be directional shadows. Lower values means higher quality shadows. Defaults to 100.
  --shadow-resolution <value>            Resolution of the shadow map. Higher values mean higher quality shadows with high performance cost. Defaults to 2048.

Controls:
  --walk <speed>               Walk speed (speed without holding shift) in units/second (typically meters). Default 10.
  --run  <speed>               Run speed (speed while holding shift) in units/second (typically meters). Default 50.
  --camera x,y,z,pitch,yaw     Spawns the camera at the given position. Press Period to get the current camera position.
";

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

pub struct GameProgrammeSettings {
    pub absolute_mouse: bool,
    pub desired_backend: Option<Backend>,
    pub desired_device_name: Option<String>,
    pub desired_profile: Option<RendererProfile>,
    pub file_to_load: Option<String>,
    pub walk_speed: f32,
    pub run_speed: f32,
    pub gltf_settings: rend3_gltf::GltfLoadSettings,
    pub directional_light_direction: Option<Vec3>,
    pub directional_light_intensity: f32,
    pub directional_light: Option<DirectionalLightHandle>,
    pub ambient_light_level: f32,
    pub present_mode: rend3::types::PresentMode,
    pub samples: SampleCount,
    pub fullscreen: bool,
    pub scancode_status: FastHashMap<u32, bool>,
    pub camera_pitch: f32,
    pub camera_yaw: f32,
    pub camera_location: Vec3A,
    pub previous_profiling_stats: Option<Vec<GpuTimerScopeResult>>,
    pub last_mouse_delta: Option<DVec2>,
    pub grabber: Option<Grabber>,
}
impl Default for GameProgrammeSettings {
    fn default() -> Self {
        Self::new()
    }
}
impl GameProgrammeSettings {
    pub fn new() -> Self {
        let mut args = Arguments::from_vec(std::env::args_os().skip(1).collect());

        // Meta
        let help = args.contains(["-h", "--help"]);

        // Rendering
        let desired_backend = option_arg(
            args.opt_value_from_fn(["-b", "--backend"], extract_backend),
            HELP,
        );
        let desired_device_name: Option<String> =
            option_arg(args.opt_value_from_str(["-d", "--device"]), HELP)
                .map(|s: String| s.to_lowercase());
        let desired_mode = option_arg(
            args.opt_value_from_fn(["-p", "--profile"], extract_profile),
            HELP,
        );
        let samples = option_arg(args.opt_value_from_fn("--msaa", extract_msaa), HELP)
            .unwrap_or(SampleCount::Four);
        let present_mode = option_arg(
            args.opt_value_from_fn(["-v", "--vsync"], extract_vsync),
            HELP,
        )
        .unwrap_or(rend3::types::PresentMode::Immediate);

        // Windowing
        let absolute_mouse: bool = args.contains("--absolute-mouse");
        let fullscreen = args.contains("--fullscreen");

        // Assets
        let normal_direction = match args.contains("--normal-y-down") {
            true => NormalTextureYDirection::Down,
            false => NormalTextureYDirection::Up,
        };
        let directional_light_direction = option_arg(
            args.opt_value_from_fn("--directional-light", extract_vec3),
            HELP,
        );
        let directional_light_intensity: f32 = option_arg(
            args.opt_value_from_str("--directional-light-intensity"),
            HELP,
        )
        .unwrap_or(4.0);
        let ambient_light_level: f32 =
            option_arg(args.opt_value_from_str("--ambient"), HELP).unwrap_or(0.10);
        let scale: Option<f32> = option_arg(args.opt_value_from_str("--scale"), HELP);
        let shadow_distance: Option<f32> =
            option_arg(args.opt_value_from_str("--shadow-distance"), HELP);
        let shadow_resolution: u16 =
            option_arg(args.opt_value_from_str("--shadow-resolution"), HELP).unwrap_or(8192);
        let gltf_disable_directional_light: bool =
            args.contains("--gltf-disable-directional-lights");

        // Controls
        let walk_speed = args.value_from_str("--walk").unwrap_or(10.0_f32);
        let run_speed = args.value_from_str("--run").unwrap_or(50.0_f32);
        let camera_default = [
            3.0,
            3.0,
            3.0,
            -std::f32::consts::FRAC_PI_8,
            std::f32::consts::FRAC_PI_4,
        ];
        let camera_info = args
            .value_from_str("--camera")
            .map_or(camera_default, |s: String| {
                extract_array(&s, camera_default).unwrap()
            });

        // Free args
        let file_to_load: Option<String> = args.free_from_str().ok();

        let remaining = args.finish();

        if !remaining.is_empty() {
            eprint!("Unknown arguments:");
            for flag in remaining {
                eprint!(" '{}'", flag.to_string_lossy());
            }
            eprintln!("\n");

            eprintln!("{}", HELP);
            std::process::exit(1);
        }

        if help {
            eprintln!("{}", HELP);
            std::process::exit(1);
        }

        let mut gltf_settings = rend3_gltf::GltfLoadSettings {
            normal_direction,
            enable_directional: !gltf_disable_directional_light,
            ..Default::default()
        };
        if let Some(scale) = scale {
            gltf_settings.scale = scale
        }
        if let Some(shadow_distance) = shadow_distance {
            gltf_settings.directional_light_shadow_distance = shadow_distance;
        }

        gltf_settings.directional_light_resolution = shadow_resolution;

        Self {
            absolute_mouse,
            desired_backend,
            desired_device_name,
            desired_profile: desired_mode,
            file_to_load,
            walk_speed,
            run_speed,
            gltf_settings,
            directional_light_direction,
            directional_light_intensity,
            directional_light: None,
            ambient_light_level,
            present_mode,
            samples,

            fullscreen,

            scancode_status: FastHashMap::default(),
            camera_pitch: camera_info[3],
            camera_yaw: camera_info[4],
            camera_location: Vec3A::new(camera_info[0], camera_info[1], camera_info[2]),
            previous_profiling_stats: None,

            last_mouse_delta: None,

            grabber: None,
        }
    }
}
