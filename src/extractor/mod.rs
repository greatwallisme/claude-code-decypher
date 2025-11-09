//! Extraction module for pulling structured data from JavaScript AST.

pub mod beautified_tools;
pub mod config;
pub mod prompts;
pub mod strings;
pub mod tools;

use crate::analyzer::Analyzer;
use crate::Result;

/// Main extractor that coordinates all extraction operations.
pub struct Extractor<'a> {
    analyzer: Analyzer<'a>,
}

impl<'a> Extractor<'a> {
    /// Create a new extractor.
    pub fn new(analyzer: Analyzer<'a>) -> Self {
        Self { analyzer }
    }

    /// Extract system prompts.
    pub fn extract_prompts(&self) -> Result<Vec<prompts::SystemPrompt>> {
        prompts::PromptExtractor::new(&self.analyzer).extract()
    }

    /// Extract tool definitions from AST.
    pub fn extract_tools(&self) -> Result<Vec<tools::ToolDefinition>> {
        tools::ToolExtractor::new(&self.analyzer).extract()
    }

    /// Extract tools from beautified code (more effective for minified bundles).
    pub fn extract_tools_from_beautified(&self, beautified_code: &str) -> Result<Vec<tools::ToolDefinition>> {
        beautified_tools::BeautifiedToolExtractor::with_ast(beautified_code, self.analyzer.program()).extract()
    }

    /// Extract tools from system prompts (best approach!).
    pub fn extract_tools_from_prompts(&self, prompts: &[prompts::SystemPrompt]) -> Result<Vec<tools::ToolDefinition>> {
        tools::ToolExtractor::extract_from_prompts(prompts)
    }

    /// Extract configuration values.
    pub fn extract_configs(&self) -> Result<Vec<config::ConfigValue>> {
        config::ConfigExtractor::new(&self.analyzer).extract()
    }

    /// Extract interesting string literals.
    pub fn extract_strings(&self) -> Result<Vec<strings::InterestingString>> {
        strings::StringExtractor::new(&self.analyzer).extract()
    }
}
