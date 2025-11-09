//! AST analysis module for understanding code structure.

pub mod symbols;

use oxc_ast::ast::*;
pub use symbols::SymbolTable;

/// Analyzes the AST to find interesting patterns and structures.
pub struct Analyzer<'a> {
    program: &'a Program<'a>,
}

impl<'a> Analyzer<'a> {
    /// Create a new analyzer for the given program.
    pub fn new(program: &'a Program<'a>) -> Self {
        Self { program }
    }

    /// Find all string literals in the program.
    pub fn find_string_literals(&self) -> Vec<StringLiteralInfo<'a>> {
        let mut collector = StringLiteralCollector::new();
        collector.visit_program(self.program);
        collector.literals
    }

    /// Find all object expressions in the program.
    pub fn find_object_expressions(&self) -> Vec<ObjectExpressionInfo<'a>> {
        let mut collector = ObjectCollector::new();
        collector.visit_program(self.program);
        collector.objects
    }

    /// Get the program reference.
    pub fn program(&self) -> &'a Program<'a> {
        self.program
    }
}

/// Information about a string literal found in the code.
#[derive(Debug, Clone)]
pub struct StringLiteralInfo<'a> {
    pub value: &'a str,
    pub length: usize,
    pub line_hint: Option<usize>,
}

/// Information about an object expression found in the code.
#[derive(Debug, Clone)]
pub struct ObjectExpressionInfo<'a> {
    pub property_count: usize,
    pub properties: Vec<PropertyInfo<'a>>,
    pub ast_object: &'a oxc_ast::ast::ObjectExpression<'a>,
}

/// Information about an object property.
#[derive(Debug, Clone)]
pub struct PropertyInfo<'a> {
    pub key: Option<&'a str>,
    pub is_method: bool,
    pub string_value: Option<&'a str>,
    pub identifier_value: Option<&'a str>,
}

/// Visitor that collects all string literals.
struct StringLiteralCollector<'a> {
    literals: Vec<StringLiteralInfo<'a>>,
}

impl<'a> StringLiteralCollector<'a> {
    fn new() -> Self {
        Self {
            literals: Vec::new(),
        }
    }

    fn visit_program(&mut self, program: &'a Program<'a>) {
        for stmt in &program.body {
            self.visit_statement(stmt);
        }
    }

    fn visit_statement(&mut self, stmt: &'a Statement<'a>) {
        match stmt {
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
            Statement::FunctionDeclaration(func) => {
                if let Some(ref body) = func.body {
                    for stmt in &body.statements {
                        self.visit_statement(stmt);
                    }
                }
            }
            _ => {}
        }
    }

    fn visit_expression(&mut self, expr: &'a Expression<'a>) {
        match expr {
            Expression::StringLiteral(str_lit) => {
                let value = str_lit.value.as_str();
                self.literals.push(StringLiteralInfo {
                    value,
                    length: value.len(),
                    line_hint: None,
                });
            }
            Expression::ObjectExpression(obj) => {
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
                // Visit ALL array elements, not just spread elements
                for elem in &arr.elements {
                    match elem {
                        ArrayExpressionElement::SpreadElement(spread) => {
                            self.visit_expression(&spread.argument);
                        }
                        ArrayExpressionElement::BooleanLiteral(_)
                        | ArrayExpressionElement::NullLiteral(_)
                        | ArrayExpressionElement::NumericLiteral(_)
                        | ArrayExpressionElement::BigIntLiteral(_)
                        | ArrayExpressionElement::RegExpLiteral(_) => {}
                        ArrayExpressionElement::StringLiteral(str_lit) => {
                            let value = str_lit.value.as_str();
                            self.literals.push(StringLiteralInfo {
                                value,
                                length: value.len(),
                                line_hint: None,
                            });
                        }
                        _ => {
                            // For other expression elements, we need to visit them
                            // This is a limitation of the current approach
                        }
                    }
                }
            }
            Expression::CallExpression(call) => {
                // Visit call expression arguments
                for arg in &call.arguments {
                    match arg {
                        Argument::SpreadElement(spread) => {
                            self.visit_expression(&spread.argument);
                        }
                        _ => {}
                    }
                }
            }
            Expression::TemplateLiteral(tmpl) => {
                // Extract template literal quasi strings
                for quasi in &tmpl.quasis {
                    let value = quasi.value.raw.as_str();
                    if !value.is_empty() {
                        self.literals.push(StringLiteralInfo {
                            value,
                            length: value.len(),
                            line_hint: None,
                        });
                    }
                }
                // Visit template expressions
                for expr in &tmpl.expressions {
                    self.visit_expression(expr);
                }
            }
            _ => {}
        }
    }
}

/// Visitor that collects all object expressions.
struct ObjectCollector<'a> {
    objects: Vec<ObjectExpressionInfo<'a>>,
}

impl<'a> ObjectCollector<'a> {
    fn new() -> Self {
        Self {
            objects: Vec::new(),
        }
    }

    fn visit_program(&mut self, program: &'a Program<'a>) {
        for stmt in &program.body {
            self.visit_statement(stmt);
        }
    }

    fn visit_statement(&mut self, stmt: &'a Statement<'a>) {
        match stmt {
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
            Statement::FunctionDeclaration(func) => {
                if let Some(ref body) = func.body {
                    for stmt in &body.statements {
                        self.visit_statement(stmt);
                    }
                }
            }
            _ => {}
        }
    }

    fn visit_expression(&mut self, expr: &'a Expression<'a>) {
        match expr {
            Expression::ObjectExpression(obj) => {
                let properties: Vec<PropertyInfo> = obj
                    .properties
                    .iter()
                    .filter_map(|prop| match prop {
                        ObjectPropertyKind::ObjectProperty(p) => {
                            let key = match &p.key {
                                PropertyKey::StaticIdentifier(id) => Some(id.name.as_str()),
                                PropertyKey::StringLiteral(s) => Some(s.value.as_str()),
                                _ => None,
                            };

                            // Extract the actual string value if it's a string literal
                            let string_value = match &p.value {
                                Expression::StringLiteral(s) => Some(s.value.as_str()),
                                _ => None,
                            };

                            // Also capture identifier references (like: name: UvA)
                            let identifier_value = match &p.value {
                                Expression::Identifier(id) => Some(id.name.as_str()),
                                _ => None,
                            };

                            Some(PropertyInfo {
                                key,
                                is_method: p.method,
                                string_value,
                                identifier_value,
                            })
                        }
                        _ => None,
                    })
                    .collect();

                self.objects.push(ObjectExpressionInfo {
                    property_count: properties.len(),
                    properties,
                    ast_object: obj,
                });

                // Visit nested objects recursively
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
                // Visit ALL array elements recursively
                for elem in &arr.elements {
                    match elem {
                        ArrayExpressionElement::SpreadElement(spread) => {
                            self.visit_expression(&spread.argument);
                        }
                        // Handle object expressions in arrays (THIS IS KEY!)
                        ArrayExpressionElement::ObjectExpression(obj_expr) => {
                            // Collect properties from objects in arrays
                            let properties: Vec<PropertyInfo> = obj_expr
                                .properties
                                .iter()
                                .filter_map(|prop| match prop {
                                    ObjectPropertyKind::ObjectProperty(p) => {
                                        let key = match &p.key {
                                            PropertyKey::StaticIdentifier(id) => Some(id.name.as_str()),
                                            PropertyKey::StringLiteral(s) => Some(s.value.as_str()),
                                            _ => None,
                                        };

                                        // Extract the actual string value
                                        let string_value = match &p.value {
                                            Expression::StringLiteral(s) => Some(s.value.as_str()),
                                            _ => None,
                                        };

                                        // Also capture identifier references
                                        let identifier_value = match &p.value {
                                            Expression::Identifier(id) => Some(id.name.as_str()),
                                            _ => None,
                                        };

                                        // Visit the property value recursively
                                        self.visit_expression(&p.value);
                                        Some(PropertyInfo {
                                            key,
                                            is_method: p.method,
                                            string_value,
                                            identifier_value,
                                        })
                                    }
                                    ObjectPropertyKind::SpreadProperty(p) => {
                                        self.visit_expression(&p.argument);
                                        None
                                    }
                                })
                                .collect();

                            self.objects.push(ObjectExpressionInfo {
                                property_count: properties.len(),
                                properties,
                                ast_object: obj_expr,
                            });
                        }
                        ArrayExpressionElement::ArrayExpression(nested_arr) => {
                            // Handle nested arrays by recursively processing elements
                            for nested_elem in &nested_arr.elements {
                                if let ArrayExpressionElement::SpreadElement(spread) = nested_elem {
                                    self.visit_expression(&spread.argument);
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
}
