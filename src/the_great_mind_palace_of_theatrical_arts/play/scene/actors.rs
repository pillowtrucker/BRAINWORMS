use std::sync::Arc;

use glam::vec2;
use parking_lot::Mutex;
use rend3::Renderer;

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

pub(crate) fn draw_actor(a: Arc<Mutex<AstinkSprite>>, renderer: Arc<Renderer>, t: f32) {
    let mut a = a.lock();
    let AstinkSprite::Loaded((_, _, ref mut actress)) = *a else {
        return;
    };
    // animate puppet
    {
        let puppet = &mut actress.inox_model.puppet;
        puppet.begin_set_params();

        puppet.set_param("Head:: Yaw-Pitch", vec2(t.cos(), t.sin()));

        puppet.end_set_params();
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
