use serde_json::Value;
use std::fs;
use std::path::Path;

const HTML_TEMPLATE: &str = include_str!("./component-graph.html");

/// Generates an HTML report from a component graph.
/// Uses d3.js to render the graph.
pub struct HtmlGenerator {
    template: String,
}

impl HtmlGenerator {
    pub fn new(workspace_data: Value) -> Self {
        let template = HTML_TEMPLATE.replace(
            "{{GRAPH_DATA}}",
            &serde_json::to_string(&workspace_data).unwrap_or_default(),
        );
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
        let project_data = json!([{
            "name": "test-project",
            "graph": {
                "nodes": [
                    {
                        "name": "ComponentA",
                        "file_path": "/path/to/ComponentA.tsx",
                        "prop_usage": {}
                    }
                ],
                "edges": []
            }
        }]);

        let generator = HtmlGenerator::new(project_data);

        assert!(generator.template.contains("ComponentA"));
        assert!(!generator.template.contains("{{GRAPH_DATA}}"));
    }

    #[test]
    fn test_html() {
        let graph_data = json!({
            "projects": [
                {
                    "nodes": [
                        {
                            "name": "ComponentA",
                            "file_path": "/path/to/ComponentA.tsx",
                            "prop_usage": {}
                        },
                        {
                            "name": "ComponentB",
                            "file_path": "/path/to/ComponentB.tsx",
                            "prop_usage": {}
                        }
                    ],
                    "edges": [
                        [0, 1]
                    ]
                },
                {
                    "nodes": [
                        {
                            "name": "ComponentC",
                            "file_path": "/path/to/ComponentC.tsx",
                            "prop_usage": {}
                        }
                    ],
                    "edges": []
                }
            ]
        });

        let generator = HtmlGenerator::new(graph_data);
        let output_path = Path::new("test.html");
        generator.save(output_path).unwrap();
        open::that(output_path).unwrap();
    }
}
