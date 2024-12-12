use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;
use oxc_semantic::{Semantic, SemanticBuilder, SemanticBuilderReturn, SymbolId};
use oxc_ast::{ast::Expression, AstKind};
use std::{collections::HashMap, sync::Arc};
use spinne_logger::Logger;
use itertools::Itertools;

pub struct ImportAnalyzer {
    imports: HashMap<String, String>,
}

impl ImportAnalyzer {
    pub fn new() -> Self {
        Self {
            imports: HashMap::new(),
        }
    }

    /// Analyze imports in a file's content using symbol analysis
    pub fn analyze(&mut self, content: Arc<str>, target_symbol: String) -> Result<&HashMap<String, String>, String> {
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

        self.analyze_references(&semantic.semantic, target_symbol);

        Logger::debug(&format!("Found imports: {:?}", self.imports), 3);
        Ok(&self.imports)
    }

    fn analyze_references(&mut self, semantic: &Semantic, target_symbol: String) {
        let symbols = semantic.symbols();
        for symbol in symbols.symbol_ids() {
            if symbols.get_name(symbol) != target_symbol {
                continue;
            }
            
            let declaration = symbols.get_declaration(symbol);
            let declaration_node = semantic.nodes().get_node(declaration);

            // Get the AST node for this declaration
            match &declaration_node.kind() {
                AstKind::VariableDeclarator(var_decl) => {
                    if let Some(init) = &var_decl.init {
                        match init {
                            Expression::Identifier(ident) => {
                                let reference = symbols.get_reference(ident.reference_id());
                                let declaration_symbol = symbols.get_declaration(reference.symbol_id().unwrap());
                                let declaration_node = semantic.nodes().get_node(declaration_symbol);
                                if matches!(declaration_node.kind(), AstKind::ImportDeclaration(_) | AstKind::ImportSpecifier(_) | AstKind::ImportDefaultSpecifier(_) | AstKind::ImportNamespaceSpecifier(_)) {
                                    println!("Import declaration: {:#?}", declaration_node);
                                }
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_named_import() {
        let mut analyzer = ImportAnalyzer::new();
        let content = r#"
            import { Button, Test } from './components/Button';

            const AnotherButton = Button;

            const CustomButton = () => {
                return <AnotherButton />;
            }
        "#;

        analyzer.analyze(Arc::from(content), "AnotherButton".to_string()).unwrap();
    } 
} 