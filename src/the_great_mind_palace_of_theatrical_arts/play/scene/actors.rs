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
