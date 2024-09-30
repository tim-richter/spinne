use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use petgraph::graph::DiGraph;
use petgraph::dot::{Dot, Config};

pub struct ComponentGraph {
    pub components: HashMap<String, Component>,
    pub graph: DiGraph<String, ()>,
}

pub struct Component {
    pub name: String,
    pub file_path: PathBuf,
    pub children: HashSet<String>,
}

impl ComponentGraph {
    pub fn new() -> Self {
        Self {
            components: HashMap::new(),
            graph: DiGraph::new(),
        }
    }

    pub fn add_component(&mut self, name: String, file_path: PathBuf) {
        if !self.components.contains_key(&name) {
            self.components.insert(name.clone(), Component {
                name: name.clone(),
                file_path,
                children: HashSet::new(),
            });
            self.graph.add_node(name);
        }
    }

    pub fn add_child(&mut self, parent: String, child: String) {
        if let Some(component) = self.components.get_mut(&parent) {
            component.children.insert(child.clone());
        }
        self.graph.add_edge(
            self.graph.node_indices().find(|i| self.graph[*i] == parent).unwrap(),
            self.graph.node_indices().find(|i| self.graph[*i] == child).unwrap(),
            (),
        );
    }

    pub fn generate_dot(&self) -> String {
        format!("{:?}", Dot::with_config(&self.graph, &[Config::EdgeNoLabel]))
    }

    pub fn generate_report(&self) -> String {
        let mut report = String::new();
        for (name, component) in &self.components {
            report.push_str(&format!("Component: {}\n", name));
            report.push_str(&format!("  File: {:?}\n", component.file_path));
            report.push_str("  Children:\n");
            for child in &component.children {
                report.push_str(&format!("    - {}\n", child));
            }
            report.push_str("\n");
        }
        report
    }
}