use std::borrow::Cow;

use rend3::util::typedefs::SsoString;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AssetError {
    #[error("Could not read {path} from disk: {error}")]
    FileError {
        path: SsoString,
        #[source]
        error: std::io::Error,
    },
}

pub enum AssetPath<'a> {
    Internal(&'a str),
    External(&'a str),
}
impl<'a> AssetPath<'a> {
    fn get_path(self, base: &str) -> Cow<'a, str> {
        match self {
            Self::Internal(p) => Cow::Owned(base.to_owned() + p),
            Self::External(p) => Cow::Borrowed(p),
        }
    }
}

pub struct AssetLoader {
    base: SsoString,
}
impl Default for AssetLoader {
    fn default() -> Self {
        Self::new_local(
            //            concat!(env!("CARGO_MANIFEST_DIR"), "/"),
            "",
            "",
            "http://localhost:8000/",
        )
    }
}
impl AssetLoader {
    pub fn new_local(_base_file: &str, _base_asset: &str, _base_url: &str) -> Self {
        cfg_if::cfg_if!(
            if #[cfg(target_os = "android")] {
                let base = _base_asset;
            } else {
                let base = _base_file;
            }
        );

        Self {
            base: SsoString::from(base),
        }
    }

    pub fn get_asset_path<'a>(&self, path: AssetPath<'a>) -> Cow<'a, str> {
        path.get_path(&self.base)
    }

    #[cfg(not(target_os = "android"))]
    pub async fn get_asset(&self, path: AssetPath<'_>) -> Result<Vec<u8>, AssetError> {
        let full_path = path.get_path(&self.base);
        std::fs::read(&*full_path).map_err(|error| AssetError::FileError {
            path: SsoString::from(full_path),
            error,
        })
    }

    #[cfg(target_os = "android")]
    pub async fn get_asset(&self, path: AssetPath<'_>) -> Result<Vec<u8>, AssetError> {
        use std::ffi::CString;

        let manager = ndk_glue::native_activity().asset_manager();

        let full_path = path.get_path(&self.base);
        manager
            .open(&CString::new(&*full_path).unwrap())
            .ok_or_else(|| {
                AssetError::FileError(FileError {
                    path: SsoString::from(&*full_path),
                    error: std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        "could not find file in asset manager",
                    ),
                })
            })
            .and_then(|mut file| {
                file.get_buffer().map(|b| b.to_vec()).map_err(|error| {
                    AssetError::FileError(FileError {
                        path: SsoString::from(full_path),
                        error,
                    })
                })
            })
    }
}
