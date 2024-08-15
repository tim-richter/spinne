use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::fs;
use std::io;
use swc_common::{errors::{ColorConfig, Handler}, FileName, SourceMap};
use swc_common::sync::Lrc;
use swc_ecma_loader::resolve::Resolve;
use swc_ecma_loader::{resolvers::node::NodeModulesResolver, TargetEnv};
use swc_ecma_parser::{lexer::Lexer, Parser, StringInput, Syntax, TsConfig};
use std::collections::HashSet;
use swc_ecma_visit::Visit;

pub struct ProjectTraverser {
    resolver: Arc<NodeModulesResolver>,
    source_map: Lrc<SourceMap>,
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
            source_map,
            handler,
        }
    }

    pub fn traverse(&self, entry_file: &Path) -> std::io::Result<HashSet<PathBuf>> {
        let mut visited = HashSet::new();
        self.traverse_recursive(entry_file, &mut visited)?;
        Ok(visited)
    }

    fn traverse_recursive(&self, file_path: &Path, visited: &mut std::collections::HashSet<PathBuf>) -> std::io::Result<()> {
        let canonical_path = file_path.canonicalize().map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
        if !visited.insert(canonical_path.clone()) {
            return Ok(());
        }

        println!("Processing file: {:?}", canonical_path);

        // Read and parse the file content using swc
        let content = fs::read_to_string(&canonical_path).map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

        // Use swc to parse the file
        let imports = self.parse_imports(&content).map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

        for import in imports {
            if let Ok(resolved) = self.resolver.resolve(&FileName::Real(canonical_path.clone()), &import) {
                match resolved.filename {
                    FileName::Real(ref resolved_path) => {
                        // Resolved to a local file path, so continue traversing
                        self.traverse_recursive(resolved_path, visited)?;
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

    fn parse_imports(&self, content: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let lexer = Lexer::new(
            Syntax::Typescript(TsConfig {
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
        let mut visitor = ImportVisitor::default();
        visitor.visit_module(&module);

        Ok(visitor.imports)
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
