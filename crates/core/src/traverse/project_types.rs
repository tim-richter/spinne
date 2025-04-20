use std::{
    any::Any,
    collections::HashSet,
    path::PathBuf,
};

use crate::{
    analyze::react::analyzer::ReactAnalyzer,
    config::{Config, ConfigValues},
    graph::{Component, ComponentGraph},
    package_json::PackageJson,
    parse::parse_tsx,
    traverse::{PackageResolver, ProjectResolver},
    util::replace_absolute_path_with_project_name,
    Exports,
};
use spinne_logger::Logger;

/// Trait defining common functionality for all project types
pub trait Project: Any {
    /// Gets the root path of the project
    fn get_root(&self) -> &PathBuf;
    
    /// Gets the name of the project
    fn get_name(&self) -> &str;
    
    /// Gets the component graph of the project
    fn get_component_graph(&self) -> &ComponentGraph;
    
    /// Gets mutable access to the component graph
    fn get_component_graph_mut(&mut self) -> &mut ComponentGraph;
    
    /// Gets all dependencies of the project
    fn get_dependencies(&self) -> Option<HashSet<String>>;
    
    /// Finds a specific dependency by name
    fn find_dependency(&self, name: &str) -> Option<String>;
    
    /// Traverses the project to analyze its components
    fn traverse(&mut self, exclude: &Vec<String>, include: &Vec<String>);

    /// Returns a reference to this object as an Any trait object
    fn as_any(&self) -> &dyn Any;
}

/// A project that is used as a dependency by other projects
#[derive(Clone)]
pub struct SourceProject {
    pub project_root: PathBuf,
    pub project_name: String,
    pub component_graph: ComponentGraph,
    resolver: ProjectResolver,
    package_resolver: PackageResolver,
    config: Option<ConfigValues>,
}

impl SourceProject {
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

    /// Builds a walker with correct overrides and patterns.
    fn build_walker(&self, exclude: &Vec<String>, include: &Vec<String>) -> ignore::Walk {
        let exclude_patterns: Vec<String> = exclude
            .iter()
            .map(|pattern| format!("!{}", pattern)) // Add '!' to each pattern
            .collect();

        let mut override_builder = ignore::overrides::OverrideBuilder::new(&self.project_root);

        for pattern in include {
            override_builder.add(pattern).unwrap();
        }
        for pattern in &exclude_patterns {
            override_builder.add(pattern).unwrap();
        }
        let overrides = override_builder.build().unwrap();

        Logger::debug(&format!("Walking using include patterns: {:?}", include), 1);
        Logger::debug(&format!("Walking using exclude patterns: {:?}", exclude), 1);

        ignore::WalkBuilder::new(&self.project_root)
            .git_ignore(true)
            .overrides(overrides)
            .build()
    }

    /// Analyzes a file and adds the found components to the component graph.
    fn analyze_file(&mut self, path: &PathBuf) {
        if !path.is_file() {
            return;
        }

        let file_extension = path.extension().and_then(|ext| ext.to_str());
        if file_extension != Some("tsx") && file_extension != Some("ts") {
            return;
        }

        let file_content = match std::fs::read_to_string(path) {
            Ok(content) => content,
            Err(e) => {
                Logger::error(&format!("Failed to read file {}: {}", path.display(), e));
                return;
            }
        };

        let allocator = oxc_allocator::Allocator::default();
        let (parser_return, semantic_return) = match parse_tsx(&allocator, path, &file_content) {
            Ok(result) => result,
            Err(e) => {
                Logger::error(&format!("Failed to parse file {}: {}", path.display(), e));
                return;
            }
        };

        // Create a ReactAnalyzer instance
        let mut react_analyzer = ReactAnalyzer::new(
            &semantic_return.semantic,
            path.clone(),
            &self.resolver,
            &self.package_resolver,
        );
        
        // Extract components using the analyzer
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
}

impl Project for SourceProject {
    fn get_root(&self) -> &PathBuf {
        &self.project_root
    }

    fn get_name(&self) -> &str {
        &self.project_name
    }

    fn get_component_graph(&self) -> &ComponentGraph {
        &self.component_graph
    }

    fn get_component_graph_mut(&mut self) -> &mut ComponentGraph {
        &mut self.component_graph
    }

    fn get_dependencies(&self) -> Option<HashSet<String>> {
        PackageJson::read(&self.project_root.join("package.json"), true)
            .and_then(|package_json| package_json.get_all_dependencies())
    }

    fn find_dependency(&self, name: &str) -> Option<String> {
        PackageJson::read(&self.project_root.join("package.json"), true)
            .and_then(|package_json| package_json.find_dependency(name))
    }

    fn traverse(&mut self, exclude: &Vec<String>, include: &Vec<String>) {
        let mut exclude_patterns = exclude.clone();
        let mut include_patterns = include.clone();

        // Merge config values with CLI values
        if let Some(config) = &self.config {
            if let Some(config_exclude) = &config.exclude {
                exclude_patterns.extend(config_exclude.clone());
            }
            if let Some(config_include) = &config.include {
                include_patterns.extend(config_include.clone());
            }
            // Handle entry points from config
            if let Some(config_entry_points) = &config.entry_points {
                Logger::info("Analyzing entry points from config file");
                let entry_points = config_entry_points.iter()
                    .map(|path| self.project_root.join(path))
                    .collect::<Vec<_>>();
                let exports = Exports::new(entry_points);
                exports.analyze();
            }
        }

        Logger::info(&format!(
            "Starting traversal of source project: {}",
            self.project_name
        ));

        let walker = self.build_walker(&exclude_patterns, &include_patterns);

        for entry in walker {
            match entry {
                Ok(entry) => {
                    let path = entry.path().to_path_buf();

                    if path.is_file() {
                        Logger::debug(&format!("Analyzing file: {}", path.display()), 2);
                        self.analyze_file(&path);
                    }
                }
                Err(e) => Logger::error(&format!("Error while walking file: {}", e)),
            }
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// A project that depends on other projects
#[derive(Clone)]
pub struct ConsumerProject {
    pub project_root: PathBuf,
    pub project_name: String,
    pub component_graph: ComponentGraph,
    resolver: ProjectResolver,
    package_resolver: PackageResolver,
    config: Option<ConfigValues>,
    source_projects: Vec<SourceProject>,
}

impl ConsumerProject {
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
            source_projects: Vec::new(),
        }
    }

    pub fn add_source_project(&mut self, project: SourceProject) {
        self.source_projects.push(project);
    }

    pub fn get_source_projects(&self) -> &[SourceProject] {
        &self.source_projects
    }

    /// Builds a walker with correct overrides and patterns.
    fn build_walker(&self, exclude: &Vec<String>, include: &Vec<String>) -> ignore::Walk {
        let exclude_patterns: Vec<String> = exclude
            .iter()
            .map(|pattern| format!("!{}", pattern)) // Add '!' to each pattern
            .collect();

        let mut override_builder = ignore::overrides::OverrideBuilder::new(&self.project_root);

        for pattern in include {
            override_builder.add(pattern).unwrap();
        }
        for pattern in &exclude_patterns {
            override_builder.add(pattern).unwrap();
        }
        let overrides = override_builder.build().unwrap();

        Logger::debug(&format!("Walking using include patterns: {:?}", include), 1);
        Logger::debug(&format!("Walking using exclude patterns: {:?}", exclude), 1);

        ignore::WalkBuilder::new(&self.project_root)
            .git_ignore(true)
            .overrides(overrides)
            .build()
    }

    /// Analyzes a file and adds the found components to the component graph.
    fn analyze_file(&mut self, path: &PathBuf) {
        if !path.is_file() {
            return;
        }

        let file_extension = path.extension().and_then(|ext| ext.to_str());
        if file_extension != Some("tsx") && file_extension != Some("ts") {
            return;
        }

        let file_content = match std::fs::read_to_string(path) {
            Ok(content) => content,
            Err(e) => {
                Logger::error(&format!("Failed to read file {}: {}", path.display(), e));
                return;
            }
        };

        let allocator = oxc_allocator::Allocator::default();
        let (parser_return, semantic_return) = match parse_tsx(&allocator, path, &file_content) {
            Ok(result) => result,
            Err(e) => {
                Logger::error(&format!("Failed to parse file {}: {}", path.display(), e));
                return;
            }
        };

        // Create a ReactAnalyzer instance
        let mut react_analyzer = ReactAnalyzer::new(
            &semantic_return.semantic,
            path.clone(),
            &self.resolver,
            &self.package_resolver,
        );
        
        // Extract components using the analyzer
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
}

impl Project for ConsumerProject {
    fn get_root(&self) -> &PathBuf {
        &self.project_root
    }

    fn get_name(&self) -> &str {
        &self.project_name
    }

    fn get_component_graph(&self) -> &ComponentGraph {
        &self.component_graph
    }

    fn get_component_graph_mut(&mut self) -> &mut ComponentGraph {
        &mut self.component_graph
    }

    fn get_dependencies(&self) -> Option<HashSet<String>> {
        PackageJson::read(&self.project_root.join("package.json"), true)
            .and_then(|package_json| package_json.get_all_dependencies())
    }

    fn find_dependency(&self, name: &str) -> Option<String> {
        PackageJson::read(&self.project_root.join("package.json"), true)
            .and_then(|package_json| package_json.find_dependency(name))
    }

    fn traverse(&mut self, exclude: &Vec<String>, include: &Vec<String>) {
        let mut exclude_patterns = exclude.clone();
        let mut include_patterns = include.clone();

        // Merge config values with CLI values
        if let Some(config) = &self.config {
            if let Some(config_exclude) = &config.exclude {
                exclude_patterns.extend(config_exclude.clone());
            }
            if let Some(config_include) = &config.include {
                include_patterns.extend(config_include.clone());
            }
            // Handle entry points from config
            if let Some(config_entry_points) = &config.entry_points {
                Logger::info("Analyzing entry points from config file");
                let entry_points = config_entry_points.iter()
                    .map(|path| self.project_root.join(path))
                    .collect::<Vec<_>>();
                let exports = Exports::new(entry_points);
                exports.analyze();
            }
        }

        Logger::info(&format!(
            "Starting traversal of consumer project: {}",
            self.project_name
        ));

        // First traverse all source projects
        for source_project in &mut self.source_projects {
            source_project.traverse(exclude, include);
        }

        let walker = self.build_walker(&exclude_patterns, &include_patterns);

        for entry in walker {
            match entry {
                Ok(entry) => {
                    let path = entry.path().to_path_buf();

                    if path.is_file() {
                        Logger::debug(&format!("Analyzing file: {}", path.display()), 2);
                        self.analyze_file(&path);
                    }
                }
                Err(e) => Logger::error(&format!("Error while walking file: {}", e)),
            }
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
} 