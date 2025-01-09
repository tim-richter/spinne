use std::{collections::HashMap, path::PathBuf};

use oxc_semantic::Semantic;

use crate::{analyze::component::Component, traverse::ProjectResolver};

use super::extract_components;

pub struct ReactAnalyzer<'a> {
    pub semantic: &'a Semantic<'a>,
    pub file_path: PathBuf,
    pub resolver: &'a ProjectResolver,
}

impl<'a> ReactAnalyzer<'a> {
    pub fn new(
        semantic: &'a Semantic<'a>,
        file_path: PathBuf,
        resolver: &'a ProjectResolver,
    ) -> Self {
        Self {
            semantic,
            file_path,
            resolver,
        }
    }

    pub fn analyze(&self) -> Vec<Component> {
        let root_components =
            extract_components(&self.semantic, &self.resolver, self.file_path.clone());
        let mut components = Vec::new();

        for component in root_components {
            let component = Component::new(
                component.name.to_string(),
                self.file_path.clone(),
                HashMap::new(),
                component.children,
            );
            components.push(component);
        }

        components
    }
}

#[cfg(test)]
mod tests {
    use oxc_allocator::Allocator;
    use oxc_parser::Parser;
    use oxc_semantic::{SemanticBuilder, SemanticBuilderReturn};
    use oxc_span::SourceType;

    use crate::util::test_utils;

    use super::*;

    fn setup_semantic<'a>(allocator: &'a Allocator, content: &'a str) -> SemanticBuilderReturn<'a> {
        let source_type = SourceType::default().with_typescript(true).with_jsx(true);

        // Parse the source code
        let parser_ret = Parser::new(&allocator, &content, source_type).parse();

        let program = parser_ret.program;

        // Build semantic analysis
        SemanticBuilder::new().build(&program)
    }

    #[test]
    fn test_analyze() {
        let files = vec![
            (
                "src/index.tsx",
                r#"
            import React from 'react';
            import { Button } from './Button';

            const App: React.FC = () => {
                return <Button />;
            }
        "#,
            ),
            (
                "src/Button.tsx",
                r#"
            import React from 'react';

            export const Button: React.FC = () => {
                return <div>Button</div>;
            }
            "#,
            ),
        ];

        let temp_dir = test_utils::create_mock_project(&files);

        let allocator = Allocator::default();
        let semantic = setup_semantic(&allocator, files[0].1);

        let file_path = PathBuf::from(temp_dir.path().join(files[0].0));
        let resolver = ProjectResolver::new(None);
        let analyzer = ReactAnalyzer::new(&semantic.semantic, file_path, &resolver);

        let components = analyzer.analyze();

        assert_eq!(components.len(), 1);
        assert_eq!(components[0].name, "App");
        assert_eq!(components[0].children.len(), 1);
        assert_eq!(components[0].children[0].name, "Button");
    }
}
