//! JavaScript parser module using Oxc.
//!
//! This module provides functionality to parse JavaScript code and work with
//! the resulting Abstract Syntax Tree (AST).

pub mod visitor;

use oxc_allocator::Allocator;
use oxc_ast::ast::Program;
use oxc_parser::{Parser as OxcParser, ParserReturn};
use oxc_span::SourceType;
use std::path::Path;
use tracing::{debug, info, warn};

use crate::error::{DecypherError, Result};

/// JavaScript parser that wraps Oxc parser.
pub struct Parser {
    source_text: String,
    source_path: Option<String>,
}

/// Result of parsing JavaScript code.
pub struct ParseResult<'a> {
    /// The parsed program (AST root).
    pub program: Program<'a>,

    /// Number of errors encountered during parsing.
    pub error_count: usize,

    /// Whether parsing was successful (no fatal errors).
    pub is_success: bool,
}

impl Parser {
    /// Create a new parser from source text.
    pub fn new(source_text: String) -> Self {
        Self {
            source_text,
            source_path: None,
        }
    }

    /// Create a new parser from a file path.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        info!("Reading JavaScript file: {}", path.display());

        let source_text = std::fs::read_to_string(path)
            .map_err(|e| DecypherError::io(path, e))?;

        Ok(Self {
            source_text,
            source_path: Some(path.display().to_string()),
        })
    }

    /// Parse the JavaScript code and return the AST.
    pub fn parse<'a>(&'a self, allocator: &'a Allocator) -> Result<ParseResult<'a>> {
        info!("Parsing JavaScript (size: {} bytes)", self.source_text.len());

        // Detect source type (JS, JSX, TS, TSX)
        let source_type = if let Some(ref path) = self.source_path {
            SourceType::from_path(path).unwrap_or_default()
        } else {
            // Default to JavaScript
            SourceType::default()
        };

        debug!("Source type: {:?}", source_type);

        // Parse with Oxc
        let parser_return = OxcParser::new(allocator, &self.source_text, source_type).parse();

        let ParserReturn {
            program,
            errors,
            ..
        } = parser_return;

        let error_count = errors.len();
        let is_success = error_count == 0;

        if is_success {
            info!("Parsing completed successfully");
        } else {
            warn!("Parsing completed with {} errors/warnings", error_count);

            // Log first few errors for debugging
            for (idx, error) in errors.iter().take(5).enumerate() {
                debug!("Error {}: {:?}", idx + 1, error);
            }

            if error_count > 5 {
                debug!("... and {} more errors", error_count - 5);
            }
        }

        Ok(ParseResult {
            program,
            error_count,
            is_success,
        })
    }

    /// Get the source text.
    pub fn source_text(&self) -> &str {
        &self.source_text
    }

    /// Get the source path if available.
    pub fn source_path(&self) -> Option<&str> {
        self.source_path.as_deref()
    }
}

impl<'a> ParseResult<'a> {
    /// Get a reference to the program (AST root).
    pub fn program(&self) -> &Program<'a> {
        &self.program
    }

    /// Check if parsing was successful.
    pub fn is_success(&self) -> bool {
        self.is_success
    }

    /// Get the number of errors.
    pub fn error_count(&self) -> usize {
        self.error_count
    }

    /// Print error count to stderr.
    pub fn print_errors(&self) {
        if self.error_count > 0 {
            eprintln!("Encountered {} parsing errors", self.error_count);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_code() {
        let code = "var x = 1;";
        let parser = Parser::new(code.to_string());
        let allocator = Allocator::default();
        let result = parser.parse(&allocator).unwrap();

        assert!(result.is_success());
        assert_eq!(result.error_count(), 0);
    }

    #[test]
    fn test_parse_function() {
        let code = r#"
            function hello(name) {
                return "Hello, " + name;
            }
        "#;
        let parser = Parser::new(code.to_string());
        let allocator = Allocator::default();
        let result = parser.parse(&allocator).unwrap();

        assert!(result.is_success());
    }

    #[test]
    fn test_parse_es6() {
        let code = r#"
            const arrow = (x) => x * 2;
            const obj = { name: "test", value: 42 };
            const [a, b] = [1, 2];
        "#;
        let parser = Parser::new(code.to_string());
        let allocator = Allocator::default();
        let result = parser.parse(&allocator).unwrap();

        assert!(result.is_success());
    }

    #[test]
    fn test_parse_minified() {
        let code = "var a=1;function b(){return a}var c=b();";
        let parser = Parser::new(code.to_string());
        let allocator = Allocator::default();
        let result = parser.parse(&allocator).unwrap();

        assert!(result.is_success());
    }

    #[test]
    fn test_parse_with_errors() {
        // Intentionally malformed code
        let code = "var x = ;";
        let parser = Parser::new(code.to_string());
        let allocator = Allocator::default();
        let result = parser.parse(&allocator).unwrap();

        assert!(!result.is_success());
        assert!(result.error_count() > 0);
    }

    #[test]
    fn test_source_text_accessor() {
        let code = "var x = 1;";
        let parser = Parser::new(code.to_string());

        assert_eq!(parser.source_text(), code);
    }
}
