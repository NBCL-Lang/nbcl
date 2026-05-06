use std::path::{PathBuf, Path};
use crate::error::{Result, NbclError};

#[derive(Debug, Clone)]
pub struct FileModuleResolver {
    file_path: PathBuf
}

impl FileModuleResolver {
    pub fn new(fpath: PathBuf) -> Self {
        Self {
            file_path: fpath,
        }
    }

    pub fn find_target(&self, path_str: &str) -> Result<PathBuf> {
        // Determine the base directory
        // If file_path is "main.nbl", parent() might be empty, so we default to "."
        let current_dir = self.file_path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));

        // Join and attempt to resolve the absolute path
        let full_path = current_dir.join(path_str);

        full_path.canonicalize().map_err(|e| {
            let message = match e.kind() {
                std::io::ErrorKind::NotFound => {
                    format!("Module not found: '{}' (searched in '{}')", path_str, current_dir.display())
                }
                std::io::ErrorKind::PermissionDenied => {
                    format!("Permission denied accessing module: '{}'", path_str)
                }
                _ => format!("Failed to resolve module path '{}': {}", path_str, e),
            };

            NbclError::IO {
                message,
                path: full_path,
            }
        })
    }
}