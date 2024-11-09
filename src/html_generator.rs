use std::fs;
use std::path::Path;
use serde_json::Value;

const HTML_TEMPLATE: &str = include_str!("./templates/component-graph.html");

pub struct HtmlGenerator {
    template: String,
}

impl HtmlGenerator {
    pub fn new(graph_data: Value) -> Self {
        let template = HTML_TEMPLATE.replace("{{GRAPH_DATA}}", &graph_data.to_string());
        Self { template }
    }

    pub fn save(&self, output_path: &Path) -> std::io::Result<()> {
        fs::write(output_path, self.template.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_html_generation() {
        let graph_data = json!({
            "nodes": [
                {
                    "name": "ComponentA",
                    "file_path": "/path/to/ComponentA.tsx",
                    "prop_usage": {}
                }
            ],
            "edges": []
        });

        let generator = HtmlGenerator::new(graph_data);

        assert!(generator.template.contains("ComponentA"));
        assert!(!generator.template.contains("{{GRAPH_DATA}}"));
    }
}