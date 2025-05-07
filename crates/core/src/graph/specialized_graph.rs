use std::{
    collections::{HashMap, HashSet},
    hash::{DefaultHasher, Hash, Hasher},
    path::PathBuf,
};
use serde_json::Value;

/// Represents a component with its project context
#[derive(Debug, Clone)]
pub struct ComponentNode {
    /// Unique identifier for the component
    pub id: u64,
    /// Name of the component
    pub name: String,
    /// Path to the component's file
    pub file_path: PathBuf,
    /// Properties of the component
    pub props: HashMap<String, usize>,
}

impl ComponentNode {
    pub fn new(
        name: String,
        file_path: PathBuf,
        props: HashMap<String, usize>,
    ) -> Self {
        let id = Self::compute_hash(&name, &file_path);
        Self {
            id,
            name,
            file_path,
            props,
        }
    }

    /// Computes a hash from a component's name and path
    fn compute_hash(name: &str, path: &PathBuf) -> u64 {
        let mut hasher = DefaultHasher::new();
        name.hash(&mut hasher);
        path.hash(&mut hasher);
        hasher.finish()
    }
}

/// Represents a dependency between components
#[derive(Debug, Clone)]
pub struct ComponentEdge {
    /// If cross-project, which project is being referenced
    pub project_context: Option<String>,
}

/// Information about a component including its project context
#[derive(Debug, Clone)]
pub struct ComponentInfo {
    /// The component node itself
    pub node: ComponentNode,
    /// Which project this component belongs to
    pub project: String,
}

/// Bidirectional dependency information for a component
#[derive(Debug, Clone, Default)]
pub struct DependencyInfo {
    /// Components that depend on this component
    pub dependents: HashSet<u64>,
    /// Components this component depends on
    pub dependencies: HashMap<u64, ComponentEdge>,
}

/// Indices for looking up components
#[derive(Debug, Clone, Default)]
pub struct ComponentIndices {
    /// Lookup by name and project
    pub by_name: HashMap<(String, String), u64>,
    /// Lookup by import path
    pub by_path: HashMap<PathBuf, u64>,
    /// Lookup by project
    pub by_project: HashMap<String, HashSet<u64>>,
}

/// Represents a node in the traversal result
#[derive(Debug)]
pub struct TraversalNode {
    pub component_id: u64,
    pub component_name: String,
    pub file_path: PathBuf,
    pub project: String,
    pub dependency_ids: Vec<u64>,
    pub depth: usize,
}

/// A specialized graph structure for tracking components and their dependencies
#[derive(Debug, Clone)]
pub struct ComponentRegistry {
    /// All components indexed by their ID
    components: HashMap<u64, ComponentInfo>,
    /// Dependencies between components (bidirectional)
    dependencies: HashMap<u64, DependencyInfo>,
    /// Indices for looking up components
    indices: ComponentIndices,
}

impl ComponentRegistry {
    /// Creates a new empty registry
    pub fn new() -> Self {
        Self {
            components: HashMap::new(),
            dependencies: HashMap::new(),
            indices: ComponentIndices::default(),
        }
    }

    /// Adds a component to the registry
    pub fn add_component(&mut self, component: ComponentNode, project: String) {
        let id = component.id;
        let name = component.name.clone();
        let file_path = component.file_path.clone();
        
        let info = ComponentInfo {
            node: component,
            project: project.clone(),
        };

        // Add to main component store
        self.components.insert(id, info);

        // Add to indices
        self.indices.by_name.insert((name, project.clone()), id);
        self.indices.by_path.insert(file_path, id);
        let project_components = self.indices.by_project
            .entry(project)
            .or_insert_with(HashSet::new);
        project_components.insert(id);

        // Initialize empty dependency info
        self.dependencies.insert(id, DependencyInfo::default());
    }

    /// Adds a dependency between components
    pub fn add_dependency(&mut self, from: u64, to: u64, project_context: Option<String>) -> Result<(), String> {
        // Validate both components exist
        if !self.components.contains_key(&from) {
            return Err(format!("Source component {} not found", from));
        }
        if !self.components.contains_key(&to) {
            return Err(format!("Target component {} not found", to));
        }

        let edge = ComponentEdge { project_context };
        
        // Add forward dependency
        let from_deps = self.dependencies
            .entry(from)
            .or_insert_with(DependencyInfo::default);
        from_deps.dependencies.insert(to, edge);

        // Add reverse dependency
        let to_deps = self.dependencies
            .entry(to)
            .or_insert_with(DependencyInfo::default);
        to_deps.dependents.insert(from);
        
        Ok(())
    }

    /// Gets a component by its ID
    pub fn get_component(&self, id: u64) -> Option<&ComponentInfo> {
        self.components.get(&id)
    }

    /// Finds a component by name and project
    pub fn find_component(&self, name: &str, project: &str) -> Option<&ComponentInfo> {
        self.indices.by_name
            .get(&(name.to_string(), project.to_string()))
            .and_then(|&id| self.components.get(&id))
    }

    /// Finds a component by import path
    pub fn find_by_import(&self, path: &PathBuf) -> Option<&ComponentInfo> {
        self.indices.by_path
            .get(path)
            .and_then(|&id| self.components.get(&id))
    }

    /// Gets all components in a project
    pub fn get_project_components(&self, project: &str) -> Vec<&ComponentInfo> {
        self.indices.by_project
            .get(project)
            .map(|ids| {
                ids.iter()
                    .filter_map(|&id| self.components.get(&id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Gets all dependencies for a component
    pub fn get_dependencies(&self, component_id: u64) -> Vec<(u64, &ComponentEdge)> {
        self.dependencies
            .get(&component_id)
            .map(|info| {
                info.dependencies
                    .iter()
                    .map(|(id, edge)| (*id, edge))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Gets all components that depend on a given component
    pub fn get_dependents(&self, component_id: u64) -> Vec<u64> {
        self.dependencies
            .get(&component_id)
            .map(|info| info.dependents.iter().copied().collect())
            .unwrap_or_default()
    }

    /// Traverses the graph starting from a component
    pub fn traverse_from(&self, start_id: u64) -> Vec<TraversalNode> {
        let mut visited = HashSet::new();
        let mut result = Vec::new();
        self.traverse_recursive(start_id, 0, &mut visited, &mut result);
        result
    }

    /// Recursive helper for graph traversal
    fn traverse_recursive(
        &self,
        component_id: u64,
        depth: usize,
        visited: &mut HashSet<u64>,
        result: &mut Vec<TraversalNode>,
    ) {
        if visited.contains(&component_id) {
            return;
        }
        visited.insert(component_id);

        if let Some(component) = self.components.get(&component_id) {
            let deps = self.get_dependencies(component_id);
            let dep_ids: Vec<u64> = deps.iter().map(|(id, _)| *id).collect();
            
            result.push(TraversalNode {
                component_id,
                component_name: component.node.name.clone(),
                file_path: component.node.file_path.clone(),
                project: component.project.clone(),
                dependency_ids: dep_ids.clone(),
                depth,
            });

            for dep_id in dep_ids {
                self.traverse_recursive(dep_id, depth + 1, visited, result);
            }
        }
    }

    /// Removes a component and all its dependencies
    pub fn remove_component(&mut self, component_id: u64) {
        if let Some(component) = self.components.remove(&component_id) {
            // Remove from indices
            self.indices.by_name.remove(&(component.node.name, component.project.clone()));
            self.indices.by_path.remove(&component.node.file_path);
            if let Some(project_components) = self.indices.by_project.get_mut(&component.project) {
                project_components.remove(&component_id);
            }
            
            // Remove from dependencies
            if let Some(deps) = self.dependencies.remove(&component_id) {
                // Remove this component from other components' dependents
                for dep_id in deps.dependencies.keys() {
                    if let Some(dep_info) = self.dependencies.get_mut(dep_id) {
                        dep_info.dependents.remove(&component_id);
                    }
                }
                
                // Remove this component from other components' dependencies
                for dep_id in deps.dependents {
                    if let Some(dep_info) = self.dependencies.get_mut(&dep_id) {
                        dep_info.dependencies.remove(&component_id);
                    }
                }
            }
        }
    }

    /// Converts the registry into a serializable format for JSON output
    pub fn to_serializable(&self) -> Value {
        // Group components by project
        let mut projects = HashMap::new();
        
        for (id, info) in &self.components {
            let project = info.project.clone();
            let entry = projects.entry(project).or_insert_with(|| {
                serde_json::json!({
                    "components": Vec::<Value>::new(),
                    "edges": Vec::<Value>::new()
                })
            });

            // Add component
            let component_json = serde_json::json!({
                "id": id,
                "name": info.node.name,
                "path": info.node.file_path,
                "props": info.node.props,
                "project": info.project
            });
            entry["components"].as_array_mut().unwrap().push(component_json);

            // Add edges
            if let Some(deps) = self.dependencies.get(id) {
                for (target_id, edge) in &deps.dependencies {
                    let edge_json = serde_json::json!({
                        "from": id,
                        "to": target_id,
                        "project_context": edge.project_context
                    });
                    entry["edges"].as_array_mut().unwrap().push(edge_json);
                }
            }
        }

        // Convert to final format
        let mut result = Vec::new();
        for (project_name, data) in projects {
            result.push(serde_json::json!({
                "name": project_name,
                "graph": data
            }));
        }

        serde_json::Value::Array(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_component() {
        let mut registry = ComponentRegistry::new();
        let component = ComponentNode::new(
            "TestComponent".to_string(),
            PathBuf::from("src/TestComponent.tsx"),
            HashMap::new(),
        );
        registry.add_component(component.clone(), "test-project".to_string());
        
        assert_eq!(registry.get_project_components("test-project").len(), 1);
        assert!(registry.find_component("TestComponent", "test-project").is_some());
    }

    #[test]
    fn test_add_dependency() {
        let mut registry = ComponentRegistry::new();
        let component1 = ComponentNode::new(
            "Component1".to_string(),
            PathBuf::from("src/Component1.tsx"),
            HashMap::new(),
        );
        let component2 = ComponentNode::new(
            "Component2".to_string(),
            PathBuf::from("src/Component2.tsx"),
            HashMap::new(),
        );
        
        registry.add_component(component1.clone(), "test-project".to_string());
        registry.add_component(component2.clone(), "test-project".to_string());
        
        assert!(registry
            .add_dependency(component1.id, component2.id, None)
            .is_ok());
            
        assert_eq!(registry.get_dependencies(component1.id).len(), 1);
        assert_eq!(registry.get_dependents(component2.id).len(), 1);
    }

    #[test]
    fn test_cross_project_dependency() {
        let mut registry = ComponentRegistry::new();
        let component1 = ComponentNode::new(
            "Component1".to_string(),
            PathBuf::from("src/Component1.tsx"),
            HashMap::new(),
        );
        let component2 = ComponentNode::new(
            "Component2".to_string(),
            PathBuf::from("src/Component2.tsx"),
            HashMap::new(),
        );
        
        registry.add_component(component1.clone(), "project1".to_string());
        registry.add_component(component2.clone(), "project2".to_string());
        
        assert!(registry
            .add_dependency(component1.id, component2.id, Some("project2".to_string()))
            .is_ok());
            
        let deps = registry.get_dependencies(component1.id);
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].1.project_context, Some("project2".to_string()));
    }

    #[test]
    fn test_traverse() {
        let mut registry = ComponentRegistry::new();
        let component1 = ComponentNode::new(
            "Component1".to_string(),
            PathBuf::from("src/Component1.tsx"),
            HashMap::new(),
        );
        let component2 = ComponentNode::new(
            "Component2".to_string(),
            PathBuf::from("src/Component2.tsx"),
            HashMap::new(),
        );
        let component3 = ComponentNode::new(
            "Component3".to_string(),
            PathBuf::from("src/Component3.tsx"),
            HashMap::new(),
        );
        
        registry.add_component(component1.clone(), "test-project".to_string());
        registry.add_component(component2.clone(), "test-project".to_string());
        registry.add_component(component3.clone(), "test-project".to_string());
        
        registry.add_dependency(component1.id, component2.id, None).unwrap();
        registry.add_dependency(component2.id, component3.id, None).unwrap();
        
        let traversal = registry.traverse_from(component1.id);
        assert_eq!(traversal.len(), 3);
        assert_eq!(traversal[0].depth, 0);
        assert_eq!(traversal[1].depth, 1);
        assert_eq!(traversal[2].depth, 2);
    }

    #[test]
    fn test_remove_component() {
        let mut registry = ComponentRegistry::new();
        let component1 = ComponentNode::new(
            "Component1".to_string(),
            PathBuf::from("src/Component1.tsx"),
            HashMap::new(),
        );
        let component2 = ComponentNode::new(
            "Component2".to_string(),
            PathBuf::from("src/Component2.tsx"),
            HashMap::new(),
        );
        
        registry.add_component(component1.clone(), "test-project".to_string());
        registry.add_component(component2.clone(), "test-project".to_string());
        registry.add_dependency(component1.id, component2.id, None).unwrap();
        
        registry.remove_component(component1.id);
        assert_eq!(registry.get_project_components("test-project").len(), 1);
        assert!(registry.get_dependencies(component1.id).is_empty());
        assert!(registry.get_dependents(component2.id).is_empty());
    }
} 