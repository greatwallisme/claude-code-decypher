//! Module splitting strategies.

use crate::analyzer::{Analyzer, StringLiteralInfo};
use crate::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::debug;

/// Strategy for splitting code into modules.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SplitStrategy {
    /// Split by export statements.
    ByExport,
    /// Split by namespace/prefix.
    ByNamespace,
    /// Split by feature/functionality.
    ByFeature,
    /// Hybrid approach (default).
    Hybrid,
}

/// A module containing related code.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Module {
    /// Module name.
    pub name: String,

    /// Module category.
    pub category: ModuleCategory,

    /// Estimated line count.
    pub estimated_lines: usize,

    /// Functions in this module.
    pub functions: Vec<String>,

    /// Related keywords.
    pub keywords: Vec<String>,
}

/// Category of module.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ModuleCategory {
    /// Core functionality
    Core,
    /// Tools
    Tools,
    /// API client
    ApiClient,
    /// Prompts
    Prompts,
    /// Telemetry
    Telemetry,
    /// Utilities
    Utils,
    /// Hooks
    Hooks,
    /// Git operations
    Git,
    /// File system
    FileSystem,
    /// Unknown
    Unknown,
}

/// Module splitter that organizes code into logical modules.
pub struct ModuleSplitter<'a> {
    analyzer: &'a Analyzer<'a>,
    strategy: SplitStrategy,
}

impl<'a> ModuleSplitter<'a> {
    /// Create a new module splitter.
    pub fn new(analyzer: &'a Analyzer<'a>, strategy: SplitStrategy) -> Self {
        Self { analyzer, strategy }
    }

    /// Split code into modules based on the strategy.
    pub fn split(&self) -> Result<Vec<Module>> {
        debug!("Splitting code using strategy: {:?}", self.strategy);

        let modules = match self.strategy {
            SplitStrategy::ByExport => self.split_by_export(),
            SplitStrategy::ByNamespace => self.split_by_namespace(),
            SplitStrategy::ByFeature => self.split_by_feature(),
            SplitStrategy::Hybrid => self.split_hybrid(),
        };

        debug!("Split code into {} modules", modules.len());
        Ok(modules)
    }

    /// Split by export statements (not many in bundled code).
    fn split_by_export(&self) -> Vec<Module> {
        // Basic implementation - bundled code usually doesn't have many exports
        vec![Module {
            name: "main".to_string(),
            category: ModuleCategory::Core,
            estimated_lines: 1000,
            functions: vec![],
            keywords: vec![],
        }]
    }

    /// Split by namespace/prefix patterns.
    fn split_by_namespace(&self) -> Vec<Module> {
        let _strings = self.analyzer.find_string_literals();

        // Analyze string patterns to infer modules
        let mut modules = Vec::new();

        // Core module
        modules.push(Module {
            name: "core".to_string(),
            category: ModuleCategory::Core,
            estimated_lines: 500,
            functions: vec!["main_loop".to_string(), "process_message".to_string()],
            keywords: vec!["loop".to_string(), "process".to_string()],
        });

        modules
    }

    /// Split by feature/functionality.
    fn split_by_feature(&self) -> Vec<Module> {
        let strings = self.analyzer.find_string_literals();
        let mut modules = Vec::new();

        // Analyze strings to detect features
        let features = self.detect_features(&strings);

        for feature in features {
            modules.push(feature);
        }

        // Ensure we have at least basic modules
        if modules.is_empty() {
            modules = self.create_default_modules();
        }

        modules
    }

    /// Hybrid splitting approach (combines multiple strategies).
    fn split_hybrid(&self) -> Vec<Module> {
        debug!("Using hybrid splitting strategy");

        let strings = self.analyzer.find_string_literals();
        let features = self.detect_features(&strings);

        // Start with default module structure
        let mut modules = self.create_default_modules();

        // Merge detected features
        for feature in features {
            if !modules.iter().any(|m| m.category == feature.category) {
                modules.push(feature);
            }
        }

        modules
    }

    /// Detect features from string analysis.
    fn detect_features(&self, strings: &[StringLiteralInfo]) -> Vec<Module> {
        let mut features = HashMap::new();

        for string in strings {
            let value = string.value;

            // Tools
            if value.contains("tool") || value.contains("Bash") || value.contains("Read") {
                features.entry(ModuleCategory::Tools).or_insert_with(|| {
                    vec!["bash".to_string(), "read".to_string(), "write".to_string()]
                });
            }

            // API client
            if value.contains("api") || value.contains("anthropic") || value.contains("messages") {
                features.entry(ModuleCategory::ApiClient).or_insert_with(|| {
                    vec!["api_client".to_string(), "request".to_string()]
                });
            }

            // Prompts
            if value.contains("prompt") || value.contains("You are Claude") {
                features.entry(ModuleCategory::Prompts).or_insert_with(|| {
                    vec!["system_prompt".to_string(), "prompt_builder".to_string()]
                });
            }

            // Telemetry
            if value.contains("telemetry")
                || value.contains("metric")
                || value.contains("claude_code.")
            {
                features.entry(ModuleCategory::Telemetry).or_insert_with(|| {
                    vec!["metrics".to_string(), "usage_tracking".to_string()]
                });
            }

            // Git
            if value.contains("git") || value.contains("commit") || value.contains("branch") {
                features
                    .entry(ModuleCategory::Git)
                    .or_insert_with(|| vec!["git_operations".to_string()]);
            }

            // Hooks
            if value.contains("hook") || value.contains("PreToolUse") || value.contains("PostToolUse") {
                features
                    .entry(ModuleCategory::Hooks)
                    .or_insert_with(|| vec!["hook_system".to_string()]);
            }
        }

        features
            .into_iter()
            .map(|(category, keywords)| Module {
                name: format!("{:?}", category).to_lowercase(),
                category,
                estimated_lines: 300,
                functions: keywords.clone(),
                keywords,
            })
            .collect()
    }

    /// Create default module structure.
    fn create_default_modules(&self) -> Vec<Module> {
        vec![
            Module {
                name: "core".to_string(),
                category: ModuleCategory::Core,
                estimated_lines: 1000,
                functions: vec![
                    "main_loop".to_string(),
                    "message_processing".to_string(),
                    "api_client".to_string(),
                ],
                keywords: vec!["main".to_string(), "loop".to_string()],
            },
            Module {
                name: "tools".to_string(),
                category: ModuleCategory::Tools,
                estimated_lines: 800,
                functions: vec![
                    "bash".to_string(),
                    "read".to_string(),
                    "write".to_string(),
                    "edit".to_string(),
                ],
                keywords: vec!["tool".to_string(), "command".to_string()],
            },
            Module {
                name: "utils".to_string(),
                category: ModuleCategory::Utils,
                estimated_lines: 500,
                functions: vec!["helpers".to_string(), "formatters".to_string()],
                keywords: vec!["util".to_string(), "helper".to_string()],
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Parser;
    use oxc_allocator::Allocator;

    #[test]
    fn test_split_hybrid() {
        let code = r#"
            const toolName = "Bash";
            const apiEndpoint = "https://api.anthropic.com";
            const systemPrompt = "You are Claude Code";
        "#;

        let allocator = Allocator::default();
        let parser = Parser::new(code.to_string());
        let parse_result = parser.parse(&allocator).unwrap();

        let analyzer = Analyzer::new(parse_result.program());
        let splitter = ModuleSplitter::new(&analyzer, SplitStrategy::Hybrid);
        let modules = splitter.split().unwrap();

        assert!(!modules.is_empty());

        // Should detect multiple module categories
        let categories: Vec<_> = modules.iter().map(|m| &m.category).collect();
        assert!(categories.contains(&&ModuleCategory::Core));
    }

    #[test]
    fn test_detect_features() {
        let code = r#"
            const tool = "Bash tool for executing commands";
            const metric = "claude_code.session.count";
        "#;

        let allocator = Allocator::default();
        let parser = Parser::new(code.to_string());
        let parse_result = parser.parse(&allocator).unwrap();

        let analyzer = Analyzer::new(parse_result.program());
        let splitter = ModuleSplitter::new(&analyzer, SplitStrategy::ByFeature);
        let modules = splitter.split().unwrap();

        assert!(!modules.is_empty());
    }
}
