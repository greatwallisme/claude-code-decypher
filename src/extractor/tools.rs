//! Enhanced tool definition extraction using symbol table and schema extractor.

use crate::analyzer::{Analyzer, ObjectExpressionInfo, SymbolTable};
use crate::extractor::prompts::SystemPrompt;
use crate::extractor::schemas::SchemaExtractor;
use crate::Result;
use oxc_ast::ast::*;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use tracing::{debug, trace};

/// A complete tool definition with all metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// Tool name.
    pub name: String,

    /// Short description (one-liner).
    pub short_description: String,

    /// Full prompt/documentation (multi-paragraph).
    pub full_prompt: String,

    /// Input parameter schema (JSON Schema format).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_schema: Option<JsonValue>,

    /// Output schema (JSON Schema format).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_schema: Option<JsonValue>,

    /// Tool properties and flags.
    pub properties: ToolProperties,

    /// Confidence score (0.0-1.0).
    pub confidence: f32,
}

/// Tool properties and behavioral flags.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolProperties {
    /// Whether the tool uses strict validation.
    pub is_strict: bool,

    /// Whether the tool is enabled.
    pub is_enabled: bool,

    /// Whether the tool only reads data (no mutations).
    pub is_read_only: bool,

    /// Whether the tool can be called concurrently.
    pub is_concurrency_safe: bool,

    /// User-facing display name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_facing_name: Option<String>,
}

impl Default for ToolProperties {
    fn default() -> Self {
        Self {
            is_strict: false,
            is_enabled: true,
            is_read_only: false,
            is_concurrency_safe: false,
            user_facing_name: None,
        }
    }
}

/// Extractor for tool definitions.
pub struct ToolExtractor<'a> {
    analyzer: &'a Analyzer<'a>,
    symbol_table: SymbolTable<'a>,
}

impl<'a> ToolExtractor<'a> {
    /// Create a new tool extractor.
    pub fn new(analyzer: &'a Analyzer<'a>) -> Self {
        let symbol_table = SymbolTable::new(analyzer.program());
        Self {
            analyzer,
            symbol_table,
        }
    }

    /// Extract all tool definitions.
    pub fn extract(&self) -> Result<Vec<ToolDefinition>> {
        debug!("Extracting tool definitions with enhanced AST analysis");

        let objects = self.analyzer.find_object_expressions();
        debug!("Found {} total objects to analyze", objects.len());

        // Debug: check first few objects
        for (i, obj) in objects.iter().take(10).enumerate() {
            let props: Vec<_> = obj.properties.iter()
                .filter_map(|p| p.key.map(|k| (k, p.is_method)))
                .collect();
            trace!("Object {}: {} properties: {:?}", i, obj.property_count, props);
        }

        let mut tools = Vec::new();

        for obj in &objects {
            if let Some(tool) = self.extract_tool_from_object(obj) {
                debug!(
                    "Extracted tool: {} (confidence: {:.2})",
                    tool.name, tool.confidence
                );
                tools.push(tool);
            }
        }

        debug!("Extracted {} tool definitions", tools.len());
        Ok(tools)
    }

    /// Extract tools from system prompts (alternative method).
    pub fn extract_from_prompts(prompts: &[SystemPrompt]) -> Result<Vec<ToolDefinition>> {
        debug!("Extracting tool definitions from system prompts");

        let mut tools = Vec::new();

        // Known tool names
        let tool_names = [
            "Bash",
            "Read",
            "Write",
            "Edit",
            "Grep",
            "Glob",
            "Task",
            "TodoWrite",
            "NotebookEdit",
            "WebFetch",
            "WebSearch",
            "Skill",
            "SlashCommand",
            "AskUserQuestion",
            "ExitPlanMode",
            "BashOutput",
            "KillShell",
            "LSP",
        ];

        for tool_name in &tool_names {
            if let Some(tool_prompt) = Self::find_tool_prompt(prompts, tool_name) {
                let tool = ToolDefinition {
                    name: tool_name.to_string(),
                    short_description: String::new(),
                    full_prompt: tool_prompt.content.clone(),
                    input_schema: None,
                    output_schema: None,
                    properties: ToolProperties::default(),
                    confidence: 1.0,
                };

                debug!(
                    "Extracted tool from prompts: {} ({} chars)",
                    tool.name,
                    tool.full_prompt.len()
                );
                tools.push(tool);
            }
        }

        debug!("Extracted {} tools from system prompts", tools.len());
        Ok(tools)
    }

    /// Find the prompt that describes a specific tool.
    fn find_tool_prompt<'b>(
        prompts: &'b [SystemPrompt],
        tool_name: &str,
    ) -> Option<&'b SystemPrompt> {
        // Strategy: Find prompt with tool name prominently featured
        prompts
            .iter()
            .filter(|p| {
                let first_200 = p.content.chars().take(200).collect::<String>();
                first_200.contains(tool_name)
                    && p.content.len() > 200
                    && p.category == crate::extractor::prompts::PromptCategory::Tool
            })
            .max_by_key(|p| p.length)
    }

    /// Check if an object matches tool definition pattern.
    fn is_tool_object(&self, obj: &ObjectExpressionInfo) -> bool {
        let has_name = obj.properties.iter().any(|p| p.key == Some("name"));

        // Check for description/prompt as method OR as function expression
        let has_description_func = obj.properties.iter().any(|p| {
            if p.key == Some("description") {
                // Method shorthand OR function expression
                let result = p.is_method || self.property_is_function(obj.ast_object, "description");
                if result {
                    trace!("Found description function in object");
                }
                result
            } else {
                false
            }
        });

        let has_prompt_func = obj.properties.iter().any(|p| {
            if p.key == Some("prompt") {
                let result = p.is_method || self.property_is_function(obj.ast_object, "prompt");
                if result {
                    trace!("Found prompt function in object");
                }
                result
            } else {
                false
            }
        });

        let has_input_schema = obj
            .properties
            .iter()
            .any(|p| p.key == Some("inputSchema"));

        let is_match = has_name && (has_description_func || has_prompt_func || has_input_schema);

        if is_match {
            let name_val = obj.properties.iter()
                .find(|p| p.key == Some("name"))
                .and_then(|p| p.string_value.or(p.identifier_value))
                .unwrap_or("unknown");
            trace!("MATCHED tool object: name={}, has_desc_func={}, has_prompt_func={}, has_schema={}",
                   name_val, has_description_func, has_prompt_func, has_input_schema);
        }

        // Must have name + (description OR prompt OR schema)
        is_match
    }

    /// Check if a property is a function expression.
    fn property_is_function(&self, obj: &ObjectExpression, prop_name: &str) -> bool {
        for prop in &obj.properties {
            if let ObjectPropertyKind::ObjectProperty(p) = prop {
                let key_matches = match &p.key {
                    PropertyKey::StaticIdentifier(id) => id.name.as_str() == prop_name,
                    PropertyKey::StringLiteral(s) => s.value.as_str() == prop_name,
                    _ => false,
                };

                if key_matches {
                    return matches!(
                        &p.value,
                        Expression::FunctionExpression(_) | Expression::ArrowFunctionExpression(_)
                    );
                }
            }
        }
        false
    }

    /// Extract complete tool definition from object.
    fn extract_tool_from_object(&self, obj: &ObjectExpressionInfo) -> Option<ToolDefinition> {
        // Check if this matches tool pattern
        if !self.is_tool_object(obj) {
            return None;
        }

        // Extract name (resolving variable reference)
        let name = self.extract_property_value(obj, "name")?;
        let name_str = self.resolve_to_string(&name)?;

        trace!("Extracting tool: {}", name_str);

        // Extract descriptions - prioritize prompt() for full documentation
        let full_prompt = self
            .extract_method_return_value(obj, "prompt")
            .and_then(|v| self.resolve_to_string(v))
            .unwrap_or_default();

        let short_desc = self
            .extract_method_return_value(obj, "description")
            .and_then(|v| self.resolve_to_string(v))
            .unwrap_or_else(|| {
                // If no separate description, use first 200 chars of prompt
                full_prompt.chars().take(200).collect()
            });

        // Extract schemas
        let schema_extractor = SchemaExtractor::new(&self.symbol_table);

        let input_schema = self
            .extract_property_value(obj, "inputSchema")
            .and_then(|v| self.resolve_to_schema(&schema_extractor, v));

        let output_schema = self
            .extract_property_value(obj, "outputSchema")
            .and_then(|v| self.resolve_to_schema(&schema_extractor, v));

        // Extract properties
        let properties = self.extract_tool_properties(obj);

        // Calculate confidence
        let confidence = self.calculate_confidence(&name_str, &short_desc, &full_prompt, &input_schema);

        debug!("Extracted tool from AST: {} (prompt: {} chars, desc: {} chars, confidence: {:.2})",
               name_str, full_prompt.len(), short_desc.len(), confidence);

        Some(ToolDefinition {
            name: name_str,
            short_description: short_desc,
            full_prompt,
            input_schema,
            output_schema,
            properties,
            confidence,
        })
    }

    /// Extract property value from object.
    fn extract_property_value<'b>(
        &self,
        obj: &'b ObjectExpressionInfo,
        prop_name: &str,
    ) -> Option<&'b Expression<'b>> {
        for prop in &obj.ast_object.properties {
            if let ObjectPropertyKind::ObjectProperty(p) = prop {
                let key_matches = match &p.key {
                    PropertyKey::StaticIdentifier(id) => id.name.as_str() == prop_name,
                    PropertyKey::StringLiteral(s) => s.value.as_str() == prop_name,
                    _ => false,
                };

                if key_matches {
                    return Some(&p.value);
                }
            }
        }
        None
    }

    /// Extract return value from async method.
    fn extract_method_return_value<'b>(
        &self,
        obj: &'b ObjectExpressionInfo,
        method_name: &str,
    ) -> Option<&'b Expression<'b>> {
        for prop in &obj.ast_object.properties {
            if let ObjectPropertyKind::ObjectProperty(p) = prop {
                // Check if this is the right method
                let key_matches = match &p.key {
                    PropertyKey::StaticIdentifier(id) => id.name.as_str() == method_name,
                    PropertyKey::StringLiteral(s) => s.value.as_str() == method_name,
                    _ => false,
                };

                if !key_matches || !p.method {
                    continue;
                }

                // Extract from function body
                if let Expression::FunctionExpression(func) = &p.value {
                    if let Some(body) = &func.body {
                        for stmt in &body.statements {
                            if let Statement::ReturnStatement(ret) = stmt {
                                if let Some(arg) = &ret.argument {
                                    return Some(arg);
                                }
                            }
                        }
                    }
                }
                // Also handle arrow functions
                else if let Expression::ArrowFunctionExpression(arrow) = &p.value {
                    // For arrow functions, check if body is a direct expression or block
                    for stmt in &arrow.body.statements {
                        if let Statement::ReturnStatement(ret) = stmt {
                            if let Some(arg) = &ret.argument {
                                return Some(arg);
                            }
                        }
                    }
                }
            }
        }
        None
    }

    /// Resolve expression to string using symbol table.
    fn resolve_to_string(&self, expr: &Expression) -> Option<String> {
        self.symbol_table.resolve_template_expr(expr)
    }

    /// Resolve schema reference to JSON.
    fn resolve_to_schema(
        &self,
        schema_extractor: &SchemaExtractor,
        expr: &Expression,
    ) -> Option<JsonValue> {
        match expr {
            Expression::Identifier(id) => {
                // Look up schema in symbol table
                self.symbol_table.get_schema(id.name.as_str())
            }
            Expression::CallExpression(call) => {
                // Parse k.object() call
                schema_extractor.parse_schema_builder_call(call)
            }
            _ => None,
        }
    }

    /// Extract tool properties from object.
    fn extract_tool_properties(&self, obj: &ObjectExpressionInfo) -> ToolProperties {
        let mut props = ToolProperties::default();

        // Extract strict flag
        if let Some(strict_expr) = self.extract_property_value(obj, "strict") {
            if let Expression::BooleanLiteral(b) = strict_expr {
                props.is_strict = b.value;
            }
        }

        // Extract other boolean properties similarly
        // (isEnabled, isReadOnly, isConcurrencySafe)

        props
    }

    /// Calculate confidence score.
    fn calculate_confidence(
        &self,
        name: &str,
        short: &str,
        full: &str,
        schema: &Option<JsonValue>,
    ) -> f32 {
        let mut score: f32 = 0.0;

        if !name.is_empty() {
            score += 0.2;
        }
        if !short.is_empty() && short.len() > 20 {
            score += 0.2;
        }
        if !full.is_empty() && full.len() > 100 {
            score += 0.4;
        }
        if schema.is_some() {
            score += 0.2;
        }

        score.min(1.0)
    }
}
