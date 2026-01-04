//! Extraction module for pulling structured data from JavaScript AST.

pub mod beautified_tools;
pub mod config;
pub mod prompts;
pub mod prompts_enhanced;
pub mod schemas;
pub mod strings;
pub mod tools;

use crate::analyzer::{Analyzer, SymbolTable};
use crate::Result;
use tracing::{debug, trace};

/// Main extractor that coordinates all extraction operations.
pub struct Extractor<'a> {
    analyzer: Analyzer<'a>,
}

impl<'a> Extractor<'a> {
    /// Create a new extractor.
    pub fn new(analyzer: Analyzer<'a>) -> Self {
        Self { analyzer }
    }

    /// Extract system prompts (legacy method).
    pub fn extract_prompts(&self) -> Result<Vec<prompts::SystemPrompt>> {
        prompts::PromptExtractor::new(&self.analyzer).extract()
    }

    /// Extract enhanced system prompts with fragment merging and tool association.
    pub fn extract_prompts_enhanced(&self) -> Result<Vec<prompts_enhanced::EnhancedSystemPrompt>> {
        let symbol_table = SymbolTable::new(self.analyzer.program());
        prompts_enhanced::EnhancedPromptExtractor::new(&self.analyzer, &symbol_table).extract()
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

    /// Extract tools with enhanced descriptions from enhanced prompts.
    pub fn extract_tools_with_enhanced_prompts(
        &self,
        enhanced_prompts: &[prompts_enhanced::EnhancedSystemPrompt],
    ) -> Result<Vec<tools::ToolDefinition>> {
        debug!("Extracting tools using hybrid approach (regex + symbol table)");

        // CONCRETE SOLUTION: Use beautified_tools regex extractor BUT with
        // the enhanced symbol table that has lazy_init and function returns resolved
        let beautified_code = self.generate_beautified_code()?;
        debug!("Generated {} bytes of beautified code", beautified_code.len());

        // Use beautified_tools extractor with our fully-resolved symbol table
        let beautified_extractor = beautified_tools::BeautifiedToolExtractor::with_ast(
            &beautified_code,
            self.analyzer.program()
        );

        let mut tools = beautified_extractor.extract()?;
        debug!("Extracted {} base tools from regex patterns", tools.len());

        // Enrich with prompts from enhanced extraction (fixes remaining wrong matches)
        for tool in &mut tools {
            if let Some(prompt) = Self::find_best_prompt_for_tool(enhanced_prompts, &tool.name) {
                // Replace if:
                // 1. New prompt is significantly longer (2x), OR
                // 2. Current has low confidence (<0.8), OR
                // 3. Current is placeholder/generic
                let should_replace = prompt.length > tool.full_prompt.len() * 2
                    || tool.confidence < 0.8
                    || tool.full_prompt.starts_with("Tool:")
                    || tool.full_prompt.starts_with("A powerful search tool"); // Wrong match

                if should_replace {
                    tool.full_prompt = prompt.content.clone();
                    tool.short_description = prompt.content.chars().take(200).collect();
                    tool.confidence = 1.0;
                    debug!("Enhanced tool {} with {}-char prompt (was {} chars, conf {})",
                           tool.name, prompt.length, tool.full_prompt.len(), tool.confidence);
                }
            }
        }

        Ok(tools)
    }

    /// Find the best matching prompt for a tool (static method).
    fn find_best_prompt_for_tool<'b>(
        prompts: &'b [prompts_enhanced::EnhancedSystemPrompt],
        tool_name: &str,
    ) -> Option<&'b prompts_enhanced::EnhancedSystemPrompt> {
        // Use tool-specific patterns - prefer longer, more complete documentation
        let candidates: Vec<_> = prompts.iter().filter(|p| {
            let content = &p.content;
            let first_100 = content.chars().take(100).collect::<String>();

            match tool_name {
                "Read" => content.starts_with("Reads a file from the local filesystem"),
                "Write" => content.starts_with("Writes a file to the local filesystem"),
                "Edit" => content.starts_with("Performs exact string replacements"),
                "Bash" => content.starts_with("Executes a given bash command"),
                "Grep" => first_100.contains("powerful search tool") &&
                         first_100.contains("ripgrep"),
                "Glob" => first_100.contains("Fast file pattern matching") ||
                         (first_100.contains("glob patterns") && first_100.contains("**/*.js")),
                "Task" => content.starts_with("Launch a new agent to handle complex") ||
                         first_100.contains("launches specialized agents"),
                "TodoWrite" => content.starts_with("Use this tool to create and manage a structured task list"),
                "WebFetch" => first_100.contains("Fetches content from a specified URL"),
                "WebSearch" => first_100.contains("Allows Claude to search the web"),
                "NotebookEdit" => content.starts_with("Completely replaces the contents of a specific cell") ||
                                 (first_100.contains("Jupyter notebook") && first_100.contains("cell") && first_100.contains("ipynb")),
                "AskUserQuestion" => content.starts_with("Use this tool when you need to ask the user"),
                "Skill" => first_100.contains("Execute a skill within the main conversation"),
                "SlashCommand" => first_100.contains("Execute a slash command"),
                "ExitPlanMode" => first_100.contains("plan mode") && first_100.contains("ready to code"),
                "BashOutput" => first_100.contains("Retrieves output from a running") ||
                               first_100.contains("Retrieves output from a running or completed background"),
                "KillShell" => first_100.contains("Kills a running background bash") ||
                              first_100.contains("Kills a running background bash shell"),
                "LSP" => content.starts_with("Interact with Language Server Protocol"),
                "Skill" => first_100.contains("Execute a skill within"),
                "LocalVariables" | "Anr" | "SENT" => false, // Skip these for now
                _ => false,
            }
        }).collect();

        // Return the longest matching prompt (most complete documentation)
        candidates.into_iter().max_by_key(|p| p.length)
    }

    /// Validate that a description is actual documentation, not code.
    fn is_valid_tool_description(&self, content: &str) -> bool {
        // Reject if it looks like code
        let code_indicators = [
            "function(",
            "async function",
            "() =>",
            "} catch {",
            "throw Error(",
            "stdio:",
            ".forEach(",
            "\t}",
            "});",
        ];

        let code_count = code_indicators.iter().filter(|&&ind| content.contains(ind)).count();
        if code_count >= 2 {
            return false;
        }

        // Reject if starts with code syntax
        let trimmed = content.trim();
        if trimmed.starts_with(",") || trimmed.starts_with("}") || trimmed.starts_with(")") {
            return false;
        }

        // Must have some prose characteristics
        let has_sentences = content.contains(". ") || content.contains(".\n");
        let has_prose_words = content.contains("the ") || content.contains("to ") || content.contains("this ");

        has_sentences || has_prose_words
    }

    /// Find the best prompt for a given tool.
    fn find_tool_prompt<'b>(
        &self,
        prompts: &'b [prompts_enhanced::EnhancedSystemPrompt],
        tool_name: &str,
    ) -> Option<&'b prompts_enhanced::EnhancedSystemPrompt> {
        // Strategy 1: Exact match via associated_tool
        let exact_match = prompts
            .iter()
            .find(|p| p.associated_tool.as_deref() == Some(tool_name));

        if exact_match.is_some() {
            debug!("Strategy 1 (exact) matched for {}", tool_name);
            return exact_match;
        }

        // Strategy 2: Tool-specific patterns (high confidence)
        debug!("Strategy 1 failed for {}, trying tool-specific patterns", tool_name);
        let specific_match = self.find_tool_specific_prompt(prompts, tool_name);
        if specific_match.is_some() {
            debug!("Strategy 2 (tool-specific) matched for {}", tool_name);
            return specific_match;
        }

        // Strategy 3: Fallback - Best match via content analysis
        debug!("Strategy 2 failed for {}, trying fallback", tool_name);
        let fallback = prompts
            .iter()
            .filter(|p| p.category == prompts::PromptCategory::Tool)
            .filter(|p| {
                // Tool name appears in first 200 chars AND content is valid
                let first_200 = p.content.chars().take(200).collect::<String>();
                first_200.contains(tool_name) && self.is_valid_tool_description(&p.content)
            })
            .max_by_key(|p| p.length);

        if fallback.is_some() {
            debug!("Strategy 3 (fallback) matched for {}", tool_name);
        } else {
            debug!("No match found for {}", tool_name);
        }

        fallback
    }

    /// Find tool-specific prompts using tailored patterns.
    fn find_tool_specific_prompt<'b>(
        &self,
        prompts: &'b [prompts_enhanced::EnhancedSystemPrompt],
        tool_name: &str,
    ) -> Option<&'b prompts_enhanced::EnhancedSystemPrompt> {
        if tool_name == "Read" {
            debug!("find_tool_specific_prompt for Read: have {} prompts to check", prompts.len());
        }

        let result = prompts.iter().find(|p| {
            let content = &p.content;
            let first_100 = content.chars().take(100).collect::<String>();

            let matches = match tool_name {
                "Read" => {
                    let check = content.starts_with("Reads a file from the local filesystem");
                    if p.content.contains("Reads a file") {
                        debug!("Read prompt {}: starts_with={}, first_40='{}'", p.id, check, &content.chars().take(40).collect::<String>());
                    }
                    check
                },
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
                _ => false,
            };

            if matches {
                trace!("Tool-specific match for {}: {} ({})", tool_name, p.id, p.length);
            }

            matches
        });

        if result.is_some() {
            debug!("Found tool-specific prompt for {}", tool_name);
        }

        result
    }

    /// Generate beautified code for extraction.
    fn generate_beautified_code(&self) -> Result<String> {
        use crate::transformer::codegen::CodeGenerator;
        use oxc_allocator::Allocator;
        let allocator = Allocator::default();
        let codegen = CodeGenerator::new(&allocator, self.analyzer.program());
        codegen.generate()
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
