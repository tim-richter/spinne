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

use super::{find_component_root, find_import::find_import_for_symbol};

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
    file_path: PathBuf,
) -> Vec<ComponentRoot> {
    let mut components: Vec<ComponentRoot> = Vec::new();

    for node in semantic.nodes().iter() {
        match node.kind() {
            AstKind::Function(fn_decl) => {
                match fn_decl.name() {
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
                                        let body = &arrow_fn_expr.body;
                                        println!("file_path: {}", file_path.display());

                                        let child_components = get_child_components(
                                            semantic,
                                            body,
                                            resolver,
                                            file_path.parent().unwrap().to_path_buf(),
                                        );
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
    file_path: PathBuf,
    child_components: Vec<ComponentChild>,
}

impl<'a> ReturnVisitor<'a> {
    fn new(semantic: &'a Semantic<'a>, resolver: &'a ProjectResolver, file_path: PathBuf) -> Self {
        Self {
            semantic,
            resolver,
            file_path,
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
                    name: ident_name.clone(),
                    props: HashMap::new(),
                    origin_file_path: PathBuf::new(),
                };

                let reference_id = self
                    .semantic
                    .symbols()
                    .get_reference(identifier.reference_id());

                if let Some(symbol_id) = reference_id.symbol_id() {
                    let import_node_id = find_import_for_symbol(self.semantic, symbol_id);

                    if let Ok(import_node_id) = import_node_id {
                        let component_root = find_component_root(
                            self.semantic,
                            self.resolver,
                            &self.file_path,
                            import_node_id,
                            &ident_name,
                        );

                        if let Some(component_root) = component_root {
                            component_child.origin_file_path = component_root.1;
                        }
                    }
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
    file_path: PathBuf,
) -> Vec<ComponentChild> {
    let mut visitor = ReturnVisitor::new(semantic, resolver, file_path);
    visitor.visit_function_body(body);

    visitor.child_components
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use oxc_allocator::Allocator;
    use oxc_parser::Parser;
    use oxc_semantic::{SemanticBuilder, SemanticBuilderReturn};
    use oxc_span::SourceType;

    use crate::{analyze::extract_components, traverse::ProjectResolver, util::test_utils::create_mock_project};

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
        let files = vec![(
            "src/components/Button.tsx",
            r#"
    import React from 'react';

    const Button: React.FC = () => {
      return <div>Hello</div>;
    }
  "#,
        )];
        let temp_dir = create_mock_project(&files);

        let allocator = Allocator::default();
        let semantic = setup_semantic(&allocator, files[0].1);
        let components = extract_components(
            &semantic.semantic,
            &ProjectResolver::new(None),
            PathBuf::from(temp_dir.path().join("src/components/Button.tsx")),
        );

        assert_eq!(components[0].name, "Button");
    }

    #[test]
    fn test_find_fc_components() {
        let files = vec![
            (
                "src/components/Button.tsx",
                r#"
                import { FC } from 'react';
                import { Input } from './Input';

                const Button: FC = () => {
                return <Input placeholder="Hello" />;
                }

                const input = <Input placeholder="Hello" />;
            "#,
            ),
            (
                "src/components/Input.tsx",
                r#"
                import React from 'react';

                export const Input: React.FC = () => {
                    return <input />;
                }
            "#,
            ),
        ];
        let temp_dir = create_mock_project(&files);

        let allocator = Allocator::default();
        let semantic = setup_semantic(&allocator, &files[0].1);
        let components = extract_components(
            &semantic.semantic,
            &ProjectResolver::new(None),
            PathBuf::from(temp_dir.path().join("src/components/Button.tsx")),
        );

        assert_eq!(components[0].name, "Button");
        assert_eq!(components[0].children[0].name, "Input");
        assert_eq!(components[0].children[0].origin_file_path, PathBuf::from(temp_dir.path().join("src/components/Input.tsx")));
        assert_eq!(components[0].children[0].props.len(), 1);
        assert_eq!(components[0].children[0].props.get("placeholder"), Some(&1));
    }
}
