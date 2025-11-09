//! Tool extraction from beautified code.
//!
//! This module extracts tool definitions by analyzing the beautified JavaScript
//! code, which is more effective than AST analysis for minified bundles.

use crate::analyzer::SymbolTable;
use crate::extractor::tools::ToolDefinition;
use crate::Result;
use oxc_ast::ast::Program;
use regex::Regex;
use tracing::debug;

/// Extract tools from beautified JavaScript code.
pub struct BeautifiedToolExtractor<'a> {
    beautified_code: &'a str,
    symbol_table: Option<SymbolTable<'a>>,
}

impl<'a> BeautifiedToolExtractor<'a> {
    /// Create a new beautified tool extractor.
    pub fn new(beautified_code: &'a str) -> Self {
        Self {
            beautified_code,
            symbol_table: None,
        }
    }

    /// Create with AST for symbol resolution.
    pub fn with_ast(beautified_code: &'a str, program: &'a Program<'a>) -> Self {
        let symbol_table = SymbolTable::new(program);
        Self {
            beautified_code,
            symbol_table: Some(symbol_table),
        }
    }

    /// Extract all tools from the beautified code.
    pub fn extract(&self) -> Result<Vec<ToolDefinition>> {
        debug!("Extracting tools from beautified code");

        let mut tools = Vec::new();

        // Pattern 1: Find tool name constants like: var x4 = "Bash";
        // Also handles: var G7 = "Read", oWA = 2e3, ...
        let name_pattern = Regex::new(r#"var\s+([a-zA-Z0-9_]+)\s*=\s*"([A-Z][a-zA-Z]+)"[,;]"#)
            .map_err(|e| crate::error::DecypherError::Other(e.into()))?;

        for cap in name_pattern.captures_iter(self.beautified_code) {
            let var_name = &cap[1];
            let tool_name = &cap[2];

            // Filter to likely tool names (not error codes, signals, etc.)
            if Self::is_likely_tool_name(tool_name) {
                // Try to find the description for this tool
                let description = self.find_tool_description(tool_name, var_name);

                // Try to find if this has an inputSchema
                let has_schema = self.has_input_schema(var_name);

                let has_desc = description.is_some();

                // Resolve template variables in description if we have a symbol table
                let final_description = if let Some(desc) = description {
                    if let Some(ref table) = self.symbol_table {
                        table.resolve_template(&desc)
                    } else {
                        desc
                    }
                } else {
                    format!("Tool: {}", tool_name)
                };

                let tool = ToolDefinition {
                    name: tool_name.to_string(),
                    description: final_description,
                    parameters: None,
                    properties: if has_schema {
                        vec!["name".to_string(), "description".to_string(), "inputSchema".to_string()]
                    } else {
                        vec!["name".to_string(), "description".to_string()]
                    },
                    confidence: if has_desc && has_schema {
                        1.0
                    } else if has_desc {
                        0.8
                    } else {
                        0.6
                    },
                };

                debug!("Found tool from beautified code: {} (confidence: {:.2})",
                       tool.name, tool.confidence);
                tools.push(tool);
            }
        }

        debug!("Extracted {} tools from beautified code", tools.len());
        Ok(tools)
    }

    /// Check if a name is likely a tool name.
    fn is_likely_tool_name(name: &str) -> bool {
        // Known tool names (expanded list)
        let known_tools = [
            "Bash", "Read", "Write", "Edit", "Grep", "Glob", "Task",
            "TodoWrite", "NotebookEdit", "WebFetch", "WebSearch",
            "Skill", "SlashCommand", "AskUserQuestion", "ExitPlanMode",
            "BashOutput", "KillShell", "ListMcpResources", "ReadMcpResource",
        ];

        if known_tools.contains(&name) {
            return true;
        }

        // Heuristics for tool names
        // - Capitalized
        // - Reasonable length (3-20 chars)
        // - Not error codes or constants
        if name.len() < 3 || name.len() > 20 {
            return false;
        }

        // Exclude common non-tool patterns
        let excluded_prefixes = ["SIG", "SYSRES", "ERROR", "HTTP", "CONST"];
        for prefix in excluded_prefixes {
            if name.starts_with(prefix) {
                return false;
            }
        }

        // Must start with capital letter
        name.chars().next().map(|c| c.is_uppercase()).unwrap_or(false)
    }

    /// Find the description for a tool.
    fn find_tool_description(&self, tool_name: &str, var_name: &str) -> Option<String> {
        // Pattern 1: Description in same line
        // var G7 = "Read", oWA = 2e3, zr4 = 2e3, FHB = "Read a file...", CHB;
        let same_line_pattern = format!(
            r#"var\s+{}\s*=\s*"{}",.*?"([^"]+)""#,
            regex::escape(var_name),
            regex::escape(tool_name)
        );

        if let Ok(re) = Regex::new(&same_line_pattern) {
            if let Some(cap) = re.captures(self.beautified_code) {
                return Some(cap[1].to_string());
            }
        }

        // Pattern 2: Description in following variable
        // var Qz1 = `description here...`;
        let desc_var_pattern = format!(
            r#"var\s+{}\s*=\s*"{}";?\s*var\s+[a-zA-Z0-9_]+\s*=\s*`([^`]+)`"#,
            regex::escape(var_name),
            regex::escape(tool_name)
        );

        if let Ok(re) = Regex::new(&desc_var_pattern) {
            if let Some(cap) = re.captures(self.beautified_code) {
                // Take first 500 chars of description
                let desc = &cap[1];
                return Some(desc.chars().take(500).collect());
            }
        }

        // Pattern 3: Search for tool name in context
        self.find_description_near_tool_name(tool_name)
    }

    /// Find description near tool name mention.
    fn find_description_near_tool_name(&self, tool_name: &str) -> Option<String> {
        // Look for patterns like:
        // var toolName = "ToolName";
        // ... some code ...
        // description text mentioning the tool

        // Simple heuristic: find the tool name and look nearby for descriptive text
        let search_pattern = format!(r#""{}""#, tool_name);

        if let Some(pos) = self.beautified_code.find(&search_pattern) {
            // Look in a window around this position
            let start = pos.saturating_sub(1000);
            let end = (pos + 2000).min(self.beautified_code.len());
            let window = &self.beautified_code[start..end];

            // Look for description-like strings
            let desc_pattern = Regex::new(r#"`([^`]{100,500})`"#).ok()?;
            if let Some(cap) = desc_pattern.captures(window) {
                return Some(cap[1].to_string());
            }
        }

        None
    }

    /// Check if a variable has an associated inputSchema.
    fn has_input_schema(&self, var_name: &str) -> bool {
        // Look for patterns like:
        // name: varName,
        // ...
        // inputSchema: ...

        let schema_pattern = format!(
            r#"name:\s*{},.*?inputSchema:"#,
            regex::escape(var_name)
        );

        if let Ok(re) = Regex::new(&schema_pattern) {
            re.is_match(self.beautified_code)
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_tools_from_beautified() {
        let code = r#"
            var x4 = "Bash";
            var G7 = "Read", FHB = "Read a file from the local filesystem.";
            var oW = "Write";
        "#;

        let extractor = BeautifiedToolExtractor::new(code);
        let tools = extractor.extract().unwrap();

        assert!(tools.len() >= 3);

        let tool_names: Vec<_> = tools.iter().map(|t| t.name.as_str()).collect();
        assert!(tool_names.contains(&"Bash"));
        assert!(tool_names.contains(&"Read"));
        assert!(tool_names.contains(&"Write"));
    }

    #[test]
    fn test_is_likely_tool_name() {
        assert!(BeautifiedToolExtractor::is_likely_tool_name("Bash"));
        assert!(BeautifiedToolExtractor::is_likely_tool_name("TodoWrite"));
        assert!(!BeautifiedToolExtractor::is_likely_tool_name("SIGHUP"));
        assert!(!BeautifiedToolExtractor::is_likely_tool_name("ERROR"));
        assert!(!BeautifiedToolExtractor::is_likely_tool_name("A")); // Too short
    }
}
