use ignore::{DirEntry, WalkBuilder};
use petgraph::{algo::toposort, graph::NodeIndex, Graph};
use spinne_logger::Logger;
use std::path::PathBuf;

use super::Project;

/// Represents a workspace containing multiple projects
pub struct Workspace {
    workspace_root: PathBuf,
    projects: Vec<Project>,
}

impl Workspace {
    /// Creates a new Workspace instance from a given path
    pub fn new(workspace_root: PathBuf) -> Self {
        Self {
            workspace_root,
            projects: Vec::new(),
        }
    }

    /// Discovers and analyzes all projects in the workspace
    pub fn discover_projects(&mut self) {
        Logger::info(&format!(
            "Traversing workspace: {}",
            self.workspace_root.display()
        ));

        let walker = WalkBuilder::new(&self.workspace_root)
            .hidden(false) // We want to find .git folders
            .git_ignore(true)
            .build();

        for entry in walker {
            match entry {
                Ok(entry) => self.check_project_root(&entry),
                Err(e) => Logger::error(&format!("Error while walking directory: {}", e)),
            }
        }

        Logger::info(&format!("Found {} projects", self.projects.len()));
    }

    /// Traverses all discovered projects to analyze their components in dependency order
    pub fn traverse_projects(&mut self, exclude: &Vec<String>, include: &Vec<String>) {
        // Build dependency graph
        let dep_graph = self.build_dependency_graph();

        // Get topological sort
        match toposort(&dep_graph, None) {
            Ok(sorted_projects) => {
                Logger::info("Traversing projects in dependency order");
                // Traverse in order
                for node_idx in sorted_projects {
                    let project_idx = dep_graph[node_idx];
                    let project = &mut self.projects[project_idx];
                    Logger::info(&format!("Traversing project: {}", project.project_name));
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
    pub fn get_projects(&self) -> &Vec<Project> {
        &self.projects
    }

    /// Checks if a directory entry is a project root by looking for package.json and .git
    fn check_project_root(&mut self, entry: &DirEntry) {
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
                Logger::info(&format!("Found project at: {}", project_root.display()));

                // Create and add the project
                let project = Project::new(project_root);
                self.projects.push(project);
            }
        }
    }

    fn build_dependency_graph(&self) -> Graph<usize, ()> {
        let mut graph = Graph::<usize, ()>::new();

        // Add nodes for each project with their index
        let node_indices: Vec<_> = (0..self.projects.len())
            .map(|i| graph.add_node(i))
            .collect();

        // Add edges based on dependencies
        for (i, project) in self.projects.iter().enumerate() {
            if let Some(dep_name) = project.find_dependency("react") {
                // Find the project index with this name
                if let Some(dep_idx) = self
                    .projects
                    .iter()
                    .position(|p| p.project_name == dep_name)
                {
                    // Add edge from dependent to dependency
                    graph.add_edge(node_indices[i], node_indices[dep_idx], ());
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
}
