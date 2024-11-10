use std::fs;
use std::path::PathBuf;

use crate::logging::Logger;

/// Handles interactions with tsconfig.json
pub struct TsConfigReader;

impl TsConfigReader {
    /// Read the tsconfig.json file and return the baseUrl and paths.
    pub fn read_tsconfig(tsconfig_path: &PathBuf) -> (PathBuf, Vec<(String, Vec<String>)>) {
        Logger::debug(&format!("Reading tsconfig.json from: {:?}", tsconfig_path), 1);

        match fs::read_to_string(tsconfig_path) {
            Ok(content) => {
                match jsonc_parser::parse_to_serde_value(&content, &Default::default()) {
                    Ok(json) => {
                        match json {
                            Some(json) => {
                                // TODO: Use a better parser with support for extends
                                let base_url = if let Some(base_url) = json["compilerOptions"]["baseUrl"].as_str() {
                                    match PathBuf::from(base_url).canonicalize() {
                                        Ok(path) => path,
                                        Err(e) => {
                                            Logger::debug(&format!("Failed to canonicalize baseUrl: {}", e), 1);
                                            PathBuf::from("")
                                        }
                                    }
                                } else {
                                    Logger::debug("No baseUrl found in tsconfig.json", 1);
                                    PathBuf::from("")
                                };

                                let paths = if let Some(paths) = json["compilerOptions"]["paths"].as_object() {
                                    paths
                                            .iter()
                                            .map(|(key, value)| {
                                                (
                                                    key.clone(),
                                                    value
                                                        .as_array()
                                                        .unwrap_or(&Vec::new())
                                                        .iter()
                                                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                                        .collect(),
                                                )
                                            })
                                            .collect()
                                } else {
                                    Logger::debug("No paths found in tsconfig.json", 1);
                                    Vec::new()
                                };

                                return (PathBuf::from(base_url), paths);
                            }
                            None => {
                                Logger::debug("Failed to parse tsconfig.json, no content", 1);
                            }
                        }
                    }
                    Err(e) => {
                        Logger::debug(&format!("Failed to parse tsconfig.json: {}", e), 1);
                    }
                }
            }
            Err(e) => {
                Logger::debug(&format!("Failed to read tsconfig.json: {}", e), 1);
            }
        }

        Logger::debug("Falling back to default values for tsconfig.json", 1);
        (PathBuf::from(""), Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use std::env;

    use tempfile::TempDir;

    use super::*;

    #[test]
    fn test_read_tsconfig() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        let tsconfig_path = root.join("tsconfig.json");
        let tsconfig_content = format!(r#"
            {{
                "compilerOptions": {{
                    "baseUrl": "{}",
                    "paths": {{
                        "@components/*": ["src/components/*"]
                    }}
                }}
            }}
        "#, root.to_string_lossy());
        fs::write(&tsconfig_path, tsconfig_content).unwrap();
        let (base_url, paths) = TsConfigReader::read_tsconfig(&tsconfig_path);

        assert_eq!(base_url, PathBuf::from(root));
        assert_eq!(
            paths,
            vec![("@components/*".to_string(), vec!["src/components/*".to_string()])]
        );
    }

    #[test]
    fn test_read_tsconfig_no_file() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        let tsconfig_path = root.join("tsconfig.json");
        let (base_url, paths) = TsConfigReader::read_tsconfig(&tsconfig_path);

        assert_eq!(base_url, PathBuf::from(""));
        assert_eq!(paths, Vec::new());
    }

    #[test]
    fn test_read_tsconfig_no_base_url() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        let tsconfig_path = root.join("tsconfig.json");
        fs::write(
            &tsconfig_path,
            r#"
            {
                "compilerOptions": {
                    "paths": {
                        "@/*": ["src/*"]
                    }
                }
            }
        "#,
        )
        .unwrap();
        let (base_url, paths) = TsConfigReader::read_tsconfig(&tsconfig_path);

        assert_eq!(base_url, PathBuf::from(""));
        assert_eq!(paths, vec![("@/*".to_string(), vec!["src/*".to_string()])]);
    }

    #[test]
    fn test_read_tsconfig_no_paths() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        let tsconfig_path = root.join("tsconfig.json");
        let tsconfig_content = format!(r#"
            {{
                "compilerOptions": {{
                    "baseUrl": "{}"
                }}
            }}
        "#, root.to_string_lossy());
        fs::write(&tsconfig_path, tsconfig_content).unwrap();

        let (base_url, paths) = TsConfigReader::read_tsconfig(&tsconfig_path);

        assert_eq!(base_url, PathBuf::from(root));
        assert_eq!(paths, Vec::new());
    }

    #[test]
    fn should_parse_jsonc() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        let tsconfig_path = root.join("tsconfig.json");
        let tsconfig_content = format!(r#"
            {{
                "compilerOptions": {{
                    /* comment */
                    "baseUrl": "{}",
                    // another comment
                    "paths": {{
                        "@/*": ["src/*"]
                    }}
                }}
            }}
        "#, root.to_string_lossy());
        println!("tsconfig_content: {}", tsconfig_content);
        fs::write(&tsconfig_path, tsconfig_content).unwrap();

        let (base_url, paths) = TsConfigReader::read_tsconfig(&tsconfig_path);

        assert_eq!(base_url, PathBuf::from(root));
        assert_eq!(paths, vec![("@/*".to_string(), vec!["src/*".to_string()])]);
    }

    #[test]
    fn should_convert_base_url_to_absolute_path() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        env::set_current_dir(root).unwrap();

        let tsconfig_path = root.join("tsconfig.json");
        fs::write(
            &tsconfig_path,
            r#"
            {
                "compilerOptions": {
                    "baseUrl": "."
                }
            }
        "#,
        )
        .unwrap();
        let (base_url, paths) = TsConfigReader::read_tsconfig(&tsconfig_path);

        assert_eq!(base_url, PathBuf::from(root));
        assert_eq!(paths, Vec::new());
    }
}
