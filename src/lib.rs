use std::path::{Path, PathBuf};
use std::sync::Arc;
use swc_common::FileName;
use swc_ecma_loader::resolve::Resolve;
use swc_ecma_loader::{resolvers::node::NodeModulesResolver, TargetEnv};

pub struct ProjectTraverser {
    resolver: Arc<NodeModulesResolver>,
}

impl ProjectTraverser {
    pub fn new(base_path: &Path) -> Self {
        let resolver = NodeModulesResolver::new(
            TargetEnv::Node,
            Default::default(),
            Default::default(),
        );

        Self {
            resolver: Arc::new(resolver),
        }
    }

    pub fn traverse(&self, entry_file: &Path) -> std::io::Result<()> {
        let mut visited = std::collections::HashSet::new();
        self.traverse_recursive(entry_file, &mut visited)
    }

    fn traverse_recursive(&self, file_path: &Path, visited: &mut std::collections::HashSet<PathBuf>) -> std::io::Result<()> {
        let canonical_path = file_path.canonicalize()?;
        if !visited.insert(canonical_path.clone()) {
            return Ok(());
        }

        println!("Processing file: {:?}", canonical_path);

        // Here you would typically load and parse the file
        // For this example, we'll just simulate resolving imports
        let imports = vec!["./some_module".to_string(), "external-package".to_string()];

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
}

fn main() -> std::io::Result<()> {
    let base_path = Path::new("path/to/your/project");
    let entry_file = base_path.join("entry_file.ts");

    let traverser = ProjectTraverser::new(base_path);
    traverser.traverse(&entry_file)?;

    Ok(())
}
