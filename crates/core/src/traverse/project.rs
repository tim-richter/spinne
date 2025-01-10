use std::{
    fs,
    path::{Path, PathBuf},
    sync::RwLock,
};

use ignore::{overrides::OverrideBuilder, DirEntry, Error, WalkBuilder, WalkParallel, WalkState};
use oxc_allocator::Allocator;
use spinne_logger::Logger;

use crate::{
    analyze::react::analyzer::ReactAnalyzer, parse::parse_tsx,
    util::replace_absolute_path_with_project_name, ComponentGraph, PackageJson,
};

use super::ProjectResolver;

/// Represents a project and its components.
/// A Project is typically a repository with a package.json file.
pub struct Project {
    project_root: PathBuf,
    project_name: String,
    pub component_graph: ComponentGraph,
    resolver: ProjectResolver,
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

        let package_json = PackageJson::read(project_root.join("package.json"))
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

        Self {
            project_root,
            project_name,
            component_graph: ComponentGraph::new(),
            resolver,
        }
    }

    /// Traverses the project and tries to find components.
    ///
    /// # Arguments
    ///
    /// - `exclude`: A list of patterns to exclude from the traversal.
    /// - `include`: A list of patterns to include in the traversal.
    pub fn traverse(&mut self, exclude: &[String], include: &[String]) {
        Logger::info(&format!(
            "Starting traversal of project: {}",
            self.project_name
        ));

        let walker = self.build_walker(exclude, include);
        // we have to use a RwLock here because we are traversing the project in parallel
        let project = RwLock::new(self);

        // starting the traversal
        walker.run(|| {
            Box::new(|result: Result<DirEntry, Error>| {
                match result {
                    Ok(entry) => {
                        let path = entry.path();

                        if path.is_file() {
                            Logger::debug(&format!("Analyzing file: {}", path.display()), 2);
                            project.write().unwrap().analyze_file(&path);
                        }
                    }
                    Err(e) => Logger::error(&format!("Error while walking file: {}", e)),
                }

                WalkState::Continue
            })
        });
    }

    /// Builds a walker with correct overrides and patterns.
    ///
    /// # Arguments
    ///
    /// - `exclude`: A list of patterns to exclude from the traversal.
    /// - `include`: A list of patterns to include in the traversal.
    fn build_walker(&self, exclude: &[String], include: &[String]) -> WalkParallel {
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
            .build_parallel()
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

        let react_analyzer = ReactAnalyzer::new(&semantic_ret.semantic, path_buf, &self.resolver);
        let components = react_analyzer.analyze();

        for component in components {
            let path_relative = replace_absolute_path_with_project_name(
                self.project_root.clone(),
                component.file_path.clone(),
                &self.project_name,
            );
            self.component_graph
                .add_component(component.name, path_relative);
        }
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
        project.traverse(&[], &["**/*.tsx".to_string(), "**/*.ts".to_string()]);

        assert_eq!(project.component_graph.graph.node_count(), 1);
        assert!(project
            .component_graph
            .has_component("App", &PathBuf::from("test/src/index.tsx")));
    }
}
