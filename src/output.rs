//! Output module for writing extraction results.

use crate::extractor::{config::ConfigValue, prompts::SystemPrompt, strings::InterestingString, tools::ToolDefinition};
use crate::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{debug, info};

/// Writer for extraction results.
pub struct OutputWriter {
    output_dir: PathBuf,
}

impl OutputWriter {
    /// Create a new output writer.
    pub fn new(output_dir: impl AsRef<Path>) -> Self {
        Self {
            output_dir: output_dir.as_ref().to_path_buf(),
        }
    }

    /// Create the output directory structure.
    pub fn create_structure(&self) -> Result<()> {
        let extracted_dir = self.output_dir.join("extracted");
        fs::create_dir_all(&extracted_dir)
            .map_err(|e| crate::error::DecypherError::io(&extracted_dir, e))?;
        info!("Created output directory: {}", extracted_dir.display());
        Ok(())
    }

    /// Write system prompts to JSON file.
    pub fn write_prompts(&self, prompts: &[SystemPrompt]) -> Result<()> {
        let path = self.output_dir.join("extracted/system-prompts.json");
        self.write_json(&path, prompts)?;
        info!("Wrote {} prompts to {}", prompts.len(), path.display());
        Ok(())
    }

    /// Write tool definitions to JSON file.
    pub fn write_tools(&self, tools: &[ToolDefinition]) -> Result<()> {
        let path = self.output_dir.join("extracted/tool-definitions.json");
        self.write_json(&path, tools)?;
        info!("Wrote {} tools to {}", tools.len(), path.display());
        Ok(())
    }

    /// Write configuration values to JSON file.
    pub fn write_configs(&self, configs: &[ConfigValue]) -> Result<()> {
        let path = self.output_dir.join("extracted/configurations.json");
        self.write_json(&path, configs)?;
        info!("Wrote {} configs to {}", configs.len(), path.display());
        Ok(())
    }

    /// Write interesting strings to JSON file.
    pub fn write_strings(&self, strings: &[InterestingString]) -> Result<()> {
        let path = self.output_dir.join("extracted/strings.json");
        self.write_json(&path, strings)?;
        info!("Wrote {} strings to {}", strings.len(), path.display());
        Ok(())
    }

    /// Write a summary of all extractions.
    pub fn write_summary(&self, summary: &ExtractionSummary) -> Result<()> {
        let path = self.output_dir.join("extracted/summary.json");
        self.write_json(&path, summary)?;
        info!("Wrote extraction summary to {}", path.display());
        Ok(())
    }

    /// Helper to write JSON to a file.
    fn write_json<T: Serialize + ?Sized>(&self, path: &Path, data: &T) -> Result<()> {
        let json = serde_json::to_string_pretty(data)
            .map_err(|e| crate::error::DecypherError::Other(e.into()))?;
        fs::write(path, json)
            .map_err(|e| crate::error::DecypherError::io(path, e))?;
        debug!("Wrote JSON to {}", path.display());
        Ok(())
    }
}

/// Summary of extraction results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionSummary {
    /// Number of prompts extracted.
    pub prompt_count: usize,

    /// Number of tools extracted.
    pub tool_count: usize,

    /// Number of configs extracted.
    pub config_count: usize,

    /// Number of strings extracted.
    pub string_count: usize,

    /// Longest prompt found.
    pub longest_prompt: usize,

    /// Categories breakdown for prompts.
    pub prompt_categories: std::collections::HashMap<String, usize>,

    /// Categories breakdown for configs.
    pub config_categories: std::collections::HashMap<String, usize>,
}

impl ExtractionSummary {
    /// Create a summary from extraction results.
    pub fn new(
        prompts: &[SystemPrompt],
        tools: &[ToolDefinition],
        configs: &[ConfigValue],
        strings: &[InterestingString],
    ) -> Self {
        use std::collections::HashMap;

        let mut prompt_categories = HashMap::new();
        for prompt in prompts {
            let category = format!("{:?}", prompt.category);
            *prompt_categories.entry(category).or_insert(0) += 1;
        }

        let mut config_categories = HashMap::new();
        for config in configs {
            let category = format!("{:?}", config.category);
            *config_categories.entry(category).or_insert(0) += 1;
        }

        let longest_prompt = prompts.iter().map(|p| p.length).max().unwrap_or(0);

        Self {
            prompt_count: prompts.len(),
            tool_count: tools.len(),
            config_count: configs.len(),
            string_count: strings.len(),
            longest_prompt,
            prompt_categories,
            config_categories,
        }
    }

    /// Print a summary to stdout.
    pub fn print(&self) {
        println!("\n=== Extraction Summary ===\n");
        println!("System Prompts:     {}", self.prompt_count);
        println!("Tool Definitions:   {}", self.tool_count);
        println!("Configuration:      {}", self.config_count);
        println!("Interesting Strings: {}", self.string_count);
        println!("Longest Prompt:     {} chars", self.longest_prompt);

        if !self.prompt_categories.is_empty() {
            println!("\nPrompt Categories:");
            for (category, count) in &self.prompt_categories {
                println!("  {:<15} {}", category, count);
            }
        }

        if !self.config_categories.is_empty() {
            println!("\nConfig Categories:");
            for (category, count) in &self.config_categories {
                println!("  {:<15} {}", category, count);
            }
        }
    }
}
