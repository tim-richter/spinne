use std::path::PathBuf;

use oxc_resolver::{Resolution, ResolveOptions, Resolver, TsconfigOptions, TsconfigReferences};

#[derive(Clone)]
pub struct ProjectResolver {
    tsconfig_path: Option<PathBuf>,
}

impl ProjectResolver {
    pub fn new(tsconfig_path: Option<PathBuf>) -> Self {
        Self { tsconfig_path }
    }

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
    /// use spinne_core::ProjectResolver;
    ///
    /// let dir = PathBuf::from("/Users/tim/projects/spinne/src/index.ts");
    /// let resolver = ProjectResolver::new(None);
    /// let resolution = resolver.resolve(&dir, "./components/Button");
    /// ```
    pub fn resolve(&self, dir: &PathBuf, specifier: &str) -> Result<Resolution, String> {
        let options = ResolveOptions {
            tsconfig: self.tsconfig_path.as_ref().map(|tsconfig| TsconfigOptions {
                config_file: tsconfig.to_path_buf(),
                references: TsconfigReferences::Auto,
            }),
            condition_names: vec![
                "default".to_string(),
                "types".to_string(),
                "import".to_string(),
                "require".to_string(),
                "node".to_string(),
                "node-addons".to_string(),
                "browser".to_string(),
                "esm2020".to_string(),
                "es2020".to_string(),
                "es2015".to_string(),
            ],
            extensions: vec![
                ".ts".to_string(),
                ".tsx".to_string(),
                ".d.ts".to_string(),
                ".js".to_string(),
                ".jsx".to_string(),
                ".mjs".to_string(),
                ".cjs".to_string(),
                ".json".to_string(),
                ".node".to_string(),
            ],
            extension_alias: vec![
                (
                    ".js".to_string(),
                    vec![
                        ".ts".to_string(),
                        ".tsx".to_string(),
                        ".d.ts".to_string(),
                        ".js".to_string(),
                    ],
                ),
                (
                    ".jsx".to_string(),
                    vec![".tsx".to_string(), ".d.ts".to_string(), ".jsx".to_string()],
                ),
                (
                    ".mjs".to_string(),
                    vec![".mts".to_string(), ".mjs".to_string()],
                ),
                (
                    ".cjs".to_string(),
                    vec![".cts".to_string(), ".cjs".to_string()],
                ),
            ],
            main_fields: vec![
                "types".to_string(),
                "typings".to_string(),
                "module".to_string(),
                "main".to_string(),
                "browser".to_string(),
                "jsnext:main".to_string(),
            ],
            alias_fields: vec![vec!["browser".to_string()]],
            ..ResolveOptions::default()
        };

        match Resolver::new(options).resolve(dir, &specifier) {
            Ok(resolved_path) => Ok(resolved_path),
            Err(e) => Err(e.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::util::test_utils;

    use super::*;

    #[test]
    fn test_resolve_file_path() {
        let temp_dir = test_utils::create_mock_project(&vec![
            (
                "src/components/Button.tsx",
                "export function Button() { return <div>Button</div>; }",
            ),
            (
                "src/components/index.ts",
                "import { Button } from './Button';",
            ),
        ]);

        let specifier = "./components/Button";
        let resolver = ProjectResolver::new(None);
        let resolution = resolver.resolve(&temp_dir.path().join("src"), &specifier);

        assert!(resolution.is_ok());
        assert_eq!(
            resolution.unwrap().path(),
            temp_dir.path().join("src/components/Button.tsx")
        );
    }

    #[test]
    fn test_resolve_file_path_with_tsconfig_paths() {
        let temp_dir = test_utils::create_mock_project(&vec![
            (
                "tsconfig.json",
                r#"{
                "compilerOptions": {
                "baseUrl": ".",
                "paths": {
                    "@components/*": ["./src/components/*"]
                }
            }
            }"#,
            ),
            (
                "src/components/Button.tsx",
                "export function Button() { return <div>Button</div>; }",
            ),
            (
                "src/components/index.ts",
                "import { Button } from './Button';",
            ),
        ]);

        let specifier = "@components/Button";
        let resolver = ProjectResolver::new(Some(temp_dir.path().join("tsconfig.json")));
        let resolution = resolver.resolve(&temp_dir.path().join("src"), &specifier);

        assert!(resolution.is_ok());
        assert_eq!(
            resolution.unwrap().path(),
            temp_dir.path().join("src/components/Button.tsx")
        );
    }

    #[test]
    fn test_resolve_file_path_with_node_modules() {
        let temp_dir = test_utils::create_mock_project(&vec![
            (
                "node_modules/framer-motion/index.js",
                "module.exports = { motion: () => <div>Framer Motion</div> };",
            ),
            (
                "src/components/Button.tsx",
                "export function Button() { return <div>Button</div>; }",
            ),
            (
                "src/components/index.ts",
                "import { Button } from './Button';",
            ),
        ]);

        let specifier = "framer-motion";
        let resolver = ProjectResolver::new(None);
        let resolution = resolver.resolve(&temp_dir.path().join("src"), &specifier);

        assert!(resolution.is_ok());
        assert_eq!(
            resolution.unwrap().path(),
            temp_dir.path().join("node_modules/framer-motion/index.js")
        );
    }

    #[test]
    fn test_resolve_file_path_with_node_modules_in_sub_directory() {
        let temp_dir = test_utils::create_mock_project(&vec![
            (
                "node_modules/@radix-ui/react-accordion/package.json",
                r#"{
                    "name": "@radix-ui/react-accordion",
                    "main": "dist/index.js"
                }"#,
            ),
            (
                "node_modules/@radix-ui/react-accordion/dist/index.js",
                "module.exports = { Accordion: () => <div>Accordion</div> };",
            ),
            (
                "src/components/Button.tsx",
                "export function Button() { return <div>Button</div>; }",
            ),
            (
                "src/components/index.ts",
                "import { Button } from './Button';",
            ),
        ]);

        let specifier = "@radix-ui/react-accordion";
        let resolver = ProjectResolver::new(None);
        let resolution = resolver.resolve(&temp_dir.path().join("src"), &specifier);

        assert!(resolution.is_ok());
        assert_eq!(
            resolution.unwrap().path(),
            temp_dir
                .path()
                .join("node_modules/@radix-ui/react-accordion/dist/index.js")
        );
    }

    #[test]
    fn test_resolve_node16_imports() {
        let temp_dir = test_utils::create_mock_project(&vec![
            (
                "package.json",
                r#"{
                "type": "module"
            }"#,
            ),
            (
                "tsconfig.json",
                r#"{
                "compilerOptions": {
                    "baseUrl": ".",
                    "module": "NodeNext",
                    "moduleResolution": "NodeNext",
                    "paths": {
                        "@components/*": ["./src/components/*"]
                    }
                }
            }"#,
            ),
            (
                "src/components/index.ts",
                "export { Button } from './Button';",
            ),
            (
                "src/components/Button/index.ts",
                "export function Button() { return <div>Button</div>; }",
            ),
            (
                "src/components/Button.tsx",
                "export function Button() { return <div>Button</div>; }",
            ),
        ]);

        let specifier = "./components/Button.js";
        let resolver = ProjectResolver::new(Some(temp_dir.path().join("tsconfig.json")));
        let resolution = resolver.resolve(&temp_dir.path().join("src"), &specifier);

        assert!(resolution.is_ok());
        assert_eq!(
            resolution.unwrap().path(),
            temp_dir.path().join("src/components/Button.tsx")
        );

        let specifier = "./components/index.js";
        let resolver = ProjectResolver::new(Some(temp_dir.path().join("tsconfig.json")));
        let resolution = resolver.resolve(&temp_dir.path().join("src"), &specifier);

        assert!(resolution.is_ok());
        assert_eq!(
            resolution.unwrap().path(),
            temp_dir.path().join("src/components/index.ts")
        );

        let specifier = "./components/Button/index.js";
        let resolver = ProjectResolver::new(Some(temp_dir.path().join("tsconfig.json")));
        let resolution = resolver.resolve(&temp_dir.path().join("src"), &specifier);

        assert!(resolution.is_ok());
        assert_eq!(
            resolution.unwrap().path(),
            temp_dir.path().join("src/components/Button/index.ts")
        );

        let specifier = "@components/Button/index.js";
        let resolver = ProjectResolver::new(Some(temp_dir.path().join("tsconfig.json")));
        let resolution = resolver.resolve(&temp_dir.path().join("src"), &specifier);

        assert!(resolution.is_ok());
        assert_eq!(
            resolution.unwrap().path(),
            temp_dir.path().join("src/components/Button/index.ts")
        );
    }
}
