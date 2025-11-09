//! Symbol table for resolving variable references.

use oxc_ast::ast::*;
use std::collections::HashMap;
use tracing::debug;

/// Symbol table that maps variable names to their string values.
pub struct SymbolTable<'a> {
    symbols: HashMap<String, String>,
    program: &'a Program<'a>,
}

impl<'a> SymbolTable<'a> {
    /// Create a new symbol table.
    pub fn new(program: &'a Program<'a>) -> Self {
        let mut table = Self {
            symbols: HashMap::new(),
            program,
        };
        table.build();
        table
    }

    /// Build the symbol table by traversing the AST.
    fn build(&mut self) {
        for stmt in &self.program.body {
            self.visit_statement(stmt);
        }
        debug!("Built symbol table with {} symbols", self.symbols.len());
    }

    /// Visit a statement to collect variable declarations.
    fn visit_statement(&mut self, stmt: &Statement) {
        match stmt {
            Statement::VariableDeclaration(decl) => {
                for declarator in &decl.declarations {
                    // Handle simple identifier patterns
                    if let BindingPatternKind::BindingIdentifier(id) = &declarator.id.kind {
                        let var_name = id.name.as_str();

                        if let Some(ref init) = declarator.init {
                            if let Some(value) = self.extract_string_value(init) {
                                self.symbols.insert(var_name.to_string(), value);
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    /// Extract string value from an expression.
    fn extract_string_value(&self, expr: &Expression) -> Option<String> {
        match expr {
            Expression::StringLiteral(s) => Some(s.value.as_str().to_string()),
            Expression::TemplateLiteral(tmpl) => {
                // For template literals, we need to resolve variables
                let mut result = String::new();

                for (i, quasi) in tmpl.quasis.iter().enumerate() {
                    result.push_str(quasi.value.raw.as_str());

                    if i < tmpl.expressions.len() {
                        // Try to resolve the expression
                        if let Some(var_value) = self.resolve_expression(&tmpl.expressions[i]) {
                            result.push_str(&var_value);
                        } else {
                            result.push_str("${...}");
                        }
                    }
                }

                Some(result)
            }
            _ => None,
        }
    }

    /// Try to resolve an expression to a string value.
    fn resolve_expression(&self, expr: &Expression) -> Option<String> {
        match expr {
            Expression::Identifier(id) => {
                // Look up the identifier in our symbol table
                self.symbols.get(id.name.as_str()).cloned()
            }
            Expression::StringLiteral(s) => Some(s.value.as_str().to_string()),
            _ => None,
        }
    }

    /// Get a symbol's value.
    pub fn get(&self, name: &str) -> Option<&String> {
        self.symbols.get(name)
    }

    /// Resolve template literal with variable substitution.
    pub fn resolve_template(&self, template: &str) -> String {
        let mut result = template.to_string();

        // Replace ${varName} with actual values
        let re = regex::Regex::new(r"\$\{([a-zA-Z0-9_]+)\}").unwrap();

        loop {
            let mut changed = false;

            result = re.replace_all(&result, |caps: &regex::Captures| {
                let var_name = &caps[1];
                if let Some(value) = self.symbols.get(var_name) {
                    changed = true;
                    value.clone()
                } else {
                    caps[0].to_string() // Keep unresolved
                }
            }).to_string();

            if !changed {
                break;
            }
        }

        result
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
