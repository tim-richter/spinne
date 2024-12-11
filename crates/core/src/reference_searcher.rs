use swc_ecma_ast::*;
use swc_ecma_visit::{Visit, VisitWith};
use std::collections::HashMap;
use spinne_logger::Logger;

#[derive(Debug, Clone, Hash)]
enum Target {
    JSXMemberExpr(JSXMemberExpr),
    JSXElement(JSXElement),
}

pub struct ReferenceSearcher<'a> {
    target: Target,
    imports: &'a HashMap<String, String>,
    current_reference: Option<String>,
    found_import: Option<String>,
}

impl<'a> ReferenceSearcher<'a> {
    pub fn new(target: Target, imports: &'a HashMap<String, String>) -> Self {
        Self {
            target,
            imports,
            current_reference: None,
            found_import: None,
        }
    }

    /// Get the first identifier in the target
    /// 
    /// This is used to find the 'root' of the target
    /// 
    /// For example, if the target is `CustomButton.Button`, the first ident is `CustomButton`
    fn first_ident(&self, target: Target) -> Option<String> {
        match target {
            Target::JSXMemberExpr(expr) => {
                if expr.obj.is_ident() {
                    let ident = expr.obj.expect_ident();
                    return Some(ident.sym.to_string())
                }

                // traverse objects and find 'root'
                if expr.obj.is_jsx_member_expr() {
                    let member = expr.obj.expect_jsx_member_expr();
                    let mut member_expr = member;
                    while member_expr.obj.is_jsx_member_expr() {
                        member_expr = member_expr.obj.expect_jsx_member_expr();
                    }
                    let ident = member_expr.obj.expect_ident();
                    return Some(ident.sym.to_string())
                }

                None
            },
            Target::JSXElement(element) => {
                match &element.opening.name {
                    JSXElementName::Ident(ident) => Some(ident.sym.to_string()),
                    _ => None,
                }
            },
        }
    }

    /// Check if the target is imported
    fn find_import(&self, target: &Target) -> Option<&String> {
        if let Some(ident) = self.first_ident(target.clone()) {
            return self.imports.get(&ident)
        }

        None
    }

    pub fn find_original_import(&mut self, module: &Module) -> Option<String> {
        Logger::debug(&format!("Searching for original import of {:?}", self.target), 2);

        // First check if our target is directly imported
        if let Some(import_path) = self.find_import(&self.target) {
            Logger::debug(&format!("Found direct import: {:?}", self.target), 2);
            return Some(import_path.clone());
        }

        // If not directly imported, start tracking references
        self.current_reference = Some(self.target.clone());
        self.visit_module(module);

        if let Some(found_import) = &self.found_import {
            Logger::debug(&format!("Found import: {:?}", found_import), 2);
            return Some(found_import.clone());
        }

        Logger::debug("No import found", 2);
        None
    }
}

impl<'a> Visit for ReferenceSearcher<'a> {
    fn visit_module(&mut self, n: &Module) {
        n.visit_children_with(self);
    }

    fn visit_var_decl(&mut self, n: &VarDecl) {
        for decl in &n.decls {
            // declaration has initializer
            let decl_init = match &decl.init {
                Some(init) => init,
                None => continue,
            };

            // get the identifier
            let decl_ident = match &decl.name {
                Pat::Ident(ident) => ident,
                _ => continue,
            };

            // get the current reference
            let current_ref = match &self.current_reference {
                Some(ref_) => ref_,
                None => continue,
            };

            // get the initializer
            let init_ident = match &**decl_init {
                Expr::Ident(ident) => ident,
                _ => continue,
            };

            // get the first identifier of the current reference
            let first_ident = match self.first_ident(current_ref.clone()) {
                Some(ident) => ident,
                None => continue,
            };

            // Skip if identifiers don't match
            if decl_ident.sym.to_string() != first_ident {
                continue;
            }

            // Check if it's in imports
            if let Some(import_path) = self.imports.get(&init_ident.sym.to_string()) {
                self.found_import = Some(import_path.clone());
                return;
            }

            // Update current reference and continue tracking
            self.current_reference = Some(Target::JSXMemberExpr(JSXMemberExpr {
                span: Span::default(),
                obj: JSXObject::Ident(Ident {
                    span: Span::default(),
                    sym: init_ident.sym.to_string(),
                    ctxt: SyntaxContext::empty(),
                    optional: false,
                }),
                prop: IdentName {
                    span: Span::default(),
                    sym: init_ident.sym.to_strin    g(),
                },
            }));
        }

        n.visit_children_with(self);
    }

    fn visit_assign_expr(&mut self, n: &AssignExpr) {
        if let AssignTarget::Simple(SimpleAssignTarget::Ident(ident)) = &n.left {
            if let Some(current) = &self.current_reference {
                if let Expr::Ident(right_ident) = &*n.right {
                    if right_ident.sym.to_string() == *current {
                        // Update current reference to track this new assignment
                        self.current_reference = Some(ident.sym.to_string());
                    }
                }
            }
        }
        n.visit_children_with(self);
    }
}

#[cfg(test)]
mod tests {
    use swc_common::{Span, SyntaxContext};

    use super::*;
    use crate::ProjectTraverser;

    fn create_jsx_element(name: &str) -> JSXElement {
        JSXElement {
            span: Span::default(),
            opening: JSXOpeningElement {
                span: Span::default(),
                name: JSXElementName::Ident(Ident {
                    span: Span::default(),
                    sym: name.into(),
                    ctxt: SyntaxContext::empty(),
                    optional: false,
                }),
                self_closing: false,
                type_args: None,
                attrs: vec![],
            },
            children: vec![],
            closing: None,
        }
    }

    fn create_jsx_member_expr(name: &str) -> JSXMemberExpr {
        JSXMemberExpr {
            span: Span::default(),
            obj: JSXObject::Ident(Ident {
                span: Span::default(),
                sym: name.into(),
                ctxt: SyntaxContext::empty(),
                optional: false,
            }),
            prop: IdentName {
                span: Span::default(),
                sym: name.into(),
            },
        }
    }

    #[test]
    fn test_find_direct_import() {
        let code = r#"
            import { Button } from './components/Button';
            function MyComponent() {
                return <Button />;
            }
        "#;

        let mut imports = HashMap::new();
        imports.insert("Button".to_string(), "./components/Button".to_string());

        let module = ProjectTraverser::parse_typescript(code);
        let mut searcher = ReferenceSearcher::new(Target::JSXElement(create_jsx_element("Button")), &imports);
        let result = searcher.find_original_import(&module);

        assert_eq!(
            result,
            Some("./components/Button".to_string())
        );
    }

    #[test]
    fn test_find_import_through_jsx_element() {
        let code = r#"
            import { Button } from './components/Button';
            const CustomButton = Button;
            function MyComponent() {
                return <CustomButton />;
            }
        "#;

        let mut imports = HashMap::new();
        imports.insert("Button".to_string(), "./components/Button".to_string());

        let module = ProjectTraverser::parse_typescript(code);
        let mut searcher = ReferenceSearcher::new(Target::JSXElement(create_jsx_element("CustomButton")), &imports);
        let result = searcher.find_original_import(&module);

        assert_eq!(
            result,
            Some("./components/Button".to_string())
        );
    }

    #[test]
    fn test_find_import_through_object_assignment() {
        let code = r#"
            import { Button } from './components/Button';
            const CustomButton = {
                Button: Button
            };
            function MyComponent() {
                return <CustomButton.Button />;
            }
        "#;

        let mut imports = HashMap::new();
        imports.insert("Button".to_string(), "./components/Button".to_string());

        let module = ProjectTraverser::parse_typescript(code);
        let mut searcher = ReferenceSearcher::new(Target::JSXMemberExpr(create_jsx_member_expr("CustomButton.Button")), &imports);
        let result = searcher.find_original_import(&module);

        assert_eq!(
            result,
            Some("./components/Button".to_string())
        );
    }
} 