use std::path::PathBuf;

use itertools::Itertools;
use miette::NamedSource;
use oxc_allocator::Allocator;
use oxc_parser::{Parser, ParserReturn};
use oxc_semantic::{SemanticBuilder, SemanticBuilderReturn};
use oxc_span::SourceType;

/// parse a tsx file with oxc
pub fn parse_tsx<'a>(
    allocator: &'a Allocator,
    file_path: &PathBuf,
    file_content: &'a str,
) -> Result<(ParserReturn<'a>, SemanticBuilderReturn<'a>), String> {
    let source_type = SourceType::default().with_typescript(true).with_jsx(true);

    let parser = Parser::new(&allocator, &file_content, source_type);
    let parser_ret = parser.parse();
    let named_source = NamedSource::new(
        file_path.to_string_lossy().to_string(),
        file_content.to_string(),
    );

    if !parser_ret.errors.is_empty() {
        let error_message: String = parser_ret
            .errors
            .into_iter()
            .map(|error| format!("{:?}", error.with_source_code(named_source.clone())))
            .join("\n");
        println!("Parsing failed:\n\n{error_message}",);
        return Err(error_message);
    }

    let semantic_ret = SemanticBuilder::new().build(&parser_ret.program);

    if !semantic_ret.errors.is_empty() {
        let error_message: String = semantic_ret
            .errors
            .into_iter()
            .map(|error| format!("{:?}", error.with_source_code(named_source.clone())))
            .join("\n");
        println!("Parsing failed:\n\n{error_message}",);
        return Err(error_message);
    }

    Ok((parser_ret, semantic_ret))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_tsx() {
        let allocator = Allocator::default();
        let file_path = PathBuf::from("test.tsx");
        let file_content = "const App = () => <div>Hello, world!</div>;";
        let result = parse_tsx(&allocator, &file_path, &file_content);

        assert!(result.is_ok());
    }
}
