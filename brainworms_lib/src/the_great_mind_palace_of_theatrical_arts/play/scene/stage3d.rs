// how to set the stage for a 3d scene
// for now I'm going to test and experiment in main() and then dump the results here

use std::{
    collections::{BTreeMap, HashMap, VecDeque},
    future::Future,
    path::Path,
    sync::Arc,
};
pub type ColliderName = String;
pub type ColliderMap = HashMap<ColliderName, Vec<parry3d::shape::TriMesh>>;
pub type CollisionMap = HashMap<ColliderName, Vec<nalgebra::OPoint<f32, nalgebra::Const<3>>>>;
pub struct Colliders {
    pub col_map: ColliderMap,
}
use glam::{vec3, vec4, Mat3A, UVec2};

use log::info;
use nalgebra::{Isometry3, Matrix, Point3, Translation3};
use parking_lot::Mutex;

use parry3d::query::{Ray, RayCast};
use rend3::{
    types::{CameraProjection, Handedness, Texture, TextureFormat},
    util::typedefs::SsoString,
    Renderer,
};
use rend3_gltf::{GltfLoadError, GltfLoadSettings, GltfSceneInstance};
use rend3_routine::skybox::SkyboxRoutine;
use uuid::Uuid;
use winit::{dpi::PhysicalPosition, event_loop::EventLoopProxy};

use std::time;

use crate::{
    theater::{
        basement::input_handling::InputContext,
        play::backstage::plumbing::asset_loader::{AssetError, AssetLoader, AssetPath},
    },
    GameProgrammeState, MyEvent, MyWinitEvent,
};

use super::{AstinkScene, CamInfo, Camera};

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

pub async fn load_stage3d(
    name: String,
    directory: String,
    sc_id: Uuid,
    renderer: Arc<Renderer>,
    gltf_settings: GltfLoadSettings,
    event_loop_proxy: EventLoopProxy<MyEvent>,
    collider_ids: HashMap<String, String>,
) {
    let loader = AssetLoader::default();
    let path = format!("{}/{}.glb", directory, name);

    let ret = load_gltf(
        &renderer,
        &loader,
        &gltf_settings,
        AssetPath::Internal(&path),
        collider_ids,
    )
    .await;
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
    collider_ids: HashMap<String, String>,
) -> Option<(rend3_gltf::LoadedGltfScene, GltfSceneInstance, Colliders)> {
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
    let io_func = |uri: SsoString| async {
        if let Some(base64) = rend3_gltf::try_load_base64(&uri) {
            Ok(base64)
        } else {
            log::info!("Loading resource {}", uri);
            let uri = uri;
            let full_uri = parent_str.clone() + "/" + uri.as_str();
            loader.get_asset(AssetPath::External(&full_uri)).await
        }
    };
    let resources_start = time::Instant::now();
    let Ok(colliders) = load_colliders_from_gltf(collider_ids, &gltf_data, io_func, settings).await
    else {
        panic!("fucked colliders");
    };
    info!("built colliders: {:?};", colliders.col_map.keys());
    let (scene, instance) = rend3_gltf::load_gltf(renderer, &gltf_data, settings, io_func)
        .await
        .unwrap();

    log::info!(
        "Loaded gltf in {:.3?}, resources loaded in {:.3?}",
        gltf_elapsed,
        resources_start.elapsed()
    );
    Some((scene, instance, colliders))
}

pub(crate) async fn load_colliders_from_gltf<F, Fut, E>(
    collider_ids: HashMap<String, String>,
    gltf_data: &[u8],
    io_func: F,
    settings: &GltfLoadSettings,
) -> Result<Colliders, GltfLoadError<E>>
where
    F: FnMut(SsoString) -> Fut,
    Fut: Future<Output = Result<Vec<u8>, E>>,
    E: std::error::Error + 'static,
{
    let mut file = gltf::Gltf::from_slice_without_validation(gltf_data)?;
    let mut out = Colliders {
        col_map: HashMap::default(),
    };
    let blob = file.blob.take();
    let buffers = rend3_gltf::load_buffers(file.buffers(), blob, io_func).await?;
    let nodes: Vec<gltf::Node<'_>> = file.nodes().collect();
    let (topological_order, parents) = node_indices_topological_sort(&nodes);
    let num_nodes = nodes.len();

    debug_assert_eq!(topological_order.len(), num_nodes);

    let node_transforms = vec![glam::Mat4::IDENTITY; num_nodes];
    let parent_transform = glam::Mat4::from_scale(glam::Vec3::new(
        settings.scale,
        settings.scale,
        settings.scale,
    ));
    for node_idx in topological_order.iter() {
        let node = &nodes[*node_idx];

        let local_transform = glam::Mat4::from_cols_array_2d(&node.transform().matrix());

        let parent_transform = parents
            .get(&node.index())
            .map(|p| node_transforms[*p])
            .unwrap_or(parent_transform);
        let transform = parent_transform * local_transform;
        //node_transforms[*node_idx] = transform;
        if let Some(m) = node.mesh() {
            //    for m in file.meshes() {
            if m.name()
                .is_some_and(|n| collider_ids.keys().any(|c| c == &n.to_owned()))
            {
                let thename = m.name().unwrap();
                info!("trying to build collider for {}", &thename);
                for p in m.primitives() {
                    if p.mode() != gltf::mesh::Mode::Triangles {
                        return Err(GltfLoadError::UnsupportedPrimitiveMode(
                            m.index(),
                            p.index(),
                            p.mode(),
                        ));
                    }
                    let reader = p.reader(|b| Some(&buffers[b.index()][..b.length()]));

                    let vertex_positions: Vec<_> = reader
                        .read_positions()
                        .ok_or_else(|| GltfLoadError::MissingPositions(m.index()))?
                        .map(Point3::from)
                        .collect();

                    if let Some(indices) = reader.read_indices() {
                        let mut new_trimesh = parry3d::shape::TriMesh::new(
                            vertex_positions,
                            indices.into_u32().array_chunks().collect(),
                        );
                        //                        let transform = IsometryMatrix3::new();
                        let (s, r, t) = transform.to_scale_rotation_translation();
                        let fff = Isometry3::from_parts(Translation3::new(t.x, t.y, t.z), r.into());

                        new_trimesh = new_trimesh.scaled(&Matrix::from(s));
                        new_trimesh.transform_vertices(&fff);

                        match out.col_map.get_mut(thename) {
                            Some(oldv) => {
                                oldv.push(new_trimesh);
                            }
                            None => {
                                out.col_map.insert(thename.to_owned(), vec![new_trimesh]);
                            }
                        };
                    }
                }
            }
        }
    }
    Ok(out)
}
fn node_indices_topological_sort(nodes: &[gltf::Node]) -> (Vec<usize>, BTreeMap<usize, usize>) {
    // NOTE: The algorithm uses BTreeMaps to guarantee consistent ordering.

    // Maps parent to list of children
    let mut children = BTreeMap::<usize, Vec<usize>>::new();
    for node in nodes {
        children.insert(node.index(), node.children().map(|n| n.index()).collect());
    }

    // Maps child to parent
    let parents: BTreeMap<usize, usize> = children
        .iter()
        .flat_map(|(parent, children)| children.iter().map(|ch| (*ch, *parent)))
        .collect();

    // Initialize the BFS queue with nodes that don't have any parent (i.e. roots)
    let mut queue: VecDeque<usize> = children
        .keys()
        .filter(|n| parents.get(n).is_none())
        .cloned()
        .collect();

    let mut topological_sort = Vec::<usize>::new();

    while let Some(n) = queue.pop_front() {
        topological_sort.push(n);
        for ch in &children[&n] {
            queue.push_back(*ch);
        }
    }

    (topological_sort, parents)
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
        info: CamInfo::from_arr(&cam_attributes),
        rotation: Mat3A::IDENTITY,
    }
}
pub(crate) fn compute_projection_matrix(
    data: rend3::types::Camera,
    handedness: Handedness,
    aspect_ratio: f32,
) -> glam::Mat4 {
    match data.projection {
        CameraProjection::Orthographic { size } => {
            let half = size * 0.5;
            if handedness == Handedness::Left {
                glam::Mat4::orthographic_lh(-half.x, half.x, -half.y, half.y, half.z, -half.z)
            } else {
                glam::Mat4::orthographic_rh(-half.x, half.x, -half.y, half.y, half.z, -half.z)
            }
        }
        CameraProjection::Perspective { vfov, near } => {
            if handedness == Handedness::Left {
                glam::Mat4::perspective_infinite_reverse_lh(vfov.to_radians(), aspect_ratio, near)
            } else {
                glam::Mat4::perspective_infinite_reverse_rh(vfov.to_radians(), aspect_ratio, near)
            }
        }
        CameraProjection::Raw(proj) => proj,
    }
}
pub fn make_ray(
    cur_camera: &Camera,
    mouse_physical_poz: &PhysicalPosition<f64>,
    win_w: f64,
    win_h: f64,
    handedness: Handedness,
) -> Ray {
    let cam_x = cur_camera.info.x;
    let cam_y = cur_camera.info.y;
    let cam_z = cur_camera.info.z;
    let cam_pitch = cur_camera.info.pitch;
    let cam_yaw = cur_camera.info.yaw;
    info!("{cam_x},{cam_y},{cam_z},{cam_pitch},{cam_yaw}",);
    let mouse_x = mouse_physical_poz.x;
    let mouse_y = mouse_physical_poz.y;
    info!("mouse at {},{}", mouse_x, mouse_y);

    let x = (2.0 * mouse_x) / win_w - 1.0;

    let y = 1.0 - (2.0 * mouse_y) / win_h;
    let z = 1.0;
    let ray_nds = vec3(x as f32, y as f32, z as f32);
    info!("ray_nds: {ray_nds}");
    let ray_clip = vec4(ray_nds.x, ray_nds.y, -1.0, 1.0);
    //        let cur_camera = make_camera(("".to_owned(), [cam_x, cam_y, cam_z, cam_pitch, cam_yaw]));
    let ray_eye = compute_projection_matrix(
        cur_camera.renderer_camera,
        handedness,
        (win_w / win_h) as f32,
    )
    .inverse()
        * ray_clip;
    let ray_eye = vec4(ray_eye.x, ray_eye.y, -1.0, 0.0);
    info!("ray_eye: {ray_eye}");
    let ray_wor4 = cur_camera.renderer_camera.view.inverse() * ray_eye;
    let ray_wor = vec3(ray_wor4.x, ray_wor4.y, ray_wor4.z);
    let ray_wor = ray_wor.normalize();
    info!("ray_world: {ray_wor}");
    Ray::new(nalgebra::Point3::new(cam_x, cam_y, cam_z), ray_wor.into())
}
const MAX_TOI: f32 = 100000.0;
pub fn get_collisions_from_camera(
    cur_camera: &Camera,
    mouse_physical_poz: &PhysicalPosition<f64>,
    win_w: f64,
    win_h: f64,
    handedness: Handedness,
    col_map: &ColliderMap,
) -> CollisionMap {
    let cam_point: nalgebra::Point3<f32> = cur_camera.info.location().into();
    let rayman = make_ray(cur_camera, mouse_physical_poz, win_w, win_h, handedness);
    let collisions: CollisionMap = col_map.iter().filter_map(
        |(c_name, colliders)| -> Option<(String, Vec<nalgebra::OPoint<f32, nalgebra::Const<3>>>)> {
            let intersections: Vec<_> = colliders
                .iter()
                .filter_map(|c| -> Option<nalgebra::OPoint<f32, nalgebra::Const<3>>> {
                    if let Some(toi) = c.cast_local_ray(&rayman, MAX_TOI, true) {
                        let intersection = rayman.point_at(toi);

                        if cam_point != intersection {
                            log::info!("{} intersects mouse ray at {}", c_name, intersection);
                            Some(intersection)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }).collect();
            if intersections.is_empty() {
                None
            } else {
                Some((c_name.clone(), intersections))
            }
        },
    ).collect();
    collisions
}
#[cfg(feature = "extra_debugging")]
pub fn draw_line(points: Vec<[f32; 3]>) -> rend3::types::Mesh {
    const WIDTH: f32 = 0.5;
    let mut vertices: Vec<glam::Vec3> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    let w = WIDTH / 2.0;

    let x1 = points[0][0];
    let x2 = points[1][0];
    let y1 = points[0][1];
    let y2 = points[1][1];
    let z1 = points[0][2];
    let z2 = points[1][2];

    vertices.push(glam::Vec3::from([x1 + w, y1 - w, z1]));
    vertices.push(glam::Vec3::from([x1 - w, y1 + w, z1]));
    vertices.push(glam::Vec3::from([x2 - w, y2 + w, z2]));
    vertices.push(glam::Vec3::from([x2 + w, y2 - w, z2]));

    indices.push(2);
    indices.push(1);
    indices.push(0);
    indices.push(2);
    indices.push(0);
    indices.push(3);

    rend3::types::MeshBuilder::new(vertices, rend3::types::Handedness::Right)
        .with_indices(indices)
        .build()
        .unwrap()
}

// I want to use ! for side effect dings but of course rust had a different idea so I will use the prefix do_ to distinguish do_update_camera as "update the actual camera view" from update_camera that just updates the parameters
pub fn do_update_camera<InputContextEnum: InputContext>(
    state: &mut GameProgrammeState<InputContextEnum>,
) {
    if let Some(cur_camera) = &mut state.cur_camera.lock().as_mut() {
        let view = glam::Mat4::from_euler(
            glam::EulerRot::XYZ,
            -cur_camera.info.pitch,
            -cur_camera.info.yaw,
            0.0,
        );
        let view = view * glam::Mat4::from_translation((-cur_camera.info.location()).into());
        let renderer_camera = rend3::types::Camera {
            projection: CameraProjection::Perspective {
                vfov: 60.0,
                near: 0.1,
            },
            view,
        };
        cur_camera.renderer_camera = renderer_camera;
        if let Some(renderer) = &state.renderer.lock().as_ref() {
            renderer.set_camera_data(renderer_camera);
        }
    }
}
pub fn update_camera_mouse_params<InputContextEnum: InputContext>(
    absolute_mouse: bool,
    state: &mut GameProgrammeState<InputContextEnum>,
    delta_x: f64,
    delta_y: f64,
) {
    if !state.grabber.lock().as_ref().unwrap().grabbed() {
        return;
    }

    const TAU: f32 = std::f32::consts::PI * 2.0;

    let mouse_delta = if absolute_mouse {
        let prev = state
            .input_status
            .lock()
            .last_mouse_delta
            .replace(glam::DVec2::new(delta_x, delta_y));
        if let Some(prev) = prev {
            (glam::DVec2::new(delta_x, delta_y) - prev) / 4.0
        } else {
            return;
        }
    } else {
        glam::DVec2::new(delta_x, delta_y)
    };
    if let Some(cur_camera) = &mut state.cur_camera.lock().as_mut() {
        cur_camera.info.yaw -= (mouse_delta.x / 1000.0) as f32;
        cur_camera.info.pitch -= (mouse_delta.y / 1000.0) as f32;
        if cur_camera.info.yaw < 0.0 {
            cur_camera.info.yaw += TAU;
        } else if cur_camera.info.yaw >= TAU {
            cur_camera.info.yaw -= TAU;
        }
        cur_camera.info.pitch = cur_camera.info.pitch.clamp(
            -std::f32::consts::FRAC_PI_2 + 0.0001,
            std::f32::consts::FRAC_PI_2 - 0.0001,
        )
    }
}

pub fn update_camera_rotation<InputContextEnum: InputContext>(
    state: &mut GameProgrammeState<InputContextEnum>,
) {
    if let Some(cur_camera) = &mut state.cur_camera.lock().as_mut() {
        cur_camera.rotation = glam::Mat3A::from_euler(
            glam::EulerRot::XYZ,
            -cur_camera.info.pitch,
            -cur_camera.info.yaw,
            0.0,
        )
        .transpose();
    }
}
