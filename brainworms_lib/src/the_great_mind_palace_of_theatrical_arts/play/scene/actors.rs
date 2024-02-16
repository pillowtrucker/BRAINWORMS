use std::{num::NonZeroU32, sync::Arc};

use glam::uvec2;
use inox2d::formats::inp::parse_inp;
use parking_lot::Mutex;
use rend3::{types::MipmapCount, Renderer};
use uuid::Uuid;
use wgpu::TextureFormat;
use winit::event_loop::EventLoopProxy;

use crate::{
    theater::{
        basement::quad_damage::create_quad,
        play::backstage::plumbing::asset_loader::{AssetLoader, AssetPath},
    },
    MyEvent, MyWinitEvent,
};
#[derive(Clone, Debug)]
pub struct ActressDefinition {
    pub name: String,
    pub directory: String,
    pub transform: glam::Mat4,
    pub size: f32,
}
pub struct Actress {
    pub sprite_object_handle: rend3::types::ObjectHandle,
    pub texture_wgpu: wgpu::Texture,
    pub texture_rend3_handle: rend3::types::Texture2DHandle,
    pub inox_renderer: inox2d_wgpu::Renderer,
    pub texture_wgpu_view: wgpu::TextureView,
    pub inox_model: inox2d::model::Model,
}
pub enum AstinkSprite {
    Loading,
    Loaded((String, uuid::Uuid, Actress)),
}

pub(crate) fn draw_actor(a: Arc<Mutex<AstinkSprite>>, renderer: Arc<Renderer>, t: f32, dt: f32) {
    let mut a = a.lock();
    let AstinkSprite::Loaded((_, _, ref mut actress)) = *a else {
        return;
    };
    // animate puppet
    {
        let puppet = &mut actress.inox_model.puppet;

        puppet.begin_set_params();

        puppet.set_named_param("Head:: Yaw-Pitch", glam25compat::vec2(t.cos(), t.sin()));

        puppet.end_set_params(dt);
    }

    let inox_texture_rend3_handle = actress.texture_rend3_handle.clone();
    let ir = &mut actress.inox_renderer;
    let dc = renderer.data_core.lock();
    let inox_texture_wgpu_view = &actress.texture_wgpu_view;
    let inox_texture_wgpu = &actress.texture_wgpu;
    let inox_texture_rend3_raw = &dc
        .d2_texture_manager
        .get_internal(inox_texture_rend3_handle.get_raw())
        .texture;
    // render to the inox texture
    ir.render(
        &renderer.queue,
        &renderer.device,
        &actress.inox_model.puppet,
        inox_texture_wgpu_view,
    );
    // copy the data into sprite material texture
    let mut encoder = renderer
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Part Render Encoder"),
        });

    encoder.copy_texture_to_texture(
        inox_texture_wgpu.as_image_copy(),
        inox_texture_rend3_raw.as_image_copy(),
        inox_texture_wgpu.size(),
    );

    renderer.queue.submit(std::iter::once(encoder.finish()));
}
pub async fn create_actor(
    name: String,
    directory: String,
    renderer: Arc<Renderer>,
    event_loop_proxy: EventLoopProxy<MyEvent>,
    transform: glam::Mat4,
    size: f32,
    sc_id: Uuid,
) {
    let path = format!("{}/{}.inp", &directory, name);
    let format = TextureFormat::Bgra8Unorm;
    let texture_size_uvec2 = glam25compat::uvec2(8192, 8192); // we no longer care about the surface size for the sprite texture

    let texture_size = wgpu::Extent3d {
        width: texture_size_uvec2.x,
        height: texture_size_uvec2.y,
        depth_or_array_layers: 1,
    };
    let loader = AssetLoader::default();
    let loaded_data = loader.get_asset(AssetPath::Internal(&path)).await.unwrap();
    let inox_model = parse_inp(loaded_data.as_slice()).unwrap();

    let mut inox_renderer = inox2d_wgpu::Renderer::new(
        &renderer.device,
        &renderer.queue,
        format,
        &inox_model,
        texture_size_uvec2,
    );

    inox_renderer.camera.scale = glam25compat::Vec2::splat(1.0);

    let inox_texture_descriptor = wgpu::TextureDescriptor {
        size: texture_size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::RENDER_ATTACHMENT,
        label: Some("inox texture"),
        view_formats: &[wgpu::TextureFormat::Bgra8Unorm],
    };

    let inox_texture_wgpu = renderer.device.create_texture(&inox_texture_descriptor);

    let inox_texture_wgpu_view = inox_texture_wgpu.create_view(&wgpu::TextureViewDescriptor {
        mip_level_count: None,
        base_mip_level: 0,
        ..Default::default()
    });
    let texture_size_uvec2 = uvec2(8192, 8192); // we no longer care about the surface size for the sprite texture
    let inox_texture_rend3 = rend3::types::Texture {
        label: Some("inox texture but rend3".to_owned()),
        format,
        size: texture_size_uvec2,
        mip_count: MipmapCount::Specific(NonZeroU32::new(1).unwrap()),

        mip_source: rend3::types::MipmapSource::Uploaded,
        data: vec![0; (texture_size_uvec2.x * texture_size_uvec2.y * 4) as usize],
    };
    let inox_texture_rend3_handle = renderer.add_texture_2d(inox_texture_rend3).unwrap();

    // Create mesh and calculate smooth normals based on vertices
    let sprite_mesh = create_quad(size);
    // Add mesh to renderer's world.
    //
    // All handles are refcounted, so we only need to hang onto the handle until we
    // make an object.
    let sprite_mesh_handle = renderer.add_mesh(sprite_mesh).unwrap();
    let sprite_material = rend3_routine::pbr::PbrMaterial {
        albedo: rend3_routine::pbr::AlbedoComponent::Texture(inox_texture_rend3_handle.clone()),
        transparency: rend3_routine::pbr::Transparency::Blend,
        ..Default::default()
    };

    let sprite_material_handle = renderer.add_material(sprite_material);
    // Combine the mesh and the material with a location to give an object.
    let sprite_object = rend3::types::Object {
        mesh_kind: rend3::types::ObjectMeshKind::Static(sprite_mesh_handle),
        material: sprite_material_handle.clone(),
        transform,
    };

    // Creating an object will hold onto both the mesh and the material
    // even if they are deleted.
    //
    // We need to keep the object handle alive.
    let sprite_object_handle = renderer.add_object(sprite_object);

    let built_actress = Actress {
        sprite_object_handle,
        texture_wgpu: inox_texture_wgpu,
        texture_rend3_handle: inox_texture_rend3_handle,
        inox_renderer,
        texture_wgpu_view: inox_texture_wgpu_view,
        inox_model,
    };
    let _ = event_loop_proxy.send_event(MyWinitEvent::Actress(AstinkSprite::Loaded((
        name,
        sc_id,
        built_actress,
    ))));
}
