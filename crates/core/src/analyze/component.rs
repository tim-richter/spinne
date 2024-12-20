use std::{collections::HashMap, path::PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentChild {
    pub name: String,
    pub props: HashMap<String, usize>,
    pub origin_file_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentRoot {
    pub name: String,
    pub props: HashMap<String, usize>,
    pub children: Vec<ComponentChild>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Component {
    pub name: String,
    pub file_path: PathBuf,
    pub file_path_relative_to_root: Option<PathBuf>,
    pub props: HashMap<String, usize>,
    pub children: Vec<ComponentChild>,
}

impl Component {
    pub fn new(
        name: String,
        file_path: PathBuf,
        props: HashMap<String, usize>,
        children: Vec<ComponentChild>,
    ) -> Self {
        Self {
            name,
            file_path,
            file_path_relative_to_root: None,
            props,
            children,
        }
    }

    pub fn add_child(&mut self, child: ComponentChild) {
        self.children.push(child);
    }

    pub fn set_file_path_relative_to_root(&mut self, file_path_relative_to_root: PathBuf) {
        self.file_path_relative_to_root = Some(file_path_relative_to_root);
    }
}
