use std::path::{Path, PathBuf};

/// Normalize a file path to an absolute path.
pub fn normalize_path(file_path: &str) -> PathBuf {
  Path::new(file_path)
      .canonicalize()
      .unwrap_or_else(|_| Path::new(file_path).to_path_buf())
}