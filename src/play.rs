pub mod stage2d;
pub mod stage3d;
#[cfg(target_arch = "wasm32")]
use crate::backstage::plumbing::resize_observer;
pub use parking_lot::{Mutex, MutexGuard};
/// I definitely want the minimal asset loader and the grabber from r3f
pub use rend3_framework::{AssetError, AssetLoader, AssetPath, Grabber, UserResizeEvent};
