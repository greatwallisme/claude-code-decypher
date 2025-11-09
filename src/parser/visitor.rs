//! AST visitor utilities for traversing the JavaScript AST.

use oxc_ast::ast::*;
use tracing::debug;

/// Statistics collected during AST traversal.
#[derive(Debug, Default, Clone)]
pub struct AstStats {
    /// Total number of functions (declarations + expressions).
    pub function_count: usize,

    /// Number of variable declarations.
    pub variable_count: usize,

    /// Number of string literals.
    pub string_literal_count: usize,

    /// Number of object expressions.
    pub object_count: usize,

    /// Number of array expressions.
    pub array_count: usize,

    /// Number of call expressions.
    pub call_count: usize,

    /// Number of import statements.
    pub import_count: usize,

    /// Number of export statements.
    pub export_count: usize,

    /// Total number of nodes visited.
    pub total_nodes: usize,

    /// Longest string literal found.
    pub longest_string: usize,

    /// Deepest nesting level.
    pub max_depth: usize,
}

/// Simple AST visitor that collects statistics.
pub struct StatsVisitor {
    stats: AstStats,
    current_depth: usize,
}

impl StatsVisitor {
    /// Create a new statistics visitor.
    pub fn new() -> Self {
        Self {
            stats: AstStats::default(),
            current_depth: 0,
        }
    }

    /// Visit a program and collect statistics.
    pub fn visit_program(&mut self, program: &Program) -> AstStats {
        debug!("Starting AST traversal for statistics");

        // Visit the program body
        for stmt in &program.body {
            self.visit_statement(stmt);
        }

        debug!("AST traversal complete: {:?}", self.stats);
        self.stats.clone()
    }

    fn visit_statement(&mut self, stmt: &Statement) {
        self.stats.total_nodes += 1;
        self.current_depth += 1;

        if self.current_depth > self.stats.max_depth {
            self.stats.max_depth = self.current_depth;
        }

        match stmt {
            Statement::VariableDeclaration(decl) => {
                self.stats.variable_count += decl.declarations.len();
                for declarator in &decl.declarations {
                    if let Some(ref init) = declarator.init {
                        self.visit_expression(init);
                    }
                }
            }
            Statement::FunctionDeclaration(func) => {
                self.stats.function_count += 1;
                // Visit function body
                if let Some(ref body) = func.body {
                    for stmt in &body.statements {
                        self.visit_statement(stmt);
                    }
                }
            }
            Statement::BlockStatement(block) => {
                for stmt in &block.body {
                    self.visit_statement(stmt);
                }
            }
            Statement::ExpressionStatement(expr_stmt) => {
                self.visit_expression(&expr_stmt.expression);
            }
            Statement::IfStatement(if_stmt) => {
                self.visit_expression(&if_stmt.test);
                self.visit_statement(&if_stmt.consequent);
                if let Some(ref alt) = if_stmt.alternate {
                    self.visit_statement(alt);
                }
            }
            Statement::ForStatement(for_stmt) => {
                if let Some(ref init) = for_stmt.init {
                    match init {
                        ForStatementInit::VariableDeclaration(decl) => {
                            self.stats.variable_count += decl.declarations.len();
                        }
                        _ => {
                            // Other expression types in for loop init
                        }
                    }
                }
                self.visit_statement(&for_stmt.body);
            }
            Statement::ReturnStatement(ret) => {
                if let Some(ref arg) = ret.argument {
                    self.visit_expression(arg);
                }
            }
            _ => {}
        }

        self.current_depth -= 1;
    }

    fn visit_expression(&mut self, expr: &Expression) {
        self.stats.total_nodes += 1;

        match expr {
            Expression::StringLiteral(str_lit) => {
                self.stats.string_literal_count += 1;
                let len = str_lit.value.as_str().len();
                if len > self.stats.longest_string {
                    self.stats.longest_string = len;
                }
            }
            Expression::ObjectExpression(obj) => {
                self.stats.object_count += 1;
                for prop in &obj.properties {
                    match prop {
                        ObjectPropertyKind::ObjectProperty(p) => {
                            self.visit_expression(&p.value);
                        }
                        ObjectPropertyKind::SpreadProperty(p) => {
                            self.visit_expression(&p.argument);
                        }
                    }
                }
            }
            Expression::ArrayExpression(arr) => {
                self.stats.array_count += 1;
                for elem in &arr.elements {
                    match elem {
                        ArrayExpressionElement::SpreadElement(spread) => {
                            self.visit_expression(&spread.argument);
                        }
                        _ => {
                            // For other expression types, we could visit recursively
                            // but for stats collection, we just count the array
                        }
                    }
                }
            }
            Expression::CallExpression(_) => {
                self.stats.call_count += 1;
            }
            Expression::FunctionExpression(_) | Expression::ArrowFunctionExpression(_) => {
                self.stats.function_count += 1;
            }
            _ => {}
        }
    }

    /// Get the collected statistics.
    pub fn stats(&self) -> &AstStats {
        &self.stats
    }
}

impl Default for StatsVisitor {
    fn default() -> Self {
        Self::new()
    }
}

impl AstStats {
    /// Print a summary of the statistics.
    pub fn print_summary(&self) {
        println!("AST Statistics:");
        println!("  Total nodes:        {}", self.total_nodes);
        println!("  Functions:          {}", self.function_count);
        println!("  Variables:          {}", self.variable_count);
        println!("  String literals:    {}", self.string_literal_count);
        println!("  Objects:            {}", self.object_count);
        println!("  Arrays:             {}", self.array_count);
        println!("  Function calls:     {}", self.call_count);
        println!("  Imports:            {}", self.import_count);
        println!("  Exports:            {}", self.export_count);
        println!("  Longest string:     {} chars", self.longest_string);
        println!("  Max nesting depth:  {}", self.max_depth);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Parser;
    use oxc_allocator::Allocator;

    #[test]
    fn test_stats_visitor_simple() {
        let code = r#"
            var x = 1;
            function foo() {
                return "hello";
            }
        "#;

        let parser = Parser::new(code.to_string());
        let allocator = Allocator::default();
        let result = parser.parse(&allocator).unwrap();

        let mut visitor = StatsVisitor::new();
        let stats = visitor.visit_program(&result.program);

        // Basic assertions (accounting for AST differences)
        assert!(stats.variable_count >= 1);
        assert!(stats.function_count >= 1);
        assert_eq!(stats.string_literal_count, 1);
    }

    #[test]
    fn test_stats_visitor_complex() {
        let code = r#"
            const obj = { name: "test", value: 42 };
            const arr = [1, 2, 3];
            const result = foo(obj, arr);
        "#;

        let parser = Parser::new(code.to_string());
        let allocator = Allocator::default();
        let result = parser.parse(&allocator).unwrap();

        let mut visitor = StatsVisitor::new();
        let stats = visitor.visit_program(&result.program);

        assert_eq!(stats.variable_count, 3);
        assert_eq!(stats.object_count, 1);
        assert_eq!(stats.array_count, 1);
        assert_eq!(stats.call_count, 1);
    }

    #[test]
    fn test_longest_string() {
        let code = r#"
            const short = "hi";
            const long = "this is a much longer string";
        "#;

        let parser = Parser::new(code.to_string());
        let allocator = Allocator::default();
        let result = parser.parse(&allocator).unwrap();

        let mut visitor = StatsVisitor::new();
        let stats = visitor.visit_program(&result.program);

        assert_eq!(stats.longest_string, "this is a much longer string".len());
    }
}
