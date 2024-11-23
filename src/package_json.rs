use crate::logging::Logger;
use serde_json::Value;
use std::fs;
use std::path::PathBuf;

/// Handles interactions with package.json
#[derive(Debug, Clone)]
pub struct PackageJson {
    content: Value,
}

impl PackageJson {
    /// Creates a new PackageJson instance by reading package.json from the current directory
    pub fn read() -> Option<Self> {
        let package_json_path = PathBuf::from("package.json");

        if !package_json_path.exists() {
            Logger::debug("No package.json found in current directory", 1);
            return None;
        }

        match fs::read_to_string(&package_json_path) {
            Ok(content) => match serde_json::from_str(&content) {
                Ok(parsed) => {
                    Logger::debug("Successfully read package.json", 1);
                    Some(PackageJson { content: parsed })
                }
                Err(e) => {
                    Logger::debug(&format!("Failed to parse package.json: {}", e), 1);
                    None
                }
            },
            Err(e) => {
                Logger::debug(&format!("Failed to read package.json: {}", e), 1);
                None
            }
        }
    }

    /// Gets the package name
    pub fn name(&self) -> Option<&str> {
        self.content.get("name")?.as_str()
    }

    /// Gets a value from package.json using a dot-notation path
    pub fn get(&self, path: &str) -> Option<&Value> {
        let parts: Vec<&str> = path.split('.').collect();
        let mut current = &self.content;

        for part in parts {
            current = current.get(part)?;
        }

        Some(current)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use tempfile::TempDir;

    fn setup_test_dir() -> TempDir {
        let temp_dir = TempDir::new().unwrap();
        env::set_current_dir(&temp_dir).unwrap();

        fs::write(
            "package.json",
            r#"{
                "name": "test-project",
                "version": "1.0.0",
                "config": {
                    "components": {
                        "path": "src/components"
                    }
                }
            }"#,
        )
        .unwrap();

        temp_dir
    }

    #[test]
    fn test_read_package_json() {
        let _temp_dir = setup_test_dir();

        let package_json = PackageJson::read().expect("Failed to read package.json");
        assert_eq!(package_json.name(), Some("test-project"));
    }

    #[test]
    fn test_get_nested_value() {
        let _temp_dir = setup_test_dir();

        let package_json = PackageJson::read().expect("Failed to read package.json");
        assert_eq!(
            package_json
                .get("config.components.path")
                .and_then(|v| v.as_str()),
            Some("src/components")
        );
    }

    #[test]
    fn test_missing_package_json() {
        let temp_dir = TempDir::new().unwrap();
        env::set_current_dir(&temp_dir).unwrap();

        assert!(PackageJson::read().is_none());
    }
}
