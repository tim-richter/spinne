use std::{collections::HashMap, path::PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentChild {
    pub name: String,
    pub props: HashMap<String, usize>,
    pub origin_file_path: PathBuf,
    /// The name of the project this component belongs to, derived from the package.json name field
    pub project_name: Option<String>,
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
}
