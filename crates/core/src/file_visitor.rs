use crate::util::normalize_path;
use crate::{component_graph::ComponentGraph, config::Config, ProjectTraverser};
use std::path::PathBuf;
use std::{collections::HashMap, fs, sync::Arc};
use swc_common::{collections::AHashMap, FileName};
use swc_ecma_ast::*;
use swc_ecma_loader::TargetEnv;
use swc_ecma_loader::{
    resolve::Resolve, resolvers::node::NodeModulesResolver, resolvers::tsc::TsConfigResolver,
};
use swc_ecma_visit::{Visit, VisitWith};
use spinne_logger::Logger;

/// FileVisitor is a visitor for TypeScript files.
/// It traverses the file system and updates the component graph.
///
/// The general approach here is:
/// 1. Traverse the given file to find react components
/// 2. Find used children components
/// 3. For each child component we have to traverse the file system to find the root component
/// 4. If the child component uses props, we also add that to the graph
/// 5. Add the component + info to the graph
pub struct FileVisitor<'a> {
    pub component_graph: &'a mut ComponentGraph,
    pub imports: HashMap<String, String>,
    current_component: Option<String>,
    file_path: PathBuf,
    resolver: TsConfigResolver<NodeModulesResolver>,
}

impl<'a> FileVisitor<'a> {
    pub fn new(
        file_path: String,
        component_graph: &'a mut ComponentGraph,
        config: Arc<Config>,
    ) -> Self {
        Self {
            component_graph,
            imports: HashMap::new(),
            current_component: None,
            file_path: PathBuf::from(normalize_path(&file_path)),
            resolver: TsConfigResolver::new(
                NodeModulesResolver::without_node_modules(
                    TargetEnv::Node,
                    AHashMap::default(),
                    true,
                ),
                config.base_url.clone(),
                config.paths.clone(),
            ),
        }
    }

    /// Resolve a component name to a file path.
    fn resolve_component_path(&mut self, component_name: &str) -> Option<PathBuf> {
        Logger::debug(&format!("Starting to resolve import path for: {:?}", component_name), 2);
        let import_path = self.imports.get(component_name)?;
        let resolved_path = self.resolve_import(import_path, component_name);

        resolved_path
    }

    /// Resolves an import path to a file path.
    fn resolve_import(&self, import_path: &str, component_name: &str) -> Option<PathBuf> {
        let base = FileName::Real(self.file_path.clone());

        match self.resolver.resolve(&base, import_path) {
            Ok(resolved) => {
                Logger::debug(&format!("Resolved path: {:?}", resolved.filename), 2);

                if resolved.filename.to_string().contains("node_modules") {
                    return Some(PathBuf::from(resolved.filename.to_string()));
                }

                let path = PathBuf::from(resolved.filename.to_string());
                Logger::debug(&format!("Starting to traverse path: {:?}", path), 2);
                self.traverse_import(&path, component_name)
            }
            Err(_) => {
                Logger::debug(&format!("Resolved import to node_module: {:?}", import_path), 2);
                Some(PathBuf::from(import_path))
            }
        }
    }

    /// Traverse an import path and update the component graph
    fn traverse_import(&self, path: &PathBuf, component_name: &str) -> Option<PathBuf> {
        if path.is_file() {
            let content = fs::read_to_string(path).ok()?;
            let extension = path.extension().unwrap_or_default();

            // abort traversing if the path is not a JS/TS file
            if extension != "ts" && extension != "tsx" && extension != "js" && extension != "jsx" {
                Logger::debug("Not a JS/TS file", 2);
                return None;
            }

            // If it's a .tsx file and exports our component, return it
            if extension == "tsx" {
                return Some(path.clone());
            }

            let module = ProjectTraverser::parse_typescript(&content);

            for item in &module.body {
                if let ModuleItem::ModuleDecl(ModuleDecl::ExportNamed(export_named)) = item {
                    // Check if this export includes our component
                    for specifier in &export_named.specifiers {
                        match specifier {
                            ExportSpecifier::Named(named) => {
                                if let ModuleExportName::Ident(ident) = &named.orig {
                                    if ident.sym.to_string() == component_name {
                                        // Found our component, follow this export
                                        if let Some(src) = &export_named.src {
                                            let new_path_str = src.value.to_string();
                                            let base = FileName::Real(path.clone());
                                            if let Ok(resolved) = self.resolver.resolve(&base, &new_path_str) {
                                                let new_path = PathBuf::from(resolved.filename.to_string());
                                                return self.traverse_import(&new_path, component_name);
                                            }
                                        }
                                    }
                                }
                            },
                            ExportSpecifier::Default(_) => {
                                if component_name == "default" {
                                    if let Some(src) = &export_named.src {
                                        let new_path_str = src.value.to_string();
                                        let base = FileName::Real(path.clone());
                                        if let Ok(resolved) = self.resolver.resolve(&base, &new_path_str) {
                                            let new_path = PathBuf::from(resolved.filename.to_string());
                                            return self.traverse_import(&new_path, component_name);
                                        }
                                    }
                                }
                            },
                            _ => {}
                        }
                    }
                } else if let ModuleItem::ModuleDecl(ModuleDecl::ExportDefaultExpr(_)) = item {
                    if component_name == "default" {
                        return Some(path.clone());
                    }
                } else if let ModuleItem::ModuleDecl(ModuleDecl::ExportDefaultDecl(_)) = item {
                    if component_name == "default" {
                        return Some(path.clone());
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
    fn visit_module(&mut self, n: &swc_ecma_visit::swc_ecma_ast::Module) {
        n.visit_children_with(self);
    }

    /// Visits function declarations and checks if they are React components.
    fn visit_fn_decl(&mut self, n: &swc_ecma_visit::swc_ecma_ast::FnDecl) {
        Logger::debug(&format!("Visiting fn decl: {:?}", n), 3);

        if self.is_component(&n.ident) {
            let component_name = n.ident.sym.to_string();

            self.current_component = Some(component_name.clone());

            self.component_graph
                .add_component(component_name.clone(), self.file_path.clone());
        }

        n.visit_children_with(self);
        self.current_component = None;
    }

    /// Visits variable declarations and checks if they are React components.
    fn visit_var_decl(&mut self, n: &VarDecl) {
        Logger::debug(&format!("Visiting var decl: {:?}", n), 3);

        for decl in &n.decls {
            if let Some(init) = &decl.init {
                if let Expr::Arrow(_arrow_expr) = &**init {
                    if let Pat::Ident(ident) = &decl.name {
                        if self.is_component(&ident.id) {
                            let component_name = ident.id.sym.to_string();
                            self.current_component = Some(component_name.clone());
                            self.component_graph
                                .add_component(component_name, self.file_path.clone());
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
                    Logger::debug(&format!("Found component: {:?}", ident), 3);
                    let component_name = ident.sym.to_string();
                    self.current_component = Some(component_name.clone());
                    self.component_graph
                        .add_component(component_name, self.file_path.clone());
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
                    Logger::debug(&format!("Found component: {:?}", ident), 3);
                    let component_name = ident.sym.to_string();
                    let component_path = self
                        .resolve_component_path(&component_name)
                        .unwrap_or_else(|| self.file_path.clone());

                    self.component_graph.add_child(
                        (&current_component, &self.file_path),
                        (&component_name, &component_path),
                    );

                    // Add prop usage
                    for attr in &n.attrs {
                        if let JSXAttrOrSpread::JSXAttr(jsx_attr) = attr {
                            if let JSXAttrName::Ident(attr_name) = &jsx_attr.name {
                                self.component_graph.add_prop_usage(
                                    &component_name,
                                    &component_path,
                                    attr_name.sym.to_string(),
                                );
                            }
                        }
                    }
                }
            }
        }

        n.visit_children_with(self);
    }

    fn visit_import_decl(&mut self, n: &ImportDecl) {
        for specifier in &n.specifiers {
            if let ImportSpecifier::Named(named) = specifier {
                Logger::debug(&format!("Found import: {:?}", named.local), 3);
                self.imports
                    .insert(named.local.sym.to_string(), n.src.value.to_string());
            }
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
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        let my_component_file_path = root.join("MyComponent.tsx");
        let code = r#"
            function MyComponent() {
                return <div>Hello</div>;
            }
        "#;
        fs::write(&my_component_file_path, code).unwrap();

        let module = parse_module(code);
        let mut component_graph = ComponentGraph::new();
        let mut visitor = FileVisitor::new(
            my_component_file_path.display().to_string(),
            &mut component_graph,
            Config::new(PathBuf::from("."), Vec::new()),
        );
        visitor.visit_module(&module);

        assert!(visitor
            .component_graph
            .has_component("MyComponent", &visitor.file_path));
        assert!(visitor.component_graph.graph.node_count() == 1);
    }

    #[test]
    fn test_detect_function_component_with_children_components() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        let my_component_file_path = root.join("MyComponent.tsx");
        let code = r#"
            function MyComponent() {
                return <ChildComponent />;
            }

            function ChildComponent() {
                return <div>Child</div>;
            }
        "#;
        fs::write(&my_component_file_path, code).unwrap();

        let module = parse_module(code);
        let mut component_graph = ComponentGraph::new();
        let mut visitor = FileVisitor::new(
            my_component_file_path.display().to_string(),
            &mut component_graph,
            Config::new(PathBuf::from("."), Vec::new()),
        );
        visitor.visit_module(&module);

        assert!(visitor
            .component_graph
            .has_component("MyComponent", &visitor.file_path));
        assert!(visitor
            .component_graph
            .has_component("ChildComponent", &visitor.file_path));
        assert!(visitor.component_graph.graph.node_count() == 2);

        let my_component_node = visitor
            .component_graph
            .get_component("MyComponent", &visitor.file_path)
            .unwrap();
        let child_component_node = visitor
            .component_graph
            .get_component("ChildComponent", &visitor.file_path)
            .unwrap();
        assert!(visitor
            .component_graph
            .graph
            .contains_edge(my_component_node, child_component_node));
        assert!(
            visitor
                .component_graph
                .graph
                .edges(my_component_node)
                .count()
                == 1
        );
        assert!(
            visitor
                .component_graph
                .graph
                .edges(child_component_node)
                .count()
                == 0
        );
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
        let mut visitor = FileVisitor::new(
            String::new(),
            &mut component_graph,
            Config::new(PathBuf::from("."), Vec::new()),
        );
        visitor.visit_module(&module);

        assert!(visitor
            .component_graph
            .has_component("MyComponent", &visitor.file_path));
        assert!(visitor
            .component_graph
            .has_component("Button", &visitor.file_path));
        assert!(visitor.component_graph.graph.node_count() == 2);

        let button_node = visitor
            .component_graph
            .get_component("Button", &visitor.file_path)
            .unwrap();
        assert_eq!(
            visitor.component_graph.graph[button_node]
                .prop_usage
                .get("className")
                .unwrap(),
            &1
        );
    }

    #[test]
    fn test_assign_component() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        let my_component_file_path = root.join("MyComponent.tsx");
        let code = r#"
            let MyComponent = () => <div>Hello</div>;
            const MyComponent2 = () => <div>Hello</div>;
        "#;
        fs::write(&my_component_file_path, code).unwrap();

        let module = parse_module(code);
        let mut component_graph = ComponentGraph::new();
        let mut visitor = FileVisitor::new(
            my_component_file_path.display().to_string(),
            &mut component_graph,
            Config::new(PathBuf::from("."), Vec::new()),
        );
        visitor.visit_module(&module);

        assert!(visitor
            .component_graph
            .has_component("MyComponent", &visitor.file_path));
        assert!(visitor
            .component_graph
            .has_component("MyComponent2", &visitor.file_path));
        assert!(visitor.component_graph.graph.node_count() == 2);
    }

    #[test]
    fn test_import_component() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        let file_path = root.join("Button.tsx");
        fs::write(
            &file_path,
            "export function Button() { return <button>Click me</button>; }",
        )
        .unwrap();

        let my_component_file_path = root.join("MyComponent.tsx");
        let code = r#"
            import { Button } from "./Button";

            function MyComponent() {
                return <Button />;
            }
        "#;
        fs::write(&my_component_file_path, code).unwrap();

        let module = parse_module(code);
        let mut component_graph = ComponentGraph::new();
        let mut visitor = FileVisitor::new(
            my_component_file_path.display().to_string(),
            &mut component_graph,
            Config::new(PathBuf::from("."), Vec::new()),
        );
        visitor.visit_module(&module);

        assert!(visitor
            .component_graph
            .has_component("MyComponent", &my_component_file_path));
        assert!(visitor.component_graph.has_component("Button", &file_path));
        assert!(visitor.component_graph.graph.node_count() == 2);
        assert!(
            visitor
                .component_graph
                .graph
                .edges(
                    visitor
                        .component_graph
                        .get_component("MyComponent", &my_component_file_path)
                        .unwrap()
                )
                .count()
                == 1
        );
    }

    #[test]
    fn test_import_component_from_nested_directory() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        fs::create_dir_all(root.join("components")).unwrap();

        let file_path = root.join("components/Button.tsx");
        fs::write(
            &file_path,
            "export function Button() { return <button>Click me</button>; }",
        )
        .unwrap();

        let my_component_file_path = root.join("MyComponent.tsx");
        let code = r#"
            import { Button } from "./components/Button";

            function MyComponent() {
                return <Button />;
            }
        "#;
        fs::write(&my_component_file_path, code).unwrap();

        let module = parse_module(code);
        let mut component_graph = ComponentGraph::new();
        let mut visitor = FileVisitor::new(
            my_component_file_path.display().to_string(),
            &mut component_graph,
            Config::new(PathBuf::from("."), Vec::new()),
        );
        visitor.visit_module(&module);

        assert!(visitor
            .component_graph
            .has_component("MyComponent", &my_component_file_path));
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
        fs::write(
            &button_file_path,
            "export function Button() { return <button>Click me</button>; }",
        )
        .unwrap();

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
        let mut visitor = FileVisitor::new(
            my_component_file_path.display().to_string(),
            &mut component_graph,
            Config::new(PathBuf::from("."), Vec::new()),
        );
        visitor.visit_module(&module);

        assert!(visitor
            .component_graph
            .has_component("MyComponent", &visitor.file_path));
        assert!(visitor
            .component_graph
            .has_component("Button", &button_file_path));
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
        fs::write(
            &button_file_path,
            "export function Button() { return <button>Click me</button>; }",
        )
        .unwrap();

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
        let mut visitor = FileVisitor::new(
            my_component_file_path.display().to_string(),
            &mut component_graph,
            Config::new(PathBuf::from("."), Vec::new()),
        );
        visitor.visit_module(&module);

        assert!(visitor
            .component_graph
            .has_component("MyComponent", &visitor.file_path));
        assert!(visitor
            .component_graph
            .has_component("Button", &button_file_path));
        assert!(visitor.component_graph.graph.node_count() == 2);
    }

    #[test]
    fn test_import_component_without_extension() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        let file_path = root.join("Button.tsx");
        fs::write(
            &file_path,
            "export function Button() { return <button>Click me</button>; }",
        )
        .unwrap();

        let my_component_file_path = root.join("MyComponent.tsx");
        let my_component_code = r#"
            import { Button } from "./Button";

            function MyComponent() {
                return <Button />;
            }
        "#;

        fs::write(&my_component_file_path, my_component_code).unwrap();

        let module = parse_module(my_component_code);
        let mut component_graph = ComponentGraph::new();
        let mut visitor = FileVisitor::new(
            my_component_file_path.display().to_string(),
            &mut component_graph,
            Config::new(PathBuf::from("."), Vec::new()),
        );
        visitor.visit_module(&module);

        assert!(visitor
            .component_graph
            .has_component("MyComponent", &visitor.file_path));
        assert!(visitor.component_graph.has_component("Button", &file_path));
        assert!(visitor.component_graph.graph.node_count() == 2);
    }

    #[test]
    fn test_import_component_from_tsconfig_paths() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create the necessary directories and files
        fs::create_dir_all(root.join("src/components")).unwrap();
        fs::write(
            root.join("src/components/Button.tsx"),
            "export const Button = () => <button>Click me</button>;",
        )
        .unwrap();

        let my_component_file_path = root.join("src/MyComponent.tsx");
        let my_component_code = r#"
    import { Button } from "@components/Button";

    const MyComponent = () => <Button />;
    "#;
        fs::write(&my_component_file_path, my_component_code).unwrap();

        // Read tsconfig and create Config
        let config = Config::new(
            root.to_path_buf(),
            vec![(
                "@components/*".to_string(),
                vec!["src/components/*".to_string()],
            )],
        );

        let module = parse_module(my_component_code);
        let mut component_graph = ComponentGraph::new();
        let mut visitor = FileVisitor::new(
            my_component_file_path.display().to_string(),
            &mut component_graph,
            config,
        );
        visitor.visit_module(&module);

        assert!(visitor
            .component_graph
            .has_component("MyComponent", &my_component_file_path));
        assert!(visitor
            .component_graph
            .has_component("Button", &root.join("src/components/Button.tsx")));
        assert_eq!(visitor.component_graph.graph.node_count(), 2);
    }

    #[test]
    fn should_skip_resolving_imports_for_non_components() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        let button_file_path = root.join("Button.tsx");
        fs::write(
            &button_file_path,
            "export function Button() { return <button>Click me</button>; }",
        )
        .unwrap();

        let file_path = root.join("MyComponent.tsx");
        let code = r#"
            import { useState } from "react";
            import { blah } from "./blah";
            import "./styles.css";
            import * as NonExistent from "./NonExistent";
            import { Button } from "./Button";

            export function MyComponent() { return <Button />; }
        "#;
        fs::write(&file_path, code).unwrap();

        let module = parse_module(code);
        let mut component_graph = ComponentGraph::new();
        let mut visitor = FileVisitor::new(
            file_path.display().to_string(),
            &mut component_graph,
            Config::new(PathBuf::from("."), Vec::new()),
        );
        visitor.visit_module(&module);

        assert_eq!(visitor.component_graph.graph.node_count(), 2);
        assert!(visitor
            .component_graph
            .has_component("MyComponent", &file_path));
        assert!(visitor
            .component_graph
            .has_component("Button", &button_file_path));
    }

    #[test]
    fn should_use_node_modules_path_when_resolving_components_imported_from_node_modules() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        let file_path = root.join("MyComponent.tsx");
        let code = r#"
            import { Button } from "react-bootstrap";

            export function MyComponent() { return <Button />; }
        "#;
        fs::write(&file_path, code).unwrap();

        let module = parse_module(code);
        let mut component_graph = ComponentGraph::new();
        let mut visitor = FileVisitor::new(
            file_path.display().to_string(),
            &mut component_graph,
            Config::new(PathBuf::from("."), Vec::new()),
        );
        visitor.visit_module(&module);

        assert_eq!(visitor.component_graph.graph.node_count(), 2);
        assert!(visitor
            .component_graph
            .has_component("MyComponent", &file_path));
        assert!(visitor
            .component_graph
            .has_component("Button", &PathBuf::from("react-bootstrap")));
    }
}
