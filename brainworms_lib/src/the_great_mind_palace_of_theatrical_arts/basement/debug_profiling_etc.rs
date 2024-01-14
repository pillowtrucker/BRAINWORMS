use std::{path::Path, sync::Arc};

use rend3::types::Handedness;

use crate::theater::play::scene::stage3d::draw_line;

use super::cla::GameProgrammeSettings;

pub fn write_profiling_json(stats: &Option<&Vec<wgpu_profiler::GpuTimerScopeResult>>) {
    // write out gpu side performance info into a trace readable by chrome://tracing
    if let Some(ref stats) = stats {
        println!("Outputing gpu timing chrome trace to profile.json");
        wgpu_profiler::chrometrace::write_chrometrace(Path::new("profile.json"), stats).unwrap();
    } else {
        println!("No gpu timing trace available, either timestamp queries are unsupported or not enough frames have elapsed yet!");
    }
}
pub enum DebugPickingDoodad {
    TheRay,
    TheColliderShape,
}

pub fn draw_debug_mouse_picking_doodad(
    which: DebugPickingDoodad,
    cam_point: &nalgebra::Point3<f32>,
    intersection: &nalgebra::Point3<f32>,
    renderer: &Arc<rend3::Renderer>,
    handedness: Handedness,
    c: &parry3d::shape::TriMesh,
) {
    let cam_x = cam_point.x;
    let cam_y = cam_point.y;
    let cam_z = cam_point.z;

    let doodad_material_handle = renderer.add_material(rend3_routine::pbr::PbrMaterial::default());
    match which {
        DebugPickingDoodad::TheRay => {
            let line = draw_line(vec![
                [cam_x, cam_y, cam_z],
                [intersection.x, intersection.y, intersection.z],
            ]);
            let line_mesh_handle = renderer.add_mesh(line).unwrap();
            let line_mesh_object = rend3::types::Object {
                mesh_kind: rend3::types::ObjectMeshKind::Static(line_mesh_handle),
                material: doodad_material_handle,
                transform: glam::Mat4::from_scale_rotation_translation(
                    glam::Vec3::new(1.0, 1.0, 1.0),
                    glam::Quat::from_euler(glam::EulerRot::XYZ, 0.0, 0.0, 0.0),
                    glam::Vec3::new(0.0, 0.0, 0.0),
                ),
            };
            Box::leak(Box::new(renderer.add_object(line_mesh_object)));
        }
        DebugPickingDoodad::TheColliderShape => {
            let col_mesh_material_handle = doodad_material_handle;
            let col_mesh = rend3::types::MeshBuilder::new(
                c.vertices()
                    .iter()
                    .map(|v| glam::Vec3::new(v.x, v.y, v.z))
                    .collect(),
                handedness,
            )
            .with_indices(c.flat_indices().into())
            .build()
            .unwrap();
            let col_mesh_handle = renderer.add_mesh(col_mesh).unwrap();
            let col_mesh_object = rend3::types::Object {
                mesh_kind: rend3::types::ObjectMeshKind::Static(col_mesh_handle),
                material: col_mesh_material_handle,
                transform: glam::Mat4::from_scale_rotation_translation(
                    glam::Vec3::new(1.0, 1.0, 1.0),
                    glam::Quat::from_euler(glam::EulerRot::XYZ, 0.0, 0.0, 0.0),
                    glam::Vec3::new(0.0, 0.0, 0.0),
                ),
            };
            Box::leak(Box::new(renderer.add_object(col_mesh_object)));
        }
    }
}
