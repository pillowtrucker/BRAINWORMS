use std::collections::HashMap;

use rend3::types::ObjectHandle;

pub struct Prop {
    pub object: ObjectHandle,
    pub raw_textures: Option<HashMap<String, wgpu::Texture>>,
}
