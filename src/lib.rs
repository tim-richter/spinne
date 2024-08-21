use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::fs;
use std::io;
use swc_ecma_ast::*;
use petgraph::dot::{Dot, Config};
use petgraph::graphmap::DiGraphMap;
use swc_common::{errors::{ColorConfig, Handler}, FileName, SourceMap};
use swc_common::sync::Lrc;
use swc_ecma_loader::resolve::Resolve;
use swc_ecma_loader::{resolvers::node::NodeModulesResolver, TargetEnv};
use swc_ecma_parser::{lexer::Lexer, Parser, StringInput, Syntax, TsSyntax};
use std::collections::{HashSet,HashMap};
use swc_ecma_visit::{Visit,VisitWith};

pub struct ProjectTraverser {
    resolver: Arc<NodeModulesResolver>,
    handler: Lrc<Handler>,
}

impl ProjectTraverser {
    pub fn new() -> Self {
        let source_map: Lrc<SourceMap> = Lrc::new(SourceMap::default());
        let handler: Lrc<Handler> = Lrc::new(Handler::with_tty_emitter(
            ColorConfig::Auto,
            true,
            false,
            Some(source_map.clone()),
        ));
        let resolver = NodeModulesResolver::new(
            TargetEnv::Node,
            Default::default(),
            Default::default(),
        );

        Self {
            resolver: Arc::new(resolver),
            handler,
        }
    }

    pub fn traverse(&self, entry_file: &Path) -> std::io::Result<(HashSet<PathBuf>, String)> {
        let mut visited = HashSet::new();
        let mut component_graph = HashMap::new();
        self.traverse_recursive(entry_file, &mut visited, &mut component_graph)?;

        let graph = ProjectTraverser::build_and_render_graph(&component_graph);
        Ok((visited, graph))
    }

    pub fn build_and_render_graph(component_graph: &HashMap<String, Vec<String>>) -> String {
        let mut graph = DiGraphMap::new();

        for (component, children) in component_graph {
            for child in children {
                graph.add_edge(component, child, ());
            }
        }

        format!("{:?}", Dot::with_config(&graph, &[Config::EdgeNoLabel]))
    }

    fn traverse_recursive(&self, file_path: &Path, visited: &mut std::collections::HashSet<PathBuf>, component_graph: &mut HashMap<String, Vec<String>>) -> std::io::Result<()> {
        let canonical_path = file_path.canonicalize().map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
        if !visited.insert(canonical_path.clone()) {
            return Ok(());
        }

        println!("Processing file: {:?}", canonical_path);

        // Read and parse the file content using swc
        let content = fs::read_to_string(&canonical_path).map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

        // Use swc to parse the file
        let (imports, components) = self.parse_imports(&content).map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

        for (component, children) in components {
            component_graph.entry(component).or_default().extend(children);
        }

        for import in imports {
            if let Ok(resolved) = self.resolver.resolve(&FileName::Real(canonical_path.clone()), &import) {
                match resolved.filename {
                    FileName::Real(ref resolved_path) => {
                        // Resolved to a local file path, so continue traversing
                        self.traverse_recursive(resolved_path, visited, component_graph)?;
                    }
                    FileName::Url(ref url) => {
                        println!("Resolved to URL: {}", url);
                        // Handle URLs if necessary
                    }
                    FileName::Macros(ref macro_name) => {
                        println!("Resolved to macro: {}", macro_name);
                        // Handle macro source, if relevant to your use case
                    }
                    FileName::QuoteExpansion | FileName::Anon | FileName::MacroExpansion | FileName::ProcMacroSourceCode => {
                        println!("Resolved to an internal or generated source: {:?}", resolved.filename);
                        // Handle or log these cases as needed
                    }
                    FileName::Internal(ref description) => {
                        println!("Resolved to an internal source: {}", description);
                        // Handle internal sources if necessary
                    }
                    FileName::Custom(ref custom_name) => {
                        println!("Resolved to a custom source: {}", custom_name);
                        // Handle custom sources if necessary
                    }
                }
            }
        }

        Ok(())
    }

    fn parse_imports(&self, content: &str) -> Result<(Vec<String>, HashMap<String, Vec<String>>), Box<dyn std::error::Error>> {
        let lexer = Lexer::new(
            Syntax::Typescript(TsSyntax {
                tsx: true,
                ..Default::default()
            }),
            Default::default(),
            StringInput::new(content, swc_common::BytePos(0), swc_common::BytePos(1)),
            None,
        );

        let mut parser = Parser::new_from(lexer);
        let module = parser
            .parse_module()
            .map_err(|e| {
                e.into_diagnostic(&self.handler).emit();
                "Failed to parse module".to_string()
            })?;

        // Visit the AST to extract import declarations
        let mut import_visitor = ImportVisitor::default();
        import_visitor.visit_module(&module);

        let mut component_visitor = ComponentUsageVisitor::default();
        component_visitor.visit_module(&module);

        Ok((import_visitor.imports, component_visitor.component_usages))
    }
}

#[derive(Default)]
struct ImportVisitor {
    pub imports: Vec<String>,
}

impl Visit for ImportVisitor {
    fn visit_import_decl(&mut self, import_decl: &swc_ecma_ast::ImportDecl) {
        self.imports.push(import_decl.src.value.to_string());
    }
}

#[derive(Default)]
struct ComponentUsageVisitor {
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

    // Check if an identifier is a base HTML element
    fn is_base_element(&self, ident: &Ident) -> bool {
        Self::BASE_ELEMENTS.contains(&ident.sym.as_ref())
    }

    // Helper function to check if a name is a potential React component
    fn is_potential_component_name(&self, ident: &Ident) -> bool {
        let first_char = ident.sym.chars().next().unwrap_or_default();
        first_char.is_uppercase()
    }

    // Helper function to check if a type is related to React
    fn is_react_component_type(&self, type_ann: Option<&TsTypeAnn>) -> bool {
        if let Some(type_ann) = type_ann {
            if let TsType::TsTypeRef(type_ref) = &*type_ann.type_ann {
                if let TsEntityName::Ident(ident) = &type_ref.type_name {
                    return ident.sym == "React" || ident.sym == "JSX" || ident.sym == "FC";
                }
            }
        }
        false
    }

    // Check if the initializer expression is a React component
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
}

impl Visit for ComponentUsageVisitor {
    fn visit_fn_decl(&mut self, n: &FnDecl) {
        if self.is_potential_component_name(&n.ident) {
            if let Some(return_type) = &n.function.return_type {
                if self.is_react_component_type(Some(return_type)) {
                    self.current_component = Some(n.ident.sym.to_string());
                }
            }
        }

        n.visit_children_with(self);

        self.current_component = None;
    }

    fn visit_var_declarator(&mut self, n: &VarDeclarator) {
        if let Some(ident) = n.name.as_ident() {
            if self.is_potential_component_name(ident) {
                if let Some(init) = &n.init {
                    if self.is_react_component_expr(init) {
                        self.current_component = Some(ident.sym.to_string());
                    }
                }
            }
        }

        n.visit_children_with(self);

        self.current_component = None;
    }

    fn visit_jsx_opening_element(&mut self, n: &JSXOpeningElement) {
        if let Some(ref current_component) = self.current_component {
            if let JSXElementName::Ident(ident) = &n.name {
                if !self.is_base_element(ident) {
                    let component_name = ident.sym.to_string();
                    self.component_usages
                        .entry(current_component.clone())
                        .or_default()
                        .push(component_name);
                }
            }
        }
        n.visit_children_with(self);
    }
}
