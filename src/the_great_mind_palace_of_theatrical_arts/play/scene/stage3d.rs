// how to set the stage for a 3d scene
// for now I'm going to test and experiment in main() and then dump the results here

use std::{
    collections::{BTreeMap, HashMap, VecDeque},
    future::Future,
    hash::BuildHasher,
    path::Path,
    sync::Arc,
};
pub struct Colliders {
    pub col_map: HashMap<String, Vec<parry3d::shape::TriMesh>>,
}
use glam::UVec2;

use log::info;
use nalgebra::{Isometry3, Matrix, Point3, Translation3};
use parking_lot::Mutex;

use rend3::{
    types::{CameraProjection, Handedness, Texture, TextureFormat},
    util::typedefs::SsoString,
    Renderer,
};
use rend3_gltf::{GltfLoadError, GltfLoadSettings, GltfSceneInstance};
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
                    //                    info!("vertices {:?}", vertex_positions);
                    if let Some(indices) = reader.read_indices() {
                        //                        info!("indices: {:?}", indices);
                        let mut new_trimesh = parry3d::shape::TriMesh::new(
                            vertex_positions,
                            indices.into_u32().array_chunks().collect(),
                        );
                        //                        let transform = IsometryMatrix3::new();
                        let (s, r, t) = transform.to_scale_rotation_translation();
                        let fff = Isometry3::from_parts(Translation3::new(t.x, t.y, t.z), r.into());
                        /*
                        match Isometry3::try_from(transform.as_dmat4()) {
                            Ok(transform) => {
                                info!("Actually successfully transformed {}", thename);
                                new_trimesh.transform_vertices(&transform.cast());
                            }
                            Err(e) => {
                                info!("no transform for {} because {:?}", thename, e);
                            }
                        }
                        */
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
