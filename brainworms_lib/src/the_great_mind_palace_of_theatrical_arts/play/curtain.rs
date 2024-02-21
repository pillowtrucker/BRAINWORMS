use std::{collections::HashMap, sync::Arc};

use parking_lot::Mutex;

use super::scene::actors::{ActressDefinition, AstinkSprite};

#[derive(Debug, Default)]
pub struct CurtainDefinition {
    pub backgrounds: Vec<(String, String)>,
    pub actors: Vec<ActressDefinition>,
}

pub struct CurtainImplementation {
    pub actresses: HashMap<String, Arc<Mutex<AstinkSprite>>>,
}
