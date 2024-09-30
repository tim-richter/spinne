use swc_ecma_ast::*;
use swc_ecma_visit::{Visit, VisitWith};
use std::{collections::HashMap, path::Path};
use crate::component_graph::ComponentGraph;
use std::path::PathBuf;

pub struct FileVisitor<'a> {
    pub component_graph: &'a mut ComponentGraph,
    pub imports: Vec<String>,
    current_component: Option<String>,
    file_path: PathBuf,
    import_paths: HashMap<String, String>,
}

impl<'a> FileVisitor<'a> {
    // List of standard HTML elements to exclude
    const BASE_ELEMENTS: &'static [&'static str] = &[
        "div", "span", "p", "a", "ul", "li", "h1", "h2", "h3", "h4", "h5", "h6", 
        "button", "input", "form", "table", "thead", "tbody", "tr", "td", "th",
        "img", "nav", "header", "footer", "main", "section", "article", "aside",
        "textarea", "select", "option", "label", "iframe", "audio", "video",
        "canvas", "details", "summary", "fieldset", "legend", "abbr", "blockquote",
        "cite", "code", "figure", "figcaption", "mark", "small", "strong", "sub",
        "sup", "time", "var", "wbr"
    ];

    pub fn new(file_path: String, component_graph: &'a mut ComponentGraph) -> Self {
        println!("Creating FileVisitor for {}", file_path);
        Self {
            component_graph,
            imports: Vec::new(),
            current_component: None,
            file_path: PathBuf::from(Self::normalize_path(file_path)),
            import_paths: HashMap::new(),
        }
    }

    /// Normalize the file path to an absolute path.
    fn normalize_path(file_path: String) -> String {
        Path::new(&file_path)
            .canonicalize()
            .unwrap_or_else(|_| Path::new(&file_path).to_path_buf())
            .to_string_lossy()
            .to_string()
    }

    /// Checks if an identifier is a base HTML element.
    fn is_base_element(&self, ident: &Ident) -> bool {
        Self::BASE_ELEMENTS.contains(&ident.sym.as_ref())
    }

    /// Checks if an expression contains JSX elements or fragments.
    fn contains_jsx(&self, expr: &Expr) -> bool {
        match expr {
            Expr::JSXElement(_) | Expr::JSXFragment(_) => true,
            Expr::Call(call_expr) => {
                // Check if any argument of the call expression contains JSX
                call_expr.args.iter().any(|arg| self.contains_jsx(&arg.expr))
            }
            Expr::Paren(paren_expr) => {
                // Check if the expression inside parentheses contains JSX
                self.contains_jsx(&paren_expr.expr)
            }
            _ => false,
        }
    }

    /// Checks if a block statement contains JSX elements or fragments.
    fn contains_jsx_in_block(&self, block: &BlockStmt) -> bool {
        block.stmts.iter().any(|stmt| match stmt {
            Stmt::Return(ReturnStmt { arg: Some(expr), .. }) => self.contains_jsx(expr),
            Stmt::Expr(expr_stmt) => self.contains_jsx(&expr_stmt.expr),
            _ => false,
        })
    }

    /// Checks if a block statement uses React hooks.
    fn uses_react_hooks(&self, body: &BlockStmt) -> bool {
        body.stmts.iter().any(|stmt| match stmt {
            Stmt::Expr(ExprStmt { expr, .. }) => match &**expr {
                Expr::Call(CallExpr { callee, .. }) => match callee {
                    Callee::Expr(callee_expr) => match &**callee_expr {
                        Expr::Ident(ident) => ident.sym.starts_with("use"),
                        _ => false,
                    },
                    _ => false,
                },
                _ => false,
            },
            _ => false,
        })
    }

    /// Determines if a function is a React component by checking for JSX or React hooks.
    fn is_react_component_function(&self, function: &Function) -> bool {
        if let Some(body) = &function.body {
            self.contains_jsx_in_block(body) || self.uses_react_hooks(body)
        } else {
            false
        }
    }

    /// Checks if an identifier is a potential component name (starts with an uppercase letter).
    fn is_potential_component_name(&self, ident: &Ident) -> bool {
        // Check if the name starts with an uppercase letter
        ident.sym.chars().next().map_or(false, |c| c.is_uppercase())
    }

    /// Checks if the initializer expression is a React component.
    fn is_react_component_expr(&self, expr: &Expr) -> bool {
        match expr {
            Expr::Fn(_) | Expr::Arrow(_) | Expr::JSXElement(_) => true,
            Expr::Call(call_expr) => {
                if let Callee::Expr(callee_expr) = &call_expr.callee {
                    if let Expr::Ident(callee_ident) = &**callee_expr {
                        if callee_ident.sym == "React" {
                            return true;
                        }
                    }
                }
                false
            }
            _ => false,
        }
    }

    /// Checks if a class extends React.Component or React.PureComponent.
    fn is_react_class_component(&self, class: &Class) -> bool {
        if let Some(super_class) = &class.super_class {
            match &**super_class {
                Expr::Member(member_expr) => {
                    if let Expr::Ident(obj_ident) = &*member_expr.obj {
                        if obj_ident.sym == "React" {
                            if let MemberProp::Ident(prop_ident) = &member_expr.prop {
                                return prop_ident.sym == "Component" || prop_ident.sym == "PureComponent";
                            }
                        }
                    }
                }
                Expr::Ident(ident) => {
                    return ident.sym == "Component";
                }
                _ => {}
            }
        }
        false
    }

    fn generate_component_key(&self, name: &str) -> String {
        format!("{}:{}", self.file_path.to_string_lossy(), name)
    }

    fn add_component_usage(&mut self, component_key: String, used_component_key: String, import_path: String) {
        self.component_graph.add_component(component_key.clone(), self.file_path.clone());
        self.component_graph.add_component(used_component_key.clone(), PathBuf::from(import_path));
        self.component_graph.add_child(component_key, used_component_key);
    }
}

impl<'a> Visit for FileVisitor<'a> {
    fn visit_module(&mut self, n: &Module) {
        n.visit_children_with(self);
    }
    /// Visits function declarations and checks if they are React components.
    fn visit_fn_decl(&mut self, n: &FnDecl) {
        if self.is_potential_component_name(&n.ident) && self.is_react_component_function(&n.function) {
            let component_name = n.ident.sym.to_string();
            let component_key = self.generate_component_key(&component_name);
            self.current_component = Some(component_key.clone());
            self.component_graph.add_component(component_key.clone(), self.file_path.clone());
        }

        n.visit_children_with(self);
        self.current_component = None;
    }

    /// Visits variable declarators and checks if they are React components.
    fn visit_var_declarator(&mut self, n: &VarDeclarator) {
        if let Some(ident) = n.name.as_ident() {
            println!("Visiting VarDeclarator: {:?}", ident.sym);

            if self.is_potential_component_name(ident) {
                if let Some(init) = &n.init {
                    if self.is_react_component_expr(init) {
                        println!("Ident is a React component: {:?}", ident.sym);
                        let component_name = ident.sym.to_string();
                        let component_key = self.generate_component_key(&component_name);
                        self.current_component = Some(component_key.clone());
                        println!("Set current_component to {}", component_key);

                        if let Some(ref current_component) = self.current_component {
                            println!("Adding component to graph: {}", current_component);
                            self.component_graph.add_component(current_component.clone(), self.file_path.clone());
                        }
                    }
                }
            }
        }

        n.visit_children_with(self);

        self.current_component = None;
    }

    /// Visits class declarations and checks if they are React components.
    fn visit_class_decl(&mut self, n: &ClassDecl) {
        println!("Visiting ClassDecl: {:?}", n.ident.sym);
        if self.is_potential_component_name(&n.ident) && self.is_react_class_component(&n.class) {
            let component_name = n.ident.sym.to_string();
            let component_key = self.generate_component_key(&component_name);
            self.current_component = Some(component_key.clone());
            println!("Set current_component to {}", component_key);
            self.component_graph.add_component(component_key.clone(), self.file_path.clone());
        }

        n.visit_children_with(self);

        self.current_component = None;
    }

    /// Visits import declarations and resolves the actual file path of the imported component.
    fn visit_import_decl(&mut self, n: &ImportDecl) {
        self.imports.push(n.src.value.to_string());
        for specifier in &n.specifiers {
            match specifier {
                ImportSpecifier::Named(named_specifier) => {
                    let imported_name = named_specifier.local.sym.to_string();
                    if self.is_potential_component_name(&named_specifier.local) {
                        self.import_paths.insert(imported_name, n.src.value.to_string());
                    }
                }
                ImportSpecifier::Default(default_specifier) => {
                    let imported_name = default_specifier.local.sym.to_string();
                    if self.is_potential_component_name(&default_specifier.local) {
                        self.import_paths.insert(imported_name, n.src.value.to_string());
                    }
                }
                _ => {}
            }
        }
        n.visit_children_with(self);
    }

    /// Visits JSX opening elements and records their usage within the current component.
    fn visit_jsx_opening_element(&mut self, n: &JSXOpeningElement) {
        if let Some(ref current_component) = self.current_component {
            if let JSXElementName::Ident(ident) = &n.name {
                if !self.is_base_element(ident) {
                    let component_name = ident.sym.to_string();
                    let component_key = self.generate_component_key(&component_name);
                    let import_path = self.import_paths.get(&component_name)
                        .cloned()
                        .unwrap_or_else(|| self.file_path.to_string_lossy().to_string());
                    self.add_component_usage(current_component.clone(), component_key, import_path);
                }
            }
        }
        n.visit_children_with(self);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use petgraph::graph::NodeIndex;
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

        let component_key = format!("{}:MyComponent", visitor.file_path.to_string_lossy());
        assert!(visitor.component_graph.components.contains_key(&component_key));
        assert!(visitor.component_graph.graph.node_count() == 1);
    }

    #[test]
    fn test_detect_variable_component() {
        let code = r#"
            const MyComponent = () => {
                return <div>Hello</div>;
            };
        "#;

        let module = parse_module(code);
        let mut component_graph = ComponentGraph::new();
        let mut visitor = FileVisitor::new(String::new(), &mut component_graph);
        visitor.visit_module(&module);

        let component_key = format!("{}:MyComponent", visitor.file_path.to_string_lossy());
        assert!(visitor.component_graph.components.contains_key(&component_key));
        assert!(visitor.component_graph.graph.node_count() == 1);
    }

    #[test]
    fn test_detect_class_component() {
        let code = r#"
            class MyComponent extends React.Component {
                render() {
                    return <div>Hello</div>;
                }
            }
        "#;

        let module = parse_module(code);
        let mut component_graph = ComponentGraph::new();
        let mut visitor = FileVisitor::new(String::new(), &mut component_graph);
        visitor.visit_module(&module);

        let component_key = format!("{}:MyComponent", visitor.file_path.to_string_lossy());
        assert!(visitor.component_graph.components.contains_key(&component_key));
        assert!(visitor.component_graph.graph.node_count() == 1);
    }

    #[test]
    fn test_detect_jsx_elements() {
        let code = r#"
            const MyComponent = () => {
                return (
                    <div>
                        <CustomComponent />
                    </div>
                );
            }
        "#;

        let module = parse_module(code);
        let mut component_graph = ComponentGraph::new();
        let mut visitor = FileVisitor::new(String::new(), &mut component_graph);
        visitor.visit_module(&module);

        let component_key = format!("{}:MyComponent", visitor.file_path.to_string_lossy());
        let custom_component_key = format!("{}:CustomComponent", visitor.file_path.to_string_lossy());

        assert!(visitor.component_graph.graph.node_count() == 2);
        assert!(visitor.component_graph.graph.contains_edge(NodeIndex::new(0), NodeIndex::new(1)));
        assert!(visitor.component_graph.graph.node_weight(NodeIndex::new(0)).unwrap() == &component_key);
        assert!(visitor.component_graph.graph.node_weight(NodeIndex::new(1)).unwrap() == &custom_component_key);
    }

    #[test]
    fn test_detect_jsx_elements_with_fn() {
        let code = r#"
            function MyComponent() {
                return (
                    <div>
                        <CustomComponent />
                    </div>
                );
            }
        "#;

        let module = parse_module(code);
        let mut component_graph = ComponentGraph::new();
        let mut visitor = FileVisitor::new(String::new(), &mut component_graph);
        visitor.visit_module(&module);

        let component_key = format!("{}:MyComponent", visitor.file_path.to_string_lossy());
        let custom_component_key = format!("{}:CustomComponent", visitor.file_path.to_string_lossy());
        
        assert!(visitor.component_graph.graph.node_count() == 2);
        assert!(visitor.component_graph.graph.contains_edge(NodeIndex::new(0), NodeIndex::new(1)));
        assert!(visitor.component_graph.graph.node_weight(NodeIndex::new(0)).unwrap() == &component_key);
        assert!(visitor.component_graph.graph.node_weight(NodeIndex::new(1)).unwrap() == &custom_component_key);
    }

    #[test]
    fn test_exclude_base_elements() {
        let code = r#"
            const MyComponent = () => {
                return (
                    <div>
                        <span>Hello</span>
                    </div>
                );
            }
        "#;

        let module = parse_module(code);
        let mut component_graph = ComponentGraph::new();
        let mut visitor = FileVisitor::new(String::new(), &mut component_graph);
        visitor.visit_module(&module);

        assert!(visitor.component_graph.graph.node_count() == 1);
    }

    #[test]
    fn test_exclude_base_elements_with_fn() {
        let code = r#"
            function MyComponent() {
                return (
                    <div>
                        <span>Hello</span>
                    </div>
                );
            }
        "#;

        let module = parse_module(code);
        let mut component_graph = ComponentGraph::new();
        let mut visitor = FileVisitor::new("/Home.tsx".to_string(), &mut component_graph);
        visitor.visit_module(&module);
        
        assert!(visitor.component_graph.graph.node_count() == 1);
    }

    #[test]
    fn test_detects_child_components() {
        let code = r#"
            function ParentComponent() {
                return (
                    <div>
                        <ChildComponent1 />
                        <ChildComponent2 />
                    </div>
                );
            }

            function ChildComponent1() {
                return <div>Child 1</div>;
            }

            function ChildComponent2() {
                return <div>Child 2</div>;
            }
        "#;

        let module = parse_module(code);
        let mut component_graph = ComponentGraph::new();
        let mut visitor = FileVisitor::new(String::new(), &mut component_graph);
        visitor.visit_module(&module);

        let parent_component_key = format!("{}:ParentComponent", visitor.file_path.to_string_lossy());

        let child_component1_key = format!("{}:ChildComponent1", visitor.file_path.to_string_lossy());
        let child_component2_key = format!("{}:ChildComponent2", visitor.file_path.to_string_lossy());
        
        assert!(visitor.component_graph.graph.contains_edge(NodeIndex::new(0), NodeIndex::new(1)));
        assert!(visitor.component_graph.graph.contains_edge(NodeIndex::new(0), NodeIndex::new(2)));
        assert!(visitor.component_graph.graph.node_weight(NodeIndex::new(0)).unwrap() == &parent_component_key);
        assert!(visitor.component_graph.graph.node_weight(NodeIndex::new(1)).unwrap() == &child_component1_key);
        assert!(visitor.component_graph.graph.node_weight(NodeIndex::new(2)).unwrap() == &child_component2_key);
    }

    #[test]
    fn test_detects_child_components_in_imported_file() {
        let code = r#"
            import ChildComponent1 from './ChildComponent1';
            import ChildComponent2 from './ChildComponent2';

            function ParentComponent() {
                return (
                    <div>
                        <ChildComponent1 />
                        <ChildComponent2 />
                    </div>
                );
            }
        "#;
        let module = parse_module(code);
        let mut component_graph = ComponentGraph::new();
        let mut visitor = FileVisitor::new(String::new(), &mut component_graph);
        visitor.visit_module(&module);

        let parent_component_key = format!("{}:ParentComponent", visitor.file_path.to_string_lossy());
        let child_component1_key = format!("{}:ChildComponent1", visitor.file_path.to_string_lossy());
        let child_component2_key = format!("{}:ChildComponent2", visitor.file_path.to_string_lossy());

        assert!(visitor.component_graph.graph.contains_edge(NodeIndex::new(0), NodeIndex::new(1)));
        assert!(visitor.component_graph.graph.contains_edge(NodeIndex::new(0), NodeIndex::new(2)));
        assert!(visitor.component_graph.graph.node_weight(NodeIndex::new(0)).unwrap() == &parent_component_key);
        assert!(visitor.component_graph.graph.node_weight(NodeIndex::new(1)).unwrap() == &child_component1_key);
        assert!(visitor.component_graph.graph.node_weight(NodeIndex::new(2)).unwrap() == &child_component2_key);
    }
}