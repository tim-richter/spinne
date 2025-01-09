use std::path::PathBuf;

use oxc_allocator::Allocator;
use oxc_ast::{
    ast::{ExportNamedDeclaration, ModuleExportName},
    AstKind, Visit,
};
use oxc_semantic::{NodeId, Semantic, SymbolFlags, SymbolId};
use oxc_span::Atom;
use spinne_logger::Logger;

use crate::{parse::parse_tsx, traverse::ProjectResolver};

use super::find_import::find_import_for_symbol;

struct ComponentExportVisitor<'a> {
    component_name: &'a str,
    source_path: Option<Atom<'a>>,
}

impl<'a> Visit<'a> for ComponentExportVisitor<'a> {
    fn visit_export_named_declaration(
        &mut self,
        export_named_declaration: &ExportNamedDeclaration<'a>,
    ) {
        let specifiers = &export_named_declaration.specifiers;
        let source_path = if let Some(source) = &export_named_declaration.source {
            Some(source.value.clone())
        } else {
            None
        };

        for specifier in specifiers {
            match &specifier.local {
                ModuleExportName::IdentifierReference(identifier_reference) => {
                    if identifier_reference.name == self.component_name {
                        self.source_path = source_path.clone();
                    }
                }
                ModuleExportName::IdentifierName(identifier_name) => {
                    if identifier_name.name == self.component_name {
                        self.source_path = source_path.clone();
                    }
                }
                ModuleExportName::StringLiteral(string_literal) => {
                    if string_literal.value == self.component_name {
                        self.source_path = source_path.clone();
                    }
                }
            }
        }
    }
}

pub fn find_component_root(
    semantic: &Semantic,
    resolver: &ProjectResolver,
    file_path: &PathBuf,
    node_id: NodeId,
    component_name: &str,
) -> Option<(String, PathBuf)> {
    let import_node = semantic.nodes().get_node(node_id);

    if let AstKind::ImportDeclaration(import_decl) = import_node.kind() {
        let specifier = import_decl.source.value.clone();

        return recursive_find(resolver, file_path, &specifier, component_name);
    }

    None
}

fn recursive_find(
    resolver: &ProjectResolver,
    file_path: &PathBuf,
    specifier: &str,
    component_name: &str,
) -> Option<(String, PathBuf)> {
    let resolved_path = resolver.resolve(file_path, specifier);

    if let Err(e) = resolved_path {
        Logger::error(&format!("Error resolving path: {:?}", e));
        return None;
    }

    let resolved_path = resolved_path.unwrap();

    if resolved_path.path().is_file() {
        let content = std::fs::read_to_string(&resolved_path.path()).unwrap();
        let directory = resolved_path.path().parent().unwrap().to_path_buf();
        let allocator = Allocator::default();
        let result = parse_tsx(&allocator, &resolved_path.path().to_path_buf(), &content);

        if result.is_err() {
            Logger::error(&format!(
                "Failed to parse file: {}",
                resolved_path.path().display()
            ));
            return None;
        }

        let (parser_ret, semantic_ret) = result.unwrap();
        let semantic = semantic_ret.semantic;

        let symbol_id = find_symbol_id(&semantic, component_name);
        if let Some(symbol_id) = symbol_id {
            let flags = semantic.symbols().get_flags(symbol_id);

            // if the symbol is a variable, then we have found the component root
            if SymbolFlags::is_variable(&flags) {
                return Some((
                    component_name.to_string(),
                    resolved_path.path().to_path_buf(),
                ));
            }

            // if the symbol is an import, then we need to find the component root of the imported file
            if SymbolFlags::is_import(&flags) {
                let import_node_id = find_import_for_symbol(&semantic, symbol_id);
                let import_node = semantic.nodes().get_node(import_node_id.unwrap());

                if let AstKind::ImportDeclaration(import_decl) = import_node.kind() {
                    let specifier = import_decl.source.value.clone();

                    return recursive_find(resolver, &directory, &specifier, component_name);
                }
            }
        }

        // else we try to find the component root by searching for the component name in the export statements
        let mut visitor = ComponentExportVisitor {
            component_name,
            source_path: None,
        };
        visitor.visit_program(&parser_ret.program);

        if let Some(source_path) = visitor.source_path {
            return recursive_find(resolver, &directory, &source_path, component_name);
        }
    }

    None
}

fn find_symbol_id(semantic: &Semantic, symbol_name: &str) -> Option<SymbolId> {
    let symbol_id = semantic.symbols().symbol_ids();

    for node in symbol_id {
        let symbol = semantic.symbols().get_name(node);
        if symbol == symbol_name {
            return Some(node);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use std::fs;

    use oxc_parser::Parser;
    use oxc_semantic::{SemanticBuilder, SemanticBuilderReturn};
    use oxc_span::SourceType;
    use tempfile::TempDir;

    use super::*;

    fn setup_semantic<'a>(allocator: &'a Allocator, content: &'a str) -> SemanticBuilderReturn<'a> {
        let source_type = SourceType::default().with_typescript(true).with_jsx(true);

        // Parse the source code
        let parser_ret = Parser::new(&allocator, &content, source_type).parse();

        let program = parser_ret.program;

        // Build semantic analysis
        SemanticBuilder::new().build(&program)
    }

    fn create_mock_project(files: &Vec<(&str, &str)>) -> TempDir {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create mock .tsx files
        for (path, content) in files {
            // create directories before creating files
            let file_path = root.join(path);
            if let Some(parent) = file_path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(file_path, content).unwrap();
        }

        // Create non-tsx files
        fs::write(root.join("README.md"), "# Mock Project").unwrap();
        fs::write(root.join("package.json"), "{}").unwrap();

        temp_dir
    }

    #[test]
    fn test_find_direct_export() {
        let allocator = Allocator::default();
        let resolver = ProjectResolver::new(None);

        let project_files = vec![
            (
                "src/components/Button.tsx",
                r#"
                import { FC } from 'react';
                import { Input } from './Input';

                const Button: FC = () => {
                  return <Input placeholder="Hello" />;
                }
            "#,
            ),
            (
                "src/components/Input.tsx",
                r#"
                export function Input() {
                    return <input />;
                }
            "#,
            ),
        ];
        let temp_dir = create_mock_project(&project_files);
        let semantic = setup_semantic(&allocator, &project_files[0].1);

        let result = find_component_root(
            &semantic.semantic,
            &resolver,
            &temp_dir.path().join("src/components"),
            NodeId::new(8),
            "Input",
        )
        .unwrap();

        assert_eq!(result.0, "Input");
        assert_eq!(result.1, temp_dir.path().join("src/components/Input.tsx"));
    }

    #[test]
    fn test_find_barrel_file_export() {
        let allocator = Allocator::default();
        let resolver = ProjectResolver::new(None);

        let project_files = vec![
            (
                "src/components/Button.tsx",
                r#"
                import { FC } from 'react';
                import { Input } from './';

                const Button: FC = () => {
                  return <Input placeholder="Hello" />;
                }
            "#,
            ),
            (
                "src/components/Input.tsx",
                r#"
                export function Input() {
                    return <input />;
                }
            "#,
            ),
            (
                "src/components/index.tsx",
                r#"
                import { Input } from './Input';

                export { Input };
            "#,
            ),
        ];
        let temp_dir = create_mock_project(&project_files);
        let semantic = setup_semantic(&allocator, &project_files[0].1);

        let result = find_component_root(
            &semantic.semantic,
            &resolver,
            &temp_dir.path().join("src/components"),
            NodeId::new(8),
            "Input",
        )
        .unwrap();

        assert_eq!(result.0, "Input");
        assert_eq!(result.1, temp_dir.path().join("src/components/Input.tsx"));
    }

    #[test]
    fn test_find_barrel_file_export_with_direct_export() {
        let allocator = Allocator::default();
        let resolver = ProjectResolver::new(None);

        let project_files = vec![
            (
                "src/components/Button.tsx",
                r#"
                import { FC } from 'react';
                import { Input } from './';

                const Button: FC = () => {
                  return <Input placeholder="Hello" />;
                }
            "#,
            ),
            (
                "src/components/Input.tsx",
                r#"
                export function Input() {
                    return <input />;
                }
            "#,
            ),
            (
                "src/components/index.tsx",
                r#"
                export { Input } from './Input';
            "#,
            ),
        ];
        let temp_dir = create_mock_project(&project_files);
        let semantic = setup_semantic(&allocator, &project_files[0].1);

        let result = find_component_root(
            &semantic.semantic,
            &resolver,
            &temp_dir.path().join("src/components"),
            NodeId::new(8),
            "Input",
        )
        .unwrap();

        assert_eq!(result.0, "Input");
        assert_eq!(result.1, temp_dir.path().join("src/components/Input.tsx"));
    }

    #[test]
    fn test_find_barrel_file_export_with_direct_export_multiple_exports() {
        let allocator = Allocator::default();
        let resolver = ProjectResolver::new(None);

        let project_files = vec![
            (
                "src/components/Button.tsx",
                r#"
                import { FC } from 'react';
                import { Input } from './';

                const Button: FC = () => {
                  return <Input placeholder="Hello" />;
                }
            "#,
            ),
            (
                "src/components/Input.tsx",
                r#"
                export function Input() {
                    return <input />;
                }
            "#,
            ),
            (
                "src/components/index.tsx",
                r#"
                export { Input, Button } from './Input';
                export { Test } from './Test';
            "#,
            ),
        ];
        let temp_dir = create_mock_project(&project_files);
        let semantic = setup_semantic(&allocator, &project_files[0].1);

        let result = find_component_root(
            &semantic.semantic,
            &resolver,
            &temp_dir.path().join("src/components"),
            NodeId::new(8),
            "Input",
        )
        .unwrap();

        assert_eq!(result.0, "Input");
        assert_eq!(result.1, temp_dir.path().join("src/components/Input.tsx"));
    }
}
