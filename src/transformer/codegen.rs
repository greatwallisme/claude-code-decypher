//! Code generation and beautification using oxc_codegen.

use crate::Result;
use oxc_allocator::Allocator;
use oxc_ast::ast::Program;
use oxc_codegen::{Codegen, CodegenOptions};
use tracing::debug;

/// Code generator that produces beautified JavaScript.
pub struct CodeGenerator<'a> {
    allocator: &'a Allocator,
    program: &'a Program<'a>,
}

impl<'a> CodeGenerator<'a> {
    /// Create a new code generator.
    pub fn new(allocator: &'a Allocator, program: &'a Program<'a>) -> Self {
        Self { allocator, program }
    }

    /// Generate beautified JavaScript code.
    pub fn generate(&self) -> Result<String> {
        debug!("Generating beautified code");

        let codegen = Codegen::new();
        let generated = codegen.build(self.program);

        debug!("Generated {} bytes of code", generated.code.len());

        Ok(generated.code)
    }

    /// Generate code with custom options.
    pub fn generate_with_options(&self, _options: CodegenOptions) -> Result<String> {
        debug!("Generating code with custom options");

        let codegen = Codegen::new();
        let generated = codegen.build(self.program);

        Ok(generated.code)
    }
}

/// Post-process generated code for better readability.
pub fn beautify_code(code: &str) -> String {
    let mut result = code.to_string();

    // Add newlines after semicolons for better readability
    result = result.replace(";var ", ";\nvar ");
    result = result.replace(";function ", ";\n\nfunction ");
    result = result.replace(";const ", ";\nconst ");
    result = result.replace(";let ", ";\nlet ");

    // Add spacing around operators
    result = result.replace("=function", " = function");
    result = result.replace("=>", " => ");

    // Add newlines after opening braces
    result = result.replace("{var ", "{\n  var ");
    result = result.replace("{const ", "{\n  const ");
    result = result.replace("{let ", "{\n  let ");

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Parser;
    use oxc_allocator::Allocator;

    #[test]
    fn test_generate_code() {
        let code = "var x=1;function f(){return x}";

        let allocator = Allocator::default();
        let parser = Parser::new(code.to_string());
        let parse_result = parser.parse(&allocator).unwrap();

        let generator = CodeGenerator::new(&allocator, parse_result.program());
        let generated = generator.generate().unwrap();

        // Generated code should be valid
        assert!(!generated.is_empty());
        assert!(generated.contains("var x"));
        assert!(generated.contains("function f"));
    }

    #[test]
    fn test_beautify_code() {
        let code = "var x=1;var y=2;function f(){return x+y}";

        let beautified = beautify_code(code);

        // Should have newlines
        assert!(beautified.contains('\n'));
        assert!(beautified.contains(";\nvar y"));
    }

    #[test]
    fn test_generate_complex_code() {
        let code = r#"
            const obj = { name: "test", value: 42 };
            const arrow = (x) => x * 2;
            function process(data) {
                return data.map(arrow);
            }
        "#;

        let allocator = Allocator::default();
        let parser = Parser::new(code.to_string());
        let parse_result = parser.parse(&allocator).unwrap();

        let generator = CodeGenerator::new(&allocator, parse_result.program());
        let generated = generator.generate().unwrap();

        assert!(!generated.is_empty());
        assert!(generated.contains("const obj"));
        assert!(generated.contains("function process"));
    }
}
