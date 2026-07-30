//! Module resolver for handling import statements
use crate::error::{NbclError, Result};
use std::path::{Path, PathBuf};

/// Trait for implementing custom module resolvers
pub trait ModuleResolver: Send + Sync + std::fmt::Debug {
    /// Resolves a module relative path into absolute path
    ///
    /// ## Paramters
    ///
    /// - current: Current file in which this import takes place.
    /// - path: The path the file is trying to import.
    fn find_target(&self, current: Option<PathBuf>, path: &str) -> Result<PathBuf>;
}

/// Simple module resolver.
#[derive(Debug, Clone)]
pub struct FileModuleResolver;

impl ModuleResolver for FileModuleResolver {
    /// Resolve a module based on relative string (e.g. test/node.nbl)
    fn find_target(&self, current_file: Option<PathBuf>, path_str: &str) -> Result<PathBuf> {
        // Determine the base directory
        /* 
         * We should always get the base directory based on the relative path of the 
         * current file where the import takes place.
         * If parent() is empty, so we default to "."
         */
        let relative_dir = Path::new(".");
        let current_dir = current_file
            .as_deref()
            .and_then(|cf| {
                cf.parent().filter(|p| !p.as_os_str().is_empty())
            })
            .unwrap_or(relative_dir);

        // Join and attempt to resolve the absolute path
        let full_path = current_dir.join(path_str);

        full_path.canonicalize().map_err(|e| {
            let (message, hint) = match e.kind() {
                std::io::ErrorKind::NotFound => {
                    let msg = format!(
                        "Module not found: '{}' (searched in '{}')",
                        path_str,
                        current_dir.display()
                    );
                    let hint =
                        "Ensure that the module exists and try adjusting the path".to_string();

                    (msg, Some(hint))
                }
                std::io::ErrorKind::PermissionDenied => {
                    let msg = format!("Permission denied accessing module: '{}'", path_str);
                    let hint = "Set proper file permissions and try again.".to_string();

                    (msg, Some(hint))
                }
                _ => (format!("Failed to resolve module path '{}': {}", path_str, e), None),
            };

            NbclError::IO { message, hint, path: full_path }
        })
    }
}
