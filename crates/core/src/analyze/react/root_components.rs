use std::{collections::HashMap, path::PathBuf};

use oxc_ast::{
    ast::{
        Expression, FunctionBody, JSXAttributeItem, JSXAttributeName, JSXElementName,
        JSXOpeningElement, TSType, TSTypeName,
    },
    AstKind, Visit,
};
use oxc_semantic::Semantic;
use spinne_logger::Logger;

use crate::{
    analyze::component::{ComponentChild, ComponentRoot},
    traverse::ProjectResolver,
    util,
};

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
pub fn extract_components<'a>(
    semantic: &'a Semantic<'a>,
    resolver: &'a ProjectResolver,
) -> Vec<ComponentRoot> {
    let mut components: Vec<ComponentRoot> = Vec::new();

    for node in semantic.nodes().iter() {
        match node.kind() {
            AstKind::Function(fn_decl) => {
                let name = match fn_decl.name() {
                    Some(name) => name,
                    None => continue,
                };
                // TODO: check if the function is a react component
            }
            AstKind::VariableDeclaration(var_decl) => {
                let name = var_decl.declarations.first();

                if let Some(name) = name {
                    let id = &name.id;
                    let identifier = id.get_identifier();
                    let type_annotation = &id.type_annotation;

                    let init = match &name.init {
                        Some(init) => init,
                        None => continue,
                    };

                    let identifier = match identifier {
                        Some(identifier) => identifier,
                        None => continue,
                    };
                    Logger::debug(
                        &format!("Analyzing variable declaration: {}", identifier),
                        3,
                    );

                    let type_annotation = type_annotation.as_ref();

                    if let Some(type_annotation) = type_annotation {
                        if has_react_type_annotation(&type_annotation.type_annotation)
                            && has_correct_case(&identifier.to_string())
                        {
                            let mut component = ComponentRoot {
                                name: identifier.to_string(),
                                props: HashMap::new(),
                                children: Vec::new(),
                            };

                            if init.is_function() {
                                match init {
                                    Expression::FunctionExpression(fn_expr) => {
                                        println!("fn_expr: {:?}", fn_expr);
                                    }
                                    Expression::ArrowFunctionExpression(arrow_fn_expr) => {
                                        let params = &arrow_fn_expr.params.items;
                                        let body = &arrow_fn_expr.body;

                                        let child_components =
                                            get_child_components(semantic, body, resolver);
                                        component.children = child_components;
                                    }
                                    _ => {}
                                }
                            }

                            components.push(component);
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

struct ReturnVisitor<'a> {
    semantic: &'a Semantic<'a>,
    resolver: &'a ProjectResolver,
    child_components: Vec<ComponentChild>,
}

impl<'a> ReturnVisitor<'a> {
    fn new(semantic: &'a Semantic<'a>, resolver: &'a ProjectResolver) -> Self {
        Self {
            semantic,
            resolver,
            child_components: Vec::new(),
        }
    }
}

impl<'a> Visit<'a> for ReturnVisitor<'a> {
    fn visit_jsx_opening_element(&mut self, jsx_opening_element: &JSXOpeningElement<'a>) {
        match &jsx_opening_element.name {
            JSXElementName::IdentifierReference(identifier) => {
                let ident_name = &identifier.name;
                let ident_name = ident_name.to_string();

                let mut component_child = ComponentChild {
                    name: ident_name,
                    props: HashMap::new(),
                    origin_file_path: PathBuf::new(),
                };

                let reference_id = self
                    .semantic
                    .symbols()
                    .get_reference(identifier.reference_id());

                if let Some(symbol_id) = reference_id.symbol_id() {
                    let declaration = self.semantic.symbols().get_declaration(symbol_id);
                    let declaration_node = self.semantic.nodes().get_node(declaration);

                    println!("declaration_node: {:?}", declaration_node);
                }

                jsx_opening_element
                    .attributes
                    .iter()
                    .for_each(|attribute| match attribute {
                        JSXAttributeItem::Attribute(jsx_attribute) => {
                            let attribute_name = &jsx_attribute.name;

                            if let JSXAttributeName::Identifier(identifier) = attribute_name {
                                let ident_name = &identifier.name;
                                let ident_name = ident_name.to_string();

                                component_child.props.insert(ident_name, 1);
                            }
                        }
                        JSXAttributeItem::SpreadAttribute(jsx_spread_attribute) => {
                            println!("jsx_spread_attribute: {:?}", jsx_spread_attribute);
                        }
                    });

                self.child_components.push(component_child);
            }
            _ => {}
        }
    }
}

pub fn get_child_components<'a>(
    semantic: &'a Semantic<'a>,
    body: &'a FunctionBody<'a>,
    resolver: &'a ProjectResolver,
) -> Vec<ComponentChild> {
    let mut visitor = ReturnVisitor::new(semantic, resolver);
    visitor.visit_function_body(body);

    visitor.child_components
}

#[cfg(test)]
mod tests {
    use oxc_allocator::Allocator;
    use oxc_parser::Parser;
    use oxc_semantic::{SemanticBuilder, SemanticBuilderReturn};
    use oxc_span::SourceType;

    use crate::{analyze::extract_components, traverse::ProjectResolver};

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
        let components = extract_components(&semantic.semantic, &ProjectResolver::new(None));

        assert_eq!(components[0].name, "Button");
    }

    #[test]
    fn test_find_fc_components() {
        let allocator = Allocator::default();
        let content = r#"
    import { FC } from 'react';
    import { Input } from './Input';

    const Button: FC = () => {
      return <Input placeholder="Hello" />;
    }

    const input = <Input placeholder="Hello" />;
  "#;
        let semantic = setup_semantic(&allocator, content);
        let components = extract_components(&semantic.semantic, &ProjectResolver::new(None));

        assert_eq!(components[0].name, "Button");
        assert_eq!(components[1].name, "Input");
    }
}
