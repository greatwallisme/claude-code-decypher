//! Symbol table for resolving variable references.

use oxc_ast::ast::*;
use oxc_span::Span;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use tracing::{debug, trace};

/// Value that a symbol can hold.
#[derive(Debug, Clone)]
pub enum SymbolValue {
    /// Simple string value.
    String(String),
    /// Template literal value (resolved as much as possible).
    TemplateLiteral(String),
    /// Numeric value.
    Number(f64),
    /// Boolean value.
    Boolean(bool),
    /// Reference to another symbol.
    ObjectRef(String),
    /// JSON-serializable schema object.
    Schema(JsonValue),
    /// Unknown/unresolved value.
    Unknown,
}

/// A lazy initialization block containing variable assignments.
#[derive(Debug, Clone)]
pub struct LazyInitBlock {
    /// Assignments within this block: (variable_name, value).
    pub assignments: Vec<(String, SymbolValue)>,
    /// Span of the lazy_init call.
    pub span: Span,
}

/// Symbol table that maps variable names to their values.
pub struct SymbolTable<'a> {
    /// Map of symbol name to value.
    pub symbols: HashMap<String, SymbolValue>,
    /// Lazy init blocks found during parsing.
    pub lazy_blocks: Vec<LazyInitBlock>,
    /// The program AST.
    program: &'a Program<'a>,
}

impl<'a> SymbolTable<'a> {
    /// Create a new symbol table.
    pub fn new(program: &'a Program<'a>) -> Self {
        let mut table = Self {
            symbols: HashMap::new(),
            lazy_blocks: Vec::new(),
            program,
        };
        table.build();
        table
    }

    /// Build the symbol table by traversing the AST.
    fn build(&mut self) {
        // Pass 1: Extract top-level declarations (variables AND functions)
        for stmt in &self.program.body {
            self.visit_statement(stmt);
        }

        debug!("Pass 1: Extracted {} symbols and {} lazy blocks",
               self.symbols.len(), self.lazy_blocks.len());

        // Pass 2: Process lazy_init blocks
        self.process_lazy_init_blocks();

        // Pass 3: Resolve references
        self.resolve_references();

        debug!("Built symbol table with {} resolved symbols", self.symbols.len());
    }

    /// Visit a statement to collect variable declarations and function declarations.
    fn visit_statement(&mut self, stmt: &Statement) {
        match stmt {
            Statement::VariableDeclaration(decl) => {
                self.visit_variable_declaration(decl);
            }
            Statement::FunctionDeclaration(func) => {
                self.visit_function_declaration(func);
            }
            _ => {}
        }
    }

    /// Visit function declarations and extract their return values.
    fn visit_function_declaration(&mut self, func: &Function) {
        if let Some(id) = &func.id {
            let func_name = id.name.as_str().to_string();

            if let Some(body) = &func.body {
                // Look for return statements with template literals
                for stmt in &body.statements {
                    if let Statement::ReturnStatement(ret) = stmt {
                        if let Some(arg) = &ret.argument {
                            let value = self.extract_value(arg);
                            trace!("Function {} returns {:?}", func_name, value);
                            self.symbols.insert(func_name.clone(), value);
                            break; // Only take first return
                        }
                    }
                }
            }
        }
    }

    /// Visit variable declarations and extract values.
    fn visit_variable_declaration(&mut self, decl: &VariableDeclaration) {
        for declarator in &decl.declarations {
            // Handle simple identifier patterns
            if let BindingPatternKind::BindingIdentifier(id) = &declarator.id.kind {
                let var_name = id.name.as_str().to_string();

                if let Some(ref init) = declarator.init {
                    let value = self.extract_value(init);
                    trace!("Variable declaration: {} = {:?}", var_name, value);
                    self.symbols.insert(var_name, value);
                }
            }
        }
    }

    /// Extract value from any expression.
    fn extract_value(&mut self, expr: &Expression) -> SymbolValue {
        match expr {
            Expression::StringLiteral(s) => {
                SymbolValue::String(s.value.as_str().to_string())
            }
            Expression::TemplateLiteral(tmpl) => {
                let mut content = String::new();
                for (i, quasi) in tmpl.quasis.iter().enumerate() {
                    content.push_str(quasi.value.raw.as_str());
                    if i < tmpl.expressions.len() {
                        // For now, use placeholder - will resolve later
                        content.push_str("${...}");
                    }
                }
                SymbolValue::TemplateLiteral(content)
            }
            Expression::NumericLiteral(n) => {
                SymbolValue::Number(n.value)
            }
            Expression::BooleanLiteral(b) => {
                SymbolValue::Boolean(b.value)
            }
            Expression::Identifier(id) => {
                SymbolValue::ObjectRef(id.name.as_str().to_string())
            }
            Expression::CallExpression(call) => {
                // Check if this is a lazy_init call
                if self.is_lazy_init_call(call) {
                    self.extract_lazy_init_block(call);
                }
                SymbolValue::Unknown
            }
            _ => SymbolValue::Unknown,
        }
    }

    /// Check if a call is to lazy_init (or its minified equivalent).
    fn is_lazy_init_call(&self, call: &CallExpression) -> bool {
        if let Expression::Identifier(id) = &call.callee {
            let name = id.name.as_str();
            // Match: lazy_init, T, or any single uppercase letter (common minification)
            name == "lazy_init" || name == "T" || (name.len() == 1 && name.chars().next().unwrap().is_uppercase())
        } else {
            false
        }
    }

    /// Extract assignments from lazy_init block.
    fn extract_lazy_init_block(&mut self, call: &CallExpression) {
        if let Some(arg) = call.arguments.first() {
            if let Argument::ArrowFunctionExpression(arrow) = arg {
                let assignments = self.extract_assignments_from_function_body(&arrow.body);

                if !assignments.is_empty() {
                    trace!("Extracted {} assignments from lazy_init block", assignments.len());
                    self.lazy_blocks.push(LazyInitBlock {
                        assignments,
                        span: arrow.span,
                    });
                }
            }
        }
    }

    /// Extract variable assignments from a function body.
    fn extract_assignments_from_function_body(&mut self, body: &FunctionBody) -> Vec<(String, SymbolValue)> {
        let mut assignments = Vec::new();

        for stmt in &body.statements {
            if let Statement::ExpressionStatement(expr_stmt) = stmt {
                if let Expression::AssignmentExpression(assign) = &expr_stmt.expression {
                    // Extract: variableName = value
                    if let AssignmentTarget::AssignmentTargetIdentifier(id) = &assign.left {
                        let name = id.name.as_str().to_string();
                        let value = self.extract_value(&assign.right);
                        trace!("Assignment in lazy_init: {} = {:?}", name, value);
                        assignments.push((name, value));
                    }
                }
            }
        }

        assignments
    }

    /// Process lazy_init blocks and merge into symbol table.
    fn process_lazy_init_blocks(&mut self) {
        for block in &self.lazy_blocks {
            for (name, value) in &block.assignments {
                self.symbols.insert(name.clone(), value.clone());
            }
        }
    }

    /// Resolve indirect references (variable references to other variables).
    fn resolve_references(&mut self) {
        let max_iterations = 10;

        for iteration in 0..max_iterations {
            let mut changed = false;
            let symbols_copy = self.symbols.clone();

            for (_name, value) in self.symbols.iter_mut() {
                if let SymbolValue::ObjectRef(ref_name) = value {
                    if let Some(resolved) = symbols_copy.get(ref_name) {
                        if !matches!(resolved, SymbolValue::ObjectRef(_)) {
                            *value = resolved.clone();
                            changed = true;
                        }
                    }
                }
            }

            if !changed {
                trace!("Reference resolution completed in {} iterations", iteration + 1);
                break;
            }
        }
    }

    /// Get a symbol's string value (legacy compatibility).
    pub fn get(&self, name: &str) -> Option<&String> {
        match self.symbols.get(name)? {
            SymbolValue::String(s) | SymbolValue::TemplateLiteral(s) => Some(s),
            _ => None,
        }
    }

    /// Get a symbol's value as SymbolValue.
    pub fn get_value(&self, name: &str) -> Option<&SymbolValue> {
        self.symbols.get(name)
    }

    /// Get a symbol's string value, resolving references if needed.
    pub fn get_string_value(&self, name: &str) -> Option<String> {
        match self.symbols.get(name)? {
            SymbolValue::String(s) => Some(s.clone()),
            SymbolValue::TemplateLiteral(s) => Some(s.clone()),
            SymbolValue::ObjectRef(ref_name) => {
                // Try to resolve one level
                self.get_string_value(ref_name)
            }
            _ => None,
        }
    }

    /// Get a schema value.
    pub fn get_schema(&self, name: &str) -> Option<JsonValue> {
        match self.symbols.get(name)? {
            SymbolValue::Schema(schema) => Some(schema.clone()),
            _ => None,
        }
    }

    /// Resolve template literal with variable substitution (legacy compatibility).
    pub fn resolve_template(&self, template: &str) -> String {
        let mut result = template.to_string();
        let re = regex::Regex::new(r"\$\{([a-zA-Z0-9_]+)\}").unwrap();

        loop {
            let mut changed = false;

            result = re.replace_all(&result, |caps: &regex::Captures| {
                let var_name = &caps[1];
                if let Some(value) = self.get_string_value(var_name) {
                    changed = true;
                    value
                } else {
                    caps[0].to_string()
                }
            }).to_string();

            if !changed {
                break;
            }
        }

        result
    }

    /// Resolve template expression from AST node.
    pub fn resolve_template_expr(&self, expr: &Expression) -> Option<String> {
        match expr {
            Expression::StringLiteral(s) => {
                Some(s.value.as_str().to_string())
            }
            Expression::TemplateLiteral(tmpl) => {
                let mut result = String::new();
                for (i, quasi) in tmpl.quasis.iter().enumerate() {
                    result.push_str(quasi.value.raw.as_str());
                    if i < tmpl.expressions.len() {
                        if let Some(resolved) = self.try_resolve_expr(&tmpl.expressions[i]) {
                            result.push_str(&resolved);
                        } else {
                            result.push_str("${...}");
                        }
                    }
                }
                Some(result)
            }
            Expression::Identifier(id) => {
                self.get_string_value(id.name.as_str())
            }
            _ => None,
        }
    }

    /// Try to resolve an expression to a string.
    fn try_resolve_expr(&self, expr: &Expression) -> Option<String> {
        match expr {
            Expression::Identifier(id) => {
                self.get_string_value(id.name.as_str())
            }
            Expression::StringLiteral(s) => {
                Some(s.value.as_str().to_string())
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
    fn test_symbol_table() {
        let code = r#"
            var x4 = "Bash";
            var wH = "Grep";
            var description = "Use ${x4} and ${wH} tools";
        "#;

        let allocator = Allocator::default();
        let parser = Parser::new(code.to_string());
        let parse_result = parser.parse(&allocator).unwrap();

        let table = SymbolTable::new(parse_result.program());

        assert_eq!(table.get("x4"), Some(&"Bash".to_string()));
        assert_eq!(table.get("wH"), Some(&"Grep".to_string()));

        let resolved = table.resolve_template("Use ${x4} and ${wH} tools");
        assert_eq!(resolved, "Use Bash and Grep tools");
    }
}
