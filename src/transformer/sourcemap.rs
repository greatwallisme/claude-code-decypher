//! Source map generation for transformed code.

use crate::Result;
use oxc_allocator::Allocator;
use oxc_ast::ast::Program;
use oxc_codegen::{Codegen, CodegenOptions};
use tracing::debug;

/// Source map generator.
pub struct SourceMapGenerator<'a> {
    allocator: &'a Allocator,
    program: &'a Program<'a>,
    source_name: String,
}

impl<'a> SourceMapGenerator<'a> {
    /// Create a new source map generator.
    pub fn new(allocator: &'a Allocator, program: &'a Program<'a>, source_name: String) -> Self {
        Self {
            allocator,
            program,
            source_name,
        }
    }

    /// Generate code with source map.
    pub fn generate_with_sourcemap(&self) -> Result<(String, String)> {
        debug!("Generating code with source map");

        let codegen = Codegen::new();
        let result = codegen.build(self.program);

        // For now, return empty source map as oxc_sourcemap integration is complex
        // This is a placeholder for future enhancement
        let sourcemap = self.generate_sourcemap_stub();

        Ok((result.code, sourcemap))
    }

    /// Generate a stub source map.
    fn generate_sourcemap_stub(&self) -> String {
        format!(
            r#"{{"version":3,"sources":["{}"],"names":[],"mappings":""}}"#,
            self.source_name
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Parser;
    use oxc_allocator::Allocator;

    #[test]
    fn test_generate_with_sourcemap() {
        let code = "var x = 1;";
        let allocator = Allocator::default();
        let parser = Parser::new(code.to_string());
        let parse_result = parser.parse(&allocator).unwrap();

        let generator = SourceMapGenerator::new(
            &allocator,
            parse_result.program(),
            "input.js".to_string(),
        );

        let (code, sourcemap) = generator.generate_with_sourcemap().unwrap();

        assert!(!code.is_empty());
        assert!(sourcemap.contains("version"));
    }
}
