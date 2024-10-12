use std::{fs, sync::Arc};
use std::path::Path;
use log::debug;
use swc_ecma_parser::{lexer::Lexer, Parser, StringInput, Syntax, TsSyntax};

use swc_ecma_visit::Visit;

use crate::{component_graph::ComponentGraph, file_visitor::FileVisitor, config::Config, ts_config_reader::TsConfigReader};

/// ProjectTraverser is responsible for traversing the project and analyzing TypeScript files
pub struct ProjectTraverser {
    component_graph: ComponentGraph,
    config: Arc<Config>,
}

impl ProjectTraverser {
    pub fn new(project_root: &Path) -> Self {
        let tsconfig_path = project_root.join("tsconfig.json");
        let (base_url, paths) = TsConfigReader::read_tsconfig(&tsconfig_path);
        let config = Config::new(base_url, paths);

        Self {
            component_graph: ComponentGraph::new(),
            config,
        }
    }

    /// Traverse the project and analyze TypeScript files
    pub fn traverse(&mut self, entry_point: &Path, ignore: &[String]) -> std::io::Result<&ComponentGraph> {
        self.traverse_directory(entry_point, ignore)?;
        Ok(&self.component_graph)
    }

    /// Traverse the directory and analyze TypeScript files
    fn traverse_directory(&mut self, dir: &Path, ignore: &[String]) -> std::io::Result<()> {
        debug!("Traversing directory: {:?}", dir);
        if !dir.exists() {
            return Err(std::io::Error::new(std::io::ErrorKind::NotFound, format!("Entry point does not exist: {:?}", dir)));
        }

        if dir.is_file() {
            return Err(std::io::Error::new(std::io::ErrorKind::Other, format!("Entry point is a file: {:?}", dir)));
        }

        if dir.is_dir() {
            for entry in fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();

                if ignore.iter().any(|pattern| glob::Pattern::new(pattern).unwrap().matches_path(&path)) {
                    continue;
                }

                if path.is_dir() {
                    self.traverse_directory(&path, ignore)?;
                } else if let Some(extension) = path.extension() {
                    if extension == "tsx" {
                        self.analyze_file(&path)?;
                    }
                }
            }
        }

        Ok(())
    }

    /// Analyze a TypeScript file and update the component graph
    fn analyze_file(&mut self, file_path: &Path) -> std::io::Result<()> {
        let source_code = fs::read_to_string(file_path)?;
        let module = ProjectTraverser::parse_typescript(&source_code);

        let mut visitor = FileVisitor::new(file_path.to_str().unwrap().to_string(), &mut self.component_graph, self.config.clone());
        visitor.visit_module(&module);

        Ok(())
    }

    /// Parse the TypeScript source code into an AST module
    pub fn parse_typescript(source_code: &str) -> swc_ecma_ast::Module {
        let lexer = Lexer::new(
            Syntax::Typescript(TsSyntax {
                tsx: true,
                ..Default::default()
            }),
            Default::default(),
            StringInput::new(source_code, swc_common::BytePos(0), swc_common::BytePos(1)),
            None,
        );

        let mut parser = Parser::new_from(lexer);
        parser
            .parse_module().expect("Failed to parse TypeScript module")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn create_mock_project() -> TempDir {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create a nested directory structure
        fs::create_dir_all(root.join("src/components")).unwrap();
        fs::create_dir_all(root.join("src/pages")).unwrap();

        // Create some mock .tsx files
        let files = vec![
            ("src/components/Button.tsx", "export function Button() { return <button>Click me</button>; }"),
            ("src/components/Header.tsx", "import { Button } from './Button'; export function Header() { return <header><Button /></header>; }"),
            ("src/pages/Home.tsx", "import { Header } from '../components/Header'; export function Home() { return <div><Header /><main>Welcome</main></div>; }"),
            ("src/index.tsx", "import { Home } from './pages/Home'; export function App() { return <Home />; }"),
        ];

        for (path, content) in files {
            let file_path = root.join(path);
            let mut file = File::create(file_path).unwrap();
            file.write_all(content.as_bytes()).unwrap();
        }

        // Create a non-tsx file
        let mut other_file = File::create(root.join("README.md")).unwrap();
        other_file.write_all(b"# Mock Project").unwrap();

        temp_dir
    }

    #[test]
    fn test_project_traverser() {
        let temp_dir = create_mock_project();
        let mut traverser = ProjectTraverser::new(temp_dir.path());
        let result = traverser.traverse(temp_dir.path(), &vec![]);

        assert!(result.is_ok());
        let graph = result.unwrap();

        // Check if all components were found
        assert!(graph.has_component("Button", &PathBuf::from(temp_dir.path().join("src/components/Button.tsx"))));
        assert!(graph.has_component("Header", &PathBuf::from(temp_dir.path().join("src/components/Header.tsx"))));
        assert!(graph.has_component("Home", &PathBuf::from(temp_dir.path().join("src/pages/Home.tsx"))));
        assert!(graph.has_component("App", &PathBuf::from(temp_dir.path().join("src/index.tsx"))));
    }

    #[test]
    fn test_empty_directory() {
        let temp_dir = TempDir::new().unwrap();
        let mut traverser = ProjectTraverser::new(temp_dir.path());
        let result = traverser.traverse(temp_dir.path(), &vec![]);

        assert!(result.is_ok());
        let graph = result.unwrap();
        assert_eq!(graph.graph.node_count(), 0);
    }

    #[test]
    fn test_non_existent_directory() {
        let non_existent_path = Path::new("/path/to/non/existent/directory");
        let mut traverser = ProjectTraverser::new(non_existent_path);
        let result = traverser.traverse(non_existent_path, &vec![]).map_err(|e| e.to_string());

        let expected_error = format!("Entry point does not exist: {:?}", non_existent_path);
        assert_eq!(result.unwrap_err(), expected_error);
    }

    #[test]
    fn test_ignore_directory() {
        let temp_dir = create_mock_project();
        let mut traverser = ProjectTraverser::new(temp_dir.path());
        let result = traverser.traverse(temp_dir.path(), &vec!["**/src/**".to_string()]);

        assert!(result.is_ok());
        let graph = result.unwrap();
        assert_eq!(graph.graph.node_count(), 0);
    }

    #[test]
    fn test_should_not_traverse_file() {
        let temp_dir = create_mock_project();
        let mut traverser = ProjectTraverser::new(temp_dir.path());
        let result = traverser.traverse(temp_dir.path().join("src/index.tsx").as_path(), &vec![]);

        assert!(result.is_err());
        assert_eq!(result.err().unwrap().to_string(), format!("Entry point is a file: {:?}", temp_dir.path().join("src/index.tsx")));
    }
}
