use petgraph::{graph::NodeIndex, Graph};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf};

use crate::logging::Logger;

#[derive(Debug, Serialize, Deserialize)]
pub struct Component {
    pub name: String,
    pub file_path: PathBuf,
    pub prop_usage: HashMap<String, usize>,
}

/// ComponentGraph is a graph of components and their relationships.
/// Components are nodes and relationships are edges.
#[derive(Debug)]
pub struct ComponentGraph {
    pub graph: Graph<Component, ()>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SerializableComponentGraph {
    pub nodes: Vec<Component>,
    pub edges: Vec<(usize, usize)>,
}

impl ComponentGraph {
    pub fn new() -> Self {
        Self {
            graph: Graph::new(),
        }
    }

    pub fn has_component(&self, key: &str, file_path: &PathBuf) -> bool {
        self.graph
            .node_indices()
            .any(|i| self.graph[i].name == key && self.graph[i].file_path == *file_path)
    }

    pub fn get_component(&self, key: &str, file_path: &PathBuf) -> Option<NodeIndex> {
        self.graph
            .node_indices()
            .find(|i| self.graph[*i].name == key && self.graph[*i].file_path == *file_path)
    }

    pub fn add_component(&mut self, key: String, file_path: PathBuf) -> NodeIndex {
        if !self.has_component(&key, &file_path) {
            Logger::debug(&format!("Adding new component: {:?}", key), 2);
            let node_index = self.graph.add_node(Component {
                name: key,
                file_path,
                prop_usage: HashMap::new(),
            });
            node_index
        } else {
            Logger::debug(&format!("Called add_component for existing component: {:?}", key), 2);
            self.graph
                .node_indices()
                .find(|i| self.graph[*i].name == key && self.graph[*i].file_path == file_path)
                .unwrap()
        }
    }

    pub fn add_child(&mut self, parent: (&str, &PathBuf), child: (&str, &PathBuf)) {
        let parent_index = self.get_or_add_component(parent.0, parent.1.clone());
        let child_index = self.get_or_add_component(child.0, child.1.clone());

        Logger::debug(&format!("Adding child edge: {:?} -> {:?}", parent.0, child.0), 2);
        self.graph.add_edge(parent_index, child_index, ());
    }

    fn get_or_add_component(&mut self, name: &str, file_path: PathBuf) -> NodeIndex {
        match self.get_component(name, &file_path) {
            Some(index) => index,
            None => self.add_component(name.to_string(), file_path),
        }
    }

    pub fn add_prop_usage(&mut self, component: &str, file_path: &PathBuf, prop: String) {
        if let Some(node_index) = self.get_component(component, file_path) {
            let component = &mut self.graph[node_index];

            Logger::debug(&format!("Adding prop usage: {:?} -> {:?}", component, prop), 2);
            *component.prop_usage.entry(prop).or_insert(0) += 1;
        }
    }

    pub fn print_graph(&self) {
        println!("{:?}", self.graph);
    }

    pub fn to_serializable(&self) -> SerializableComponentGraph {
        let nodes: Vec<Component> = self
            .graph
            .node_indices()
            .map(|i| Component {
                name: self.graph[i].name.clone(),
                file_path: self.graph[i].file_path.clone(),
                prop_usage: self.graph[i].prop_usage.clone(),
            })
            .collect();

        let edges: Vec<(usize, usize)> = self
            .graph
            .edge_indices()
            .map(|e| {
                let (a, b) = self.graph.edge_endpoints(e).unwrap();
                (a.index(), b.index())
            })
            .collect();

        SerializableComponentGraph { nodes, edges }
    }

    pub fn from_serializable(serializable: SerializableComponentGraph) -> Self {
        let mut graph = Graph::new();
        let node_indices: Vec<NodeIndex> = serializable
            .nodes
            .into_iter()
            .map(|component| graph.add_node(component))
            .collect();

        for (from, to) in serializable.edges {
            graph.add_edge(node_indices[from], node_indices[to], ());
        }

        ComponentGraph { graph }
    }
}

#[test]
fn test_new() {
    let graph = ComponentGraph::new();
    assert_eq!(graph.graph.node_count(), 0);
    assert_eq!(graph.graph.edge_count(), 0);
}

#[test]
fn test_add_component() {
    let mut graph = ComponentGraph::new();
    let file_path = PathBuf::from("/path/to/Component.tsx");
    let key = "MyComponent".to_string();

    let node_index = graph.add_component(key.clone(), file_path.clone());

    assert_eq!(graph.graph.node_count(), 1);
    assert_eq!(graph.graph.edge_count(), 0);
    assert!(graph.has_component(&key, &file_path));
    assert_eq!(graph.graph[node_index].name, key);
    assert_eq!(graph.graph[node_index].file_path, file_path);
}

#[test]
fn test_add_duplicate_component() {
    let mut graph = ComponentGraph::new();
    let file_path = PathBuf::from("/path/to/Component.tsx");
    let key = "MyComponent".to_string();

    let node_index = graph.add_component(key.clone(), file_path.clone());
    let second_node_index = graph.add_component(key.clone(), file_path.clone());

    assert_eq!(graph.graph.node_count(), 1);
    assert_eq!(graph.graph.edge_count(), 0);
    assert!(graph.has_component(&key, &file_path));
    assert_eq!(second_node_index, node_index);
}

#[test]
fn test_add_child() {
    let mut graph = ComponentGraph::new();
    let parent_key = "ParentComponent".to_string();
    let child_key = "ChildComponent".to_string();

    let parent_node_index = graph.add_component(
        parent_key.clone(),
        PathBuf::from("/path/to/ParentComponent.tsx"),
    );
    let child_node_index = graph.add_component(
        child_key.clone(),
        PathBuf::from("/path/to/ChildComponent.tsx"),
    );

    graph.add_child(
        (&parent_key, &PathBuf::from("/path/to/ParentComponent.tsx")),
        (&child_key, &PathBuf::from("/path/to/ChildComponent.tsx")),
    );

    assert_eq!(graph.graph.node_count(), 2);
    assert_eq!(graph.graph.edge_count(), 1);
    assert!(graph.has_component(&parent_key, &PathBuf::from("/path/to/ParentComponent.tsx")));
    assert!(graph.has_component(&child_key, &PathBuf::from("/path/to/ChildComponent.tsx")));
    assert!(graph
        .graph
        .contains_edge(parent_node_index, child_node_index));
}

#[test]
fn test_complex_graph_structure() {
    let mut graph = ComponentGraph::new();

    // Add components
    let app_path = PathBuf::from("/path/to/App.tsx");
    let app = graph.add_component("App".to_string(), app_path.clone());
    let header = graph.add_component("Header".to_string(), PathBuf::from("/path/to/Header.tsx"));

    let footer = graph.add_component("Footer".to_string(), PathBuf::from("/path/to/Footer.tsx"));
    let content = graph.add_component("Content".to_string(), PathBuf::from("/path/to/Content.tsx"));

    // Add relationships
    graph.add_child(
        ("App", &app_path),
        ("Header", &PathBuf::from("/path/to/Header.tsx")),
    );
    graph.add_child(
        ("App", &app_path),
        ("Footer", &PathBuf::from("/path/to/Footer.tsx")),
    );
    graph.add_child(
        ("App", &app_path),
        ("Content", &PathBuf::from("/path/to/Content.tsx")),
    );

    // Assertions
    assert_eq!(graph.graph.node_count(), 4);
    assert_eq!(graph.graph.edge_count(), 3);

    let app_component = &graph.graph[app];
    assert_eq!(app_component.name, "App");
    assert_eq!(app_component.file_path, app_path);

    let header_component = &graph.graph[header];
    assert_eq!(header_component.name, "Header");
    assert_eq!(
        header_component.file_path,
        PathBuf::from("/path/to/Header.tsx")
    );

    let footer_component = &graph.graph[footer];
    assert_eq!(footer_component.name, "Footer");
    assert_eq!(
        footer_component.file_path,
        PathBuf::from("/path/to/Footer.tsx")
    );

    let content_component = &graph.graph[content];
    assert_eq!(content_component.name, "Content");
    assert_eq!(
        content_component.file_path,
        PathBuf::from("/path/to/Content.tsx")
    );

    assert!(graph.graph.contains_edge(app, header));
    assert!(graph.graph.contains_edge(app, footer));
    assert!(graph.graph.contains_edge(app, content));

    graph.print_graph();
}

#[test]
fn test_cyclic_dependency() {
    let mut graph = ComponentGraph::new();

    let component_a = graph.add_component(
        "ComponentA".to_string(),
        PathBuf::from("/path/to/ComponentA.tsx"),
    );
    let component_b = graph.add_component(
        "ComponentB".to_string(),
        PathBuf::from("/path/to/ComponentB.tsx"),
    );

    graph.add_child(
        ("ComponentA", &PathBuf::from("/path/to/ComponentA.tsx")),
        ("ComponentB", &PathBuf::from("/path/to/ComponentB.tsx")),
    );
    graph.add_child(
        ("ComponentB", &PathBuf::from("/path/to/ComponentB.tsx")),
        ("ComponentA", &PathBuf::from("/path/to/ComponentA.tsx")),
    );

    assert_eq!(graph.graph.node_count(), 2);

    assert_eq!(graph.graph.edge_count(), 2);
    assert!(graph.graph.contains_edge(component_a, component_b));
    assert!(graph.graph.contains_edge(component_b, component_a));
}


#[test]
fn test_to_serializable() {
    let mut graph = ComponentGraph::new();
    graph.add_component("ComponentA".to_string(), PathBuf::from("/path/to/ComponentA.tsx"));
    graph.add_component("ComponentB".to_string(), PathBuf::from("/path/to/ComponentB.tsx"));

    graph.add_child(("ComponentA", &PathBuf::from("/path/to/ComponentA.tsx")), ("ComponentB", &PathBuf::from("/path/to/ComponentB.tsx")));

    let serializable = graph.to_serializable();
    let json = serde_json::to_string_pretty(&serializable).unwrap();
    println!("{}", json);
    assert_eq!(serializable.nodes.len(), 2);
    assert_eq!(serializable.edges.len(), 1);
}