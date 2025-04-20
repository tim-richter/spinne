use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use crate::package_json::PackageJson;

#[derive(Debug)]
pub enum PackageResolverError {
    IoError(std::io::Error),
    JsonError(serde_json::Error),
    PackageJsonNotFound(PathBuf),
}

impl From<std::io::Error> for PackageResolverError {
    fn from(error: std::io::Error) -> Self {
        PackageResolverError::IoError(error)
    }
}

impl From<serde_json::Error> for PackageResolverError {
    fn from(error: serde_json::Error) -> Self {
        PackageResolverError::JsonError(error)
    }
}

/// A package resolver that can be used to find the package name for a file path
/// It caches the results of the package name for a file path to avoid reading the same package.json files multiple times
///
/// Mainly used for node_modules resolution and resolving imports from other packages
/// The resulting package name is the name of the package that contains the file and can be used to create edges between packages
#[derive(Clone)]
pub struct PackageResolver {
    /// Cache of path -> package name to avoid reading the same package.json files multiple times
    cache: HashMap<PathBuf, String>,
}

impl PackageResolver {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
        }
    }

    /// Get the package name for a file path by finding and parsing the nearest package.json
    pub fn get_package_name(&mut self, path: &Path) -> Result<String, PackageResolverError> {
        // Check if we have this path cached
        if let Some(package_name) = self.cache.get(path) {
            return Ok(package_name.clone());
        }

        // Find the nearest package.json
        let package_json_path = self
            .find_nearest_package_json(path)
            .ok_or_else(|| PackageResolverError::PackageJsonNotFound(path.to_path_buf()))?;

        // Parse the package.json and get the name
        let package_json = PackageJson::read(&package_json_path, true)
            .ok_or_else(|| PackageResolverError::PackageJsonNotFound(package_json_path.clone()))?;
        let package_name = package_json.name;

        if let Some(package_name) = package_name {
            // Cache the result for all files under this package
            self.cache.insert(path.to_path_buf(), package_name.clone());

            Ok(package_name)
        } else {
            Err(PackageResolverError::PackageJsonNotFound(
                package_json_path.clone(),
            ))
        }
    }

    /// Check if two paths belong to the same package by comparing their package names
    pub fn is_same_package(
        &mut self,
        path1: &Path,
        path2: &Path,
    ) -> Result<bool, PackageResolverError> {
        let package1 = self.get_package_name(path1)?;
        let package2 = self.get_package_name(path2)?;

        Ok(package1 == package2)
    }

    /// Find the nearest package.json file by walking up the directory tree
    fn find_nearest_package_json(&self, start_path: &Path) -> Option<PathBuf> {
        let mut current = if start_path.is_file() {
            start_path.parent()?
        } else {
            start_path
        };

        loop {
            let package_json = current.join("package.json");
            if package_json.exists() {
                return Some(package_json);
            }

            if let Some(parent) = current.parent() {
                current = parent;
            } else {
                return None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::util::test_utils::create_mock_project;

    use super::*;

    #[test]
    fn test_find_package_name() {
        let files = vec![
            ("package.json", r#"{"name": "test-package"}"#),
            ("src/index.js", ""),
        ];
        let temp_dir = create_mock_project(&files);

        let mut resolver = PackageResolver::new();
        let package_name = resolver
            .get_package_name(&temp_dir.path().join("src/index.js"))
            .unwrap();

        assert_eq!(package_name, "test-package");
    }

    #[test]
    fn test_is_same_package() {
        let files = vec![
            ("package.json", r#"{"name": "test-package"}"#),
            ("src/index.js", ""),
            ("src/component.js", ""),
        ];
        let temp_dir = create_mock_project(&files);

        let mut resolver = PackageResolver::new();

        // This should be true because the two files are in the same package
        assert!(resolver
            .is_same_package(
                &temp_dir.path().join("src/index.js"),
                &temp_dir.path().join("src/component.js")
            )
            .unwrap());
    }

    #[test]
    fn test_different_packages() {
        let files = vec![
            ("package.json", r#"{"name": "root-package"}"#),
            ("src/index.js", ""),
            ("src/component.js", ""),
            (
                "node_modules/nested-package/package.json",
                r#"{"name": "nested-package"}"#,
            ),
            ("node_modules/nested-package/src/index.js", ""),
        ];
        let temp_dir = create_mock_project(&files);

        let mut resolver = PackageResolver::new();

        // This should be false because the two files are in different packages
        assert!(!resolver
            .is_same_package(
                &temp_dir.path().join("src/index.js"),
                &temp_dir
                    .path()
                    .join("node_modules/nested-package/src/index.js")
            )
            .unwrap());
    }
}
