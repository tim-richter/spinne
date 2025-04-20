use ignore::{DirEntry, WalkBuilder};
use petgraph::{algo::toposort, Graph, graph::NodeIndex};
use spinne_logger::Logger;
use std::path::PathBuf;

use super::project_types::{Project, SourceProject, ConsumerProject};
use crate::package_json::PackageJson;

/// Represents a workspace containing multiple projects
pub struct Workspace {
    workspace_root: PathBuf,
    projects: Vec<Box<dyn Project>>,
    graph: Graph<usize, ()>,
}

impl Workspace {
    /// Creates a new Workspace instance from a given path
    pub fn new(workspace_root: PathBuf) -> Self {
        Self {
            workspace_root,
            projects: Vec::new(),
            graph: Graph::new(),
        }
    }

    /// Discovers and analyzes all projects in the workspace
    pub fn discover_projects(&mut self) {
        Logger::info(&format!(
            "Traversing workspace: {}",
            self.workspace_root.display()
        ));

        // First pass: discover all projects
        let mut discovered_projects = Vec::new();
        
        let walker = WalkBuilder::new(&self.workspace_root)
            .hidden(false) // We want to find .git folders
            .git_ignore(true)
            .build();

        for entry in walker {
            match entry {
                Ok(entry) => self.discover_project(&entry, &mut discovered_projects),
                Err(e) => Logger::error(&format!("Error while walking directory: {}", e)),
            }
        }

        Logger::info(&format!("Found {} projects", discovered_projects.len()));
        
        // Second pass: classify projects as source or consumer
        self.classify_projects(discovered_projects);
    }
    
    /// Discovers a single project and adds it to the list of discovered projects
    fn discover_project(&self, entry: &DirEntry, discovered_projects: &mut Vec<(PathBuf, String)>) {
        let path = entry.path();

        // Only check directories
        if !path.is_dir() {
            return;
        }

        // Check if this is a .git directory
        if path.file_name().map_or(false, |name| name == ".git") {
            let project_root = path.parent().unwrap_or(path).to_path_buf();

            // Check if package.json exists in the project root
            let package_json_path = project_root.join("package.json");
            if package_json_path.exists() {
                // Read the project name from package.json
                if let Some(package_json) = PackageJson::read(&package_json_path, false) {
                    if let Some(project_name) = package_json.name {
                        Logger::info(&format!("Found project at: {} ({})", project_root.display(), project_name));
                        discovered_projects.push((project_root, project_name));
                    }
                }
            }
        }
    }
    
    /// Classifies discovered projects as source or consumer projects
    fn classify_projects(&mut self, discovered_projects: Vec<(PathBuf, String)>) {
        // First, create a map of project names to their indices for quick lookup
        let mut project_indices = std::collections::HashMap::new();
        
        // Create a temporary graph to track dependencies
        let mut temp_graph = Graph::<usize, ()>::new();
        
        // First pass: add all projects to the graph and create source projects
        for (i, (project_root, project_name)) in discovered_projects.iter().enumerate() {
            // Add node to the graph
            let node_idx = temp_graph.add_node(i);
            project_indices.insert(project_name.clone(), node_idx);
            
            // Create a source project
            let source_project = SourceProject::new(project_root.clone());
            self.projects.push(Box::new(source_project));
        }
        
        // Second pass: add edges based on dependencies and identify consumer projects
        for (i, (project_root, project_name)) in discovered_projects.iter().enumerate() {
            // Read dependencies from package.json
            if let Some(package_json) = PackageJson::read(&project_root.join("package.json"), true) {
                if let Some(deps) = package_json.get_all_dependencies() {
                    // For each dependency, check if it matches any project name
                    for dep_name in deps {
                        if let Some(&dep_idx) = project_indices.get(&dep_name) {
                            // Add edge from dependent to dependency
                            let node_idx = NodeIndex::new(i);
                            temp_graph.add_edge(node_idx, dep_idx, ());
                        }
                    }
                }
            }
        }
        
        // Third pass: identify consumer projects and update the graph
        let mut consumer_indices = Vec::new();
        
        for (i, (project_root, project_name)) in discovered_projects.iter().enumerate() {
            // Check if this project has any outgoing edges (depends on other projects)
            let node_idx = NodeIndex::new(i);
            if temp_graph.edges_directed(node_idx, petgraph::Direction::Outgoing).count() > 0 {
                // This is a consumer project
                consumer_indices.push(i);
                
                // Replace the source project with a consumer project
                let mut consumer_project = ConsumerProject::new(project_root.clone());
                
                // Add source projects that this consumer depends on
                if let Some(package_json) = PackageJson::read(&project_root.join("package.json"), true) {
                    if let Some(deps) = package_json.get_all_dependencies() {
                        for dep_name in deps {
                            if let Some(&dep_idx) = project_indices.get(&dep_name) {
                                let dep_i = temp_graph[dep_idx];
                                if let Some(source_project) = self.projects.get(dep_i).and_then(|p| p.as_any().downcast_ref::<SourceProject>()) {
                                    consumer_project.add_source_project(source_project.clone());
                                }
                            }
                        }
                    }
                }
                
                // Replace the source project with the consumer project
                self.projects[i] = Box::new(consumer_project);
            }
        }
        
        // Update the graph with the final project structure
        self.graph = temp_graph;
        
        Logger::info(&format!("Classified {} projects as consumers", consumer_indices.len()));
    }

    /// Traverses all discovered projects to analyze their components in dependency order
    pub fn traverse_projects(&mut self, exclude: &Vec<String>, include: &Vec<String>) {
        // Build dependency graph
        let dep_graph = self.build_dependency_graph();
        self.graph = dep_graph;

        // Get topological sort
        match toposort(&self.graph, None) {
            Ok(sorted_projects) => {
                Logger::info("Traversing projects in dependency order");
                // Traverse in order
                for node_idx in sorted_projects {
                    let project_idx = self.graph[node_idx];
                    let project = &mut self.projects[project_idx];
                    Logger::info(&format!("Traversing project: {}", project.get_name()));
                    project.traverse(exclude, include);
                }
            }
            Err(_) => {
                Logger::warn(
                    "Circular dependencies detected, falling back to sequential traversal",
                );
                // Fallback to regular traversal
                for project in &mut self.projects {
                    project.traverse(exclude, include);
                }
            }
        }
    }

    /// Gets a reference to all discovered projects
    pub fn get_projects(&self) -> &Vec<Box<dyn Project>> {
        &self.projects
    }

    fn build_dependency_graph(&self) -> Graph<usize, ()> {
        let mut graph = Graph::<usize, ()>::new();

        // Add nodes for each project with their index
        let node_indices: Vec<_> = (0..self.projects.len())
            .map(|i| graph.add_node(i))
            .collect();

        // Add edges based on dependencies
        for (i, dependent_project) in self.projects.iter().enumerate() {
            // Get all dependencies of the current project
            if let Some(deps) = dependent_project.get_dependencies() {
                // For each dependency, check if it matches any project name
                for dep_name in deps {
                    // Find the project index with this name
                    if let Some(dep_idx) = self
                        .projects
                        .iter()
                        .position(|p| p.get_name() == dep_name)
                    {
                        Logger::debug(
                            &format!(
                                "Found dependency: {} -> {}",
                                dependent_project.get_name(), self.projects[dep_idx].get_name()
                            ),
                            2,
                        );
                        // Add edge from dependent to dependency
                        graph.add_edge(node_indices[i], node_indices[dep_idx], ());
                    }
                }
            }
        }

        graph
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::test_utils;

    #[test]
    fn test_workspace_discovery() {
        let temp_dir = test_utils::create_mock_project(&vec![
            // Project 1
            ("projects/project1/.git/HEAD", "ref: refs/heads/main"),
            ("projects/project1/package.json", r#"{"name": "project1"}"#),
            (
                "projects/project1/src/components/Button.tsx",
                r#"
                    import React from 'react';
                    export const Button = () => <button>Click me</button>;
                "#,
            ),
            // Project 2 in subdirectory
            ("projects/project2/.git/HEAD", "ref: refs/heads/main"),
            ("projects/project2/package.json", r#"{"name": "project2"}"#),
            (
                "projects/project2/src/App.tsx",
                r#"
                    import React from 'react';
                    export const App = () => <div>Hello</div>;
                "#,
            ),
        ]);

        let mut workspace = Workspace::new(temp_dir.path().to_path_buf());
        workspace.discover_projects();

        assert_eq!(workspace.get_projects().len(), 2);
    }

    #[test]
    fn test_project_sorting() {
        let temp_dir = test_utils::create_mock_project(&vec![
            // Project 1 - has no dependencies
            ("project1/.git/HEAD", "ref: refs/heads/main"),
            ("project1/package.json", r#"{"name": "project1"}"#),

            // Project 2 - depends on project1
            ("project2/.git/HEAD", "ref: refs/heads/main"),
            (
                "project2/package.json",
                r#"{
                "name": "project2",
                "dependencies": {
                    "project1": "1.0.0"
                }
            }"#,
            ),

            // Project 3 - depends on project2
            ("project3/.git/HEAD", "ref: refs/heads/main"),
            (
                "project3/package.json",
                r#"{
                "name": "project3",
                "dependencies": {
                    "project2": "1.0.0"
                }
            }"#,
            ),
        ]);

        let mut workspace = Workspace::new(temp_dir.path().to_path_buf());
        workspace.discover_projects();
        workspace.traverse_projects(&vec![], &vec![]);

        let graph = workspace.graph;

        assert_eq!(graph.edge_count(), 2);
        assert_eq!(graph.node_weight(0.into()), Some(&0));
        assert_eq!(graph.node_weight(1.into()), Some(&1));
        assert_eq!(graph.node_weight(2.into()), Some(&2));
    }
}
