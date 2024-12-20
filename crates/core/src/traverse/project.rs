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

pub struct Project {
    project_root: PathBuf,
    project_name: String,
    pub component_graph: ComponentGraph,
    resolver: ProjectResolver,
}

impl Project {
    pub fn new(project_root: PathBuf) -> Self {
        if !project_root.exists() {
            panic!("Project root does not exist");
        }

        if project_root.is_file() {
            panic!("Project root is a file");
        }

        let package_json = PackageJson::read(project_root.join("package.json"))
            .expect("Failed to read package.json");
        let project_name = package_json
            .name
            .expect("No project name found in package.json");

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

    pub fn traverse(&mut self, exclude: &[String], include: &[String]) {
        Logger::info(&format!(
            "Starting traversal of project: {}",
            self.project_root.display()
        ));
        let walker = self.build_walker(exclude, include);
        let project = RwLock::new(self);

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

    fn analyze_file(&mut self, path: &Path) {
        if path.extension().unwrap() == "tsx" || path.extension().unwrap() == "ts" {
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

            let react_analyzer =
                ReactAnalyzer::new(&semantic_ret.semantic, path_buf, &self.resolver);
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
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    #[test]
    fn test_project() {
        Logger::set_level(2);
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path().to_path_buf();

        let src_dir = project_root.join("src");
        fs::create_dir_all(&src_dir).unwrap();

        fs::write(project_root.join("package.json"), "{\"name\": \"test\"}").unwrap();
        fs::write(project_root.join("tsconfig.json"), "{}").unwrap();
        fs::write(
            src_dir.join("index.tsx"),
            r#"
            import React from 'react';

            const App: React.FC = () => { return <div>Hello World</div>; }
        "#,
        )
        .unwrap();

        let mut project = Project::new(project_root);
        project.traverse(&[], &["**/*.tsx".to_string(), "**/*.ts".to_string()]);

        assert_eq!(project.component_graph.graph.node_count(), 1);
        assert!(project
            .component_graph
            .has_component("App", &PathBuf::from("test/src/index.tsx")));
    }
}
