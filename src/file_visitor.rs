use swc_ecma_ast::*;
use swc_ecma_visit::{Visit, VisitWith};
use std::{collections::HashMap, path::Path};
use crate::component_graph::ComponentGraph;
use std::path::PathBuf;
use swc_common::FileName;
use swc_ecma_loader::{resolve::Resolve, resolvers::node::NodeModulesResolver};

pub struct FileVisitor<'a> {
    pub component_graph: &'a mut ComponentGraph,
    pub imports: HashMap<String, PathBuf>,
    current_component: Option<String>,
    file_path: PathBuf,
    resolver: NodeModulesResolver,
}

impl<'a> FileVisitor<'a> {
    pub fn new(file_path: String, component_graph: &'a mut ComponentGraph) -> Self {
        Self {
            component_graph,
            imports: HashMap::new(),
            current_component: None,
            file_path: PathBuf::from(Self::normalize_path(&file_path)),
            resolver: NodeModulesResolver::default(),
        }
    }

    /// Normalize the file path to an absolute path.
    fn normalize_path(file_path: &str) -> PathBuf {
        Path::new(file_path).canonicalize().unwrap_or_else(|_| Path::new(file_path).to_path_buf())
    }

    fn resolve_import(&self, import_path: &str) -> PathBuf {
        let base = FileName::Real(self.file_path.clone());

        match self.resolver.resolve(&base, import_path) {
            Ok(resolved) => resolved.filename.to_string().into(),
            Err(_) => self.file_path.parent().unwrap().join(import_path),
        }
    }

    /// Checks if an identifier is a potential component name (starts with an uppercase letter).
    fn is_component(&self, ident: &Ident) -> bool {
        // Check if the name starts with an uppercase letter
        ident.sym.chars().next().map_or(false, |c| c.is_uppercase())
    }
}

impl<'a> Visit for FileVisitor<'a> {
    fn visit_module(&mut self, n: &Module) {
        n.visit_children_with(self);
    }

    /// Visits function declarations and checks if they are React components.
    fn visit_fn_decl(&mut self, n: &FnDecl) {
        if self.is_component(&n.ident) {
            let component_name = n.ident.sym.to_string();
            
            self.current_component = Some(component_name.clone());

            self.component_graph.add_component(component_name.clone(), self.file_path.clone());
        }

        n.visit_children_with(self);
        self.current_component = None;
    }

    /// Visits variable declarations and checks if they are React components.
    fn visit_var_decl(&mut self, n: &VarDecl) {
        for decl in &n.decls {
            if let Some(init) = &decl.init {
                if let Expr::Arrow(arrow_expr) = &**init {
                    if let Pat::Ident(ident) = &decl.name {
                        if self.is_component(&ident.id) {
                            let component_name = ident.id.sym.to_string();
                            self.current_component = Some(component_name.clone());
                            self.component_graph.add_component(component_name, self.file_path.clone());
                        }
                    }
                }
            }
        }
        n.visit_children_with(self);
    }

    /// Visits expressions and checks if they are React components.
    fn visit_expr(&mut self, n: &Expr) {
        if let Expr::Assign(assign_expr) = n {
            if let AssignTarget::Simple(SimpleAssignTarget::Ident(ident)) = &assign_expr.left {
                if self.is_component(&ident) {
                    let component_name = ident.sym.to_string();
                    self.current_component = Some(component_name.clone());
                    self.component_graph.add_component(component_name, self.file_path.clone());
                }
            }
        }
        n.visit_children_with(self);
    }

    /// Visits JSX opening elements and adds them as children of the current component.
    fn visit_jsx_opening_element(&mut self, n: &JSXOpeningElement) {
        if let Some(ref current_component) = self.current_component {
            if let JSXElementName::Ident(ident) = &n.name {
                if self.is_component(ident) {
                    let component_name = ident.sym.to_string();
                    let component_path = self.imports.get(&component_name).cloned().unwrap_or_else(|| self.file_path.clone());

                    self.component_graph.add_child(
                        (current_component, &self.file_path),
                        (&component_name, &component_path)
                    );

                    // Add prop usage
                    for attr in &n.attrs {
                        if let JSXAttrOrSpread::JSXAttr(jsx_attr) = attr {
                            if let JSXAttrName::Ident(attr_name) = &jsx_attr.name {
                                self.component_graph.add_prop_usage(&component_name, &component_path, attr_name.sym.to_string());
                            }
                        }
                    }
                }
            }
        }

        n.visit_children_with(self);
    }

    fn visit_import_decl(&mut self, n: &ImportDecl) {
        let import_path = self.resolve_import(&n.src.value);
        for specifier in &n.specifiers {
            if let ImportSpecifier::Named(named) = specifier {
                self.imports.insert(named.local.sym.to_string(), import_path.clone());
            }
        }
        n.visit_children_with(self);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use swc_ecma_parser::{lexer::Lexer, Parser, StringInput, Syntax, TsSyntax};

    fn parse_module(code: &str) -> Module {
        let lexer = Lexer::new(
            Syntax::Typescript(TsSyntax {
                tsx: true,
                ..Default::default()
            }),
            Default::default(),
            StringInput::new(code, swc_common::BytePos(0), swc_common::BytePos(1)),
            None,
        );

        let mut parser = Parser::new_from(lexer);
        parser.parse_module().expect("Failed to parse module")
    }

    #[test]
    fn test_detect_function_component() {
        let code = r#"
            function MyComponent() {
                return <div>Hello</div>;
            }
        "#;

        let module = parse_module(code);
        let mut component_graph = ComponentGraph::new();
        let mut visitor = FileVisitor::new(String::new(), &mut component_graph);
        visitor.visit_module(&module);

        assert!(visitor.component_graph.has_component("MyComponent", &visitor.file_path));
        assert!(visitor.component_graph.graph.node_count() == 1);
    }

    #[test]
    fn test_detect_function_component_with_children_components() {
        let code = r#"
            function MyComponent() {
                return <ChildComponent />;
            }

            function ChildComponent() {
                return <div>Child</div>;
            }
        "#;

        let module = parse_module(code);
        let mut component_graph = ComponentGraph::new();
        let mut visitor = FileVisitor::new(String::new(), &mut component_graph);
        visitor.visit_module(&module);

        assert!(visitor.component_graph.has_component("MyComponent", &visitor.file_path));
        assert!(visitor.component_graph.has_component("ChildComponent", &visitor.file_path));
        assert!(visitor.component_graph.graph.node_count() == 2);
        
        let my_component_node = visitor.component_graph.get_component("MyComponent", &visitor.file_path).unwrap();
        let child_component_node = visitor.component_graph.get_component("ChildComponent", &visitor.file_path).unwrap();
        assert!(visitor.component_graph.graph.contains_edge(my_component_node, child_component_node));
        assert!(visitor.component_graph.graph.edges(my_component_node).count() == 1);
        assert!(visitor.component_graph.graph.edges(child_component_node).count() == 0);
    }

    #[test]
    fn test_prop_usage() {
        let code = r#"
            function MyComponent() {
                return <Button className="test">Click me</Button>;
            }
        "#;

        let module = parse_module(code);
        let mut component_graph = ComponentGraph::new();
        let mut visitor = FileVisitor::new(String::new(), &mut component_graph);
        visitor.visit_module(&module);

        assert!(visitor.component_graph.has_component("MyComponent", &visitor.file_path));
        assert!(visitor.component_graph.has_component("Button", &visitor.file_path));
        assert!(visitor.component_graph.graph.node_count() == 2);

        let button_node = visitor.component_graph.get_component("Button", &visitor.file_path).unwrap();
        assert_eq!(visitor.component_graph.graph[button_node].prop_usage.get("className").unwrap(), &1);
    }

    #[test]
    fn test_assign_component() {
        let code = r#"
            let MyComponent = () => <div>Hello</div>;
            const MyComponent2 = () => <div>Hello</div>;
        "#;

        let module = parse_module(code);
        let mut component_graph = ComponentGraph::new();
        let mut visitor = FileVisitor::new(String::new(), &mut component_graph);
        visitor.visit_module(&module);

        assert!(visitor.component_graph.has_component("MyComponent", &visitor.file_path));
        assert!(visitor.component_graph.has_component("MyComponent2", &visitor.file_path));
        assert!(visitor.component_graph.graph.node_count() == 2);
    }
}