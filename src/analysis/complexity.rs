//! Complexity metrics calculation.

use crate::Result;
use oxc_ast::ast::*;
use serde::{Deserialize, Serialize};
use tracing::debug;

/// Complexity metrics for the code.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplexityMetrics {
    /// Cyclomatic complexity (average).
    pub avg_cyclomatic: f32,

    /// Maximum cyclomatic complexity.
    pub max_cyclomatic: usize,

    /// Function with highest complexity.
    pub most_complex_function: String,

    /// Total decision points.
    pub total_decision_points: usize,

    /// Average nesting depth.
    pub avg_nesting_depth: f32,

    /// Maximum nesting depth.
    pub max_nesting_depth: usize,

    /// Per-function complexity breakdown.
    pub function_complexity: Vec<FunctionComplexity>,
}

/// Complexity for a single function.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionComplexity {
    /// Function name.
    pub name: String,

    /// Cyclomatic complexity.
    pub cyclomatic: usize,

    /// Nesting depth.
    pub nesting_depth: usize,

    /// Number of parameters.
    pub param_count: usize,

    /// Number of statements.
    pub statement_count: usize,
}

/// Calculator for complexity metrics.
pub struct ComplexityCalculator<'a> {
    program: &'a Program<'a>,
}

impl<'a> ComplexityCalculator<'a> {
    /// Create a new complexity calculator.
    pub fn new(program: &'a Program<'a>) -> Self {
        Self { program }
    }

    /// Calculate complexity metrics.
    pub fn calculate(&self) -> Result<ComplexityMetrics> {
        debug!("Calculating complexity metrics");

        let mut visitor = ComplexityVisitor::new();
        visitor.visit_program(self.program);

        let avg_cyclomatic = if visitor.function_complexity.is_empty() {
            0.0
        } else {
            visitor.total_cyclomatic as f32 / visitor.function_complexity.len() as f32
        };

        let avg_nesting_depth = if visitor.function_complexity.is_empty() {
            0.0
        } else {
            visitor.total_nesting_depth as f32 / visitor.function_complexity.len() as f32
        };

        let max_cyclomatic = visitor
            .function_complexity
            .iter()
            .map(|f| f.cyclomatic)
            .max()
            .unwrap_or(0);

        let most_complex_function = visitor
            .function_complexity
            .iter()
            .max_by_key(|f| f.cyclomatic)
            .map(|f| f.name.clone())
            .unwrap_or_else(|| "none".to_string());

        let metrics = ComplexityMetrics {
            avg_cyclomatic,
            max_cyclomatic,
            most_complex_function,
            total_decision_points: visitor.total_decision_points,
            avg_nesting_depth,
            max_nesting_depth: visitor.max_nesting_depth,
            function_complexity: visitor.function_complexity,
        };

        debug!(
            "Calculated complexity: avg={:.2}, max={}",
            metrics.avg_cyclomatic, metrics.max_cyclomatic
        );

        Ok(metrics)
    }
}

/// Visitor that calculates complexity.
struct ComplexityVisitor {
    function_complexity: Vec<FunctionComplexity>,
    total_cyclomatic: usize,
    total_decision_points: usize,
    total_nesting_depth: usize,
    max_nesting_depth: usize,
    current_depth: usize,
}

impl ComplexityVisitor {
    fn new() -> Self {
        Self {
            function_complexity: Vec::new(),
            total_cyclomatic: 0,
            total_decision_points: 0,
            total_nesting_depth: 0,
            max_nesting_depth: 0,
            current_depth: 0,
        }
    }

    fn visit_program(&mut self, program: &Program) {
        for stmt in &program.body {
            self.visit_statement(stmt);
        }
    }

    fn visit_statement(&mut self, stmt: &Statement) {
        self.current_depth += 1;
        if self.current_depth > self.max_nesting_depth {
            self.max_nesting_depth = self.current_depth;
        }

        match stmt {
            Statement::FunctionDeclaration(func) => {
                let name = func
                    .id
                    .as_ref()
                    .map(|id| id.name.to_string())
                    .unwrap_or_else(|| "anonymous".to_string());

                let mut cyclomatic = 1; // Base complexity
                let mut statement_count = 0;

                if let Some(ref body) = func.body {
                    let prev_depth = self.current_depth;

                    for stmt in &body.statements {
                        statement_count += 1;

                        // Count decision points
                        cyclomatic += match stmt {
                            Statement::IfStatement(_) => 1,
                            Statement::ForStatement(_) => 1,
                            Statement::WhileStatement(_) => 1,
                            Statement::DoWhileStatement(_) => 1,
                            Statement::SwitchStatement(switch) => switch.cases.len(),
                            _ => 0,
                        };

                        self.visit_statement(stmt);
                    }

                    let nesting_depth = self.current_depth - prev_depth;
                    self.total_nesting_depth += nesting_depth;

                    self.function_complexity.push(FunctionComplexity {
                        name,
                        cyclomatic,
                        nesting_depth,
                        param_count: func.params.items.len(),
                        statement_count,
                    });

                    self.total_cyclomatic += cyclomatic;
                }
            }
            Statement::IfStatement(if_stmt) => {
                self.total_decision_points += 1;
                self.visit_statement(&if_stmt.consequent);
                if let Some(ref alt) = if_stmt.alternate {
                    self.visit_statement(alt);
                }
            }
            Statement::ForStatement(for_stmt) => {
                self.total_decision_points += 1;
                self.visit_statement(&for_stmt.body);
            }
            Statement::WhileStatement(while_stmt) => {
                self.total_decision_points += 1;
                self.visit_statement(&while_stmt.body);
            }
            Statement::BlockStatement(block) => {
                for stmt in &block.body {
                    self.visit_statement(stmt);
                }
            }
            Statement::ExpressionStatement(expr_stmt) => {
                self.visit_expression(&expr_stmt.expression);
            }
            _ => {}
        }

        self.current_depth -= 1;
    }

    fn visit_expression(&mut self, expr: &Expression) {
        match expr {
            Expression::ConditionalExpression(_) => {
                self.total_decision_points += 1;
            }
            Expression::LogicalExpression(_) => {
                self.total_decision_points += 1;
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
    fn test_calculate_complexity() {
        let code = r#"
            function simple() {
                return 42;
            }

            function complex(x) {
                if (x > 0) {
                    for (let i = 0; i < x; i++) {
                        if (i % 2 === 0) {
                            console.log(i);
                        }
                    }
                }
            }
        "#;

        let allocator = Allocator::default();
        let parser = Parser::new(code.to_string());
        let parse_result = parser.parse(&allocator).unwrap();

        let calculator = ComplexityCalculator::new(parse_result.program());
        let metrics = calculator.calculate().unwrap();

        assert!(metrics.avg_cyclomatic >= 1.0);
        assert!(metrics.max_cyclomatic >= 1);
        assert!(metrics.total_decision_points >= 2); // if + for + nested if
    }
}
