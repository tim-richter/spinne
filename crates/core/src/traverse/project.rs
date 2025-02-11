use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

use ignore::{overrides::OverrideBuilder, Walk, WalkBuilder};
use oxc_allocator::Allocator;
use spinne_logger::Logger;

use crate::{
    analyze::react::analyzer::ReactAnalyzer, config::ConfigValues, graph::Component,
    parse::parse_tsx, util::replace_absolute_path_with_project_name, ComponentGraph, Config,
    PackageJson,
};

use super::{PackageResolver, ProjectResolver};

/// Represents a project and its components.
/// A Project is typically a repository with a package.json file.
pub struct Project {
    pub project_root: PathBuf,
    pub project_name: String,
    pub component_graph: ComponentGraph,
    resolver: ProjectResolver,
    package_resolver: PackageResolver,
    config: Option<ConfigValues>,
}

impl Project {
    /// Creates a new Project instance from a given path.
    /// The path is expected to be the root of the project and should be a directory.
    ///
    /// # Panics
    ///
    /// - If the project root does not exist.
    /// - If the project root is a file.
    pub fn new(project_root: PathBuf) -> Self {
        if !project_root.exists() {
            panic!("Project root does not exist");
        }

        if project_root.is_file() {
            panic!("Project root is a file");
        }

        let package_json = PackageJson::read(&project_root.join("package.json"), false)
            .expect("Failed to read package.json");

        let project_name = package_json.name.unwrap_or_else(|| {
            Logger::warn(&format!("No project name found in package.json"));
            project_root.to_string_lossy().to_string()
        });

        let tsconfig_path = project_root.join("tsconfig.json");
        let resolver = if tsconfig_path.exists() {
            ProjectResolver::new(Some(tsconfig_path))
        } else {
            ProjectResolver::new(None)
        };

        let config = Config::read(project_root.join("spinne.json"));

        Self {
            project_root,
            project_name,
            component_graph: ComponentGraph::new(),
            resolver,
            package_resolver: PackageResolver::new(),
            config,
        }
    }

    /// Traverses the project and tries to find components.
    ///
    /// # Arguments
    ///
    /// - `exclude`: A list of patterns to exclude from the traversal.
    /// - `include`: A list of patterns to include in the traversal.
    pub fn traverse(&mut self, exclude_params: &Vec<String>, include_params: &Vec<String>) {
        Logger::info(&format!(
            "Starting traversal of project: {}",
            self.project_name
        ));

        let mut exclude = exclude_params.clone();
        let mut include = include_params.clone();

        // merge the config with the exclude and include patterns
        if let Some(config) = &self.config {
            exclude = match &config.exclude {
                Some(exclude) => {
                    let mut exclude_vec = exclude.clone();
                    exclude_vec.extend(exclude_params.iter().map(|e| e.to_string()));
                    exclude_vec
                }
                None => exclude.clone(),
            };
            include = match &config.include {
                Some(include) => {
                    let mut include_vec = include.clone();
                    include_vec.extend(include_params.iter().map(|e| e.to_string()));
                    include_vec
                }
                None => include.clone(),
            };
        }

        let walker = self.build_walker(&exclude, &include);

        for entry in walker {
            match entry {
                Ok(entry) => {
                    let path = entry.path();

                    if path.is_file() {
                        Logger::debug(&format!("Analyzing file: {}", path.display()), 2);
                        self.analyze_file(&path);
                    }
                }
                Err(e) => Logger::error(&format!("Error while walking file: {}", e)),
            }
        }
    }

    /// Builds a walker with correct overrides and patterns.
    ///
    /// # Arguments
    ///
    /// - `exclude`: A list of patterns to exclude from the traversal.
    /// - `include`: A list of patterns to include in the traversal.
    fn build_walker(&self, exclude: &Vec<String>, include: &Vec<String>) -> Walk {
        let exclude_patterns: Vec<String> = exclude
            .iter()
            .map(|pattern| format!("!{}", pattern)) // Add '!' to each pattern
            .collect();

        let mut override_builder = OverrideBuilder::new(&self.project_root);

        for pattern in include {
            override_builder.add(pattern).unwrap();
        }
        for pattern in &exclude_patterns {
            override_builder.add(pattern).unwrap();
        }
        let overrides = override_builder.build().unwrap();

        Logger::debug(&format!("Walking using include patterns: {:?}", include), 1);
        Logger::debug(&format!("Walking using exclude patterns: {:?}", exclude), 1);

        WalkBuilder::new(&self.project_root)
            .git_ignore(true)
            .overrides(overrides)
            .build()
    }

    /// Analyzes a file and adds the found components to the component graph.
    fn analyze_file(&mut self, path: &Path) {
        // if a file has no extension, we skip it
        let extension = if let Some(ext) = path.extension() {
            ext.to_string_lossy().to_string()
        } else {
            return;
        };

        // currently we only support tsx and ts files
        if extension != "tsx" && extension != "ts" {
            return;
        }

        let allocator = Allocator::default();
        let path_buf = PathBuf::from(path);
        let source_code = fs::read_to_string(&path_buf).unwrap();

        Logger::debug(&format!("Parsing file: {}", path.display()), 2);
        let result = parse_tsx(&allocator, &path_buf, &source_code);

        if result.is_err() {
            Logger::error(&format!("Failed to parse file: {}", path.display()));
            return;
        }

        let (_parser_ret, semantic_ret) = result.unwrap();

        let mut react_analyzer = ReactAnalyzer::new(
            &semantic_ret.semantic,
            path_buf,
            &self.resolver,
            &self.package_resolver,
        );
        let components = react_analyzer.analyze();

        for component in components {
            let path_relative = replace_absolute_path_with_project_name(
                self.project_root.clone(),
                component.file_path.clone(),
                self.project_name.clone(),
            );

            // Create base component
            let base_component = Component::new(
                component.name.clone(),
                component.file_path.clone(),
                path_relative.clone(),
                self.project_name.clone(),
            );

            // Create child components
            let child_components: Vec<Component> = component
                .children
                .into_iter()
                .map(|child| {
                    let child_path_relative = replace_absolute_path_with_project_name(
                        self.project_root.clone(),
                        child.origin_file_path.clone(),
                        self.project_name.clone(),
                    );
                    Component::new(
                        child.name,
                        child.origin_file_path,
                        child_path_relative,
                        self.project_name.clone(),
                    )
                })
                .collect();

            // Add everything to the graph in one operation
            self.component_graph
                .add_component_with_deps(base_component, child_components);
        }
    }

    /// Gets all dependencies of the project from package.json
    pub fn get_dependencies(&self) -> Option<HashSet<String>> {
        PackageJson::read(&self.project_root.join("package.json"), true)
            .and_then(|package_json| package_json.get_all_dependencies())
    }

    pub fn find_dependency(&self, name: &str) -> Option<String> {
        PackageJson::read(&self.project_root.join("package.json"), true)
            .and_then(|package_json| package_json.find_dependency(name))
    }
}

#[cfg(test)]
mod tests {
    use crate::util::test_utils;

    use super::*;

    #[test]
    fn test_project() {
        let temp_dir = test_utils::create_mock_project(&vec![
            ("package.json", r#"{"name": "test"}"#),
            ("tsconfig.json", "{}"),
            (
                "src/index.tsx",
                r#"
                    import React from 'react';

                    const App: React.FC = () => { return <div>Hello World</div>; }
                "#,
            ),
        ]);

        let mut project = Project::new(temp_dir.path().to_path_buf());
        project.traverse(
            &vec![],
            &vec!["**/*.tsx".to_string(), "**/*.ts".to_string()],
        );

        assert_eq!(project.component_graph.node_count(), 1);
        println!("{:?}", project.component_graph);
        assert!(project
            .component_graph
            .find_component("App", &temp_dir.path().to_path_buf().join("src/index.tsx"))
            .is_some());
    }

    #[test]
    fn test_component_graph() {
        let temp_dir = test_utils::create_mock_project(&vec![
            ("package.json", r#"{"name": "test"}"#),
            ("tsconfig.json", "{}"),
            (
                "src/index.tsx",
                r#"
                    import React from 'react';
                    import { Button } from './components/Button';

                    export const App: React.FC = () => { return <div><Button /></div>; }
                "#,
            ),
            (
                "src/components/Button.tsx",
                r#"
                    import React from 'react';
                    export const Button: React.FC = () => { return <button>Click me</button>; }
                "#,
            ),
        ]);

        let mut project = Project::new(temp_dir.path().to_path_buf());
        project.traverse(
            &vec![],
            &vec!["**/*.tsx".to_string(), "**/*.ts".to_string()],
        );

        assert_eq!(project.component_graph.node_count(), 2);
        assert!(project
            .component_graph
            .find_component("App", &temp_dir.path().to_path_buf().join("src/index.tsx"))
            .is_some());
        assert!(project
            .component_graph
            .find_component(
                "Button",
                &temp_dir
                    .path()
                    .to_path_buf()
                    .join("src/components/Button.tsx")
            )
            .is_some());

        // App has edge to Button
        assert!(project.component_graph.has_edge(
            project
                .component_graph
                .find_component("App", &temp_dir.path().to_path_buf().join("src/index.tsx"))
                .unwrap()
                .id,
            project
                .component_graph
                .find_component(
                    "Button",
                    &temp_dir
                        .path()
                        .to_path_buf()
                        .join("src/components/Button.tsx")
                )
                .unwrap()
                .id,
        ));
    }

    #[test]
    fn test_component_graph_with_tsconfig() {
        let temp_dir = test_utils::create_mock_project(&vec![
            ("package.json", r#"{"name": "test"}"#),
            (
                "tsconfig.json",
                r#"{"compilerOptions": {"baseUrl": ".", "paths": {"@/*": ["src/*"]}}}"#,
            ),
            (
                "src/index.tsx",
                r#"
                    import React from 'react';
                    import { Button } from '@/components/Button';

                    export const App: React.FC = () => { return <div><Button /></div>; }
                "#,
            ),
            (
                "src/components/Button.tsx",
                r#"
                    import React from 'react';
                    export const Button: React.FC = () => { return <button>Click me</button>; }
                "#,
            ),
        ]);

        let mut project = Project::new(temp_dir.path().to_path_buf());
        project.traverse(
            &vec![],
            &vec!["**/*.tsx".to_string(), "**/*.ts".to_string()],
        );

        assert_eq!(project.component_graph.node_count(), 2);
        assert!(project
            .component_graph
            .find_component("App", &temp_dir.path().to_path_buf().join("src/index.tsx"))
            .is_some());
        assert!(project
            .component_graph
            .find_component(
                "Button",
                &temp_dir
                    .path()
                    .to_path_buf()
                    .join("src/components/Button.tsx")
            )
            .is_some());

        // App has edge to Button
        assert!(project.component_graph.has_edge(
            project
                .component_graph
                .find_component("App", &temp_dir.path().to_path_buf().join("src/index.tsx"))
                .unwrap()
                .id,
            project
                .component_graph
                .find_component(
                    "Button",
                    &temp_dir
                        .path()
                        .to_path_buf()
                        .join("src/components/Button.tsx")
                )
                .unwrap()
                .id,
        ));
    }

    #[test]
    fn test_component_graph_with_tsconfig_and_tsx() {
        let temp_dir = test_utils::create_mock_project(&vec![
            ("package.json", r#"{"name": "test"}"#),
            (
                "tsconfig.json",
                r#"{"compilerOptions": {"baseUrl": ".", "paths": {"@/*": ["src/*"]}}}"#,
            ),
            (
                "src/components/Button/ButtonGroup.tsx",
                r#"
                    import React from 'react';
                    import { Button } from '@/components/Button';

                    export const ButtonGroup: React.FC<React.PropsWithChildren> = ({ children }) => { return <Button>{children}</Button>; }
                "#,
            ),
            (
                "src/components/Button.tsx",
                r#"
                    import React from 'react';
                    export const Button = () => { return "HI"; }
                "#,
            ),
        ]);

        let mut project = Project::new(temp_dir.path().to_path_buf());
        project.traverse(
            &vec![],
            &vec!["**/*.tsx".to_string(), "**/*.ts".to_string()],
        );

        assert_eq!(project.component_graph.node_count(), 2);
        assert!(project
            .component_graph
            .find_component(
                "ButtonGroup",
                &temp_dir
                    .path()
                    .to_path_buf()
                    .join("src/components/Button/ButtonGroup.tsx")
            )
            .is_some());
        assert!(project
            .component_graph
            .find_component(
                "Button",
                &temp_dir
                    .path()
                    .to_path_buf()
                    .join("src/components/Button.tsx")
            )
            .is_some());

        // ButtonGroup has edge to Button
        assert!(project.component_graph.has_edge(
            project
                .component_graph
                .find_component(
                    "ButtonGroup",
                    &temp_dir
                        .path()
                        .to_path_buf()
                        .join("src/components/Button/ButtonGroup.tsx")
                )
                .unwrap()
                .id,
            project
                .component_graph
                .find_component(
                    "Button",
                    &temp_dir
                        .path()
                        .to_path_buf()
                        .join("src/components/Button.tsx")
                )
                .unwrap()
                .id,
        ));
    }

    #[test]
    fn should_read_exclude_from_config() {
        let temp_dir = test_utils::create_mock_project(&vec![
            ("package.json", r#"{"name": "test"}"#),
            (
                "spinne.json",
                r#"{"exclude": ["src/components/Button/ButtonGroup.tsx"]}"#,
            ),
            (
                "tsconfig.json",
                r#"{"compilerOptions": {"baseUrl": ".", "paths": {"@/*": ["src/*"]}}}"#,
            ),
            (
                "src/components/Button/ButtonGroup.tsx",
                r#"
                    import React from 'react';
                    import { Button } from '@/components/Button';

                    export const ButtonGroup: React.FC<React.PropsWithChildren> = ({ children }) => { return <Button>{children}</Button>; }
                "#,
            ),
            (
                "src/pages/Button.tsx",
                r#"
                    import React from 'react';
                    export const Button = () => { return "HI"; }
                "#,
            ),
        ]);

        let mut project = Project::new(temp_dir.path().to_path_buf());
        project.traverse(
            &vec![],
            &vec!["**/*.tsx".to_string(), "**/*.ts".to_string()],
        );

        assert_eq!(project.component_graph.node_count(), 1);
    }

    #[test]
    fn should_merge_exclude_from_config() {
        let temp_dir = test_utils::create_mock_project(&vec![
            ("package.json", r#"{"name": "test"}"#),
            (
                "spinne.json",
                r#"{"exclude": ["src/components/Button/ButtonGroup.tsx"]}"#,
            ),
            (
                "tsconfig.json",
                r#"{"compilerOptions": {"baseUrl": ".", "paths": {"@/*": ["src/*"]}}}"#,
            ),
            (
                "src/components/Button/ButtonGroup.tsx",
                r#"
                    import React from 'react';
                    import { Button } from '@/components/Button';

                    export const ButtonGroup: React.FC<React.PropsWithChildren> = ({ children }) => { return <Button>{children}</Button>; }
                "#,
            ),
            (
                "src/pages/Button.tsx",
                r#"
                    import React from 'react';
                    export const Button = () => { return "HI"; }
                "#,
            ),
        ]);

        let mut project = Project::new(temp_dir.path().to_path_buf());
        project.traverse(
            &vec![String::from("src/pages/Button.tsx")],
            &vec!["**/*.tsx".to_string(), "**/*.ts".to_string()],
        );

        assert_eq!(project.component_graph.node_count(), 0);
    }

    #[test]
    fn should_read_include_from_config() {
        let temp_dir = test_utils::create_mock_project(&vec![
            ("package.json", r#"{"name": "test"}"#),
            (
                "spinne.json",
                r#"{"include": ["src/components/Button/ButtonGroup.tsx"]}"#,
            ),
            (
                "tsconfig.json",
                r#"{"compilerOptions": {"baseUrl": ".", "paths": {"@/*": ["src/*"]}}}"#,
            ),
            (
                "src/components/Button/ButtonGroup.tsx",
                r#"
                    import React from 'react';

                    export const ButtonGroup: React.FC<React.PropsWithChildren> = ({ children }) => { return <button>{children}</button>; }
                "#,
            ),
            (
                "src/pages/Button.tsx",
                r#"
                    import React from 'react';
                    export const Button = () => { return "HI"; }
                "#,
            ),
        ]);

        let mut project = Project::new(temp_dir.path().to_path_buf());
        project.traverse(&vec![], &vec![]);

        assert_eq!(project.component_graph.node_count(), 1);
    }

    #[test]
    fn should_merge_include_from_config() {
        let temp_dir = test_utils::create_mock_project(&vec![
            ("package.json", r#"{"name": "test"}"#),
            (
                "spinne.json",
                r#"{"include": ["src/components/Button/ButtonGroup.tsx"]}"#,
            ),
            (
                "tsconfig.json",
                r#"{"compilerOptions": {"baseUrl": ".", "paths": {"@/*": ["src/*"]}}}"#,
            ),
            (
                "src/components/Button/ButtonGroup.tsx",
                r#"
                    import React from 'react';

                    export const ButtonGroup: React.FC<React.PropsWithChildren> = ({ children }) => { return <button>{children}</button>; }
                "#,
            ),
            (
                "src/pages/Button.tsx",
                r#"
                    import React from 'react';
                    export const Button = () => { return "HI"; }
                "#,
            ),
        ]);

        let mut project = Project::new(temp_dir.path().to_path_buf());
        project.traverse(&vec![], &vec!["src/pages/Button.tsx".to_string()]);

        assert_eq!(project.component_graph.node_count(), 2);
    }
}
