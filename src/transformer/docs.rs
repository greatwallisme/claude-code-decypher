//! Module documentation generation.

use crate::transformer::split::Module;
use crate::error::DecypherError;
use crate::Result;
use std::fs;
use std::path::Path;
use tracing::info;

/// Documentation generator for modules.
pub struct DocsGenerator;

impl DocsGenerator {
    /// Generate documentation for all modules.
    pub fn generate(modules: &[Module], output_dir: &Path) -> Result<()> {
        let docs_dir = output_dir.join("docs");
        fs::create_dir_all(&docs_dir)
            .map_err(|e| DecypherError::io(&docs_dir, e))?;

        // Generate modules.md
        Self::generate_modules_doc(modules, &docs_dir)?;

        // Generate architecture.md
        Self::generate_architecture_doc(modules, &docs_dir)?;

        info!("Generated documentation in {}", docs_dir.display());
        Ok(())
    }

    /// Generate modules.md documentation.
    fn generate_modules_doc(modules: &[Module], docs_dir: &Path) -> Result<()> {
        let mut content = String::from("# Module Documentation\n\n");
        content.push_str("This document describes the modules extracted from the Claude Code bundle.\n\n");

        content.push_str("## Overview\n\n");
        content.push_str(&format!("Total modules: {}\n\n", modules.len()));

        content.push_str("| Module | Category | Est. Lines | Functions |\n");
        content.push_str("|--------|----------|------------|----------|\n");

        for module in modules {
            content.push_str(&format!(
                "| {} | {:?} | {} | {} |\n",
                module.name,
                module.category,
                module.estimated_lines,
                module.functions.len()
            ));
        }

        content.push_str("\n## Module Details\n\n");

        for module in modules {
            content.push_str(&format!("### {} Module\n\n", module.name));
            content.push_str(&format!("**Category:** {:?}\n\n", module.category));
            content.push_str(&format!("**Estimated Lines:** {}\n\n", module.estimated_lines));

            if !module.functions.is_empty() {
                content.push_str("**Functions:**\n");
                for func in &module.functions {
                    content.push_str(&format!("- `{}`\n", func));
                }
                content.push_str("\n");
            }

            if !module.keywords.is_empty() {
                content.push_str("**Keywords:**\n");
                for keyword in &module.keywords {
                    content.push_str(&format!("- {}\n", keyword));
                }
                content.push_str("\n");
            }

            content.push_str("---\n\n");
        }

        let path = docs_dir.join("modules.md");
        fs::write(&path, content)
            .map_err(|e| DecypherError::io(&path, e))?;

        info!("Generated modules.md");
        Ok(())
    }

    /// Generate architecture.md documentation.
    fn generate_architecture_doc(modules: &[Module], docs_dir: &Path) -> Result<()> {
        let mut content = String::from("# Claude Code Architecture\n\n");
        content.push_str("This document provides an architectural overview of the Claude Code bundle.\n\n");

        content.push_str("## Module Structure\n\n");
        content.push_str("```\n");
        content.push_str("claude-code/\n");

        for module in modules {
            content.push_str(&format!("├── {}/\n", module.name));
            for func in &module.functions {
                content.push_str(&format!("│   ├── {}.js\n", func));
            }
        }

        content.push_str("```\n\n");

        content.push_str("## Module Categories\n\n");

        // Group by category
        let mut by_category: std::collections::HashMap<String, Vec<&Module>> =
            std::collections::HashMap::new();

        for module in modules {
            by_category
                .entry(format!("{:?}", module.category))
                .or_default()
                .push(module);
        }

        for (category, mods) in &by_category {
            content.push_str(&format!("### {}\n\n", category));
            for module in mods {
                content.push_str(&format!("- **{}**: {} estimated lines\n", module.name, module.estimated_lines));
            }
            content.push_str("\n");
        }

        content.push_str("## Statistics\n\n");
        content.push_str(&format!("- Total Modules: {}\n", modules.len()));

        let total_lines: usize = modules.iter().map(|m| m.estimated_lines).sum();
        content.push_str(&format!("- Total Estimated Lines: {}\n", total_lines));

        let total_functions: usize = modules.iter().map(|m| m.functions.len()).sum();
        content.push_str(&format!("- Total Functions: {}\n", total_functions));

        content.push_str("\n## Design Patterns\n\n");
        content.push_str("Based on the analysis, the following design patterns were identified:\n\n");
        content.push_str("- **Module Pattern**: Extensive use of module wrappers and lazy initialization\n");
        content.push_str("- **Tool System**: Pluggable tool architecture for commands like Bash, Read, Write\n");
        content.push_str("- **Hook System**: Pre/post execution hooks for tool usage\n");
        content.push_str("- **Telemetry**: Comprehensive metrics and usage tracking\n");

        let path = docs_dir.join("architecture.md");
        fs::write(&path, content)
            .map_err(|e| DecypherError::io(&path, e))?;

        info!("Generated architecture.md");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_generate_docs() -> Result<()> {
        let modules = vec![
            Module {
                name: "core".to_string(),
                category: crate::transformer::split::ModuleCategory::Core,
                estimated_lines: 1000,
                functions: vec!["main".to_string()],
                keywords: vec![],
            },
            Module {
                name: "tools".to_string(),
                category: crate::transformer::split::ModuleCategory::Tools,
                estimated_lines: 500,
                functions: vec!["bash".to_string()],
                keywords: vec![],
            },
        ];

        let temp_dir = TempDir::new()
            .map_err(|e| DecypherError::Other(e.into()))?;

        DocsGenerator::generate(&modules, temp_dir.path())?;

        // Verify files were created
        assert!(temp_dir.path().join("docs/modules.md").exists());
        assert!(temp_dir.path().join("docs/architecture.md").exists());

        Ok(())
    }
}
