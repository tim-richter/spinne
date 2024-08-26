use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::fs;
use std::io;
use std::collections::{HashSet, HashMap};
use swc_common::{errors::{ColorConfig, Handler}, FileName, SourceMap};
use swc_common::sync::Lrc;
use swc_ecma_loader::resolve::Resolve;
use swc_ecma_loader::{resolvers::node::NodeModulesResolver, TargetEnv};
use swc_ecma_parser::{lexer::Lexer, Parser, StringInput, Syntax, TsSyntax};
use petgraph::dot::{Dot, Config};
use petgraph::graphmap::DiGraphMap;
use swc_ecma_visit::Visit;
use crate::visitors::ComponentUsageVisitor;

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

    pub fn traverse(&self, entry_point: &Path) -> io::Result<(HashSet<PathBuf>, String)> {
        let mut visited = HashSet::new();
        let mut component_graph = HashMap::new();
        self.traverse_recursive(entry_point, &mut visited, &mut component_graph)?;

        let graph = self.create_graph(&component_graph);
        Ok((visited, graph))
    }

    fn traverse_recursive(&self, file_path: &Path, visited: &mut HashSet<PathBuf>, component_graph: &mut HashMap<String, Vec<String>>) -> io::Result<()> {
        let canonical_path = file_path.canonicalize()?;
        if !visited.insert(canonical_path.clone()) {
            // File already visited, no need to process again
            return Ok(());
        }
    
        println!("Processing file: {:?}", canonical_path);
    
        let content = fs::read_to_string(&canonical_path)?;
        let (component_usages, imports) = self.parse_file(&content)?;
    
        for (component, usages) in component_usages {
            let component_name = component.split(':').last().unwrap_or(&component).to_string();
            for (used_component, _) in usages {
                let used_component_name = used_component.split(':').last().unwrap_or(&used_component).to_string();
                component_graph.entry(component_name.clone()).or_default().push(used_component_name);
            }
        }
    
        // Follow imports
        for import in imports {
            if let Ok(resolved) = self.resolver.resolve(&FileName::Real(canonical_path.clone()), &import) {
                if let FileName::Real(ref resolved_path) = resolved.filename {
                    println!("Resolved path: {:?}", resolved_path);
                    self.traverse_recursive(resolved_path, visited, component_graph)?;
                }
            }
        }
    
        Ok(())
    }

    fn parse_file(&self, content: &str) -> io::Result<(HashMap<String, Vec<(String, String)>>, Vec<String>)> {
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
                io::Error::new(io::ErrorKind::Other, "Failed to parse module")
            })?;

        let mut visitor = ComponentUsageVisitor::default();
        visitor.visit_module(&module);

        Ok((visitor.component_usages, visitor.imports))
    }

    fn create_graph(&self, component_graph: &HashMap<String, Vec<String>>) -> String {
        let mut graph = DiGraphMap::new();
        for (component, usages) in component_graph {
            for used_component in usages {
                graph.add_edge(component, used_component, ());
            }
        }
        format!("{:?}", Dot::with_config(&graph, &[Config::EdgeNoLabel]))
    }
}