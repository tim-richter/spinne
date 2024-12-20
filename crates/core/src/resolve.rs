use std::path::PathBuf;

use oxc_resolver::{Resolution, ResolveOptions, Resolver, TsconfigOptions, TsconfigReferences};

/// resolve a relative file path to an absolute file path
///
/// dir: is the directory of the file that has the import statement
/// specifier: is the relative file path that the import statement is importing
/// tsconfig: is the path to the tsconfig.json file that contains the tsconfig options and tsconfigPaths
///
/// # Example
///
/// ```
/// use std::path::PathBuf;
/// use spinne_core::resolve_file_path;
///
/// let dir = PathBuf::from("/Users/tim/projects/spinne/src/index.ts");
/// let specifier = "./components/Button";
/// let tsconfig = PathBuf::from("/Users/tim/projects/spinne/tsconfig.json");
/// let resolution = resolve_file_path(&dir, &specifier, &tsconfig);
/// assert!(resolution.is_ok());
/// ```
pub fn resolve_file_path(
    dir: &PathBuf,
    specifier: &str,
    tsconfig: Option<&PathBuf>,
) -> Result<Resolution, String> {
    let options = ResolveOptions {
        tsconfig: tsconfig.map(|tsconfig| TsconfigOptions {
            config_file: tsconfig.to_path_buf(),
            references: TsconfigReferences::Auto,
        }),
        condition_names: vec!["node".to_string(), "import".to_string()],
        extensions: vec![
            ".ts".to_string(),
            ".tsx".to_string(),
            ".js".to_string(),
            ".jsx".to_string(),
        ],
        extension_alias: vec![(
            ".js".to_string(),
            vec![".ts".to_string(), ".js".to_string()],
        )],
        ..ResolveOptions::default()
    };

    match Resolver::new(options).resolve(dir, &specifier) {
        Ok(resolved_path) => Ok(resolved_path),
        Err(e) => Err(e.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::*;

    #[test]
    fn test_resolve_file_path() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        let src_dir = root.join("src");
        let components_dir = src_dir.join("components");
        fs::create_dir_all(&src_dir).unwrap();
        fs::create_dir_all(&components_dir).unwrap();

        let button_file = components_dir.join("Button.tsx");
        let index_file = components_dir.join("index.ts");
        fs::write(
            &button_file,
            "export function Button() { return <div>Button</div>; }",
        )
        .unwrap();
        fs::write(&index_file, "import { Button } from './Button';").unwrap();

        let specifier = "./components/Button";
        let resolution = resolve_file_path(&src_dir, &specifier, None);

        assert!(resolution.is_ok());
        assert_eq!(
            resolution.unwrap().path(),
            components_dir.join("Button.tsx")
        );
    }

    #[test]
    fn test_resolve_file_path_with_tsconfig_paths() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        let src_dir = root.join("src");
        let components_dir = src_dir.join("components");
        fs::create_dir_all(&src_dir).unwrap();
        fs::create_dir_all(&components_dir).unwrap();

        let button_file = components_dir.join("Button.tsx");
        let index_file = components_dir.join("index.ts");
        fs::write(
            &button_file,
            "export function Button() { return <div>Button</div>; }",
        )
        .unwrap();
        fs::write(&index_file, "import { Button } from './Button';").unwrap();

        let tsconfig_path = root.join("tsconfig.json");
        fs::write(
            &tsconfig_path,
            r#"{
        "compilerOptions": {
          "baseUrl": ".",
          "paths": {
            "@components/*": ["./src/components/*"]
          }
        }
      }"#,
        )
        .unwrap();

        let specifier = "@components/Button";
        let resolution = resolve_file_path(&src_dir, &specifier, Some(&tsconfig_path));

        assert!(resolution.is_ok());
        assert_eq!(
            resolution.unwrap().path(),
            components_dir.join("Button.tsx")
        );
    }

    #[test]
    fn test_resolve_file_path_with_node_modules() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        let src_dir = root.join("src");
        let node_modules_dir = root.join("node_modules");
        fs::create_dir_all(&src_dir).unwrap();
        fs::create_dir_all(&node_modules_dir).unwrap();

        let framer_motion_dir = node_modules_dir.join("framer-motion");
        fs::create_dir_all(&framer_motion_dir).unwrap();
        fs::write(
            &framer_motion_dir.join("index.js"),
            "module.exports = { motion: () => <div>Framer Motion</div> };",
        )
        .unwrap();

        let specifier = "framer-motion";
        let resolution = resolve_file_path(&src_dir, &specifier, None);

        assert!(resolution.is_ok());
        assert_eq!(
            resolution.unwrap().path(),
            node_modules_dir.join("framer-motion").join("index.js")
        );
    }
}
