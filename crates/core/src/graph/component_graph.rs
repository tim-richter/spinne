use serde_json::Value;
use std::{
    collections::{HashMap, HashSet},
    hash::{DefaultHasher, Hash, Hasher},
    path::PathBuf,
};

/// Represents a component with a unique identifier.
#[derive(Debug, Clone)]
pub struct Component {
    /// The unique identifier for the component.
    pub id: u64,
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
        let id = Self::compute_hash(&name, &path);
        Component {
            id,
            name,
            path,
            path_relative_to_root,
            props: HashMap::new(),
            project,
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


/// A simple graph where nodes are components identified by a unique value,
/// and edges represent the relationship "uses."
#[derive(Debug, Clone)]
pub struct ComponentGraph {
    /// Maps component IDs (hashes) to the actual components.
    nodes: HashMap<u64, Component>,
    /// Represents edges: key is a component ID, and value is a set of component IDs it uses.
    edges: HashMap<u64, HashSet<u64>>,
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
    pub fn has_component(&self, id: u64) -> bool {
        self.nodes.contains_key(&id)
    }

    /// Adds a component to the graph.
    /// If the component already exists, it updates the component.
    pub fn add_component(&mut self, component: Component) -> &Component {
        let id = component.id;
        self.nodes.insert(id, component);
        self.nodes.get(&id).unwrap()
    }

    /// Adds a directional edge from `from_id` to `to_id`.
    /// Returns `false` if either component does not exist.
    pub fn add_edge(&mut self, from_id: u64, to_id: u64) -> bool {
        if !self.has_component(from_id) || !self.has_component(to_id) {
            return false;
        }

        self.edges.entry(from_id).or_default().insert(to_id)
    }

    /// Adds a component and its dependencies to the graph in a single operation.
    /// Returns a reference to the added/updated component.
    pub fn add_component_with_deps(
        &mut self,
        component: Component,
        deps: Vec<Component>,
    ) -> &Component {
        let component_id = component.id;
        self.nodes.insert(component_id, component);

        // Add all dependencies and create edges
        for dep in deps {
            let dep_id = dep.id;
            self.nodes.insert(dep_id, dep);
            self.edges.entry(component_id).or_default().insert(dep_id);
        }

        self.nodes.get(&component_id).unwrap()
    }

    /// Retrieves a component by its unique identifier.
    pub fn get_component(&self, id: u64) -> Option<&Component> {
        self.nodes.get(&id)
    }

    /// Retrieves a component by its name and path.
    /// This is a convenience method for finding a component by its name and path.
    pub fn find_component(&self, name: &str, path: &PathBuf) -> Option<&Component> {
        let id = Component::compute_hash(name, path);
        self.nodes.get(&id)
    }

    /// Gets the neighbors (i.e., components used by the specified component).
    pub fn get_neighbors(&self, id: u64) -> Option<&HashSet<u64>> {
        self.edges.get(&id)
    }

    /// Checks if there is an edge from `from_id` to `to_id`.
    pub fn has_edge(&self, from_id: u64, to_id: u64) -> bool {
        self.edges
            .get(&from_id)
            .map_or(false, |neighbors| neighbors.contains(&to_id))
    }

    /// Adds a property to a component.
    pub fn add_prop(&mut self, component_id: u64, prop: String) {
        if let Some(component) = self.nodes.get_mut(&component_id) {
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
        let comp1 = Component {
            id: 1,
            name: "Component A".to_string(),
            path: PathBuf::from("file1.rs"),
            path_relative_to_root: PathBuf::from("file1.rs"),
            props: HashMap::new(),
            project: "Project A".to_string(),
        };

        // Adding a new component should succeed.
        graph.add_component(comp1);
        assert!(graph.has_component(1));

        // Attempting to add a duplicate component (same id) should update the component.
        let comp1_dup = Component {
            id: 1,
            name: "Duplicate Component A".to_string(),
            path: PathBuf::from("file1.rs"),
            path_relative_to_root: PathBuf::from("file1.rs"),
            props: HashMap::new(),
            project: "Project A".to_string(),
        };
        graph.add_component(comp1_dup);
        assert!(graph.has_component(1));

        // Check that the component is stored correctly.
        let retrieved = graph.get_component(1).unwrap();
        assert_eq!(retrieved.name, "Duplicate Component A");
    }

    #[test]
    fn test_add_edge() {
        let mut graph = ComponentGraph::new();
        let comp1 = Component {
            id: 100,
            name: "Component 1".to_string(),
            path: PathBuf::from("file1.rs"),
            path_relative_to_root: PathBuf::from("file1.rs"),
            props: HashMap::new(),
            project: "Project A".to_string(),
        };
        let comp2 = Component {
            id: 200,
            name: "Component 2".to_string(),
            path: PathBuf::from("file2.rs"),
            path_relative_to_root: PathBuf::from("file2.rs"),
            props: HashMap::new(),
            project: "Project A".to_string(),
        };

        // Add components first.
        graph.add_component(comp1);
        graph.add_component(comp2);
        assert!(graph.has_component(100));
        assert!(graph.has_component(200));

        // Add a valid edge.
        graph.add_edge(100, 200);
        assert!(graph.has_edge(100, 200));

        // Ensure that the neighbor is correctly registered.
        let neighbors = graph.get_neighbors(100).unwrap();
        assert!(neighbors.contains(&200));
    }

    #[test]
    fn test_invalid_edges() {
        let mut graph = ComponentGraph::new();
        let comp1 = Component {
            id: 50,
            name: "Component 50".to_string(),
            path: PathBuf::from("file1.rs"),
            path_relative_to_root: PathBuf::from("file1.rs"),
            props: HashMap::new(),
            project: "Project A".to_string(),
        };

        // Add only one component.
        graph.add_component(comp1);
        assert!(graph.has_component(50));

        // Trying to add an edge where the source node doesn't exist.
        graph.add_edge(999, 50);
        assert!(!graph.has_edge(999, 50));

        // Trying to add an edge where the target node doesn't exist.
        graph.add_edge(50, 888);
        assert!(!graph.has_edge(50, 888));
    }

    #[test]
    fn test_get_neighbors_non_existing() {
        let graph = ComponentGraph::new();
        // Querying neighbors for a non-existent node should return None.
        assert!(graph.get_neighbors(10).is_none());
    }

    #[test]
    fn test_add_prop() {
        let mut graph = ComponentGraph::new();
        let comp = Component {
            id: 1,
            name: "Test".to_string(),
            path: PathBuf::from("test.rs"),
            path_relative_to_root: PathBuf::from("test.rs"),
            props: HashMap::new(),
            project: "Test Project".to_string(),
        };

        graph.add_component(comp);

        // Add the same prop twice
        graph.add_prop(1, "test_prop".to_string());
        graph.add_prop(1, "test_prop".to_string());

        // Verify the prop count is 2
        let component = graph.get_component(1).unwrap();
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
