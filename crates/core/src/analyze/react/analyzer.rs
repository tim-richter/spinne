use std::{collections::HashMap, path::PathBuf};

use oxc_semantic::Semantic;
use spinne_logger::Logger;

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
        let root_components = extract_components(&self.semantic, &self.resolver);
        let mut components = Vec::new();

        for component in root_components {
            Logger::debug(&format!("Found root component: {}", component.name), 1);
            let component = Component::new(
                component.name.to_string(),
                self.file_path.clone(),
                HashMap::new(),
                vec![],
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
        let content = r#"
            import React from 'react';
            import { Button } from './components/Button';

            const App: React.FC = () => {
                return <Button />;
            }
        "#;

        let allocator = Allocator::default();
        let semantic = setup_semantic(&allocator, content);

        let file_path = PathBuf::from("test/src/index.tsx");
        let resolver = ProjectResolver::new(None);
        let analyzer = ReactAnalyzer::new(&semantic.semantic, file_path, &resolver);

        let components = analyzer.analyze();

        assert_eq!(components.len(), 1);
        assert_eq!(components[0].name, "App");
        assert_eq!(components[0].children.len(), 1);
        assert_eq!(components[0].children[0].name, "Button");
    }
}
