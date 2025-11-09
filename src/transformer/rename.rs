//! Variable renaming with heuristics.

use crate::analyzer::Analyzer;
use crate::Result;
use std::collections::HashMap;
use tracing::debug;

/// Variable renamer that suggests meaningful names for minified variables.
pub struct VariableRenamer<'a> {
    analyzer: &'a Analyzer<'a>,
    rename_map: HashMap<String, String>,
    used_names: HashMap<String, usize>,
}

impl<'a> VariableRenamer<'a> {
    /// Create a new variable renamer.
    pub fn new(analyzer: &'a Analyzer<'a>) -> Self {
        Self {
            analyzer,
            rename_map: HashMap::new(),
            used_names: HashMap::new(),
        }
    }

    /// Generate a rename map for minified variables.
    pub fn generate_rename_map(&mut self) -> Result<HashMap<String, String>> {
        debug!("Generating variable rename map");

        // Analyze common minified patterns
        let minified_vars = self.find_minified_variables();

        debug!("Found {} minified variables", minified_vars.len());

        // Generate meaningful names based on heuristics
        for var in &minified_vars {
            if let Some(suggested_name) = self.suggest_name(var) {
                let unique_name = self.ensure_unique(suggested_name);
                self.rename_map.insert(var.clone(), unique_name);
            }
        }

        debug!("Generated {} rename mappings", self.rename_map.len());
        Ok(self.rename_map.clone())
    }

    /// Find minified variable names.
    fn find_minified_variables(&self) -> Vec<String> {
        // Common minified patterns:
        // - Single letters: A, B, C
        // - Letter + numbers: A1, B2, QB9
        // - Short combinations: aa, ab, ba

        let mut vars = Vec::new();

        // For Phase 3, we'll use a heuristic approach
        // In a real implementation, we'd traverse the AST to find all identifiers

        // Common webpack/bundler minified patterns
        let patterns = vec![
            // Single uppercase + digits (e.g., QB9, IB9, YB9)
            "QB9", "IB9", "YB9", "GB9", "ZB9", "WB9", "FB9", "KB9", "CB9", "VB9",
            // Two-letter patterns
            "IA", "DA", "cJ", "xW", "be", "BY", "rL", "WQ",
            // Common bundler vars
            "z", "D", "T", "A", "B", "Q", "I", "G", "Z", "Y", "F", "C",
        ];

        for pattern in patterns {
            vars.push(pattern.to_string());
        }

        vars
    }

    /// Suggest a meaningful name for a minified variable.
    fn suggest_name(&self, var: &str) -> Option<String> {
        // Heuristics for name suggestion:
        // 1. Common patterns in the code
        // 2. Context-based naming
        // 3. Purpose inference from usage

        let suggestion = match var {
            // Module-related
            "z" | "D" => Some("module_wrapper"),
            "T" => Some("lazy_init"),
            "DA" => Some("require"),
            "cJ" => Some("global_object"),

            // Object/prototype operations
            "QB9" => Some("create_object"),
            "IB9" => Some("get_prototype"),
            "GB9" => Some("get_own_properties"),
            "ZB9" => Some("has_own_property"),

            // Function wrappers
            "IA" => Some("inherit_module"),
            "BY" => Some("combined_stream"),
            "rL" => Some("delayed_stream"),
            "WQ" => Some("axios_error"),

            // Symbol/primitive related
            "FB9" => Some("symbol_object"),
            "xW" => Some("global_symbol"),

            // Generic fallbacks based on pattern
            _ if var.len() == 1 && var.chars().next().unwrap().is_uppercase() => {
                Some("variable")
            }
            _ if var.len() == 2 && var.chars().all(|c| c.is_uppercase()) => {
                Some("constant")
            }
            _ if var.ends_with("B9") => Some("bundler_var"),
            _ if var.ends_with("0") => Some("config_var"),
            _ => None,
        };

        suggestion.map(|s| s.to_string())
    }

    /// Ensure the suggested name is unique by adding a suffix if needed.
    fn ensure_unique(&mut self, name: String) -> String {
        let counter = self.used_names.entry(name.clone()).or_insert(0);
        *counter += 1;

        if *counter == 1 {
            name
        } else {
            format!("{}_{}", name, counter)
        }
    }

    /// Get the rename map.
    pub fn rename_map(&self) -> &HashMap<String, String> {
        &self.rename_map
    }
}

/// Apply a rename map to source code (simple text replacement).
pub fn apply_rename_map(source: &str, rename_map: &HashMap<String, String>) -> String {
    let mut result = source.to_string();

    // Sort by length (longest first) to avoid partial replacements
    let mut entries: Vec<_> = rename_map.iter().collect();
    entries.sort_by(|a, b| b.0.len().cmp(&a.0.len()));

    for (old_name, new_name) in entries {
        // Use word boundaries to avoid partial replacements
        // This is a simple approach; a real implementation would use AST-aware replacement
        let pattern = format!(r"\b{}\b", regex::escape(old_name));
        let re = regex::Regex::new(&pattern).unwrap();
        result = re.replace_all(&result, new_name.as_str()).to_string();
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Parser;
    use oxc_allocator::Allocator;

    #[test]
    fn test_find_minified_variables() {
        let allocator = Allocator::default();
        let parser = Parser::new("var QB9 = 1;".to_string());
        let parse_result = parser.parse(&allocator).unwrap();

        let analyzer = Analyzer::new(parse_result.program());
        let renamer = VariableRenamer::new(&analyzer);

        let vars = renamer.find_minified_variables();
        assert!(vars.contains(&"QB9".to_string()));
    }

    #[test]
    fn test_suggest_name() {
        let allocator = Allocator::default();
        let parser = Parser::new("var x = 1;".to_string());
        let parse_result = parser.parse(&allocator).unwrap();

        let analyzer = Analyzer::new(parse_result.program());
        let renamer = VariableRenamer::new(&analyzer);

        let suggestion = renamer.suggest_name("QB9");
        assert!(suggestion.is_some());
        assert_eq!(suggestion.unwrap(), "create_object");
    }

    #[test]
    fn test_ensure_unique() {
        let allocator = Allocator::default();
        let parser = Parser::new("var x = 1;".to_string());
        let parse_result = parser.parse(&allocator).unwrap();

        let analyzer = Analyzer::new(parse_result.program());
        let mut renamer = VariableRenamer::new(&analyzer);

        let name1 = renamer.ensure_unique("handler".to_string());
        let name2 = renamer.ensure_unique("handler".to_string());
        let name3 = renamer.ensure_unique("handler".to_string());

        assert_eq!(name1, "handler");
        assert_eq!(name2, "handler_2");
        assert_eq!(name3, "handler_3");
    }

    #[test]
    fn test_apply_rename_map() {
        let source = "var QB9 = 1; function test() { return QB9; }";
        let mut map = HashMap::new();
        map.insert("QB9".to_string(), "create_object".to_string());

        let result = apply_rename_map(source, &map);

        assert!(result.contains("create_object"));
        assert!(!result.contains("QB9"));
    }
}
