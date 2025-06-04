use serde_json::Value;
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};
use sha2::{Digest, Sha256};

/// Represents a component with a unique identifier.
#[derive(Debug, Clone)]
pub struct Component {
    /// The unique identifier for the component.
    pub id: String,
    /// The name of the component.
    pub name: String,
    /// The path to the component's file.
    pub path: PathBuf,
    /// The path to the component's file relative to the project root.
    pub path_relative_to_root: PathBuf,
    /// The properties of the component.
    pub props: HashMap<String, usize>,
    /// The project the component belongs to.
    pub project: String,
}

impl Component {
    /// Creates a new component with a hash derived from its name and path
    pub fn new(
        name: String,
        path: PathBuf,
        path_relative_to_root: PathBuf,
        project: String,
    ) -> Self {
        let id = Self::compute_hash(&name, &path_relative_to_root);
        Component {
            id,
            name,
            path,
            path_relative_to_root,
            props: HashMap::new(),
            project,
        }
    }

    /// Computes a hash string from a component's name and path
    fn compute_hash(name: &str, path: &PathBuf) -> String {
        let mut hasher = Sha256::new();
        hasher.update(name.as_bytes());
        hasher.update(path.to_string_lossy().as_bytes());
        let result = hasher.finalize();
        hex::encode(result)
    }
}


/// A simple graph where nodes are components identified by a unique value,
/// and edges represent the relationship "uses."
#[derive(Debug, Clone)]
pub struct ComponentGraph {
    /// Maps component IDs (hashes) to the actual components.
    nodes: HashMap<String, Component>,
    /// Represents edges: key is a component ID, and value is a set of component IDs it uses.
    edges: HashMap<String, HashSet<String>>,
}

impl ComponentGraph {
    /// Creates a new, empty Graph.
    pub fn new() -> Self {
        ComponentGraph {
            nodes: HashMap::new(),
            edges: HashMap::new(),
        }
    }

    /// Checks if a component exists in the graph.
    pub fn has_component(&self, id: &str) -> bool {
        self.nodes.contains_key(id)
    }

    /// Adds a component to the graph.
    /// If the component already exists, it updates the component.
    pub fn add_component(&mut self, component: Component) -> &Component {
        let id = component.id.clone();
        self.nodes.insert(id.clone(), component);
        self.nodes.get(&id).unwrap()
    }

    /// Adds a directional edge from `from_id` to `to_id`.
    /// Returns `false` if either component does not exist or if the edge
    /// already existed.
    pub fn add_edge(&mut self, from_id: &str, to_id: &str) -> bool {
        if !self.has_component(from_id) || !self.has_component(to_id) {
            return false;
        }

        self.edges
            .entry(from_id.to_string())
            .or_default()
            .insert(to_id.to_string())
    }

    /// Adds a component and its dependencies to the graph in a single operation.
    /// Returns a reference to the added/updated component.
    pub fn add_component_with_deps(
        &mut self,
        component: Component,
        deps: Vec<Component>,
    ) -> &Component {
        let component_id = component.id.clone();
        self.nodes.insert(component_id.clone(), component);

        // Add all dependencies and create edges
        for dep in deps {
            let dep_id = dep.id.clone();
            self.nodes.insert(dep_id.clone(), dep);
            self
                .edges
                .entry(component_id.clone())
                .or_default()
                .insert(dep_id);
        }

        self.nodes.get(&component_id).unwrap()
    }

    /// Retrieves a component by its unique identifier.
    pub fn get_component(&self, id: &str) -> Option<&Component> {
        self.nodes.get(id)
    }

    /// Retrieves a component by its name and path.
    /// This is a convenience method for finding a component by its name and path.
    pub fn find_component(&self, name: &str, path: &PathBuf) -> Option<&Component> {
        let id = Component::compute_hash(name, path);
        self.nodes.get(&id)
    }

    /// Gets the neighbors (i.e., components used by the specified component).
    pub fn get_neighbors(&self, id: &str) -> Option<&HashSet<String>> {
        self.edges.get(id)
    }

    /// Checks if there is an edge from `from_id` to `to_id`.
    pub fn has_edge(&self, from_id: &str, to_id: &str) -> bool {
        self.edges
            .get(from_id)
            .map_or(false, |neighbors| neighbors.contains(to_id))
    }

    /// Adds a property to a component.
    pub fn add_prop(&mut self, component_id: &str, prop: String) {
        if let Some(component) = self.nodes.get_mut(component_id) {
            *component.props.entry(prop).or_insert(0) += 1;
        }
    }

    /// Returns the number of components in the graph.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Returns the number of edges in the graph.
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// Converts the component graph into a serializable format for JSON output
    pub fn to_serializable(&self) -> Value {
        let mut components = Vec::new();
        let mut edges = Vec::new();

        // Convert nodes to serializable format
        for component in self.nodes.values() {
            components.push(serde_json::json!({
                "id": component.id,
                "name": component.name,
                "path": component.path_relative_to_root,
                "props": component.props,
                "project": component.project,
            }));
        }

        // Convert edges to serializable format
        for (from_id, to_ids) in &self.edges {
            for to_id in to_ids {
                edges.push(serde_json::json!({
                    "from": from_id,
                    "to": to_id,
                }));
            }
        }

        serde_json::json!({
            "components": components,
            "edges": edges,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_component() {
        let mut graph = ComponentGraph::new();
        let comp1 = Component::new(
            "Component A".to_string(),
            PathBuf::from("file1.rs"),
            PathBuf::from("file1.rs"),
            "Project A".to_string(),
        );

        // Adding a new component should succeed.
        let id = comp1.id.clone();
        graph.add_component(comp1);
        assert!(graph.has_component(&id));

        // Attempting to add a duplicate component (same id) should update the component.
        let comp1_dup = Component::new(
            "Duplicate Component A".to_string(),
            PathBuf::from("file1.rs"),
            PathBuf::from("file1.rs"),
            "Project A".to_string(),
        );
        let dup_id = comp1_dup.id.clone();
        graph.add_component(comp1_dup);
        assert!(graph.has_component(&dup_id));

        // Check that the component is stored correctly.
        let retrieved = graph.get_component(&dup_id).unwrap();
        assert_eq!(retrieved.name, "Duplicate Component A");
    }

    #[test]
    fn test_add_edge() {
        let mut graph = ComponentGraph::new();
        let comp1 = Component::new(
            "Component 1".to_string(),
            PathBuf::from("file1.rs"),
            PathBuf::from("file1.rs"),
            "Project A".to_string(),
        );
        let comp2 = Component::new(
            "Component 2".to_string(),
            PathBuf::from("file2.rs"),
            PathBuf::from("file2.rs"),
            "Project A".to_string(),
        );

        // Add components first.
        let id1 = comp1.id.clone();
        let id2 = comp2.id.clone();
        graph.add_component(comp1);
        graph.add_component(comp2);
        assert!(graph.has_component(&id1));
        assert!(graph.has_component(&id2));

        // Add a valid edge.
        graph.add_edge(&id1, &id2);
        assert!(graph.has_edge(&id1, &id2));

        // Ensure that the neighbor is correctly registered.
        let neighbors = graph.get_neighbors(&id1).unwrap();
        assert!(neighbors.contains(&id2));
    }

    #[test]
    fn test_invalid_edges() {
        let mut graph = ComponentGraph::new();
        let comp1 = Component::new(
            "Component 50".to_string(),
            PathBuf::from("file1.rs"),
            PathBuf::from("file1.rs"),
            "Project A".to_string(),
        );

        // Add only one component.
        let id = comp1.id.clone();
        graph.add_component(comp1);
        assert!(graph.has_component(&id));

        // Trying to add an edge where the source node doesn't exist.
        graph.add_edge("not-exists", &id);
        assert!(!graph.has_edge("not-exists", &id));

        // Trying to add an edge where the target node doesn't exist.
        graph.add_edge(&id, "another");
        assert!(!graph.has_edge(&id, "another"));
    }

    #[test]
    fn test_get_neighbors_non_existing() {
        let graph = ComponentGraph::new();
        // Querying neighbors for a non-existent node should return None.
        assert!(graph.get_neighbors("does-not-exist").is_none());
    }

    #[test]
    fn test_add_prop() {
        let mut graph = ComponentGraph::new();
        let comp = Component::new(
            "Test".to_string(),
            PathBuf::from("test.rs"),
            PathBuf::from("test.rs"),
            "Test Project".to_string(),
        );

        let id = comp.id.clone();
        graph.add_component(comp);

        // Add the same prop twice
        graph.add_prop(&id, "test_prop".to_string());
        graph.add_prop(&id, "test_prop".to_string());

        // Verify the prop count is 2
        let component = graph.get_component(&id).unwrap();
        assert_eq!(*component.props.get("test_prop").unwrap(), 2);
    }

    #[test]
    fn test_component_hash_consistency() {
        let name = "TestComponent".to_string();
        let path = PathBuf::from("src/components/test.rs");

        // Create two components with the same data
        let component1 = Component::new(
            name.clone(),
            path.clone(),
            PathBuf::from("test.rs"),
            "Project A".to_string(),
        );
        let component2 = Component::new(
            name,
            path,
            PathBuf::from("test.rs"),
            "Project A".to_string(),
        );

        // They should have the same hash
        assert_eq!(component1.id, component2.id);
    }

    #[test]
    fn test_component_hash_uniqueness() {
        let component1 = Component::new(
            "Component1".to_string(),
            PathBuf::from("src/comp1.rs"),
            PathBuf::from("src/comp1.rs"),
            "Project A".to_string(),
        );
        let component2 = Component::new(
            "Component2".to_string(),
            PathBuf::from("src/comp2.rs"),
            PathBuf::from("src/comp2.rs"),
            "Project A".to_string(),
        );

        // Different components should have different hashes
        assert_ne!(component1.id, component2.id);
    }
}
