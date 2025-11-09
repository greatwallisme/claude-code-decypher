//! Configuration value extraction.

use crate::analyzer::{Analyzer, StringLiteralInfo};
use crate::Result;
use serde::{Deserialize, Serialize};
use tracing::debug;

/// A configuration value found in the code.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigValue {
    /// Configuration key/name.
    pub key: String,

    /// Configuration value.
    pub value: String,

    /// Type of value.
    pub value_type: ConfigType,

    /// Category.
    pub category: ConfigCategory,
}

/// Type of configuration value.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ConfigType {
    String,
    Number,
    Boolean,
    Object,
    Array,
}

/// Category of configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ConfigCategory {
    Model,
    API,
    Telemetry,
    Path,
    Timeout,
    Feature,
    Other,
}

/// Extractor for configuration values.
pub struct ConfigExtractor<'a> {
    analyzer: &'a Analyzer<'a>,
}

impl<'a> ConfigExtractor<'a> {
    /// Create a new config extractor.
    pub fn new(analyzer: &'a Analyzer<'a>) -> Self {
        Self { analyzer }
    }

    /// Extract all configuration values.
    pub fn extract(&self) -> Result<Vec<ConfigValue>> {
        debug!("Extracting configuration values");

        let string_literals = self.analyzer.find_string_literals();
        let mut configs = Vec::new();

        for (idx, literal) in string_literals.iter().enumerate() {
            if self.is_likely_config(literal) {
                let category = self.categorize_config(literal);
                let config = ConfigValue {
                    key: format!("config_{}", idx),
                    value: literal.value.to_string(),
                    value_type: ConfigType::String,
                    category,
                };
                configs.push(config);
            }
        }

        debug!("Extracted {} configuration values", configs.len());
        Ok(configs)
    }

    /// Check if a string literal is likely a config value.
    fn is_likely_config(&self, literal: &StringLiteralInfo) -> bool {
        let value = literal.value;

        // Configuration indicators
        let config_patterns = [
            "claude-sonnet",
            "claude-opus",
            "anthropic",
            ".com/",
            "http://",
            "https://",
            "/api/",
            "VERSION",
            "CLAUDE_",
            "API_KEY",
        ];

        // Should be reasonably short (not a prompt)
        if literal.length > 200 {
            return false;
        }

        config_patterns.iter().any(|&pattern| value.contains(pattern))
    }

    /// Categorize the configuration value.
    fn categorize_config(&self, literal: &StringLiteralInfo) -> ConfigCategory {
        let value = literal.value;

        if value.contains("sonnet") || value.contains("opus") || value.contains("haiku") {
            ConfigCategory::Model
        } else if value.contains("/api/") || value.contains("anthropic.com") {
            ConfigCategory::API
        } else if value.contains("telemetry") || value.contains("metric") {
            ConfigCategory::Telemetry
        } else if value.contains("/") || value.contains("\\") {
            ConfigCategory::Path
        } else if value.contains("timeout") || value.contains("ms") {
            ConfigCategory::Timeout
        } else if value.contains("feature") || value.contains("flag") {
            ConfigCategory::Feature
        } else {
            ConfigCategory::Other
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Parser;
    use oxc_allocator::Allocator;

    #[test]
    fn test_extract_model_config() {
        let code = r#"
            const model = "claude-sonnet-4-5-20250929";
        "#;

        let allocator = Allocator::default();
        let parser = Parser::new(code.to_string());
        let parse_result = parser.parse(&allocator).unwrap();

        let analyzer = Analyzer::new(parse_result.program());
        let extractor = ConfigExtractor::new(&analyzer);
        let configs = extractor.extract().unwrap();

        assert!(!configs.is_empty());
        assert_eq!(configs[0].category, ConfigCategory::Model);
    }

    #[test]
    fn test_extract_api_config() {
        let code = r#"
            const endpoint = "https://api.anthropic.com/v1/messages";
        "#;

        let allocator = Allocator::default();
        let parser = Parser::new(code.to_string());
        let parse_result = parser.parse(&allocator).unwrap();

        let analyzer = Analyzer::new(parse_result.program());
        let extractor = ConfigExtractor::new(&analyzer);
        let configs = extractor.extract().unwrap();

        assert!(!configs.is_empty());
        assert_eq!(configs[0].category, ConfigCategory::API);
    }
}
