use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::fs;
use std::io;
use std::collections::HashSet;
use swc_common::{errors::{ColorConfig, Handler}, FileName, SourceMap};
use swc_common::sync::Lrc;
use swc_ecma_loader::resolve::Resolve;
use swc_ecma_loader::{resolvers::node::NodeModulesResolver, TargetEnv};
use swc_ecma_parser::{lexer::Lexer, Parser, StringInput, Syntax, TsSyntax};
use crate::visitors::FileVisitor;
use crate::component_graph::ComponentGraph;
use swc_ecma_visit::Visit;

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

    pub fn traverse(&self, entry_point: &Path) -> io::Result<ComponentGraph> {
        let mut visited = HashSet::new();
        let mut component_graph = ComponentGraph::new();
        self.traverse_recursive(entry_point, &mut visited, &mut component_graph)?;
        Ok(component_graph)
    }

    fn traverse_recursive(&self, file_path: &Path, visited: &mut HashSet<PathBuf>, component_graph: &mut ComponentGraph) -> io::Result<()> {
        let canonical_path = file_path.canonicalize()?;
        if !visited.insert(canonical_path.clone()) {
            // File already visited, no need to process again
            return Ok(());
        }
    
        println!("Processing file: {:?}", canonical_path);
    
        let content = fs::read_to_string(&canonical_path)?;
        let imports = self.parse_file(&content, &canonical_path, component_graph)?;
    
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

    fn parse_file(&self, content: &str, canonical_path: &PathBuf, component_graph: &mut ComponentGraph) -> io::Result<Vec<String>> {
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

        let mut visitor = FileVisitor::new(canonical_path.to_string_lossy().to_string(), component_graph);
        visitor.visit_module(&module);

        Ok(visitor.imports)
    }
}