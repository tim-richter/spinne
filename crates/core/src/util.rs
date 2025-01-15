use std::path::PathBuf;

use spinne_logger::Logger;

/// Check if a string is in PascalCase.
/// We can only check here if the first character is uppercase because we don't know the context of the string.
pub fn is_pascal_case(name: &str) -> bool {
    name.chars().next().map_or(false, |c| c.is_uppercase())
}

pub fn reduce_to_node_module_name(path: &str) -> String {
    let last = path.split("node_modules/").last().unwrap().to_string();
    last.split('/').next().unwrap().to_string()
}

/// Replace an absolute path with a project name.
/// We prefix the path with the project name and return the new path.
/// If the path is not absolute, we return the original path because it's probably a node module.
pub fn replace_absolute_path_with_project_name(
    project_root: PathBuf,
    path: PathBuf,
    prepend_with: &str,
) -> PathBuf {
    if path.has_root() {
        let stripped_path = path.strip_prefix(project_root);

        if let Err(e) = stripped_path {
            Logger::error(&format!("Error stripping path: {:?}", e));
            return path;
        }

        let relative_path = stripped_path.unwrap().to_path_buf();

        return PathBuf::from(prepend_with).join(relative_path);
    }

    path
}

#[cfg(test)]
pub mod test_utils {
    use std::fs;
    use tempfile::TempDir;

    pub fn create_mock_project(files: &Vec<(&str, &str)>) -> TempDir {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create mock .tsx files
        for (path, content) in files {
            // create directories before creating files
            let file_path = root.join(path);
            if let Some(parent) = file_path.parent() {
                if parent != root {
                    fs::create_dir_all(parent).unwrap();
                }
            }
            fs::write(file_path, content).unwrap();
        }

        temp_dir
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn test_replace_path_with_relative_path_not_absolute() {
        assert_eq!(
            replace_absolute_path_with_project_name(
                PathBuf::from("/Users/asht/Projects/spinne"),
                PathBuf::from("src/main.tsx"),
                "test-project"
            ),
            PathBuf::from("src/main.tsx")
        );
    }

    #[test]
    fn test_reduce_to_node_module_name() {
        assert_eq!(
            reduce_to_node_module_name("node_modules/material-ui/index.tsx"),
            "material-ui"
        );
    }
}
