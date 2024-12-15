use std::path::{Path, PathBuf};

/// Normalize a file path to an absolute path.
pub fn normalize_path(file_path: &str) -> PathBuf {
  Path::new(file_path)
      .canonicalize()
      .unwrap_or_else(|_| Path::new(file_path).to_path_buf())
}

pub fn is_pascal_case(name: &str) -> bool {
  name.chars().next().map_or(false, |c| c.is_uppercase())
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_normalize_path() {
    assert_eq!(normalize_path("src/main.tsx"), PathBuf::from("src/main.tsx"));
  }

  #[test]
  fn test_is_pascal_case() {
    assert_eq!(is_pascal_case("PascalCase"), true);
    assert_eq!(is_pascal_case("camelCase"), false);
    assert_eq!(is_pascal_case("kebab-case"), false);
    assert_eq!(is_pascal_case("snake_case"), false);
    assert_eq!(is_pascal_case("SCREAMING_SNAKE_CASE"), true);
  }
}
