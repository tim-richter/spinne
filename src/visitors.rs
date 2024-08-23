use swc_ecma_ast::*;
use swc_ecma_visit::{Visit, VisitWith};
use std::collections::HashMap;

#[derive(Default)]
pub struct ComponentUsageVisitor {
    pub component_usages: HashMap<String, Vec<String>>,
    current_component: Option<String>,
}

impl ComponentUsageVisitor {
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
}

impl Visit for ComponentUsageVisitor {
    /// Visits function declarations and checks if they are React components.
    fn visit_fn_decl(&mut self, n: &FnDecl) {
        if self.is_potential_component_name(&n.ident) && self.is_react_component_function(&n.function) {
            let component_name = n.ident.sym.to_string();
            self.current_component = Some(component_name.clone());
            self.component_usages.entry(component_name).or_default();
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
                        let component_name = ident.sym.to_string();
                        self.current_component = Some(component_name.clone());
                        println!("Set current_component to {}", component_name);
                        self.component_usages.entry(component_name).or_default();
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
            self.current_component = Some(component_name.clone());
            println!("Set current_component to {}", component_name);
            self.component_usages.entry(component_name).or_default();
        }

        n.visit_children_with(self);

        self.current_component = None;
    }

    /// Visits JSX opening elements and records their usage within the current component.
    fn visit_jsx_opening_element(&mut self, n: &JSXOpeningElement) {
        println!("Visiting JSXOpeningElement: {:?}", n);
        if let Some(ref current_component) = self.current_component {
            if let JSXElementName::Ident(ident) = &n.name {
                if !self.is_base_element(ident) {
                    let component_name = ident.sym.to_string();
                    self.component_usages
                        .entry(current_component.clone())
                        .or_default()
                        .push(component_name.clone());
                    println!("Added usage of {} in {}", component_name, current_component);
                }
            }
        } else {
            println!("current_component is None when visiting JSXOpeningElement");
        }
        n.visit_children_with(self);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use swc_ecma_ast::Module;
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
        let mut visitor = ComponentUsageVisitor::default();
        visitor.visit_module(&module);

        println!("{:?}", visitor.component_usages);
        assert!(visitor.component_usages.contains_key("MyComponent"));
    }

    #[test]
    fn test_detect_variable_component() {
        let code = r#"
            const MyComponent = () => {
                return <div>Hello</div>;
            };
        "#;

        let module = parse_module(code);
        let mut visitor = ComponentUsageVisitor::default();
        visitor.visit_module(&module);

        println!("{:?}", visitor.component_usages);
        assert!(visitor.component_usages.contains_key("MyComponent"));
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
        let mut visitor = ComponentUsageVisitor::default();
        visitor.visit_module(&module);

        println!("{:?}", visitor.component_usages);
        assert!(visitor.component_usages.contains_key("MyComponent"));
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
        let mut visitor = ComponentUsageVisitor::default();
        visitor.visit_module(&module);

        println!("{:?}", visitor.component_usages);
        let usages = visitor.component_usages.get("MyComponent").unwrap();
        assert!(usages.contains(&"CustomComponent".to_string()));
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
        let mut visitor = ComponentUsageVisitor::default();
        visitor.visit_module(&module);

        println!("{:?}", visitor.component_usages);
        let usages = visitor.component_usages.get("MyComponent").unwrap();
        assert!(usages.contains(&"CustomComponent".to_string()));
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
        let mut visitor = ComponentUsageVisitor::default();
        visitor.visit_module(&module);

        println!("{:?}", visitor.component_usages);
        let usages = visitor.component_usages.get("MyComponent").unwrap();
        assert!(!usages.contains(&"div".to_string()));
        assert!(!usages.contains(&"span".to_string()));
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
        let mut visitor = ComponentUsageVisitor::default();
        visitor.visit_module(&module);

        println!("{:?}", visitor.component_usages);
        let usages = visitor.component_usages.get("MyComponent").unwrap();
        assert!(!usages.contains(&"div".to_string()));
        assert!(!usages.contains(&"span".to_string()));
    }
}