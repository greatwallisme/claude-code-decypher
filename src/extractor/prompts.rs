//! System prompt extraction.

use crate::analyzer::{Analyzer, StringLiteralInfo};
use crate::Result;
use serde::{Deserialize, Serialize};
use tracing::debug;

/// A system prompt found in the code.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemPrompt {
    /// Unique identifier for this prompt.
    pub id: String,

    /// The prompt content.
    pub content: String,

    /// Length of the prompt.
    pub length: usize,

    /// Category/type of prompt.
    pub category: PromptCategory,
}

/// Category of system prompt.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PromptCategory {
    /// Main system prompt
    System,
    /// Tool description
    Tool,
    /// User instruction
    Instruction,
    /// Example
    Example,
    /// Error message
    Error,
    /// Other/Unknown
    Other,
}

/// Extractor for system prompts.
pub struct PromptExtractor<'a> {
    analyzer: &'a Analyzer<'a>,
}

impl<'a> PromptExtractor<'a> {
    /// Create a new prompt extractor.
    pub fn new(analyzer: &'a Analyzer<'a>) -> Self {
        Self { analyzer }
    }

    /// Extract all system prompts.
    pub fn extract(&self) -> Result<Vec<SystemPrompt>> {
        debug!("Extracting system prompts");

        let string_literals = self.analyzer.find_string_literals();
        let mut prompts = Vec::new();

        for (idx, literal) in string_literals.iter().enumerate() {
            if self.is_likely_prompt(literal) {
                let category = self.categorize_prompt(literal);
                let prompt = SystemPrompt {
                    id: format!("prompt_{}", idx),
                    content: literal.value.to_string(),
                    length: literal.length,
                    category,
                };
                prompts.push(prompt);
            }
        }

        debug!("Extracted {} system prompts", prompts.len());
        Ok(prompts)
    }

    /// Check if a string literal is likely a system prompt.
    fn is_likely_prompt(&self, literal: &StringLiteralInfo) -> bool {
        // Heuristics for identifying prompts:
        // 1. Length > 80 chars (lowered threshold to catch more)
        // 2. Contains prompt-related keywords
        // 3. Has instruction-like language
        // 4. Multi-line or has markdown formatting

        if literal.length < 80 {
            return false;
        }

        let value = literal.value;

        // Check for key phrases (expanded list)
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
            "# ",
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
            "\n\n", // Multi-paragraph
        ];

        prompt_indicators.iter().any(|&indicator| value.contains(indicator))
    }

    /// Categorize the prompt based on content.
    fn categorize_prompt(&self, literal: &StringLiteralInfo) -> PromptCategory {
        let value = literal.value;

        if value.contains("You are Claude") || value.contains("answer the user") {
            PromptCategory::System
        } else if value.contains("tool") || value.contains("function") {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Parser;
    use oxc_allocator::Allocator;

    #[test]
    fn test_extract_system_prompt() {
        let code = r#"
            const systemPrompt = "You are Claude Code, Anthropic's official CLI for Claude. You help users with software engineering tasks.";
        "#;

        let allocator = Allocator::default();
        let parser = Parser::new(code.to_string());
        let parse_result = parser.parse(&allocator).unwrap();

        let analyzer = Analyzer::new(parse_result.program());
        let extractor = PromptExtractor::new(&analyzer);
        let prompts = extractor.extract().unwrap();

        assert_eq!(prompts.len(), 1);
        assert!(prompts[0].content.contains("You are Claude"));
        assert_eq!(prompts[0].category, PromptCategory::System);
    }

    #[test]
    fn test_ignore_short_strings() {
        let code = r#"
            const short = "hello";
            const alsoShort = "world";
        "#;

        let allocator = Allocator::default();
        let parser = Parser::new(code.to_string());
        let parse_result = parser.parse(&allocator).unwrap();

        let analyzer = Analyzer::new(parse_result.program());
        let extractor = PromptExtractor::new(&analyzer);
        let prompts = extractor.extract().unwrap();

        assert_eq!(prompts.len(), 0);
    }

    #[test]
    fn test_categorize_tool_prompt() {
        let code = r#"
            const toolDesc = "This tool allows you to execute bash commands in a persistent shell session with optional timeout. Use this for terminal operations.";
        "#;

        let allocator = Allocator::default();
        let parser = Parser::new(code.to_string());
        let parse_result = parser.parse(&allocator).unwrap();

        let analyzer = Analyzer::new(parse_result.program());
        let extractor = PromptExtractor::new(&analyzer);
        let prompts = extractor.extract().unwrap();

        // Should find the tool description prompt
        assert!(!prompts.is_empty(), "Should find at least one prompt");
        assert_eq!(prompts[0].category, PromptCategory::Tool);
    }
}
