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

pub fn replace_absolute_path_with_project_name(
    project_root: PathBuf,
    path: PathBuf,
    prepend_with: &str,
) -> PathBuf {
    let relative_path = path.strip_prefix(project_root).unwrap().to_path_buf();

    PathBuf::from(prepend_with).join(relative_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_path() {
        assert_eq!(
            normalize_path("src/main.tsx"),
            PathBuf::from("src/main.tsx")
        );
    }

    #[test]
    fn test_is_pascal_case() {
        assert_eq!(is_pascal_case("PascalCase"), true);
        assert_eq!(is_pascal_case("camelCase"), false);
        assert_eq!(is_pascal_case("kebab-case"), false);
        assert_eq!(is_pascal_case("snake_case"), false);
        assert_eq!(is_pascal_case("SCREAMING_SNAKE_CASE"), true);
    }

    #[test]
    fn test_replace_path_with_relative_path() {
        assert_eq!(
            replace_absolute_path_with_project_name(
                PathBuf::from("/Users/asht/Projects/spinne"),
                PathBuf::from("/Users/asht/Projects/spinne/src/main.tsx"),
                "test-project"
            ),
            PathBuf::from("test-project/src/main.tsx")
        );
    }
}
