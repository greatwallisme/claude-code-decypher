//! Tool definition extraction.

use crate::analyzer::{Analyzer, ObjectExpressionInfo};
use crate::extractor::prompts::SystemPrompt;
use crate::Result;
use oxc_ast::ast::*;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use tracing::debug;

/// Extract string value from an object property by key name
fn extract_string_from_object(
    obj: &oxc_ast::ast::ObjectExpression,
    key_name: &str,
) -> Option<String> {
    use tracing::trace;

    for prop in &obj.properties {
        if let ObjectPropertyKind::ObjectProperty(p) = prop {
            let key = match &p.key {
                PropertyKey::StaticIdentifier(id) => {
                    trace!("Checking property key: {}", id.name.as_str());
                    if id.name.as_str() == key_name {
                        Some(id.name.as_str())
                    } else {
                        None
                    }
                }
                PropertyKey::StringLiteral(s) => {
                    trace!("Checking property key (string): {}", s.value.as_str());
                    if s.value.as_str() == key_name {
                        Some(s.value.as_str())
                    } else {
                        None
                    }
                }
                _ => None,
            };

            if key.is_some() {
                trace!("Found key '{}', extracting value...", key_name);
                // Found the right key, now extract the value
                let value = extract_string_from_expression(&p.value);
                trace!("Extracted value for '{}': {:?}", key_name, value);
                return value;
            }
        }
    }
    trace!("Key '{}' not found in object", key_name);
    None
}

/// Extract a string from any expression type
fn extract_string_from_expression(expr: &Expression) -> Option<String> {
    use tracing::trace;

    match expr {
        Expression::StringLiteral(s) => {
            trace!("Found StringLiteral: {}", s.value.as_str());
            Some(s.value.as_str().to_string())
        }
        Expression::TemplateLiteral(tmpl) => {
            trace!("Found TemplateLiteral with {} quasis", tmpl.quasis.len());
            // For template literals, concatenate all parts
            let mut result = String::new();
            for (i, quasi) in tmpl.quasis.iter().enumerate() {
                result.push_str(quasi.value.raw.as_str());
                if i < tmpl.expressions.len() {
                    result.push_str("${...}"); // Placeholder for expressions
                }
            }
            Some(result)
        }
        Expression::Identifier(id) => {
            trace!("Found Identifier: {} (can't resolve)", id.name.as_str());
            // Can't resolve identifier values without symbol table
            None
        }
        Expression::CallExpression(_) => {
            trace!("Found CallExpression (can't extract)");
            None
        }
        Expression::StaticMemberExpression(_) => {
            trace!("Found StaticMemberExpression (can't extract)");
            None
        }
        Expression::ComputedMemberExpression(_) => {
            trace!("Found ComputedMemberExpression (can't extract)");
            None
        }
        _ => {
            trace!("Found unknown expression type");
            None
        }
    }
}

/// A tool definition found in the code.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// Tool name.
    pub name: String,

    /// Tool description.
    pub description: String,

    /// Parameter schema (if available).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<JsonValue>,

    /// Properties detected.
    pub properties: Vec<String>,

    /// Confidence score (0.0-1.0).
    pub confidence: f32,
}

/// Extractor for tool definitions.
pub struct ToolExtractor<'a> {
    analyzer: &'a Analyzer<'a>,
}

impl<'a> ToolExtractor<'a> {
    /// Create a new tool extractor.
    pub fn new(analyzer: &'a Analyzer<'a>) -> Self {
        Self { analyzer }
    }

    /// Extract all tool definitions (AST-based, limited for minified code).
    pub fn extract(&self) -> Result<Vec<ToolDefinition>> {
        debug!("Extracting tool definitions from AST");

        let objects = self.analyzer.find_object_expressions();
        debug!("Found {} total objects to analyze", objects.len());

        let mut tools = Vec::new();

        for obj in &objects {
            if let Some(tool) = self.try_extract_tool(obj) {
                debug!("Found tool from AST: {} (confidence: {:.2})",
                       tool.name, tool.confidence);
                tools.push(tool);
            }
        }

        debug!("Extracted {} tool definitions from AST", tools.len());
        Ok(tools)
    }

    /// Extract tools from system prompts (better for minified code!).
    pub fn extract_from_prompts(prompts: &[SystemPrompt]) -> Result<Vec<ToolDefinition>> {
        debug!("Extracting tool definitions from system prompts");

        let mut tools = Vec::new();

        // Tool names to look for in prompts (expanded list)
        let tool_names = [
            "Bash", "Read", "Write", "Edit", "Grep", "Glob", "Task",
            "TodoWrite", "NotebookEdit", "WebFetch", "WebSearch",
            "Skill", "SlashCommand", "AskUserQuestion", "ExitPlanMode",
            "BashOutput", "KillShell", "ListMcpResources", "ReadMcpResource",
        ];

        for tool_name in &tool_names {
            // Find prompts that describe this tool
            if let Some(tool_prompt) = Self::find_tool_prompt(prompts, tool_name) {
                let tool = ToolDefinition {
                    name: tool_name.to_string(),
                    description: tool_prompt.content.clone(),
                    parameters: None,
                    properties: vec!["name".to_string(), "description".to_string()],
                    confidence: 1.0,
                };

                debug!("Extracted tool from prompts: {} ({} chars)",
                       tool.name, tool.description.len());
                tools.push(tool);
            }
        }

        debug!("Extracted {} tools from system prompts", tools.len());
        Ok(tools)
    }

    /// Find the prompt that describes a specific tool.
    fn find_tool_prompt<'b>(prompts: &'b [SystemPrompt], tool_name: &str) -> Option<&'b SystemPrompt> {
        // Strategy 1: Find prompt with tool name prominently featured (early mention)
        // Look for prompts where the tool name appears in the first 200 characters
        let early_mention = prompts
            .iter()
            .filter(|p| {
                let first_200 = p.content.chars().take(200).collect::<String>();
                first_200.contains(tool_name) &&
                p.content.len() > 200 &&
                p.category == crate::extractor::prompts::PromptCategory::Tool
            })
            .filter(|p| {
                // Additional filter: avoid prompts that mention many other tools
                let tool_count = ["Bash", "Read", "Write", "Edit", "Grep", "Glob", "Task", "TodoWrite"]
                    .iter()
                    .filter(|t| p.content.contains(*t))
                    .count();
                tool_count <= 3 // Avoid multi-tool prompts
            })
            .max_by_key(|p| p.length);

        if early_mention.is_some() {
            return early_mention;
        }

        // Strategy 2: For tools with specific patterns
        match tool_name {
            "AskUserQuestion" => {
                // Look for "ask the user" or "Use this tool when you need to ask"
                prompts.iter().find(|p| {
                    p.content.contains("Use this tool when you need to ask the user questions")
                })
            }
            "TodoWrite" => {
                // Look for todo-specific content
                prompts.iter().find(|p| {
                    p.content.contains("todo list") && p.content.len() > 400
                })
            }
            _ => {
                // Strategy 3: Find any prompt mentioning the tool with usage info
                prompts
                    .iter()
                    .filter(|p| {
                        p.content.contains(tool_name) &&
                        p.content.len() > 200 &&
                        (p.content.contains("Usage") || p.content.contains("Use this"))
                    })
                    .max_by_key(|p| p.length)
            }
        }
    }

    /// Try to extract a tool definition from an object.
    fn try_extract_tool(&self, obj: &ObjectExpressionInfo) -> Option<ToolDefinition> {
        // Claude Code tool objects have this structure:
        // {
        //   name: variableName,           // Often a variable reference
        //   async description() { ... },   // Method, not property!
        //   async prompt() { ... },
        //   inputSchema: schemaObject,
        //   // ... other methods
        // }

        let has_name = obj.properties.iter().any(|p| p.key == Some("name"));

        // Look for async description METHOD (not property)
        let has_async_description = obj.properties.iter().any(|p| {
            p.key == Some("description") && p.is_method
        });

        // Look for inputSchema
        let has_input_schema = obj.properties.iter().any(|p| {
            matches!(
                p.key,
                Some("inputSchema")
                    | Some("input_schema")
                    | Some("outputSchema")
                    | Some("output_schema")
            )
        });

        // Look for other tool method indicators
        let has_call_method = obj.properties.iter().any(|p| {
            p.key == Some("call") && p.is_method
        });

        let has_check_permissions = obj.properties.iter().any(|p| {
            p.key == Some("checkPermissions") && p.is_method
        });

        // Calculate confidence based on properties present
        let mut confidence = 0.0;
        if has_name {
            confidence += 0.3;
        }
        if has_async_description {
            confidence += 0.4; // async description() is a strong signal!
        }
        if has_input_schema {
            confidence += 0.3;
        }
        if has_call_method {
            confidence += 0.2;
        }
        if has_check_permissions {
            confidence += 0.1;
        }

        // Require name + (async description OR inputSchema) for a tool
        if !has_name || (!has_async_description && !has_input_schema) {
            return None;
        }

        // Additional filter: description should be reasonably long
        // to avoid matching package.json-style entries
        let desc_property = obj.properties.iter().find(|p| p.key == Some("description"));
        if let Some(_desc) = desc_property {
            // This is a heuristic - if it's likely a real tool, description is substantial
            // But we can't check the actual content here, so we'll be lenient
        }

        let properties: Vec<String> = obj
            .properties
            .iter()
            .filter_map(|p| p.key.map(|k| k.to_string()))
            .collect();

        // If confidence is too low, skip it
        if confidence < 0.6 {
            return None;
        }

        // Extract actual name from properties
        let name_prop = obj.properties.iter().find(|p| p.key == Some("name"));

        // Try string value first, then identifier value
        let name = name_prop
            .and_then(|p| p.string_value.or(p.identifier_value))
            .unwrap_or("UnknownTool")
            .to_string();

        // For description, we look for the identifier since it's usually a variable reference
        // The actual description text is in the system prompts
        let description = if has_async_description {
            format!("{} tool (see system prompts for full description)", name)
        } else {
            "No description available".to_string()
        };

        // Debug logging
        debug!("Found potential tool: name={}, has_async_desc={}, has_schema={}",
               name, has_async_description, has_input_schema);

        // Try to extract parameters as JSON
        let parameters = self.extract_parameters_json(obj);

        Some(ToolDefinition {
            name,
            description,
            parameters,
            properties,
            confidence,
        })
    }

    /// Try to extract parameters as JSON from the AST object.
    fn extract_parameters_json(&self, obj: &ObjectExpressionInfo) -> Option<JsonValue> {
        // Find the parameters property in the AST
        for prop in &obj.ast_object.properties {
            if let ObjectPropertyKind::ObjectProperty(p) = prop {
                let key = match &p.key {
                    PropertyKey::StaticIdentifier(id) => id.name.as_str(),
                    PropertyKey::StringLiteral(s) => s.value.as_str(),
                    _ => continue,
                };

                if matches!(key, "parameters" | "input_schema" | "inputJSONSchema" | "schema") {
                    // Try to convert the expression to JSON
                    return self.expression_to_json(&p.value);
                }
            }
        }
        None
    }

    /// Convert an AST expression to JSON value (best effort).
    fn expression_to_json(&self, expr: &oxc_ast::ast::Expression) -> Option<JsonValue> {
        use serde_json::json;

        match expr {
            oxc_ast::ast::Expression::ObjectExpression(obj) => {
                let mut map = serde_json::Map::new();

                for prop in &obj.properties {
                    if let ObjectPropertyKind::ObjectProperty(p) = prop {
                        let key = match &p.key {
                            PropertyKey::StaticIdentifier(id) => id.name.as_str(),
                            PropertyKey::StringLiteral(s) => s.value.as_str(),
                            _ => continue,
                        };

                        if let Some(value) = self.expression_to_json(&p.value) {
                            map.insert(key.to_string(), value);
                        }
                    }
                }

                Some(JsonValue::Object(map))
            }
            oxc_ast::ast::Expression::StringLiteral(s) => {
                Some(JsonValue::String(s.value.as_str().to_string()))
            }
            oxc_ast::ast::Expression::NumericLiteral(n) => Some(json!(n.value)),
            oxc_ast::ast::Expression::BooleanLiteral(b) => Some(JsonValue::Bool(b.value)),
            oxc_ast::ast::Expression::NullLiteral(_) => Some(JsonValue::Null),
            oxc_ast::ast::Expression::ArrayExpression(arr) => {
                let elements: Vec<JsonValue> = arr
                    .elements
                    .iter()
                    .filter_map(|elem| match elem {
                        ArrayExpressionElement::StringLiteral(s) => {
                            Some(JsonValue::String(s.value.as_str().to_string()))
                        }
                        ArrayExpressionElement::NumericLiteral(n) => Some(json!(n.value)),
                        ArrayExpressionElement::BooleanLiteral(b) => Some(JsonValue::Bool(b.value)),
                        ArrayExpressionElement::ObjectExpression(obj) => {
                            // Recursively convert nested object
                            let mut map = serde_json::Map::new();
                            for prop in &obj.properties {
                                if let ObjectPropertyKind::ObjectProperty(p) = prop {
                                    let key = match &p.key {
                                        PropertyKey::StaticIdentifier(id) => id.name.as_str(),
                                        PropertyKey::StringLiteral(s) => s.value.as_str(),
                                        _ => continue,
                                    };
                                    if let Some(value) = self.expression_to_json(&p.value) {
                                        map.insert(key.to_string(), value);
                                    }
                                }
                            }
                            Some(JsonValue::Object(map))
                        }
                        _ => None,
                    })
                    .collect();

                Some(JsonValue::Array(elements))
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
    fn test_extract_tool_definition() {
        let code = r#"
            const toolDef = {
                name: "Bash",
                async description() {
                    return "Execute bash commands";
                },
                inputSchema: {
                    "$schema": "http://json-schema.org/draft-07/schema#",
                    type: "object",
                    properties: {
                        command: { type: "string" }
                    }
                }
            };
        "#;

        let allocator = Allocator::default();
        let parser = Parser::new(code.to_string());
        let parse_result = parser.parse(&allocator).unwrap();

        let analyzer = Analyzer::new(parse_result.program());
        let extractor = ToolExtractor::new(&analyzer);
        let tools = extractor.extract().unwrap();

        // Should find the tool with async description method
        assert!(!tools.is_empty(), "Should find tool with async description() method");

        // Check properties
        let tool = &tools[0];
        assert_eq!(tool.name, "Bash");
        assert!(tool.properties.contains(&"name".to_string()));
        assert!(tool.confidence >= 0.6);
    }

    #[test]
    fn test_ignore_non_tool_objects() {
        let code = r#"
            const config = {
                host: "localhost",
                port: 8080
            };
        "#;

        let allocator = Allocator::default();
        let parser = Parser::new(code.to_string());
        let parse_result = parser.parse(&allocator).unwrap();

        let analyzer = Analyzer::new(parse_result.program());
        let extractor = ToolExtractor::new(&analyzer);
        let tools = extractor.extract().unwrap();

        // Should not match non-tool objects
        assert_eq!(tools.len(), 0);
    }
}
