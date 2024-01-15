fn vertex(pos: [f32; 3]) -> glam::Vec3 {
    glam::Vec3::from(pos)
}

fn uv(pos: [f32; 2]) -> glam::Vec2 {
    glam::Vec2::from(pos)
}

pub(crate) fn create_quad(size: f32) -> rend3::types::Mesh {
    let vertex_positions = [
        vertex([-size * 0.5, size * 0.5, 0.0]),
        vertex([size * 0.5, size * 0.5, 0.0]),
        vertex([size * 0.5, -size * 0.5, 0.0]),
        vertex([-size * 0.5, -size * 0.5, 0.0]),
    ];
    let uv_positions = [
        uv([0.0, 0.0]),
        uv([1.0, 0.0]),
        uv([1.0, 1.0]),
        uv([0.0, 1.0]),
    ];
    let index_data: &[u32] = &[0, 1, 2, 2, 3, 0];

    rend3::types::MeshBuilder::new(vertex_positions.to_vec(), rend3::types::Handedness::Left)
        .with_vertex_texture_coordinates_0(uv_positions.to_vec())
        .with_indices(index_data.to_vec())
        .build()
        .unwrap()
}
