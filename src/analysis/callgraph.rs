//! Call graph analysis.

use crate::Result;
use oxc_ast::ast::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use tracing::debug;

/// Call graph representing function call relationships.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallGraph {
    /// Function definitions found.
    pub functions: Vec<FunctionNode>,

    /// Call relationships (caller -> callees).
    pub calls: HashMap<String, Vec<String>>,

    /// Total number of function calls.
    pub total_calls: usize,

    /// Number of unique functions.
    pub unique_functions: usize,
}

/// A function node in the call graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionNode {
    /// Function name.
    pub name: String,

    /// Is this a named function or anonymous?
    pub is_anonymous: bool,

    /// Number of parameters.
    pub param_count: usize,

    /// Calls made by this function.
    pub calls_out: usize,

    /// Times this function is called.
    pub calls_in: usize,
}

/// Builder for call graphs.
pub struct CallGraphBuilder<'a> {
    program: &'a Program<'a>,
}

impl<'a> CallGraphBuilder<'a> {
    /// Create a new call graph builder.
    pub fn new(program: &'a Program<'a>) -> Self {
        Self { program }
    }

    /// Build the call graph.
    pub fn build(&self) -> Result<CallGraph> {
        debug!("Building call graph");

        let mut visitor = CallGraphVisitor::new();
        visitor.visit_program(self.program);

        let call_graph = CallGraph {
            functions: visitor.functions,
            calls: visitor.calls,
            total_calls: visitor.total_calls,
            unique_functions: visitor.unique_functions,
        };

        debug!(
            "Built call graph: {} functions, {} calls",
            call_graph.unique_functions, call_graph.total_calls
        );

        Ok(call_graph)
    }
}

/// Visitor that builds the call graph.
struct CallGraphVisitor {
    functions: Vec<FunctionNode>,
    calls: HashMap<String, Vec<String>>,
    total_calls: usize,
    unique_functions: usize,
    current_function: Option<String>,
    function_names: HashSet<String>,
}

impl CallGraphVisitor {
    fn new() -> Self {
        Self {
            functions: Vec::new(),
            calls: HashMap::new(),
            total_calls: 0,
            unique_functions: 0,
            current_function: None,
            function_names: HashSet::new(),
        }
    }

    fn visit_program(&mut self, program: &Program) {
        for stmt in &program.body {
            self.visit_statement(stmt);
        }

        self.unique_functions = self.function_names.len();
    }

    fn visit_statement(&mut self, stmt: &Statement) {
        match stmt {
            Statement::FunctionDeclaration(func) => {
                let name = func
                    .id
                    .as_ref()
                    .map(|id| id.name.as_str())
                    .unwrap_or("anonymous");

                self.function_names.insert(name.to_string());

                let node = FunctionNode {
                    name: name.to_string(),
                    is_anonymous: func.id.is_none(),
                    param_count: func.params.items.len(),
                    calls_out: 0,
                    calls_in: 0,
                };

                self.functions.push(node);

                let prev_function = self.current_function.clone();
                self.current_function = Some(name.to_string());

                if let Some(ref body) = func.body {
                    for stmt in &body.statements {
                        self.visit_statement(stmt);
                    }
                }

                self.current_function = prev_function;
            }
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
            Statement::ReturnStatement(ret) => {
                if let Some(ref arg) = ret.argument {
                    self.visit_expression(arg);
                }
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
            _ => {}
        }
    }

    fn visit_expression(&mut self, expr: &Expression) {
        match expr {
            Expression::CallExpression(call) => {
                self.total_calls += 1;

                // Try to extract the function name being called
                if let Some(callee_name) = self.extract_callee_name(&call.callee) {
                    if let Some(ref caller) = self.current_function {
                        self.calls
                            .entry(caller.clone())
                            .or_default()
                            .push(callee_name);
                    }
                }

                // Visit arguments
                for arg in &call.arguments {
                    if let Argument::SpreadElement(spread) = arg {
                        self.visit_expression(&spread.argument);
                    }
                }
            }
            Expression::FunctionExpression(_) | Expression::ArrowFunctionExpression(_) => {
                // Anonymous function
                self.function_names.insert("anonymous".to_string());
            }
            _ => {}
        }
    }

    fn extract_callee_name(&self, expr: &Expression) -> Option<String> {
        match expr {
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
    fn test_build_call_graph() {
        let code = r#"
            function foo() {
                return bar();
            }

            function bar() {
                return 42;
            }

            foo();
        "#;

        let allocator = Allocator::default();
        let parser = Parser::new(code.to_string());
        let parse_result = parser.parse(&allocator).unwrap();

        let builder = CallGraphBuilder::new(parse_result.program());
        let graph = builder.build().unwrap();

        assert!(graph.unique_functions >= 2);
        assert!(graph.total_calls >= 1);
    }

    #[test]
    fn test_function_detection() {
        let code = r#"
            function named() {}
            const arrow = () => {};
            const func = function() {};
        "#;

        let allocator = Allocator::default();
        let parser = Parser::new(code.to_string());
        let parse_result = parser.parse(&allocator).unwrap();

        let builder = CallGraphBuilder::new(parse_result.program());
        let graph = builder.build().unwrap();

        assert!(graph.unique_functions >= 1);
    }
}
