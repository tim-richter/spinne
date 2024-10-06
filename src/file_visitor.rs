use swc_ecma_ast::*;
use swc_ecma_visit::{Visit, VisitWith};
use std::{collections::HashMap, fs, path::Path};
use crate::{component_graph::ComponentGraph, ProjectTraverser};
use std::path::PathBuf;
use swc_common::{collections::AHashMap, FileName};
use swc_ecma_loader::{resolve::Resolve, resolvers::node::NodeModulesResolver, TargetEnv};

pub struct FileVisitor<'a> {
    pub component_graph: &'a mut ComponentGraph,
    pub imports: HashMap<String, PathBuf>,
    current_component: Option<String>,
    file_path: PathBuf,
    resolver: NodeModulesResolver,
    resolved_imports: HashMap<String, PathBuf>,
    project_root: PathBuf,
}

impl<'a> FileVisitor<'a> {
    pub fn new(file_path: String, component_graph: &'a mut ComponentGraph, project_root: PathBuf) -> Self {
        Self {
            component_graph,
            imports: HashMap::new(),
            current_component: None,
            file_path: PathBuf::from(Self::normalize_path(&file_path)),
            resolver: NodeModulesResolver::without_node_modules(TargetEnv::Node, AHashMap::default(), true),
            resolved_imports: HashMap::new(),
            project_root,
        }
    }

    /// Normalize the file path to an absolute path.
    fn normalize_path(file_path: &str) -> PathBuf {
        Path::new(file_path).canonicalize().unwrap_or_else(|_| Path::new(file_path).to_path_buf())
    }

    /// Resolve a component name to a file path.
    fn resolve_component_path(&mut self, component_name: &str) -> Option<PathBuf> {
        if let Some(path) = self.resolved_imports.get(component_name) {
            return Some(path.clone());
        }
        
        let import_path = self.imports.get(component_name)?;
        let import_path_str = import_path.to_str().unwrap();
        let resolved_path = self.resolve_import(import_path_str);

        if let Some(resolved_path) = resolved_path {
            self.resolved_imports.insert(component_name.to_string(), resolved_path.clone());
            return Some(resolved_path);
        }
        
        None
    }

    /// Resolves an import path to a file path.
    fn resolve_import(&self, import_path: &str) -> Option<PathBuf> {
        let base = FileName::Real(self.file_path.clone());
        
        match self.resolver.resolve(&base, import_path) {
            Ok(resolved) => {
                let path = PathBuf::from(resolved.filename.to_string());
                println!("Resolved path: {:?}", path);
                self.traverse_import(&path)
            },
            Err(e) => {
                println!("Error resolving import: {:?}", e);
                let path = self.project_root.join(import_path);
                self.traverse_import(&path)
            },
        }
    }

    /// Traverse an import path and update the component graph
    fn traverse_import(&self, path: &PathBuf) -> Option<PathBuf> {
        println!("Traversing path: {:?}", path);
        if path.is_file() {
            let content = fs::read_to_string(path).ok()?;
            let extension = path.extension().unwrap_or_default();
            println!("Extension: {:?}", extension);

            // abort traversing if the path is not a JS/TS file
            if extension != "ts" && extension != "tsx" &&
               extension != "js" && extension != "jsx" {
                println!("Not a JS/TS file");
                return None;
            }

            let module = ProjectTraverser::parse_typescript(&content);
            
            for item in &module.body {
                if let ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(export_decl)) = item {
                    if let Decl::Var(var_decl) = &export_decl.decl {
                        for decl in &var_decl.decls {
                            if let Pat::Ident(ident) = &decl.name {
                                return Some(path.clone());
                            }
                        }
                    } else if let Decl::Fn(fn_decl) = &export_decl.decl {
                        return Some(path.clone());
                    }
                } else if let ModuleItem::ModuleDecl(ModuleDecl::ExportNamed(export_named)) = item {
                    if let Some(src) = &export_named.src {
                        let new_path_str = src.value.to_string();
                        let base = FileName::Real(path.clone());
                        if let Ok(resolved) = self.resolver.resolve(&base, &new_path_str) {
                            let new_path = PathBuf::from(resolved.filename.to_string());
                            return self.traverse_import(&new_path);
                        }
                    }
                } else if let ModuleItem::ModuleDecl(ModuleDecl::ExportAll(export_all)) = item {
                    let new_path_str = export_all.src.value.to_string();
                    let base = FileName::Real(path.clone());
                    if let Ok(resolved) = self.resolver.resolve(&base, &new_path_str) {
                        let new_path = PathBuf::from(resolved.filename.to_string());
                        return self.traverse_import(&new_path);
                    }
                }
            }
        }
        None
    }

    /// Checks if an identifier is a potential component name (starts with an uppercase letter).
    fn is_component(&self, ident: &Ident) -> bool {
        // Check if the name starts with an uppercase letter
        ident.sym.chars().next().map_or(false, |c| c.is_uppercase())
    }
}

impl<'a> Visit for FileVisitor<'a> {
    fn visit_module(&mut self, n: &swc_ecma_ast::Module) {
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
        if let Some(current_component) = self.current_component.clone() {
            if let JSXElementName::Ident(ident) = &n.name {
                if self.is_component(ident) {
                    let component_name = ident.sym.to_string();
                    let component_path = self.resolve_component_path(&component_name)
                        .unwrap_or_else(|| self.file_path.clone());
    
                    self.component_graph.add_child(
                        (&current_component, &self.file_path),
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
        println!("ImportDecl: {:?}", n.src.value);
        // handle none value
        let import_path = self.resolve_import(&n.src.value);

        if let Some(import_path) = import_path {
            for specifier in &n.specifiers {
                if let ImportSpecifier::Named(named) = specifier {
                    self.imports.insert(named.local.sym.to_string(), import_path.clone());
                    self.resolved_imports.insert(named.local.sym.to_string(), import_path.clone());
                }
            }
        } else {
            println!("ImportDecl: {:?} not found", n.src.value);
        }

        n.visit_children_with(self);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use swc_ecma_parser::{lexer::Lexer, Parser, StringInput, Syntax, TsSyntax};
    use tempfile::TempDir;

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
        let mut visitor = FileVisitor::new(String::new(), &mut component_graph, PathBuf::from("."));
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
        let mut visitor = FileVisitor::new(String::new(), &mut component_graph, PathBuf::from("."));
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
        let mut visitor = FileVisitor::new(String::new(), &mut component_graph, PathBuf::from("."));
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
        let mut visitor = FileVisitor::new(String::new(), &mut component_graph, PathBuf::from("."));
        visitor.visit_module(&module);

        assert!(visitor.component_graph.has_component("MyComponent", &visitor.file_path));
        assert!(visitor.component_graph.has_component("MyComponent2", &visitor.file_path));
        assert!(visitor.component_graph.graph.node_count() == 2);
    }

    #[test]
    fn test_import_component() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        let file_path = root.join("Button.tsx");
        fs::write(&file_path, "export function Button() { return <button>Click me</button>; }").unwrap();

        let code = r#"
            import { Button } from "./Button.tsx";

            function MyComponent() {
                return <Button />;
            }
        "#;

        let module = parse_module(code);
        let mut component_graph = ComponentGraph::new();
        let mut visitor = FileVisitor::new(String::new(), &mut component_graph, root.to_path_buf());
        visitor.visit_module(&module);

        assert!(visitor.component_graph.has_component("MyComponent", &visitor.file_path));
        assert!(visitor.component_graph.has_component("Button", &file_path));
        assert!(visitor.component_graph.graph.node_count() == 2);
    }

    #[test]
    fn test_import_component_from_nested_directory() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        let file_path = root.join("components/Button.tsx");
        fs::create_dir_all(file_path.parent().unwrap()).unwrap();
        fs::write(&file_path, "export function Button() { return <button>Click me</button>; }").unwrap();

        let code = r#"
            import { Button } from "./components/Button.tsx";

            function MyComponent() {
                return <Button />;
            }
        "#;

        let module = parse_module(code);
        let mut component_graph = ComponentGraph::new();
        let mut visitor = FileVisitor::new(String::new(), &mut component_graph, root.to_path_buf());
        visitor.visit_module(&module);

        assert!(visitor.component_graph.has_component("MyComponent", &visitor.file_path));
        assert!(visitor.component_graph.has_component("Button", &file_path));
        assert!(visitor.component_graph.graph.node_count() == 2);
    }

    #[test]
    fn test_import_component_from_barrel() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create the components directory
        fs::create_dir_all(root.join("components")).unwrap();

        // Create the index.ts file
        let index_file_path = root.join("components/index.ts");
        fs::write(&index_file_path, "export { Button } from './Button';").unwrap();

        // Create the Button.tsx file
        let button_file_path = root.join("components/Button.tsx");
        fs::write(&button_file_path, "export function Button() { return <button>Click me</button>; }").unwrap();

        // Create the MyComponent.tsx file
        let my_component_file_path = root.join("MyComponent.tsx");
        let my_component_code = r#"
            import { Button } from "./components";

            function MyComponent() {
                return <Button />;
            }
        "#;
        fs::write(&my_component_file_path, my_component_code).unwrap();

        let module = parse_module(my_component_code);
        let mut component_graph = ComponentGraph::new();
        let mut visitor = FileVisitor::new(my_component_file_path.display().to_string(), &mut component_graph, root.to_path_buf());
        visitor.visit_module(&module);

        assert!(visitor.component_graph.has_component("MyComponent", &visitor.file_path));
        assert!(visitor.component_graph.has_component("Button", &button_file_path));
        assert!(visitor.component_graph.graph.node_count() == 2);
    }

    #[test]
    fn test_import_component_barrel_with_other_files() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create the components directory
        fs::create_dir_all(root.join("components")).unwrap();

        // Create css file
        let css_file_path = root.join("styles.css");
        fs::write(&css_file_path, "body { background-color: red; }").unwrap();

        // Create the index.ts file
        let index_file_path = root.join("components/index.ts");
        fs::write(&index_file_path, "export { Button } from './Button';").unwrap();

        // Create the Button.tsx file
        let button_file_path = root.join("components/Button.tsx");
        fs::write(&button_file_path, "export function Button() { return <button>Click me</button>; }").unwrap();

        // Create the MyComponent.tsx file
        let my_component_file_path = root.join("MyComponent.tsx");
        let my_component_code = r#"
            import { Button } from "./components";
            import "./styles.css";

            function MyComponent() {
                return <Button />;
            }
        "#; 

        fs::write(&my_component_file_path, my_component_code).unwrap();

        let module = parse_module(my_component_code);
        let mut component_graph = ComponentGraph::new();
        let mut visitor = FileVisitor::new(my_component_file_path.display().to_string(), &mut component_graph, root.to_path_buf());
        visitor.visit_module(&module);

        assert!(visitor.component_graph.has_component("MyComponent", &visitor.file_path));
        assert!(visitor.component_graph.has_component("Button", &button_file_path));
        assert!(visitor.component_graph.graph.node_count() == 2);
    }
}
