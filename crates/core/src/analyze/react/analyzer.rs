use oxc_semantic::Semantic;
use spinne_logger::Logger;

use super::find_root_components;

pub struct ReactAnalyzer<'a> {
    pub semantic: &'a Semantic<'a>,
}

impl<'a> ReactAnalyzer<'a> {
    pub fn new(semantic: &'a Semantic<'a>) -> Self {
        Self { semantic }
    }

    pub fn analyze(&self) {
        let root_components = find_root_components(&self.semantic);

        for (name, component) in root_components {
            Logger::debug(&format!("Found root component: {}", name), 1);
        }
    }
}
