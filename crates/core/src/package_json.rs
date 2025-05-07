use serde_json::Value;
use spinne_logger::Logger;
use std::collections::HashSet;
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
    /// The dependencies of the project.
    pub dependencies: Option<HashSet<String>>,
    /// The dev dependencies of the project.
    pub dev_dependencies: Option<HashSet<String>>,
    /// The peer dependencies of the project.
    pub peer_dependencies: Option<HashSet<String>>,
}

impl PackageJson {
    /// Creates a new PackageJson instance by reading package.json from the given path
    ///
    /// # Arguments
    ///
    /// * `path` - The path to the package.json file.
    /// * `with_dependencies` - Whether to parse dependencies in the package.json. This will increase usage of memory.
    pub fn read(path: &PathBuf, with_dependencies: bool) -> Option<Self> {
        if !path.exists() {
            Logger::error(&format!("No package.json found at {}", path.display()));
            return None;
        }

        let mut package_json = Self::default();
        package_json.path = path.clone();

        match fs::read_to_string(&path) {
            Ok(content) => match serde_json::from_str::<Value>(&content) {
                Ok(mut parsed) => {
                    if let Some(json_object) = parsed.as_object_mut() {
                        // remove large fields that we don't need
                        json_object.remove("scripts");
                        json_object.remove("optionalDependencies");
                        json_object.remove("resolutions");
                        json_object.remove("overrides");
                        json_object.remove("packageManager");
                        json_object.remove("engines");

                        // Add name
                        package_json.name = json_object
                            .get("name")
                            .and_then(|field| field.as_str())
                            .map(ToString::to_string);

                        // Add workspaces
                        package_json.workspaces =
                            Self::get_workspaces(json_object.get("workspaces"));

                        if with_dependencies {
                            // Add dependencies
                            package_json.dependencies =
                                Self::get_dependencies(json_object.get("dependencies"));
                            package_json.dev_dependencies =
                                Self::get_dependencies(json_object.get("devDependencies"));
                            package_json.peer_dependencies =
                                Self::get_dependencies(json_object.get("peerDependencies"));
                        }
                    }

                    Some(package_json)
                }
                Err(e) => {
                    Logger::error(&format!("Failed to parse package.json: {}", e));
                    None
                }
            },
            Err(e) => {
                Logger::error(&format!("Failed to read package.json: {}", e));
                None
            }
        }
    }

    /// Gets all dependencies (both regular and dev dependencies)
    pub fn get_all_dependencies(&self) -> Option<HashSet<String>> {
        let mut all_deps = HashSet::new();

        if let Some(deps) = &self.dependencies {
            all_deps.extend(deps.iter().cloned());
        }

        if let Some(dev_deps) = &self.dev_dependencies {
            all_deps.extend(dev_deps.iter().cloned());
        }

        if let Some(peer_deps) = &self.peer_dependencies {
            all_deps.extend(peer_deps.iter().cloned());
        }

        if all_deps.is_empty() {
            None
        } else {
            Some(all_deps)
        }
    }

    /// Finds a dependency by name in dependencies, devDependencies, or peerDependencies
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the dependency to find.
    pub fn find_dependency(&self, name: &str) -> Option<String> {
        if let Some(deps) = &self.dependencies {
            if deps.contains(name) {
                return Some(name.to_string());
            }
        }

        if let Some(dev_deps) = &self.dev_dependencies {
            if dev_deps.contains(name) {
                return Some(name.to_string());
            }
        }

        if let Some(peer_deps) = &self.peer_dependencies {
            if peer_deps.contains(name) {
                return Some(name.to_string());
            }
        }

        None
    }

    fn get_dependencies(deps_value: Option<&Value>) -> Option<HashSet<String>> {
        deps_value.and_then(|deps| {
            if let Some(obj) = deps.as_object() {
                let deps: HashSet<String> = obj.keys().cloned().collect();
                if deps.is_empty() {
                    None
                } else {
                    Some(deps)
                }
            } else {
                None
            }
        })
    }

    // TODO: resolve workspaces with blob support
    fn get_workspaces(json: Option<&Value>) -> Option<Vec<String>> {
        let workspaces = json.and_then(|field| field.as_array());

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
    use crate::util::test_utils::create_mock_project;

    use super::*;

    #[test]
    fn test_read_package_json() {
        let temp_dir = create_mock_project(&vec![(
            "package.json",
            r#"
            {
                "name": "test-project",
                "version": "1.0.0",
                "workspaces": ["packages/*"]
            }
            "#,
        )]);

        let package_json =
            PackageJson::read(&PathBuf::from(temp_dir.path().join("package.json")), true)
                .expect("Failed to read package.json");
        assert_eq!(package_json.name, Some("test-project".to_string()));
    }

    #[test]
    fn test_missing_package_json() {
        assert!(PackageJson::read(&PathBuf::from("package.json"), true).is_none());
    }

    #[test]
    fn test_resolves_workspaces() {
        let temp_dir = create_mock_project(&vec![(
            "package.json",
            r#"
        {
            "name": "test-project",
            "version": "1.0.0",
            "workspaces": ["packages/*"]
        }"#,
        )]);

        let package_json =
            PackageJson::read(&PathBuf::from(temp_dir.path().join("package.json")), true).unwrap();

        assert_eq!(
            package_json.workspaces,
            Some(vec!["packages/*".to_string()])
        );
    }

    #[test]
    fn test_get_all_dependencies() {
        let temp_dir = create_mock_project(&vec![(
            "package.json",
            r#"
            {
                "name": "test-project",
                "version": "1.0.0",
                "dependencies": { "react": "18.3.1" },
                "devDependencies": { "typescript": "5.0.0" }
            }
            "#,
        )]);

        let package_json =
            PackageJson::read(&PathBuf::from(temp_dir.path().join("package.json")), true).unwrap();

        assert_eq!(
            package_json.dependencies,
            Some(HashSet::from(["react".to_string()]))
        );
        assert_eq!(
            package_json.dev_dependencies,
            Some(HashSet::from(["typescript".to_string()]))
        );
    }

    #[test]
    fn test_find_dependency() {
        let temp_dir = create_mock_project(&vec![(
            "package.json",
            r#"
            {
                "name": "test-project",
                "version": "1.0.0",
                "dependencies": { "react": "18.3.1" },
                "devDependencies": { "typescript": "5.0.0" },
                "peerDependencies": { "react-dom": "18.3.1" }
            }
            "#,
        )]);

        let package_json =
            PackageJson::read(&PathBuf::from(temp_dir.path().join("package.json")), true).unwrap();

        assert_eq!(
            package_json.find_dependency("react"),
            Some("react".to_string())
        );
        assert_eq!(
            package_json.find_dependency("typescript"),
            Some("typescript".to_string())
        );
        assert_eq!(
            package_json.find_dependency("react-dom"),
            Some("react-dom".to_string())
        );
        assert_eq!(package_json.find_dependency("react-router"), None);
    }
}
