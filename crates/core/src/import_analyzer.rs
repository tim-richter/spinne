use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;
use oxc_semantic::{NodeId, Semantic, SemanticBuilder};
use oxc_ast::{ast::Expression, AstKind};
use std::sync::Arc;
use spinne_logger::Logger;
use itertools::Itertools;

/// Analyze imports in a file's content using symbol analysis
pub fn find_import(content: Arc<str>, target_symbol: String) -> Result<NodeId, String> {
    Logger::debug("Analyzing imports using symbol analysis", 2);

    // Setup allocator and source type
    let allocator = Allocator::default();
    let source_type = SourceType::default().with_typescript(true).with_jsx(true);

    // Parse the source code
    let parser_ret = Parser::new(&allocator, &content, source_type)
        .parse();

    if !parser_ret.errors.is_empty() {
            let error_message: String = parser_ret
                .errors
                .into_iter()
                .map(|error| format!("{:?}", error.with_source_code(Arc::clone(&content))))
                .join("\n");
            println!("Parsing failed:\n\n{error_message}",);
            return Err(error_message);
        }

    let program = parser_ret.program;

    // Build semantic analysis
    let semantic = SemanticBuilder::new()
        .build(&program);

    if !semantic.errors.is_empty() {
            let error_message: String = semantic
                .errors
                .into_iter()
                .map(|error| format!("{:?}", error.with_source_code(Arc::clone(&content))))
                .join("\n");
            println!("Semantic analysis failed:\n\n{error_message}",);
        }

    let import_node = analyze_references(&semantic.semantic, target_symbol);

    if let Some(import_node) = import_node {
        return Ok(import_node);
    }

    Err("No import found".to_string())
}

fn analyze_references(semantic: &Semantic, target_symbol: String) -> Option<NodeId> {
    let symbols = semantic.symbols();

    let base_symbol = if let Some(dot_index) = target_symbol.find('.') {
        target_symbol[..dot_index].to_string()
    } else {
        target_symbol
    };

    for symbol_id in symbols.symbol_ids() {
        if symbols.get_name(symbol_id) != base_symbol {
            continue;
        }

        let declaration = symbols.get_declaration(symbol_id);
        let declaration_node = semantic.nodes().get_node(declaration);

        // If we found an import, we're done!
        if matches!(declaration_node.kind(), AstKind::ImportDeclaration(_) | AstKind::ImportSpecifier(_) | AstKind::ImportDefaultSpecifier(_)) {
            return Some(declaration);
        }

        // If not an import, check if it's a variable declaration and follow its reference
        if let AstKind::VariableDeclarator(var_decl) = declaration_node.kind() {
            if let Some(init) = &var_decl.init {
                if let Expression::Identifier(ident) = init {
                    // Recursively search for the new target and return its result
                    if let Some(node) = analyze_references(semantic, ident.name.to_string()) {
                        return Some(node);
                    }
                }
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_named_import() {
        let content = r#"
            import { Button, Test } from './components/Button';

            const AnotherButton = Button;

            const CustomButton = () => {
                return <AnotherButton />;
            }
        "#;

        let result = find_import(Arc::from(content), "AnotherButton".to_string());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), NodeId::new(3));
    }

    #[test]
    fn test_default_import() {
        let content = r#"
            import Button from './components/Button';

            const AnotherButton = Button;

            const CustomButton = () => {
                return <AnotherButton />;
            }
        "#;

        let result = find_import(Arc::from(content), "AnotherButton".to_string());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), NodeId::new(3));
    }

    #[test]
    fn test_import_with_multiple_assignments() {
        let content = r#"
            import Button, { Test } from './components/Button';

            const AnotherButton = Button;
            const TestButton = AnotherButton;

            const CustomButton = () => {
                return <TestButton />;
            }
        "#;

        let result = find_import(Arc::from(content), "AnotherButton".to_string());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), NodeId::new(3));
    }

    #[test]
    fn test_import_in_multiple_scopes() {
        let content = r#"
            import Button, { Test } from './components/Button';

            const AnotherButton = Button;

            const CustomButton = () => {
                const TestButton = AnotherButton;
                return <TestButton />;
            }
        "#;

        let result = find_import(Arc::from(content), "AnotherButton".to_string());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), NodeId::new(3));
    }

    #[test]
    fn test_import_with_scope_shadowing() {
        let content = r#"
            import Button, { Test } from './components/Button';

            const AnotherButton = Button;
            const TestButton = AnotherButton;

            const CustomButton = () => {
                const TestButton = Button;
                return <TestButton />;
            }
        "#;

        let result = find_import(Arc::from(content), "TestButton".to_string());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), NodeId::new(3));
    }

    #[test]
    fn test_import_object_destructuring() {
        let content = r#"
            import Button, { Test } from './components/Button';

            const { TestButton } = Button;

            const CustomButton = () => {
                return <TestButton />;
            }
        "#;

        let result = find_import(Arc::from(content), "TestButton".to_string());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), NodeId::new(3));
    }

    #[test]
    fn test_import_array_destructuring() {
        let content = r#"
            import Button, { Test } from './components/Button';

            const [ TestButton ] = Button;

            const CustomButton = () => {
                return <TestButton />;
            }
        "#;

        let result = find_import(Arc::from(content), "TestButton".to_string());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), NodeId::new(3));
    }

    #[test]
    fn test_import_component_with_dot() {
        let content = r#"
            import Button, { Test } from './components/Button';

            const TestButton = Button;

            const CustomButton = () => {
                return <TestButton.Danger />;
            }
        "#;

        let result = find_import(Arc::from(content), "TestButton.Danger".to_string());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), NodeId::new(3));
    }

    #[test]
    fn test_import_component_with_dot_multiple() {
        let content = r#"
            import Button, { Test } from './components/Button';

            const TestButton = Button;

            const CustomButton = () => {
                return <TestButton.Danger.Primary />;
            }
        "#;

        let result = find_import(Arc::from(content), "TestButton.Danger".to_string());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), NodeId::new(3));
    }
} 