//! Code metrics calculation.

use crate::Result;
use oxc_ast::ast::*;
use serde::{Deserialize, Serialize};
use tracing::debug;

/// Code metrics for the analyzed code.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeMetrics {
    /// Total lines of code (estimated).
    pub total_loc: usize,

    /// Number of functions.
    pub function_count: usize,

    /// Number of classes.
    pub class_count: usize,

    /// Number of variables.
    pub variable_count: usize,

    /// Number of imports.
    pub import_count: usize,

    /// Number of exports.
    pub export_count: usize,

    /// Average function length.
    pub avg_function_length: f32,

    /// Longest function length.
    pub max_function_length: usize,

    /// Code-to-comment ratio.
    pub comment_ratio: f32,

    /// Duplication estimate.
    pub estimated_duplication: f32,
}

/// Calculator for code metrics.
pub struct MetricsCalculator<'a> {
    program: &'a Program<'a>,
}

impl<'a> MetricsCalculator<'a> {
    /// Create a new metrics calculator.
    pub fn new(program: &'a Program<'a>) -> Self {
        Self { program }
    }

    /// Calculate code metrics.
    pub fn calculate(&self) -> Result<CodeMetrics> {
        debug!("Calculating code metrics");

        let mut visitor = MetricsVisitor::new();
        visitor.visit_program(self.program);

        let avg_function_length = if visitor.function_count > 0 {
            visitor.total_function_lines as f32 / visitor.function_count as f32
        } else {
            0.0
        };

        let metrics = CodeMetrics {
            total_loc: visitor.estimated_loc,
            function_count: visitor.function_count,
            class_count: visitor.class_count,
            variable_count: visitor.variable_count,
            import_count: visitor.import_count,
            export_count: visitor.export_count,
            avg_function_length,
            max_function_length: visitor.max_function_length,
            comment_ratio: 0.0, // Simplified
            estimated_duplication: 0.0, // Simplified
        };

        debug!(
            "Calculated metrics: {} functions, {} variables",
            metrics.function_count, metrics.variable_count
        );

        Ok(metrics)
    }
}

/// Visitor that calculates metrics.
struct MetricsVisitor {
    estimated_loc: usize,
    function_count: usize,
    class_count: usize,
    variable_count: usize,
    import_count: usize,
    export_count: usize,
    total_function_lines: usize,
    max_function_length: usize,
}

impl MetricsVisitor {
    fn new() -> Self {
        Self {
            estimated_loc: 0,
            function_count: 0,
            class_count: 0,
            variable_count: 0,
            import_count: 0,
            export_count: 0,
            total_function_lines: 0,
            max_function_length: 0,
        }
    }

    fn visit_program(&mut self, program: &Program) {
        for stmt in &program.body {
            self.visit_statement(stmt);
        }
    }

    fn visit_statement(&mut self, stmt: &Statement) {
        self.estimated_loc += 1;

        match stmt {
            Statement::FunctionDeclaration(func) => {
                self.function_count += 1;

                if let Some(ref body) = func.body {
                    let stmt_count = body.statements.len();
                    self.total_function_lines += stmt_count;
                    if stmt_count > self.max_function_length {
                        self.max_function_length = stmt_count;
                    }

                    for stmt in &body.statements {
                        self.visit_statement(stmt);
                    }
                }
            }
            Statement::VariableDeclaration(decl) => {
                self.variable_count += decl.declarations.len();
            }
            Statement::ClassDeclaration(_) => {
                self.class_count += 1;
            }
            Statement::BlockStatement(block) => {
                for stmt in &block.body {
                    self.visit_statement(stmt);
                }
            }
            Statement::IfStatement(if_stmt) => {
                self.visit_statement(&if_stmt.consequent);
                if let Some(ref alt) = if_stmt.alternate {
                    self.visit_statement(alt);
                }
            }
            Statement::ForStatement(for_stmt) => {
                self.visit_statement(&for_stmt.body);
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Parser;
    use oxc_allocator::Allocator;

    #[test]
    fn test_calculate_metrics() {
        let code = r#"
            function foo() {
                return 42;
            }

            function bar(x, y) {
                if (x > y) {
                    return x;
                }
                return y;
            }

            const x = 1;
            const y = 2;
        "#;

        let allocator = Allocator::default();
        let parser = Parser::new(code.to_string());
        let parse_result = parser.parse(&allocator).unwrap();

        let calculator = MetricsCalculator::new(parse_result.program());
        let metrics = calculator.calculate().unwrap();

        assert_eq!(metrics.function_count, 2);
        assert_eq!(metrics.variable_count, 2);
        assert!(metrics.avg_function_length > 0.0);
    }
}
