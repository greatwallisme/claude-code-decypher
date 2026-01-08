//! Extract function information from AST.

use crate::Result;
use oxc_ast::ast::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::debug;

/// Source span information (serializable).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpanInfo {
    /// Start byte offset.
    pub start: usize,

    /// End byte offset.
    pub end: usize,
}

impl From<oxc_span::Span> for SpanInfo {
    fn from(span: oxc_span::Span) -> Self {
        Self {
            start: span.start as usize,
            end: span.end as usize,
        }
    }
}

/// Information extracted for each function.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionInfo {
    /// Function name (or generated name for anonymous functions).
    pub name: String,

    /// Source span (start and end positions).
    pub span: SpanInfo,

    /// Number of parameters.
    pub param_count: usize,

    /// Is this an anonymous function?
    pub is_anonymous: bool,

    /// Functions this function calls (dependencies).
    pub dependencies: Vec<String>,

    /// Variables accessed from outer scope (closures).
    pub outer_variables: Vec<String>,

    /// Module this function is assigned to (determined later).
    pub assigned_module: Option<String>,

    /// Should this function be exported?
    pub is_exported: bool,
}

/// Extracts function information from AST.
///
/// TODO: This is a simplified implementation that only extracts top-level
/// function declarations. A complete implementation would also handle:
/// - Function expressions assigned to variables
/// - Arrow functions
/// - Class methods
/// - Nested functions
/// - Functions in object properties
///
/// For now, this is sufficient to demonstrate the pipeline architecture.
pub struct FunctionExtractor<'a> {
    program: &'a Program<'a>,
    anonymous_counter: usize,
    functions: HashMap<String, FunctionInfo>,
    current_function: Option<String>,
}

impl<'a> FunctionExtractor<'a> {
    /// Create a new function extractor.
    pub fn new(program: &'a Program<'a>) -> Self {
        Self {
            program,
            anonymous_counter: 0,
            functions: HashMap::new(),
            current_function: None,
        }
    }

    /// Extract all functions from the AST.
    ///
    /// TODO: Currently only extracts FunctionDeclaration nodes.
    /// Future enhancement: Extract FunctionExpression and ArrowFunctionExpression
    pub fn extract(mut self) -> Result<Vec<FunctionInfo>> {
        debug!("Extracting functions from AST");

        self.visit_program(self.program);

        let functions: Vec<FunctionInfo> = self.functions.into_values().collect();

        debug!("Extracted {} functions", functions.len());

        Ok(functions)
    }

    fn visit_program(&mut self, program: &Program) {
        for stmt in &program.body {
            self.visit_statement(stmt);
        }
    }

    fn visit_statement(&mut self, stmt: &Statement) {
        match stmt {
            // Extract function declarations
            Statement::FunctionDeclaration(func) => {
                let name = if let Some(ref id) = func.id {
                    id.name.to_string()
                } else {
                    let generated = format!("anonymous_func_{}", self.anonymous_counter);
                    self.anonymous_counter += 1;
                    generated
                };

                let func_info = FunctionInfo {
                    name: name.clone(),
                    span: func.span.into(),
                    param_count: func.params.items.len(),
                    is_anonymous: func.id.is_none(),
                    dependencies: Vec::new(),
                    outer_variables: Vec::new(),
                    assigned_module: None,
                    is_exported: false,
                };

                self.functions.insert(name.clone(), func_info);

                let prev = self.current_function.replace(name);

                // Visit function body to collect call dependencies
                if let Some(ref body) = func.body {
                    for stmt in &body.statements {
                        self.visit_statement(stmt);
                    }
                }

                self.current_function = prev;
            }

            // Extract functions from variable declarations
            // TODO: Implement proper function expression extraction
            // For now, just visit the initialization expression to collect call dependencies
            Statement::VariableDeclaration(decl) => {
                for declarator in &decl.declarations {
                    if let Some(ref init) = declarator.init {
                        self.visit_expression(init);
                    }
                }
            }

            Statement::ExpressionStatement(expr_stmt) => {
                self.visit_expression(&expr_stmt.expression);
            }

            Statement::BlockStatement(block) => {
                for stmt in &block.body {
                    self.visit_statement(stmt);
                }
            }

            Statement::IfStatement(if_stmt) => {
                self.visit_expression(&if_stmt.test);
                self.visit_statement(&if_stmt.consequent);
                if let Some(ref alt) = if_stmt.alternate {
                    self.visit_statement(alt);
                }
            }

            Statement::ReturnStatement(ret) => {
                if let Some(ref arg) = ret.argument {
                    self.visit_expression(arg);
                }
            }

            Statement::ForStatement(for_stmt) => {
                // TODO: Handle ForStatementInit properly
                // ForStatementInit can be VariableDeclaration or Expression
                // Skip for now to avoid complex pattern matching
                if let Some(ref test) = for_stmt.test {
                    self.visit_expression(test);
                }
                if let Some(ref update) = for_stmt.update {
                    self.visit_expression(update);
                }
                self.visit_statement(&for_stmt.body);
            }

            Statement::WhileStatement(while_stmt) => {
                self.visit_expression(&while_stmt.test);
                self.visit_statement(&while_stmt.body);
            }

            _ => {
                // Ignore other statement types
            }
        }
    }

    fn visit_expression(&mut self, expr: &Expression) {
        match expr {
            Expression::CallExpression(call) => {
                if let Some(callee_name) = self.extract_callee_name(&call.callee) {
                    if let Some(ref current) = self.current_function {
                        if let Some(func_info) = self.functions.get_mut(current) {
                            if !func_info.dependencies.contains(&callee_name) {
                                func_info.dependencies.push(callee_name);
                            }
                        }
                    }
                }

                // Visit arguments to find nested calls
                for arg in &call.arguments {
                    match arg {
                        Argument::SpreadElement(spread) => {
                            self.visit_expression(&spread.argument);
                        }
                        _ => {}
                    }
                }
            }

            // TODO: Extract function expressions and arrow functions
            // Expression::FunctionExpression(func) => { ... }
            // Expression::ArrowFunctionExpression(arrow) => { ... }

            _ => {
                // Ignore other expression types
            }
        }
    }

    fn extract_callee_name(&self, callee: &Expression) -> Option<String> {
        match callee {
            Expression::Identifier(id) => Some(id.name.to_string()),
            Expression::StaticMemberExpression(member) => {
                Some(member.property.name.to_string())
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Parser;
    use oxc_allocator::Allocator;

    #[test]
    fn test_extract_simple_functions() {
        let code = r#"
            function foo() {
                return bar();
            }

            function bar() {
                return 42;
            }
        "#;

        let allocator = Allocator::default();
        let parser = Parser::new(code.to_string());
        let parse_result = parser.parse(&allocator).unwrap();

        let extractor = FunctionExtractor::new(parse_result.program());
        let functions = extractor.extract().unwrap();

        assert_eq!(functions.len(), 2);

        let foo = functions.iter().find(|f| f.name == "foo").unwrap();
        assert_eq!(foo.dependencies.len(), 1);
        assert_eq!(foo.dependencies[0], "bar");
    }

    #[test]
    fn test_extract_no_functions() {
        let code = r#"
            const x = 42;
            const y = 100;
        "#;

        let allocator = Allocator::default();
        let parser = Parser::new(code.to_string());
        let parse_result = parser.parse(&allocator).unwrap();

        let extractor = FunctionExtractor::new(parse_result.program());
        let functions = extractor.extract().unwrap();

        assert_eq!(functions.len(), 0);
    }

    #[test]
    fn test_extract_nested_calls() {
        let code = r#"
            function foo() {
                return bar(baz());
            }

            function bar(x) {
                return x;
            }

            function baz() {
                return 1;
            }
        "#;

        let allocator = Allocator::default();
        let parser = Parser::new(code.to_string());
        let parse_result = parser.parse(&allocator).unwrap();

        let extractor = FunctionExtractor::new(parse_result.program());
        let functions = extractor.extract().unwrap();

        assert_eq!(functions.len(), 3);

        let foo = functions.iter().find(|f| f.name == "foo").unwrap();
        assert!(foo.dependencies.contains(&"bar".to_string()));
    }

    #[test]
    fn test_anonymous_function_declarations() {
        let code = r#"
            function() {
                return 1;
            }
        "#;

        let allocator = Allocator::default();
        let parser = Parser::new(code.to_string());
        let parse_result = parser.parse(&allocator).unwrap();

        let extractor = FunctionExtractor::new(parse_result.program());
        let functions = extractor.extract().unwrap();

        assert_eq!(functions.len(), 1);
        assert!(functions[0].is_anonymous);
        assert!(functions[0].name.starts_with("anonymous_func_"));
    }
}
