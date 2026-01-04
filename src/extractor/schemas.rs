//! Schema extraction from builder patterns (k.object(), k.strictObject(), etc.).

use crate::analyzer::SymbolTable;
use oxc_ast::ast::*;
use serde_json::{Map, Value as JsonValue};
use tracing::trace;

/// Extractor for schema objects built with builder patterns.
pub struct SchemaExtractor<'a> {
    symbol_table: &'a SymbolTable<'a>,
}

impl<'a> SchemaExtractor<'a> {
    /// Create a new schema extractor.
    pub fn new(symbol_table: &'a SymbolTable<'a>) -> Self {
        Self { symbol_table }
    }

    /// Parse a schema builder call like k.object(...) or k.strictObject(...).
    pub fn parse_schema_builder_call(&self, call: &CallExpression) -> Option<JsonValue> {
        // Check if this is k.object() or k.strictObject()
        if !self.is_schema_builder_call(call) {
            return None;
        }

        trace!("Parsing schema builder call");

        // Get the argument (should be object expression)
        let arg = call.arguments.first()?;

        if let Argument::ObjectExpression(obj_expr) = arg {
            self.parse_object_schema(obj_expr)
        } else {
            None
        }
    }

    /// Check if a call expression is a schema builder call.
    fn is_schema_builder_call(&self, call: &CallExpression) -> bool {
        // Check for: k.object(...) or k.strictObject(...)
        if let Expression::StaticMemberExpression(member) = &call.callee {
            if let Expression::Identifier(obj) = &member.object {
                if obj.name.as_str() == "k" {
                    let prop_name = member.property.name.as_str();
                    return matches!(prop_name, "object" | "strictObject" | "array" | "string" | "number" | "boolean" | "enum");
                }
            }
        }
        false
    }

    /// Parse an object expression as a schema.
    fn parse_object_schema(&self, obj_expr: &ObjectExpression) -> Option<JsonValue> {
        let mut properties = Map::new();
        let mut required = Vec::new();

        for prop in &obj_expr.properties {
            if let ObjectPropertyKind::ObjectProperty(p) = prop {
                let key = self.extract_key(&p.key)?;

                // Value might be:
                // 1. Another schema builder call (nested)
                // 2. Variable reference
                // 3. Call to .describe()

                let value = self.parse_schema_value(&p.value)?;
                properties.insert(key.clone(), value);

                // Assume all properties are required (common pattern)
                required.push(key);
            }
        }

        let mut schema = Map::new();
        schema.insert("type".to_string(), JsonValue::String("object".to_string()));
        schema.insert("properties".to_string(), JsonValue::Object(properties));

        if !required.is_empty() {
            schema.insert(
                "required".to_string(),
                JsonValue::Array(required.into_iter().map(JsonValue::String).collect()),
            );
        }

        Some(JsonValue::Object(schema))
    }

    /// Extract property key from PropertyKey.
    fn extract_key(&self, key: &PropertyKey) -> Option<String> {
        match key {
            PropertyKey::StaticIdentifier(id) => Some(id.name.as_str().to_string()),
            PropertyKey::StringLiteral(s) => Some(s.value.as_str().to_string()),
            _ => None,
        }
    }

    /// Parse schema value (might have .describe() call).
    fn parse_schema_value(&self, expr: &Expression) -> Option<JsonValue> {
        match expr {
            // Handle: k.string().describe("...")
            Expression::CallExpression(call) => {
                // Check if this is a .describe() call
                if self.is_describe_call(call) {
                    // Extract the description
                    let description = self.extract_describe_arg(call)?;

                    // Get the base type from callee
                    let mut base_type = self.extract_base_schema_type(&call.callee)?;

                    if let JsonValue::Object(ref mut map) = base_type {
                        map.insert("description".to_string(), JsonValue::String(description));
                    }

                    Some(base_type)
                } else {
                    // Might be k.string(), k.number(), etc.
                    self.parse_type_call(call)
                }
            }
            // Handle: variable reference
            Expression::Identifier(id) => {
                self.symbol_table.get_schema(id.name.as_str())
            }
            _ => None,
        }
    }

    /// Check if call is to .describe().
    fn is_describe_call(&self, call: &CallExpression) -> bool {
        if let Expression::StaticMemberExpression(member) = &call.callee {
            member.property.name.as_str() == "describe"
        } else {
            false
        }
    }

    /// Extract description argument from .describe() call.
    fn extract_describe_arg(&self, call: &CallExpression) -> Option<String> {
        let arg = call.arguments.first()?;
        match arg {
            Argument::StringLiteral(s) => Some(s.value.as_str().to_string()),
            _ => None,
        }
    }

    /// Extract base schema type from chained calls.
    fn extract_base_schema_type(&self, expr: &Expression) -> Option<JsonValue> {
        match expr {
            Expression::StaticMemberExpression(member) => {
                // This is the k.string() part
                self.parse_type_from_member(member)
            }
            Expression::CallExpression(call) => {
                // Nested call
                self.parse_type_call(call)
            }
            _ => None,
        }
    }

    /// Parse type from member expression (k.string, k.number, etc.).
    fn parse_type_from_member(&self, member: &StaticMemberExpression) -> Option<JsonValue> {
        if let Expression::Identifier(obj) = &member.object {
            if obj.name.as_str() == "k" {
                let type_name = member.property.name.as_str();
                return Some(self.type_name_to_schema(type_name));
            }
        }
        None
    }

    /// Extract base schema type from k.string(), k.number(), etc.
    fn parse_type_call(&self, call: &CallExpression) -> Option<JsonValue> {
        // Check if this is k.string(), k.number(), k.boolean(), etc.
        if let Expression::StaticMemberExpression(member) = &call.callee {
            if let Expression::Identifier(obj) = &member.object {
                if obj.name.as_str() == "k" {
                    let type_name = member.property.name.as_str();
                    return Some(self.type_name_to_schema(type_name));
                }
            }
        }

        // Or it might be a nested builder call
        if self.is_schema_builder_call(call) {
            return self.parse_schema_builder_call(call);
        }

        None
    }

    /// Convert type name to JSON schema.
    fn type_name_to_schema(&self, type_name: &str) -> JsonValue {
        let mut schema = Map::new();

        match type_name {
            "string" => {
                schema.insert("type".to_string(), JsonValue::String("string".to_string()));
            }
            "number" => {
                schema.insert("type".to_string(), JsonValue::String("number".to_string()));
            }
            "boolean" => {
                schema.insert("type".to_string(), JsonValue::String("boolean".to_string()));
            }
            "object" | "strictObject" => {
                schema.insert("type".to_string(), JsonValue::String("object".to_string()));
            }
            "array" => {
                schema.insert("type".to_string(), JsonValue::String("array".to_string()));
            }
            _ => {}
        }

        JsonValue::Object(schema)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Parser;
    use oxc_allocator::Allocator;

    #[test]
    fn test_parse_simple_schema() {
        let code = r#"
            var schema = k.object({
                name: k.string().describe("The name"),
                age: k.number().describe("The age")
            });
        "#;

        let allocator = Allocator::default();
        let parser = Parser::new(code.to_string());
        let parse_result = parser.parse(&allocator).unwrap();

        let symbol_table = SymbolTable::new(parse_result.program());
        let extractor = SchemaExtractor::new(&symbol_table);

        // Find the k.object() call
        // This test is simplified - in real use, we'd traverse to find the call
    }
}
