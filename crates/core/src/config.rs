use std::{fs, path::PathBuf};

use serde_json::Value;
use spinne_logger::Logger;

#[derive(Debug, PartialEq, Clone)]
pub struct ConfigValues {
    pub exclude: Option<Vec<String>>,
    pub include: Option<Vec<String>>,
    pub entry_points: Option<Vec<String>>,
}

/// Represents the config file
///
/// The config file is a JSON file that contains the configuration for the project.
/// If the config file is not found, the default values will be used.
pub struct Config {
    pub path: PathBuf,
}

impl Config {
    /// Reads the config file and returns values
    pub fn read(path: PathBuf) -> Option<ConfigValues> {
        let config = fs::read_to_string(path);

        if config.is_err() {
            Logger::error("Failed to read config file");
            return None;
        }

        match serde_json::from_str::<Value>(&config.unwrap()) {
            Ok(value) => {
                let exclude_value = value.get("exclude");
                let include_value = value.get("include");
                let entry_points_value = value.get("entry_points");

                let exclude = match exclude_value {
                    Some(value) => Some(Self::get_array_of_strings(value)),
                    None => None,
                };

                let include = match include_value {
                    Some(value) => Some(Self::get_array_of_strings(value)),
                    None => None,
                };

                let entry_points = match entry_points_value {
                    Some(value) => Some(Self::get_array_of_strings(value)),
                    None => None,
                };

                Some(ConfigValues {
                    exclude,
                    include,
                    entry_points,
                })
            }
            Err(err) => {
                Logger::error("Failed to parse config file");
                Logger::error(&err.to_string());
                return None;
            }
        }
    }

    /// Maps a Value to an array of strings
    fn get_array_of_strings(value: &Value) -> Vec<String> {
        value
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .map(|v| v.as_str().unwrap_or("").to_string())
            .collect::<Vec<String>>()
    }
}

#[cfg(test)]
mod tests {
    use crate::util::test_utils::create_mock_project;

    use super::*;

    #[test]
    fn test_config_read() {
        let temp_dir = create_mock_project(&vec![(
            "spinne.json",
            r#"{"exclude": ["test.tsx"], "include": ["test.tsx"], "entry_points": ["src/index.tsx"]}"#,
        )]);
        let config = Config::read(temp_dir.path().join("spinne.json"));

        assert_eq!(
            config,
            Some(ConfigValues {
                exclude: Some(vec!["test.tsx".to_string()]),
                include: Some(vec!["test.tsx".to_string()]),
                entry_points: Some(vec!["src/index.tsx".to_string()])
            })
        );
    }

    #[test]
    fn test_config_read_no_config() {
        let temp_dir = create_mock_project(&vec![]);
        let config = Config::read(temp_dir.path().join("spinne.json"));

        assert_eq!(config, None);
    }

    #[test]
    fn test_config_read_invalid_config() {
        let temp_dir = create_mock_project(&vec![("spinne.json", r#"{"]ht["te)}"#)]);
        let config = Config::read(temp_dir.path().join("spinne.json"));

        assert_eq!(config, None);
    }

    #[test]
    fn test_config_without_include() {
        let temp_dir = create_mock_project(&vec![("spinne.json", r#"{"exclude": ["test.tsx"]}"#)]);
        let config = Config::read(temp_dir.path().join("spinne.json"));

        assert_eq!(
            config,
            Some(ConfigValues {
                exclude: Some(vec!["test.tsx".to_string()]),
                include: None,
                entry_points: None
            })
        );
    }

    #[test]
    fn test_config_without_exclude() {
        let temp_dir = create_mock_project(&vec![("spinne.json", r#"{"include": ["test.tsx"]}"#)]);
        let config = Config::read(temp_dir.path().join("spinne.json"));

        assert_eq!(
            config,
            Some(ConfigValues {
                exclude: None,
                include: Some(vec!["test.tsx".to_string()]),
                entry_points: None
            })
        );
    }

    #[test]
    fn test_config_without_entry_points() {
        let temp_dir = create_mock_project(&vec![("spinne.json", r#"{"include": ["test.tsx"]}"#)]);
        let config = Config::read(temp_dir.path().join("spinne.json"));

        assert_eq!(
            config,
            Some(ConfigValues {
                exclude: None,
                include: Some(vec!["test.tsx".to_string()]),
                entry_points: None
            })
        );
    }

    #[test]
    fn test_config_with_only_entry_points() {
        let temp_dir = create_mock_project(&vec![(
            "spinne.json",
            r#"{"entry_points": ["src/index.tsx", "src/components/index.ts"]}"#,
        )]);
        let config = Config::read(temp_dir.path().join("spinne.json"));

        assert_eq!(
            config,
            Some(ConfigValues {
                exclude: None,
                include: None,
                entry_points: Some(vec![
                    "src/index.tsx".to_string(),
                    "src/components/index.ts".to_string()
                ])
            })
        );
    }
}
