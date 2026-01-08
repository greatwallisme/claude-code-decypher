//! Assemble final module code with imports, exports, and function code.

use crate::Result;
use oxc_allocator::Allocator;
use oxc_ast::ast::Program;
use oxc_codegen::CodeGenerator;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::debug;

use crate::transformer::{
    function_extractor::FunctionInfo,
    import_generator::{ExportInfo, ImportStatement},
};

/// Complete module with generated code.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleCode {
    /// Module name.
    pub name: String,

    /// Imports for this module.
    pub imports: Vec<ImportStatement>,

    /// Exports from this module.
    pub exports: ExportInfo,

    /// Functions in this module.
    pub functions: Vec<FunctionInfo>,

    /// Generated code.
    pub code: String,

    /// Line count.
    pub line_count: usize,
}

/// Assembles module code from components.
pub struct CodeAssembler<'a> {
    allocator: &'a Allocator,
    program: &'a Program<'a>,
    source_code: &'a str,
}

impl<'a> CodeAssembler<'a> {
    /// Create a new code assembler.
    pub fn new(allocator: &'a Allocator, program: &'a Program<'a>, source_code: &'a str) -> Self {
        Self {
            allocator,
            program,
            source_code,
        }
    }

    /// Assemble code for all modules.
    pub fn assemble_modules(
        &self,
        module_functions: &HashMap<String, Vec<FunctionInfo>>,
        module_imports: &HashMap<String, Vec<ImportStatement>>,
        module_exports: &HashMap<String, ExportInfo>,
    ) -> Result<Vec<ModuleCode>> {
        debug!("Assembling code for {} modules", module_functions.len());

        let mut modules: Vec<ModuleCode> = Vec::new();

        for (module_name, functions) in module_functions {
            let imports = module_imports.get(module_name).cloned().unwrap_or_default();
            let exports = module_exports.get(module_name).cloned().unwrap_or_else(|| ExportInfo {
                exports: vec![],
                re_exports: vec![],
            });

            let code = self.assemble_module_code(functions, &imports, &exports)?;

            let line_count = code.lines().count();

            modules.push(ModuleCode {
                name: module_name.clone(),
                imports,
                exports,
                functions: functions.clone(),
                code,
                line_count,
            });
        }

        debug!("Assembled {} modules", modules.len());

        Ok(modules)
    }

    /// Assemble code for a single module.
    fn assemble_module_code(
        &self,
        functions: &[FunctionInfo],
        imports: &[ImportStatement],
        exports: &ExportInfo,
    ) -> Result<String> {
        let mut code = String::new();

        // Add header comment
        code.push_str(&format!("// Module: auto-generated\n\n"));

        // Add imports
        if !imports.is_empty() {
            for stmt in imports {
                if stmt.is_dynamic {
                    for import in &stmt.imports {
                        code.push_str(&format!("const {} = await import('{}');\n", import, stmt.module));
                    }
                } else {
                    if stmt.imports.len() == 1 {
                        code.push_str(&format!(
                            "import {{ {} }} from '{}';\n",
                            stmt.imports[0], stmt.module
                        ));
                    } else {
                        code.push_str(&format!(
                            "import {{\n  {},\n}} from '{}';\n",
                            stmt.imports.join(",\n  "),
                            stmt.module
                        ));
                    }
                }
            }
            code.push('\n');
        }

        // Add exports
        if !exports.exports.is_empty() {
            if exports.exports.len() == 1 {
                code.push_str(&format!("export {{ {} }};\n\n", exports.exports[0]));
            } else {
                code.push_str(&format!(
                    "export {{\n  {},\n}};\n\n",
                    exports.exports.join(",\n  ")
                ));
            }
        }

        // Extract function code from source
        for func in functions {
            let func_code = self.extract_function_code(func)?;
            code.push_str(&func_code);
            code.push('\n');
        }

        Ok(code)
    }

    /// Extract function code from source using span information.
    fn extract_function_code(&self, func: &FunctionInfo) -> Result<String> {
        let start = func.span.start;
        let end = func.span.end;

        if start >= self.source_code.len() || end > self.source_code.len() {
            return Ok(format!("// Function {}: Unable to extract code (invalid span)\n", func.name));
        }

        let code = &self.source_code[start..end];

        // Add function header comment
        let mut result = format!("// === Function: {} ===\n", func.name);
        result.push_str(code);
        result.push('\n');

        Ok(result)
    }

    /// Generate beautified code using Oxc codegen.
    pub fn beautify(&self) -> Result<String> {
        debug!("Generating beautified code");

        let codegen = CodeGenerator::new().build(self.program);
        let code = codegen.code;

        debug!("Generated {} characters of beautified code", code.len());

        Ok(code)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transformer::{
        function_extractor::SpanInfo,
        import_generator::{ExportInfo, ImportStatement},
    };

    #[test]
    fn test_assemble_module_code() {
        let source_code = r#"
            function foo() {
                return 42;
            }
        "#;

        let allocator = Allocator::default();
        let parser = crate::parser::Parser::new(source_code.to_string());
        let parse_result = parser.parse(&allocator).unwrap();
        let program = parse_result.program();

        let assembler = CodeAssembler::new(&allocator, program, source_code);

        let functions = vec![FunctionInfo {
            name: "foo".to_string(),
            span: SpanInfo { start: 13, end: 40 },
            param_count: 0,
            is_anonymous: false,
            dependencies: vec![],
            outer_variables: vec![],
            assigned_module: None,
            is_exported: false,
        }];

        let imports = vec![];
        let exports = ExportInfo {
            exports: vec!["foo".to_string()],
            re_exports: vec![],
        };

        let code = assembler.assemble_module_code(&functions, &imports, &exports).unwrap();

        assert!(code.contains("foo"));
        assert!(code.contains("export"));
    }

    #[test]
    fn test_extract_function_code() {
        let source_code = "function test() { return 1; }";

        let allocator = Allocator::default();
        let parser = crate::parser::Parser::new(source_code.to_string());
        let parse_result = parser.parse(&allocator).unwrap();
        let program = parse_result.program();

        let assembler = CodeAssembler::new(&allocator, program, source_code);

        let func = FunctionInfo {
            name: "test".to_string(),
            span: SpanInfo { start: 0, end: source_code.len() },
            param_count: 0,
            is_anonymous: false,
            dependencies: vec![],
            outer_variables: vec![],
            assigned_module: None,
            is_exported: false,
        };

        let code = assembler.extract_function_code(&func).unwrap();

        assert!(code.contains("test"));
        assert!(code.contains("return 1"));
    }
}
