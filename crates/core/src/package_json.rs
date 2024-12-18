use serde_json::Value;
use spinne_logger::Logger;
use std::fs;
use std::path::PathBuf;

/// Handles interactions with package.json
#[derive(Debug, Clone, Default)]
pub struct PackageJson {
    /// Path to `package.json`. Contains the `package.json` filename.
    pub path: PathBuf,
    /// The name of the project.
    pub name: Option<String>,
    /// The workspaces of the project.
    pub workspaces: Option<Vec<String>>,
}

impl PackageJson {
    /// Creates a new PackageJson instance by reading package.json from the current directory
    pub fn read(path: PathBuf) -> Option<Self> {
        if !path.exists() {
            Logger::debug(&format!("No package.json found at {}", path.display()), 1);
            return None;
        }

        let mut package_json = Self::default();

        match fs::read_to_string(&path) {
            Ok(content) => match serde_json::from_str::<Value>(&content) {
                Ok(mut parsed) => {
                    Logger::debug("Successfully read package.json", 1);
                    if let Some(json_object) = parsed.as_object_mut() {
                        // Remove large fields that are useless for pragmatic use.
                        json_object.remove("description");
                        json_object.remove("keywords");
                        json_object.remove("scripts");
                        json_object.remove("dependencies");
                        json_object.remove("devDependencies");
                        json_object.remove("peerDependencies");
                        json_object.remove("optionalDependencies");

                        // Add name
                        package_json.name = json_object
                            .get("name")
                            .and_then(|field| field.as_str())
                            .map(ToString::to_string);

                        // Add workspaces
                        package_json.workspaces = Self::get_workspaces(&parsed);
                    }

                    Some(package_json)
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

    // TODO: resolve workspaces with blob support
    fn get_workspaces(json: &Value) -> Option<Vec<String>> {
        let workspaces = json.get("workspaces").and_then(|field| field.as_array());

        match workspaces {
            Some(workspaces) => Some(
                workspaces
                    .iter()
                    .map(|item| item.as_str().unwrap().to_string())
                    .collect(),
            ),
            None => None,
        }
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
                "workspaces": ["packages/*"],
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
        let temp_dir = setup_test_dir();

        let package_json = PackageJson::read(PathBuf::from(temp_dir.path().join("package.json")))
            .expect("Failed to read package.json");
        assert_eq!(package_json.name, Some("test-project".to_string()));
    }

    #[test]
    fn test_missing_package_json() {
        assert!(PackageJson::read(PathBuf::from("package.json")).is_none());
    }

    #[test]
    fn test_resolves_workspaces() {
        let temp_dir = setup_test_dir();

        let package_json =
            PackageJson::read(PathBuf::from(temp_dir.path().join("package.json"))).unwrap();

        assert_eq!(
            package_json.workspaces,
            Some(vec!["packages/*".to_string()])
        );
    }
}
