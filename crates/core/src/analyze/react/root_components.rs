use std::{collections::HashMap, path::PathBuf};

use oxc_ast::{
    ast::{
        Expression, FunctionBody, JSXAttributeItem, JSXAttributeName, JSXElementName,
        JSXOpeningElement, Statement, TSType, TSTypeAnnotation, TSTypeName,
    },
    AstKind, Visit,
};
use oxc_semantic::{AstNode, Semantic};
use spinne_logger::Logger;

use crate::{
    analyze::component::{ComponentChild, ComponentRoot},
    traverse::ProjectResolver,
    util,
};

use super::{find_component_root, find_import::find_import_for_symbol};

/// Check if the identifier is in pascal case
fn has_correct_case(identifier: &str) -> bool {
    util::is_pascal_case(&identifier.to_string())
}

/// Check if the function body has a return statement that matches the react return type
/// This could be a JSXElement, JSXFragment, NullLiteral etc.
fn has_react_return(body: &FunctionBody) -> bool {
    body.statements.iter().any(|node| match node {
        Statement::ReturnStatement(return_statement) => {
            if let Some(argument) = &return_statement.argument {
                match argument {
                    Expression::JSXElement(_) => {
                        return true;
                    }
                    Expression::JSXFragment(_) => {
                        return true;
                    }
                    Expression::NullLiteral(_) => {
                        return true;
                    }
                    Expression::BooleanLiteral(_) => {
                        return true;
                    }
                    Expression::StringLiteral(_) => {
                        return true;
                    }
                    Expression::NumericLiteral(_) => {
                        return true;
                    }
                    _ => {}
                }
            }

            return false;
        }
        _ => false,
    })
}

/// Check if the type annotation is a react type annotation e.g. React.FC
fn has_react_type_annotation(
    type_annotation: &Option<oxc_allocator::Box<TSTypeAnnotation>>,
) -> bool {
    if let Some(type_annotation) = type_annotation {
        if let TSType::TSTypeReference(type_reference) = &type_annotation.type_annotation {
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
    }

    false
}

/// Check if the node is a react component
fn is_react_component(node: &AstNode) -> bool {
    match node.kind() {
        AstKind::Function(fn_decl) => {
            let name = &fn_decl.id;

            Logger::debug(&format!("Analyzing function declaration: {:?}", name), 3);

            if let Some(name) = name {
                if !has_correct_case(&name.name.to_string()) {
                    return false;
                }

                if let Some(body) = &fn_decl.body {
                    if has_react_return(body) {
                        return true;
                    }
                }
            }

            return false;
        }
        AstKind::VariableDeclaration(var_decl) => {
            let name = var_decl.declarations.first();

            if let Some(name) = name {
                let id = &name.id;
                let identifier = id.get_identifier();
                let type_annotation = &id.type_annotation;

                let identifier = match identifier {
                    Some(identifier) => identifier,
                    None => return false,
                };

                Logger::debug(
                    &format!("Analyzing variable declaration: {}", identifier),
                    3,
                );

                if !has_correct_case(&identifier.to_string()) {
                    return false;
                }

                if has_react_type_annotation(type_annotation) {
                    return true;
                }

                if let Some(init) = &name.init {
                    if let Expression::ArrowFunctionExpression(arrow_fn_expr) = init {
                        let body = &arrow_fn_expr.body;
                        if has_react_return(body) {
                            return true;
                        }
                    }
                }
            }

            return false;
        }
        _ => false,
    }
}

/// Get the name of the react component
fn get_component_name(node: &AstNode) -> Option<String> {
    match node.kind() {
        AstKind::Function(fn_decl) => {
            let name = &fn_decl.id;

            if let Some(name) = name {
                return Some(name.name.to_string());
            }
        }
        AstKind::VariableDeclaration(var_decl) => {
            let name = var_decl.declarations.first();
            if let Some(name) = name {
                let identifier = name.id.get_identifier();

                if let Some(identifier) = identifier {
                    return Some(identifier.to_string());
                }
            }
        }
        _ => {}
    }

    None
}

fn get_children<'a>(
    node: &'a AstNode<'a>,
    semantic: &'a Semantic<'a>,
    resolver: &'a ProjectResolver,
    file_path: PathBuf,
) -> Vec<ComponentChild> {
    match node.kind() {
        AstKind::VariableDeclaration(var_decl) => {
            let name = var_decl.declarations.first();

            if let Some(name) = name {
                if let Some(init) = &name.init {
                    match init {
                        Expression::ArrowFunctionExpression(arrow_fn_expr) => {
                            let body = &arrow_fn_expr.body;

                            let child_components =
                                traverse_body(semantic, body, resolver, file_path.clone());

                            return child_components;
                        }
                        _ => {}
                    }
                }
            }
        }
        AstKind::Function(fn_decl) => {
            let body = &fn_decl.body;

            if let Some(body) = body {
                let child_components = traverse_body(semantic, body, resolver, file_path.clone());

                return child_components;
            }
        }
        _ => {}
    }

    Vec::new()
}

/// find react components in ast with oxc
pub fn extract_components<'a>(
    semantic: &'a Semantic<'a>,
    resolver: &'a ProjectResolver,
    file_path: PathBuf,
) -> Vec<ComponentRoot> {
    let mut components: Vec<ComponentRoot> = Vec::new();

    for node in semantic.nodes().iter() {
        if is_react_component(node) {
            let name = get_component_name(node);
            let children = get_children(node, semantic, resolver, file_path.clone());

            let component = ComponentRoot {
                name: name.unwrap(),
                props: HashMap::new(),
                children: children,
            };

            components.push(component);
        }
    }

    components
}

struct ReturnVisitor<'a> {
    semantic: &'a Semantic<'a>,
    resolver: &'a ProjectResolver,
    file_path: PathBuf,
    parent_file_path: PathBuf,
    child_components: Vec<ComponentChild>,
}

impl<'a> ReturnVisitor<'a> {
    fn new(semantic: &'a Semantic<'a>, resolver: &'a ProjectResolver, file_path: PathBuf) -> Self {
        Self {
            semantic,
            resolver,
            file_path: file_path.clone(),
            parent_file_path: file_path.parent().unwrap().to_path_buf(),
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
                            &self.parent_file_path,
                            import_node_id,
                            &ident_name,
                        );

                        if let Some(component_root) = component_root {
                            component_child.origin_file_path = component_root.1;
                        }
                    } else {
                        let declaration = self.semantic.symbols().get_declaration(symbol_id);
                        let declaration_node = self.semantic.nodes().get_node(declaration);

                        if is_react_component(declaration_node) {
                            component_child.origin_file_path = self.file_path.clone();
                        }

                        if let AstKind::FormalParameter(_) = declaration_node.kind() {
                            return;
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
                        JSXAttributeItem::SpreadAttribute(jsx_spread_attribute) => {}
                    });

                self.child_components.push(component_child);
            }
            _ => {}
        }
    }
}

pub fn traverse_body<'a>(
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

    use crate::{
        analyze::react::root_components::extract_components, traverse::ProjectResolver,
        util::test_utils::create_mock_project,
    };

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
        assert_eq!(
            components[0].children[0].origin_file_path,
            PathBuf::from(temp_dir.path().join("src/components/Input.tsx"))
        );
        assert_eq!(components[0].children[0].props.len(), 1);
        assert_eq!(components[0].children[0].props.get("placeholder"), Some(&1));
    }

    #[test]
    fn test_find_components_without_type_annotations() {
        let files = vec![
            (
                "src/components/Button.tsx",
                r#"
                import { Input } from './Input';

                const Button = () => {
                  return <Input placeholder="Hello" />;
                }
            "#,
            ),
            (
                "src/components/Input.tsx",
                r#"
                export const Input = () => {
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
        assert_eq!(
            components[0].children[0].origin_file_path,
            PathBuf::from(temp_dir.path().join("src/components/Input.tsx"))
        );
        assert_eq!(components[0].children[0].props.len(), 1);
        assert_eq!(components[0].children[0].props.get("placeholder"), Some(&1));
    }

    #[test]
    fn test_find_components_with_function_components() {
        let files = vec![
            (
                "src/components/Button.tsx",
                r#"
                import { Input } from './Input';

                function Button() {
                  return <Input placeholder="Hello" />;
                }
            "#,
            ),
            (
                "src/components/Input.tsx",
                r#"
                export function Input() {
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
        assert_eq!(
            components[0].children[0].origin_file_path,
            PathBuf::from(temp_dir.path().join("src/components/Input.tsx"))
        );
        assert_eq!(components[0].children[0].props.len(), 1);
        assert_eq!(components[0].children[0].props.get("placeholder"), Some(&1));
    }

    #[test]
    fn test_find_components_with_node_modules_imports() {
        let files = vec![
            (
                "src/components/Button.tsx",
                r#"
                import { Input } from 'material-ui';

                function Button() {
                  return <Input placeholder="Hello" />;
                }
            "#,
            ),
            (
                "node_modules/material-ui/index.tsx",
                r#"
                export function Input() {
                    return <div>Hello</div>;
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
        assert_eq!(
            components[0].children[0].origin_file_path,
            PathBuf::from("material-ui")
        );
        assert_eq!(components[0].children[0].props.len(), 1);
        assert_eq!(components[0].children[0].props.get("placeholder"), Some(&1));
    }

    #[test]
    fn test_should_use_correct_path_for_components_that_are_used_from_same_file() {
        let files = vec![(
            "src/components/Button.tsx",
            r#"
                function ButtonGroup() {
                  return <Button />;
                }

                function Button() {
                  return <button>Hello</button>;
                }
            "#,
        )];
        let temp_dir = create_mock_project(&files);

        let allocator = Allocator::default();
        let semantic = setup_semantic(&allocator, &files[0].1);
        let components = extract_components(
            &semantic.semantic,
            &ProjectResolver::new(None),
            PathBuf::from(temp_dir.path().join("src/components/Button.tsx")),
        );

        assert_eq!(components[0].name, "ButtonGroup");
        assert_eq!(components[0].children[0].name, "Button");
        assert_eq!(
            components[0].children[0].origin_file_path,
            PathBuf::from(temp_dir.path().join("src/components/Button.tsx"))
        );

        assert_eq!(components[1].name, "Button");
    }

    #[test]
    fn test_should_not_report_components_that_are_react_props() {
        let files = vec![(
            "src/components/Button.tsx",
            r#"
                function Button({ Component }) {
                  return <button><Component /></button>;
                }
            "#,
        )];
        let temp_dir = create_mock_project(&files);

        let allocator = Allocator::default();
        let semantic = setup_semantic(&allocator, &files[0].1);
        let components = extract_components(
            &semantic.semantic,
            &ProjectResolver::new(None),
            PathBuf::from(temp_dir.path().join("src/components/Button.tsx")),
        );

        assert_eq!(components[0].name, "Button");
        assert_eq!(components[0].children.len(), 0);
    }
}
