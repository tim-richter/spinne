
use oxc_semantic::{NodeId, Semantic, };
use oxc_ast::{ast::Expression, AstKind};
use spinne_logger::Logger;

/// Find the node id of the import for a given symbol in a file's content
/// 
/// Has additional logic to handle dot notation for components and tries to find the node id for the root component only
/// 
/// If the symbol is not found, it will return an error
pub fn find_import_for_symbol(semantic: &Semantic, target_symbol: String) -> Result<NodeId, String> {
    Logger::debug("Analyzing imports using symbol analysis", 2);

    let import_node = recursive_find(&semantic, target_symbol);

    if let Some(import_node) = import_node {
        return Ok(import_node);
    }

    Err("No import found".to_string())
}

/// Recursively find the node id of the import for a given symbol in a file's content
/// 
/// We need to search for declarations of the target symbol and follow references to find the root component
/// If we find a import declaration, we return the node id
/// If we find a variable declaration, we recursively search for the target symbol in the variable's initializer
fn recursive_find(semantic: &Semantic, target_symbol: String) -> Option<NodeId> {
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
                    if let Some(node) = recursive_find(semantic, ident.name.to_string()) {
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
    use oxc_allocator::Allocator;
    use oxc_parser::Parser;
    use oxc_span::SourceType;
    use oxc_semantic::{SemanticBuilder, SemanticBuilderReturn};

    fn setup_semantic<'a>(allocator: &'a Allocator, content: &'a str) -> SemanticBuilderReturn<'a> {
      let source_type = SourceType::default().with_typescript(true).with_jsx(true);
  
      // Parse the source code
      let parser_ret = Parser::new(&allocator, &content, source_type)
          .parse();
  
      let program = parser_ret.program;
  
      // Build semantic analysis
      SemanticBuilder::new()
          .build(&program)
    }

    #[test]
    fn test_named_import() {
        let content = r#"
            import { Button, Test } from './components/Button';

            const AnotherButton = Button;

            const CustomButton = () => {
                return <AnotherButton />;
            }
        "#;

        let allocator = Allocator::default();
        let semantic = setup_semantic(&allocator, content);
        let result = find_import_for_symbol(&semantic.semantic, "AnotherButton".to_string());
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

        let allocator = Allocator::default();
        let semantic = setup_semantic(&allocator, content);
        let result = find_import_for_symbol(&semantic.semantic, "AnotherButton".to_string());
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

        let allocator = Allocator::default();
        let semantic = setup_semantic(&allocator, content);
        let result = find_import_for_symbol(&semantic.semantic, "AnotherButton".to_string());
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

        let allocator = Allocator::default();
        let semantic = setup_semantic(&allocator, content);
        let result = find_import_for_symbol(&semantic.semantic, "AnotherButton".to_string());
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

        let allocator = Allocator::default();
        let semantic = setup_semantic(&allocator, content);
        let result = find_import_for_symbol(&semantic.semantic, "TestButton".to_string());
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

        let allocator = Allocator::default();
        let semantic = setup_semantic(&allocator, content);
        let result = find_import_for_symbol(&semantic.semantic, "TestButton".to_string());
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

        let allocator = Allocator::default();
        let semantic = setup_semantic(&allocator, content);
        let result = find_import_for_symbol(&semantic.semantic, "TestButton".to_string());
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

        let allocator = Allocator::default();
        let semantic = setup_semantic(&allocator, content);
        let result = find_import_for_symbol(&semantic.semantic, "TestButton.Danger".to_string());
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

        let allocator = Allocator::default();
        let semantic = setup_semantic(&allocator, content);
        let result = find_import_for_symbol(&semantic.semantic, "TestButton.Danger".to_string());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), NodeId::new(3));
    }
} 