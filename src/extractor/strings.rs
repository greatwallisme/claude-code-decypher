//! Interesting string literal extraction.

use crate::analyzer::{Analyzer, StringLiteralInfo};
use crate::Result;
use serde::{Deserialize, Serialize};
use tracing::debug;

/// An interesting string literal found in the code.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterestingString {
    /// The string value.
    pub value: String,

    /// Length of the string.
    pub length: usize,

    /// Category of the string.
    pub category: StringCategory,

    /// Relevance score (0.0-1.0).
    pub relevance: f32,
}

/// Category of interesting string.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum StringCategory {
    /// URL or endpoint
    Url,
    /// File path
    Path,
    /// Error message
    ErrorMessage,
    /// Log message
    LogMessage,
    /// Documentation
    Documentation,
    /// Code snippet
    CodeSnippet,
    /// Other
    Other,
}

/// Extractor for interesting strings.
pub struct StringExtractor<'a> {
    analyzer: &'a Analyzer<'a>,
}

impl<'a> StringExtractor<'a> {
    /// Create a new string extractor.
    pub fn new(analyzer: &'a Analyzer<'a>) -> Self {
        Self { analyzer }
    }

    /// Extract all interesting strings.
    pub fn extract(&self) -> Result<Vec<InterestingString>> {
        debug!("Extracting interesting strings");

        let string_literals = self.analyzer.find_string_literals();
        let mut strings = Vec::new();

        for literal in &string_literals {
            if let Some(interesting) = self.classify_string(literal) {
                strings.push(interesting);
            }
        }

        // Sort by relevance
        strings.sort_by(|a, b| b.relevance.partial_cmp(&a.relevance).unwrap());

        // Limit to top 1000 most relevant strings
        strings.truncate(1000);

        debug!("Extracted {} interesting strings", strings.len());
        Ok(strings)
    }

    /// Classify a string and determine if it's interesting.
    fn classify_string(&self, literal: &StringLiteralInfo) -> Option<InterestingString> {
        let value = literal.value;
        let length = literal.length;

        // Ignore very short strings
        if length < 5 {
            return None;
        }

        let (category, relevance) = self.categorize_and_score(value, length);

        // Only include if relevance is above threshold
        if relevance < 0.3 {
            return None;
        }

        Some(InterestingString {
            value: value.to_string(),
            length,
            category,
            relevance,
        })
    }

    /// Categorize and score a string.
    fn categorize_and_score(&self, value: &str, length: usize) -> (StringCategory, f32) {
        let mut score = 0.0;
        let mut category = StringCategory::Other;

        // URL detection
        if value.starts_with("http://") || value.starts_with("https://") {
            category = StringCategory::Url;
            score = 0.9;
        }
        // Path detection
        else if value.starts_with('/') || value.contains("\\") || value.contains("./") {
            category = StringCategory::Path;
            score = 0.7;
        }
        // Error message
        else if value.starts_with("Error:") || value.starts_with("Failed") {
            category = StringCategory::ErrorMessage;
            score = 0.8;
        }
        // Log message
        else if value.contains("[INFO]")
            || value.contains("[ERROR]")
            || value.contains("[DEBUG]")
        {
            category = StringCategory::LogMessage;
            score = 0.6;
        }
        // Documentation
        else if length > 50 && (value.contains("/**") || value.contains("///")) {
            category = StringCategory::Documentation;
            score = 0.5;
        }
        // Code snippet
        else if value.contains("function") || value.contains("const ") || value.contains("=>") {
            category = StringCategory::CodeSnippet;
            score = 0.4;
        }
        // Other interesting patterns
        else if length > 20 && length < 200 {
            score = 0.3;
        }

        (category, score)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Parser;
    use oxc_allocator::Allocator;

    #[test]
    fn test_extract_urls() {
        let code = r#"
            const url = "https://api.anthropic.com/v1/messages";
        "#;

        let allocator = Allocator::default();
        let parser = Parser::new(code.to_string());
        let parse_result = parser.parse(&allocator).unwrap();

        let analyzer = Analyzer::new(parse_result.program());
        let extractor = StringExtractor::new(&analyzer);
        let strings = extractor.extract().unwrap();

        assert!(!strings.is_empty());
        assert_eq!(strings[0].category, StringCategory::Url);
        assert!(strings[0].relevance >= 0.9);
    }

    #[test]
    fn test_extract_paths() {
        let code = r#"
            const path = "/usr/local/bin/claude";
        "#;

        let allocator = Allocator::default();
        let parser = Parser::new(code.to_string());
        let parse_result = parser.parse(&allocator).unwrap();

        let analyzer = Analyzer::new(parse_result.program());
        let extractor = StringExtractor::new(&analyzer);
        let strings = extractor.extract().unwrap();

        assert!(!strings.is_empty());
        assert_eq!(strings[0].category, StringCategory::Path);
    }

    #[test]
    fn test_ignore_short_strings() {
        let code = r#"
            const x = "a";
            const y = "b";
        "#;

        let allocator = Allocator::default();
        let parser = Parser::new(code.to_string());
        let parse_result = parser.parse(&allocator).unwrap();

        let analyzer = Analyzer::new(parse_result.program());
        let extractor = StringExtractor::new(&analyzer);
        let strings = extractor.extract().unwrap();

        assert_eq!(strings.len(), 0);
    }
}
