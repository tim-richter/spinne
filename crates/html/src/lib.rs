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
        // Convert numeric IDs to strings in the JSON data
        let workspace_data = convert_ids_to_strings(workspace_data);

        let template = HTML_TEMPLATE.replace(
            "[/* {{GRAPH_DATA}} */]",
            &serde_json::to_string(&workspace_data).unwrap_or_default(),
        );
        Self { template }
    }

    pub fn save(&self, output_path: &Path) -> std::io::Result<()> {
        fs::write(output_path, self.template.clone())
    }
}

/// Recursively converts numeric IDs to strings in the JSON data
fn convert_ids_to_strings(data: Value) -> Value {
    match data {
        Value::Array(arr) => Value::Array(arr.into_iter().map(convert_ids_to_strings).collect()),
        Value::Object(obj) => {
            let mut new_obj = serde_json::Map::new();
            for (key, value) in obj {
                let new_value = if key == "id" || key == "from" || key == "to" {
                    // Convert numeric IDs to strings
                    match value {
                        Value::Number(n) => Value::String(n.to_string()),
                        _ => convert_ids_to_strings(value),
                    }
                } else {
                    convert_ids_to_strings(value)
                };
                new_obj.insert(key, new_value);
            }
            Value::Object(new_obj)
        }
        _ => data,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_html_generation() {
        let project_data = json!([
          {
            "name": "consumer-app",
            "graph": {
              "components": [
                {
                  "id": 14300231078674835378u64,
                  "name": "App",
                  "path": "consumer-app/src/App.tsx",
                  "props": {},
                },
              ],
              "edges": [
                {
                  "from": 14300231078674835378u64,
                  "to": 11611080489164640768u64,
                  "project_context": "source-lib",
                },
              ],
            },
          },
          {
            "name": "source-lib",
            "graph": {
              "components": [
                {
                  "id": 11611080489164640768u64,
                  "name": "Button",
                  "path": "source-lib/src/components/Button.tsx",
                  "props": {},
                },
              ],
              "edges": [],
            },
          },
        ]);

        let generator = HtmlGenerator::new(project_data);

        assert!(generator.template.contains("App"));
        assert!(!generator.template.contains("{{GRAPH_DATA}}"));
    }

    #[test]
    fn test_html() {
        let graph_data = json!([
        {
          "name": "source-lib",
          "graph": {
            "components": [
              {
                "id": 11611080489164640768u64,
                "name": "Button",
                "path": "source-lib/src/components/Button.tsx",
                "props": {
                  "label": 1,
                  "onClick": 1,
                  "variant": 1,
                  "disabled": 1
                }
              },
              {
                "id": 11611080489164640769u64,
                "name": "Input",
                "path": "source-lib/src/components/Input.tsx",
                "props": {
                  "value": 1,
                  "onChange": 1,
                  "placeholder": 1,
                  "type": 1
                }
              },
              {
                "id": 11611080489164640770u64,
                "name": "Card",
                "path": "source-lib/src/components/Card.tsx",
                "props": {
                  "title": 1,
                  "children": 1,
                  "padding": 1
                }
              },
              {
                "id": 11611080489164640771u64,
                "name": "Modal",
                "path": "source-lib/src/components/Modal.tsx",
                "props": {
                  "isOpen": 1,
                  "onClose": 1,
                  "title": 1,
                  "children": 1
                }
              }
            ],
            "edges": [
              {
                "from": 11611080489164640771u64,
                "to": 11611080489164640770u64,
                "project_context": "source-lib"
              },
              {
                "from": 11611080489164640771u64,
                "to": 11611080489164640768u64,
                "project_context": "source-lib"
              },
              {
                "from": 11611080489164640770u64,
                "to": 11611080489164640768u64,
                "project_context": "source-lib"
              },
              {
                "from": 11611080489164640770u64,
                "to": 11611080489164640769u64,
                "project_context": "source-lib"
              }
            ]
          }
        },
        {
          "name": "consumer-app",
          "graph": {
            "components": [
              {
                "id": 14300231078674835378u64,
                "name": "App",
                "path": "consumer-app/src/App.tsx",
                "props": {}
              },
              {
                "id": 14300231078674835379u64,
                "name": "LoginForm",
                "path": "consumer-app/src/components/LoginForm.tsx",
                "props": {
                  "onSubmit": 1,
                  "error": 1
                }
              },
              {
                "id": 14300231078674835380u64,
                "name": "UserProfile",
                "path": "consumer-app/src/components/UserProfile.tsx",
                "props": {
                  "user": 1,
                  "onEdit": 1
                }
              },
              {
                "id": 14300231078674835381u64,
                "name": "SettingsModal",
                "path": "consumer-app/src/components/SettingsModal.tsx",
                "props": {
                  "isOpen": 1,
                  "onClose": 1,
                  "settings": 1
                }
              }
            ],
            "edges": [
              {
                "from": 14300231078674835378u64,
                "to": 11611080489164640768u64,
                "project_context": "source-lib"
              },
              {
                "from": 14300231078674835378u64,
                "to": 14300231078674835379u64,
                "project_context": "consumer-app"
              },
              {
                "from": 14300231078674835378u64,
                "to": 14300231078674835380u64,
                "project_context": "consumer-app"
              },
              {
                "from": 14300231078674835379u64,
                "to": 11611080489164640768u64,
                "project_context": "source-lib"
              },
              {
                "from": 14300231078674835379u64,
                "to": 11611080489164640769u64,
                "project_context": "source-lib"
              },
              {
                "from": 14300231078674835380u64,
                "to": 11611080489164640770u64,
                "project_context": "source-lib"
              },
              {
                "from": 14300231078674835380u64,
                "to": 14300231078674835381u64,
                "project_context": "consumer-app"
              },
              {
                "from": 14300231078674835381u64,
                "to": 11611080489164640771u64,
                "project_context": "source-lib"
              }
            ]
          }
        }]);

        let generator = HtmlGenerator::new(graph_data);
        let output_path = Path::new("test.html");
        generator.save(output_path).unwrap();
        open::that(output_path).unwrap();
    }
}
