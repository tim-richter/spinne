use std::{
    any::Any,
    collections::{HashMap, HashSet},
    path::PathBuf,
};

use crate::{
    analyze::react::analyzer::ReactAnalyzer,
    config::{Config, ConfigValues},
    graph::{ComponentNode, ComponentRegistry},
    package_json::PackageJson,
    parse::parse_tsx,
    traverse::{PackageResolver, ProjectResolver},
    util::replace_absolute_path_with_project_name,
};
use spinne_logger::Logger;

/// Trait defining common functionality for all project types
pub trait Project: Any {
    /// Gets the root path of the project
    fn get_root(&self) -> &PathBuf;

    /// Gets the name of the project
    fn get_name(&self) -> &str;

    /// Gets the component graph of the project
    fn get_component_graph(&self) -> &ComponentRegistry;

    /// Gets mutable access to the component graph
    fn get_component_graph_mut(&mut self) -> &mut ComponentRegistry;

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
    component_registry: *mut ComponentRegistry,
    resolver: ProjectResolver,
    package_resolver: PackageResolver,
    config: Option<ConfigValues>,
}

impl SourceProject {
    pub fn new(project_root: PathBuf, component_registry: &mut ComponentRegistry) -> Self {
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
            component_registry: component_registry as *mut ComponentRegistry,
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
            // Create base component with props
            let base_component = ComponentNode::new(
                component.name.clone(),
                replace_absolute_path_with_project_name(
                    self.project_root.clone(),
                    component.file_path.clone(),
                    self.project_name.clone(),
                ),
                component.props.clone(),
            );

            // Create child components
            let child_components: Vec<ComponentNode> = component
                .children
                .into_iter()
                .map(|child| {
                    ComponentNode::new(
                        child.name,
                        replace_absolute_path_with_project_name(
                            self.project_root.clone(),
                            child.origin_file_path.clone(),
                            self.project_name.clone(),
                        ),
                        child.props,
                    )
                })
                .collect();

            // Add everything to the graph in one operation
            unsafe {
                if let Some(existing) = (*self.component_registry)
                    .find_component(&base_component.name, &self.project_name)
                {
                    (*self.component_registry)
                        .add_props(&existing.node.id, &base_component.props);
                } else {
                    (*self.component_registry)
                        .add_component(base_component.clone(), self.project_name.clone());
                }

                for child in child_components {
                    if let Some(existing_child) = (*self.component_registry)
                        .find_component(&child.name, &self.project_name)
                    {
                        (*self.component_registry)
                            .add_props(&existing_child.node.id, &child.props);
                    } else {
                        (*self.component_registry)
                            .add_component(child.clone(), self.project_name.clone());
                    }
                    (*self.component_registry).add_dependency(
                        &base_component.id,
                        &child.id,
                        Some(self.project_name.clone()),
                    );
                }
            }
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

    fn get_component_graph(&self) -> &ComponentRegistry {
        unsafe { &*self.component_registry }
    }

    fn get_component_graph_mut(&mut self) -> &mut ComponentRegistry {
        unsafe { &mut *self.component_registry }
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
                let entry_points = config_entry_points
                    .iter()
                    .map(|path| self.project_root.join(path))
                    .collect::<Vec<_>>();
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
    component_registry: *mut ComponentRegistry,
    resolver: ProjectResolver,
    package_resolver: PackageResolver,
    config: Option<ConfigValues>,
    source_projects: Vec<SourceProject>,
}

impl ConsumerProject {
    pub fn new(project_root: PathBuf, component_registry: &mut ComponentRegistry) -> Self {
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
            component_registry: component_registry as *mut ComponentRegistry,
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
            println!("component: {}", component.name);
            // Check if this component is from a source project
            let mut is_from_source = false;
            let mut source_project_name = None;

            // Check if the component's file path matches any source project
            for source_project in &self.source_projects {
                if path.starts_with(&source_project.project_root) {
                    is_from_source = true;
                    source_project_name = Some(source_project.project_name.clone());
                    break;
                }
            }
            println!("path: {}", path.display());
            println!("is_from_source: {}", is_from_source);

            if is_from_source {
                // Don't register the component again, just create dependencies
                if let Some(source_project_name) = source_project_name {
                    // Find the component in the source project
                    if let Some(source_component) = unsafe {
                        (*self.component_registry)
                            .find_component(&component.name, &source_project_name)
                    } {
                        // Create child components
                        let child_components: Vec<ComponentNode> = component
                            .children
                            .into_iter()
                            .map(|child| {
                                ComponentNode::new(
                                    child.name,
                                    replace_absolute_path_with_project_name(
                                        self.project_root.clone(),
                                        child.origin_file_path.clone(),
                                        self.project_name.clone(),
                                    ),
                                    child.props,
                                )
                            })
                            .collect();

                        // Add dependencies for each child component
                        for child in child_components {
                            if let Some(child_component) = unsafe {
                                (*self.component_registry)
                                    .find_component(&child.name, &source_project_name)
                            } {
                                unsafe {
                                    (*self.component_registry)
                                        .add_props(&child_component.node.id, &child.props);
                                    (*self.component_registry)
                                        .add_dependency(
                                            &source_component.node.id,
                                            &child_component.node.id,
                                            Some(source_project_name.clone()),
                                        )
                                        .unwrap_or_else(|e| {
                                            Logger::error(&format!(
                                                "Failed to add dependency: {}",
                                                e
                                            ));
                                        });
                                }
                            }
                        }
                    }
                }
            } else {
                // This is a component defined in the consumer project
                // Create base component with props
                let base_component = ComponentNode::new(
                    component.name.clone(),
                    replace_absolute_path_with_project_name(
                        self.project_root.clone(),
                        component.file_path.clone(),
                        self.project_name.clone(),
                    ),
                    component.props.clone(),
                );

                println!("child_components: {:?}", component.children);

                // Create child components
                let child_components: Vec<ComponentNode> = component
                    .children
                    .into_iter()
                    .map(|child| {
                        ComponentNode::new(
                            child.name,
                            replace_absolute_path_with_project_name(
                                self.project_root.clone(),
                                child.origin_file_path.clone(),
                                self.project_name.clone(),
                            ),
                            child.props,
                        )
                    })
                    .collect();

                // Add everything to the graph in one operation
                unsafe {
                    if let Some(existing) = (*self.component_registry)
                        .find_component(&base_component.name, &self.project_name)
                    {
                        (*self.component_registry)
                            .add_props(&existing.node.id, &base_component.props);
                    } else {
                        (*self.component_registry)
                            .add_component(base_component.clone(), self.project_name.clone());
                    }

                    for child in child_components {
                        println!("child: {}", child.name);
                        // Check if the child component is from a source project
                        let mut child_is_from_source = false;
                        let mut child_source_project_name = None;

                        for source_project in &self.source_projects {
                            // Extract project name from the file path
                            if let Some(path_str) = child.file_path.to_str() {
                                if path_str.starts_with(&source_project.project_name) {
                                    child_is_from_source = true;
                                    child_source_project_name =
                                        Some(source_project.project_name.clone());
                                    break;
                                }
                            }
                        }

                        if child_is_from_source {
                            // Find the component in the source project
                            if let Some(child_source_project_name) = child_source_project_name {
                                if let Some(source_component) = unsafe {
                                    (*self.component_registry)
                                        .find_component(&child.name, &child_source_project_name)
                                } {
                                    (*self.component_registry)
                                        .add_props(&source_component.node.id, &child.props);
                                    unsafe {
                                        (*self.component_registry)
                                            .add_dependency(
                                                &base_component.id,
                                                &source_component.node.id,
                                                Some(child_source_project_name),
                                            )
                                                .unwrap_or_else(|e| {
                                                Logger::error(&format!(
                                                    "Failed to add dependency: {}",
                                                    e
                                                ));
                                            });
                                    }
                                } else {
                                    Logger::error(&format!(
                                        "Could not find component {} in source project {}",
                                        child.name, child_source_project_name
                                    ));
                                }
                            }
                        } else {
                            // Register the child component and add dependency
                            if let Some(existing_child) = (*self.component_registry)
                                .find_component(&child.name, &self.project_name)
                            {
                                (*self.component_registry)
                                    .add_props(&existing_child.node.id, &child.props);
                            } else {
                                (*self.component_registry)
                                    .add_component(child.clone(), self.project_name.clone());
                            }
                            (*self.component_registry)
                                .add_dependency(
                                    &base_component.id,
                                    &child.id,
                                    Some(self.project_name.clone()),
                                )
                                .unwrap_or_else(|e| {
                                    Logger::error(&format!("Failed to add dependency: {}", e));
                                });
                        }
                    }
                }
            }
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

    fn get_component_graph(&self) -> &ComponentRegistry {
        unsafe { &*self.component_registry }
    }

    fn get_component_graph_mut(&mut self) -> &mut ComponentRegistry {
        unsafe { &mut *self.component_registry }
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
                let entry_points = config_entry_points
                    .iter()
                    .map(|path| self.project_root.join(path))
                    .collect::<Vec<_>>();
            }
        }

        Logger::info(&format!(
            "Starting traversal of consumer project: {}",
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
