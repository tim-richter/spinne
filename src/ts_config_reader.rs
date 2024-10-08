use std::path::PathBuf;
use std::fs;
use serde_json::Value;

pub struct TsConfigReader;

impl TsConfigReader {
    /// Read the tsconfig.json file and return the baseUrl and paths.
    pub fn read_tsconfig(tsconfig_path: &PathBuf) -> (PathBuf, Vec<(String, Vec<String>)>) {
        if let Ok(content) = fs::read_to_string(tsconfig_path) {
            if let Ok(json) = serde_json::from_str::<Value>(&content) {
                let base_url = json["compilerOptions"]["baseUrl"]
                    .as_str()
                    .map(|s| s.to_string())
                    .unwrap_or("".to_string());

                let paths = json["compilerOptions"]["paths"]
                    .as_object()
                    .map(|paths| {
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
                    })
                    .unwrap_or_default();

                return (PathBuf::from(base_url), paths);
            }
        }
        (PathBuf::from(""), Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    #[test]
    fn test_read_tsconfig() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        let tsconfig_path = root.join("tsconfig.json"); 
        fs::write(&tsconfig_path, r#"
            {
                "compilerOptions": {
                    "baseUrl": "src",
                    "paths": {
                        "@/*": ["src/*"]
                    }
                }
            }
        "#).unwrap();
        let (base_url, paths) = TsConfigReader::read_tsconfig(&tsconfig_path);

        assert_eq!(base_url, PathBuf::from("src"));
        assert_eq!(paths, vec![("@/*".to_string(), vec!["src/*".to_string()])]);
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
        fs::write(&tsconfig_path, r#"
            {
                "compilerOptions": {
                    "paths": {
                        "@/*": ["src/*"]
                    }
                }
            }
        "#).unwrap();
        let (base_url, paths) = TsConfigReader::read_tsconfig(&tsconfig_path);  

        assert_eq!(base_url, PathBuf::from(""));
        assert_eq!(paths, vec![("@/*".to_string(), vec!["src/*".to_string()])]);
    }
}
