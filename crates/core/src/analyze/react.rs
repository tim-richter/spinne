use oxc_ast::{
    ast::{TSType, TSTypeName},
    AstKind,
};
use oxc_semantic::Semantic;
use spinne_logger::Logger;

use crate::util;

fn has_correct_case(identifier: &str) -> bool {
    util::is_pascal_case(&identifier.to_string())
}

fn has_react_type_annotation(type_annotation: &TSType) -> bool {
    if let TSType::TSTypeReference(type_reference) = type_annotation {
        let type_name = &type_reference.type_name;

        // React.FC
        if let TSTypeName::QualifiedName(qualified_name) = type_name {
            let left = &qualified_name.left;
            let right = &qualified_name.right;

            if let TSTypeName::IdentifierReference(identifier_reference) = left {
                let ident_ref = &identifier_reference.name;

                return ident_ref == "React" && right.name == "FC";
            }
        }

        // FC
        if let TSTypeName::IdentifierReference(identifier_reference) = type_name {
            let ident_ref = &identifier_reference.name;

            return ident_ref == "FC";
        }
    }

    false
}

/// find react components in ast with oxc
pub fn find_root_components(semantic: &Semantic) -> Vec<String> {
    let mut components = Vec::new();

    for node in semantic.nodes().iter() {
        match node.kind() {
            AstKind::Function(fn_decl) => {
                let name = match fn_decl.name() {
                    Some(name) => name,
                    None => continue,
                };

                if has_correct_case(&name.to_string()) {
                    components.push(name.to_string());
                }
            }
            AstKind::VariableDeclaration(var_decl) => {
                let name = var_decl.declarations.first();

                if let Some(name) = name {
                    let id = &name.id;
                    let identifier = &id.get_identifier();
                    let type_annotation = &id.type_annotation;

                    let identifier = match identifier {
                        Some(identifier) => identifier,
                        None => continue,
                    };

                    let type_annotation = type_annotation.as_ref();

                    if let Some(type_annotation) = type_annotation {
                        if has_react_type_annotation(&type_annotation.type_annotation)
                            && has_correct_case(&identifier.to_string())
                        {
                            components.push(identifier.to_string());
                        }
                    } else {
                        Logger::debug("type_annotation is None", 2);
                    }
                }
            }
            _ => {}
        }
    }

    components
}

#[cfg(test)]
mod tests {
    use oxc_allocator::Allocator;
    use oxc_parser::Parser;
    use oxc_semantic::{SemanticBuilder, SemanticBuilderReturn};
    use oxc_span::SourceType;

    use crate::analyze::find_root_components;

    fn setup_semantic<'a>(allocator: &'a Allocator, content: &'a str) -> SemanticBuilderReturn<'a> {
        let source_type = SourceType::default().with_typescript(true).with_jsx(true);

        // Parse the source code
        let parser_ret = Parser::new(&allocator, &content, source_type).parse();

        let program = parser_ret.program;

        // Build semantic analysis
        SemanticBuilder::new().build(&program)
    }

    #[test]
    fn test_find_react_fc_components() {
        let allocator = Allocator::default();
        let content = r#"
      import React from 'react';

      const Button: React.FC = () => {
        return <div>Hello</div>;
      }
    "#;
        let semantic = setup_semantic(&allocator, content);
        let components = find_root_components(&semantic.semantic);

        assert_eq!(components, vec!["Button"]);
    }

    #[test]
    fn test_find_fc_components() {
        let allocator = Allocator::default();
        let content = r#"
      import { FC } from 'react';

      const Button: FC = () => {
        return <div>Hello</div>;
      }

      const Input: FC = () => {
        return <input />;
      }
    "#;
        let semantic = setup_semantic(&allocator, content);
        let components = find_root_components(&semantic.semantic);

        assert_eq!(components, vec!["Button", "Input"]);
    }
}
