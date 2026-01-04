//! Enhanced system prompt extraction with fragment merging and tool association.

use crate::analyzer::{Analyzer, StringLiteralInfo, SymbolTable};
use crate::extractor::prompts::{PromptCategory, SystemPrompt};
use crate::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, trace};

/// Context for a system prompt.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PromptContext {
    /// Standalone system prompt.
    Standalone,

    /// Part of tool documentation.
    ToolDocumentation {
        tool_name: String,
    },

    /// Part of larger multi-section prompt.
    Section {
        parent_id: String,
        section_name: String,
    },
}

/// Enhanced system prompt with context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedSystemPrompt {
    /// Unique identifier for this prompt.
    pub id: String,

    /// The prompt content.
    pub content: String,

    /// Length of the prompt.
    pub length: usize,

    /// Category/type of prompt.
    pub category: PromptCategory,

    /// Context information.
    pub context: PromptContext,

    /// Associated tool name (if any).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub associated_tool: Option<String>,

    /// Fragment IDs that were merged into this prompt.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub merged_fragments: Vec<String>,
}

/// Enhanced extractor for system prompts.
pub struct EnhancedPromptExtractor<'a> {
    analyzer: &'a Analyzer<'a>,
    symbol_table: &'a SymbolTable<'a>,
}

impl<'a> EnhancedPromptExtractor<'a> {
    /// Create a new enhanced prompt extractor.
    pub fn new(analyzer: &'a Analyzer<'a>, symbol_table: &'a SymbolTable<'a>) -> Self {
        Self {
            analyzer,
            symbol_table,
        }
    }

    /// Extract all system prompts with enhanced processing.
    pub fn extract(&self) -> Result<Vec<EnhancedSystemPrompt>> {
        debug!("Extracting system prompts with enhancement");

        // Step 1: Extract prompts from string literals
        let string_literals = self.analyzer.find_string_literals();
        let mut basic_prompts = Vec::new();

        for (idx, literal) in string_literals.iter().enumerate() {
            if self.is_likely_prompt(literal) {
                let category = self.categorize_prompt(literal);
                let prompt = EnhancedSystemPrompt {
                    id: format!("prompt_lit_{}", idx),
                    content: literal.value.to_string(),
                    length: literal.length,
                    category,
                    context: PromptContext::Standalone,
                    associated_tool: None,
                    merged_fragments: Vec::new(),
                };
                basic_prompts.push(prompt);
            }
        }

        debug!("Extracted {} prompts from string literals", basic_prompts.len());

        // Step 1b: Extract prompts from symbol table (resolved template literals!)
        let mut symbol_prompts = self.extract_from_symbol_table();
        debug!("Extracted {} prompts from symbol table", symbol_prompts.len());

        // Combine both sources
        basic_prompts.append(&mut symbol_prompts);

        // Step 2: Detect and associate tool prompts
        let mut prompts_with_tools = self.associate_tools(basic_prompts);

        // Step 3: Merge related fragments
        let merged_prompts = self.merge_fragments(prompts_with_tools);

        debug!("After merge: {} prompts", merged_prompts.len());

        // Step 4: Deduplicate
        let deduplicated = self.deduplicate(merged_prompts);

        debug!("Final count: {} enhanced prompts after deduplication", deduplicated.len());

        // Debug: verify Read prompt survived
        let read_survived = deduplicated.iter().any(|p| p.content.starts_with("Reads a file"));
        debug!("Read prompt survived deduplication: {}", read_survived);

        Ok(deduplicated)
    }

    /// Extract prompts from resolved symbol table values.
    fn extract_from_symbol_table(&self) -> Vec<EnhancedSystemPrompt> {
        use crate::analyzer::symbols::SymbolValue;

        let mut prompts = Vec::new();
        let mut idx = 0;

        for (name, value) in &self.symbol_table.symbols {
            match value {
                SymbolValue::String(s) | SymbolValue::TemplateLiteral(s) => {
                    // Check if this is a prompt (lower threshold for symbol table)
                    if s.len() >= 60 && !self.is_code_fragment_str(s) {
                        if self.has_prompt_indicators(s) || self.looks_like_tool_description(s) {
                            let category = self.categorize_prompt_str(s);
                            prompts.push(EnhancedSystemPrompt {
                                id: format!("prompt_sym_{}_{}", name, idx),
                                content: s.clone(),
                                length: s.len(),
                                category,
                                context: PromptContext::Standalone,
                                associated_tool: None,
                                merged_fragments: Vec::new(),
                            });
                            idx += 1;
                        }
                    }
                }
                _ => {}
            }
        }

        prompts
    }

    /// Check if content has prompt indicators.
    fn has_prompt_indicators(&self, content: &str) -> bool {
        let prompt_indicators = [
            "You are Claude",
            "IMPORTANT:",
            "Usage notes:",
            "Usage:",
            "## ",
            "When NOT to use",
            "When to use",
            "Example:",
            "This tool",
            "Use this",
            "allows you to",
            "Supports",
            "Parameters:",
            "\n\n",
        ];

        prompt_indicators.iter().any(|&indicator| content.contains(indicator))
    }

    /// Check if content looks like a tool description (even without standard indicators).
    fn looks_like_tool_description(&self, content: &str) -> bool {
        // Descriptions for NotebookEdit, BashOutput, KillShell, Skill patterns
        let tool_desc_patterns = [
            "Completely replaces",
            "Retrieves output from",
            "Kills a running",
            "Language Server Protocol",
            "Execute a skill",
            "Execute a slash command",
            "skills_instructions",
            "available_skills",
        ];

        tool_desc_patterns.iter().any(|&pattern| content.contains(pattern))
    }

    /// Categorize prompt from string content.
    fn categorize_prompt_str(&self, value: &str) -> PromptCategory {
        if value.contains("You are Claude") || value.contains("answer the user") {
            PromptCategory::System
        } else if value.contains("tool") || value.contains("function") || value.contains("This tool") {
            PromptCategory::Tool
        } else if value.contains("Example:") || value.contains("<example>") {
            PromptCategory::Example
        } else if value.contains("Error:") || value.contains("error") {
            PromptCategory::Error
        } else if value.contains("IMPORTANT:") || value.contains("Usage") {
            PromptCategory::Instruction
        } else {
            PromptCategory::Other
        }
    }

    /// Check if string content is code (for symbol table values).
    fn is_code_fragment_str(&self, content: &str) -> bool {
        self.is_code_fragment(content)
    }

    /// Check if a string literal is likely a system prompt.
    fn is_likely_prompt(&self, literal: &StringLiteralInfo) -> bool {
        // Lower threshold to 60 to catch shorter tool descriptions
        if literal.length < 60 {
            return false;
        }

        let value = literal.value;

        // First, reject obvious code fragments
        if self.is_code_fragment(value) {
            return false;
        }

        // Reject lists of keywords/function names (no sentences)
        if self.is_keyword_list(value) {
            return false;
        }

        let prompt_indicators = [
            "You are Claude",
            "You are powered by",
            "answer the user",
            "tool_use",
            "function_calls",
            "system prompt",
            "IMPORTANT:",
            "Usage notes:",
            "Usage:",
            "## ",
            "When NOT to use",
            "When to use",
            "Example:",
            "This tool",
            "Use this",
            "Available",
            "allows you to",
            "enables",
            "Supports",
            "Note:",
            "WARNING:",
            "Caution:",
            "Description:",
            "<example>",
            "```",
            "Parameters:",
            "Returns:",
            "Throws:",
            "\n\n",
        ];

        prompt_indicators.iter().any(|&indicator| value.contains(indicator))
    }

    /// Check if content is a list of keywords/function names (not prose).
    fn is_keyword_list(&self, content: &str) -> bool {
        // Check if it looks like a space-separated list of identifiers
        // Examples: "setup loop analogWrite", "abs acos acosh activate"

        // Must have NO sentences (no periods followed by space)
        if content.contains(". ") || content.contains(".\n") {
            return false;
        }

        // Check word pattern - if mostly camelCase/snake_case identifiers
        let words: Vec<&str> = content.split_whitespace().take(20).collect();
        if words.len() < 10 {
            return false; // Too short to determine
        }

        // Count how many look like identifiers vs prose words
        let identifier_count = words.iter().filter(|w| {
            // camelCase, snake_case, or all lowercase identifier
            let is_identifier = w.chars().all(|c| c.is_alphanumeric() || c == '_')
                && !w.contains(' ')
                && w.len() > 2;
            is_identifier
        }).count();

        // If >80% are identifiers and no sentences, it's a keyword list
        identifier_count as f32 / words.len() as f32 > 0.8
    }

    /// Check if content is a code fragment (not documentation).
    fn is_code_fragment(&self, content: &str) -> bool {
        let code_indicators = [
            "function(",
            "async function",
            "() =>",
            "} catch {",
            "throw Error(",
            "return !",
            "let ",
            "const ",
            "var ",
            "if (!",
            "stdio:",
            ".forEach(",
            ".map(",
            "\t}",
            "});",
            "module.exports",
            "SNAPSHOT_FILE=", // Shell script
            "#!/bin/",        // Shell script
        ];

        // If it has multiple code indicators, it's definitely code
        let code_count = code_indicators.iter().filter(|&&ind| content.contains(ind)).count();
        if code_count >= 3 {
            return true;
        }

        // Check for YAML (GitHub Actions, etc.)
        let trimmed = content.trim();
        if trimmed.starts_with("name:") && content.contains("on:") && content.contains("jobs:") {
            return true; // YAML file
        }

        // Check for shell scripts
        if content.contains("#!/bin/") || (content.contains("SNAPSHOT_FILE") && content.contains("echo")) {
            return true;
        }

        // Check for JavaScript syntax patterns
        if content.contains("{") && content.contains("}") && content.contains(";") {
            // Count braces and semicolons
            let brace_count = content.chars().filter(|&c| c == '{' || c == '}').count();
            let semicolon_count = content.chars().filter(|&c| c == ';').count();

            // If high density of braces/semicolons, it's code
            if brace_count > content.len() / 50 || semicolon_count > content.len() / 50 {
                return true;
            }
        }

        // Check if it starts with code patterns
        let code_starts = [",", "}", ")", ";", "let ", "const ", "var ", "function ", "async ", "return "];
        if code_starts.iter().any(|&start| trimmed.starts_with(start)) {
            return true;
        }

        false
    }

    /// Categorize the prompt based on content.
    fn categorize_prompt(&self, literal: &StringLiteralInfo) -> PromptCategory {
        let value = literal.value;

        if value.contains("You are Claude") || value.contains("answer the user") {
            PromptCategory::System
        } else if value.contains("tool") || value.contains("function") || value.contains("This tool") {
            PromptCategory::Tool
        } else if value.contains("Example:") || value.contains("<example>") {
            PromptCategory::Example
        } else if value.contains("Error:") || value.contains("error") {
            PromptCategory::Error
        } else if value.contains("IMPORTANT:") || value.contains("Usage") {
            PromptCategory::Instruction
        } else {
            PromptCategory::Other
        }
    }

    /// Associate prompts with tools based on content analysis.
    fn associate_tools(&self, prompts: Vec<EnhancedSystemPrompt>) -> Vec<EnhancedSystemPrompt> {
        let known_tools = [
            "Bash", "Read", "Write", "Edit", "Grep", "Glob", "Task",
            "TodoWrite", "NotebookEdit", "WebFetch", "WebSearch",
            "Skill", "SlashCommand", "AskUserQuestion", "ExitPlanMode",
            "BashOutput", "KillShell", "LSP",
        ];

        prompts
            .into_iter()
            .map(|mut prompt| {
                // Only process tool-category prompts
                if prompt.category == PromptCategory::Tool {
                    // Find which tool this prompt is about
                    for tool_name in &known_tools {
                        if self.is_tool_prompt_for(&prompt.content, tool_name) {
                            prompt.associated_tool = Some(tool_name.to_string());
                            prompt.context = PromptContext::ToolDocumentation {
                                tool_name: tool_name.to_string(),
                            };
                            trace!("Associated prompt {} with tool {}", prompt.id, tool_name);
                            break;
                        }
                    }
                }
                prompt
            })
            .collect()
    }

    /// Check if a prompt is describing a specific tool.
    fn is_tool_prompt_for(&self, content: &str, tool_name: &str) -> bool {
        let first_100 = content.chars().take(100).collect::<String>();

        // Use specific patterns for each tool (matching find_tool_specific_prompt logic)
        match tool_name {
            "Read" => content.starts_with("Reads a file from the local filesystem"),
            "Write" => content.starts_with("Writes a file to the local filesystem"),
            "Edit" => content.starts_with("Performs exact string replacements") ||
                     first_100.contains("exact string replacements in files"),
            "Bash" => content.starts_with("Executes a given bash command") ||
                     first_100.contains("Executes a given bash command"),
            "Grep" => first_100.contains("powerful search tool") &&
                     first_100.contains("ripgrep") &&
                     !first_100.contains("glob patterns like"),
            "Glob" => first_100.contains("Fast file pattern matching") ||
                     (first_100.contains("glob patterns") && first_100.contains("**/*.js")),
            "Task" => first_100.contains("launches specialized agents") ||
                     first_100.contains("Launch a new agent"),
            "TodoWrite" => content.starts_with("Use this tool to create and manage a structured task list"),
            "WebFetch" => first_100.contains("Fetches content from a specified URL"),
            "WebSearch" => first_100.contains("search the web"),
            "NotebookEdit" => first_100.contains("Jupyter notebook") && first_100.contains("cell"),
            "AskUserQuestion" => content.starts_with("Use this tool when you need to ask the user"),
            "Skill" => first_100.contains("Execute a skill within the main conversation"),
            "SlashCommand" => first_100.contains("Execute a slash command"),
            "ExitPlanMode" => first_100.contains("plan mode") && first_100.contains("ready to code"),
            "BashOutput" => first_100.contains("Retrieves output from a running"),
            "KillShell" => first_100.contains("Kills a running background bash"),
            "LSP" => content.starts_with("Interact with Language Server Protocol"),
            _ => {
                // Fallback: tool name mentioned early
                let first_200 = content.chars().take(200).collect::<String>();
                first_200.contains(tool_name) && content.len() > 200
            }
        }
    }

    /// Merge related fragments into complete prompts.
    fn merge_fragments(&self, prompts: Vec<EnhancedSystemPrompt>) -> Vec<EnhancedSystemPrompt> {
        let mut result = Vec::new();
        let mut used_indices = std::collections::HashSet::new();

        for (i, prompt) in prompts.iter().enumerate() {
            if used_indices.contains(&i) {
                continue;
            }

            // Look for fragments that should be merged with this prompt
            let mut merged_content = prompt.content.clone();
            let mut merged_ids = vec![prompt.id.clone()];

            // Check if this prompt looks incomplete (ends mid-sentence)
            if self.is_incomplete_fragment(&prompt.content) {
                // Look for continuation in subsequent prompts
                for (j, other) in prompts.iter().enumerate().skip(i + 1) {
                    if j - i > 5 {
                        break; // Only look within a small window
                    }

                    if used_indices.contains(&j) {
                        continue;
                    }

                    // Check if this could be a continuation
                    if self.is_continuation(&merged_content, &other.content) {
                        merged_content.push_str(&other.content);
                        merged_ids.push(other.id.clone());
                        used_indices.insert(j);
                        trace!("Merged fragment {} into {}", other.id, prompt.id);
                    }
                }
            }

            used_indices.insert(i);

            result.push(EnhancedSystemPrompt {
                id: prompt.id.clone(),
                content: merged_content.clone(),
                length: merged_content.len(),
                category: prompt.category.clone(),
                context: prompt.context.clone(),
                associated_tool: prompt.associated_tool.clone(),
                merged_fragments: if merged_ids.len() > 1 {
                    merged_ids
                } else {
                    Vec::new()
                },
            });
        }

        debug!("Merged {} prompts into {} complete prompts",
               prompts.len(), result.len());
        result
    }

    /// Check if a prompt fragment appears incomplete.
    fn is_incomplete_fragment(&self, content: &str) -> bool {
        let trimmed = content.trim();

        // Ends with incomplete sentence indicators
        let incomplete_endings = ["...", " -", " *", " `", "\\"];
        if incomplete_endings.iter().any(|&end| trimmed.ends_with(end)) {
            return true;
        }

        // Ends without proper punctuation
        let last_char = trimmed.chars().last();
        if let Some(ch) = last_char {
            if !matches!(ch, '.' | '!' | '?' | '"' | '\'' | ')' | ']' | '}' | '`') {
                // But check if it's not just a code snippet
                if !trimmed.ends_with("```") && !trimmed.ends_with("</example>") {
                    return true;
                }
            }
        }

        false
    }

    /// Check if one prompt is a continuation of another.
    fn is_continuation(&self, first: &str, second: &str) -> bool {
        let first_end = first.chars().rev().take(50).collect::<String>();
        let second_start = second.chars().take(50).collect::<String>();

        // Check for matching patterns that suggest continuation

        // Pattern 1: Similar indentation/formatting
        let first_lines: Vec<&str> = first.lines().rev().take(3).collect();
        let second_lines: Vec<&str> = second.lines().take(3).collect();

        if let (Some(last_line), Some(first_line)) = (first_lines.first(), second_lines.first()) {
            // Check if indentation matches
            let first_indent = last_line.len() - last_line.trim_start().len();
            let second_indent = first_line.len() - first_line.trim_start().len();

            if first_indent > 0 && (first_indent as i32 - second_indent as i32).abs() <= 2 {
                return true;
            }
        }

        // Pattern 2: Content similarity (same category/topic)
        if first_end.contains("Example") && second_start.contains("Example") {
            return true;
        }

        if first_end.contains("Usage") && second_start.contains("Usage") {
            return true;
        }

        false
    }

    /// Remove duplicate prompts.
    fn deduplicate(&self, prompts: Vec<EnhancedSystemPrompt>) -> Vec<EnhancedSystemPrompt> {
        let mut seen_content: HashMap<String, usize> = HashMap::new();
        let mut result: Vec<EnhancedSystemPrompt> = Vec::new();

        for prompt in prompts {
            // Use first 100 chars as key for deduplication
            let key = prompt.content.chars().take(100).collect::<String>();

            if let Some(&existing_idx) = seen_content.get(&key) {
                // If we already have this prompt, keep the longer one
                if prompt.content.len() > result[existing_idx].content.len() {
                    result[existing_idx] = prompt;
                }
            } else {
                seen_content.insert(key, result.len());
                result.push(prompt);
            }
        }

        debug!("Deduplicated to {} unique prompts", result.len());
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Parser;
    use oxc_allocator::Allocator;

    #[test]
    fn test_fragment_detection() {
        let code = r#"
            const incomplete = "This is a fragment that ends with";
            const complete = "This is a complete sentence.";
        "#;

        let allocator = Allocator::default();
        let parser = Parser::new(code.to_string());
        let parse_result = parser.parse(&allocator).unwrap();

        let analyzer = Analyzer::new(parse_result.program());
        let symbol_table = SymbolTable::new(parse_result.program());
        let extractor = EnhancedPromptExtractor::new(&analyzer, &symbol_table);

        assert!(extractor.is_incomplete_fragment("This is a fragment that ends with"));
        assert!(!extractor.is_incomplete_fragment("This is a complete sentence."));
    }

    #[test]
    fn test_tool_association() {
        let code = r#"
            const bashPrompt = "Executes bash commands in a persistent shell session. Use this tool for terminal operations.";
            const readPrompt = "Reads a file from the local filesystem. You can access any file directly.";
        "#;

        let allocator = Allocator::default();
        let parser = Parser::new(code.to_string());
        let parse_result = parser.parse(&allocator).unwrap();

        let analyzer = Analyzer::new(parse_result.program());
        let symbol_table = SymbolTable::new(parse_result.program());
        let extractor = EnhancedPromptExtractor::new(&analyzer, &symbol_table);

        assert!(extractor.is_tool_prompt_for("Executes bash commands", "Bash"));
        assert!(extractor.is_tool_prompt_for("Reads a file from the local", "Read"));
    }
}
