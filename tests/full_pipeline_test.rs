//! Full pipeline integration test - All phases working together

use claude_code_decypher::{
    analyzer::Analyzer,
    extractor::Extractor,
    output::OutputWriter,
    parser::Parser,
    transformer::{split::SplitStrategy, Transformer},
    Result,
};
use oxc_allocator::Allocator;
use std::path::PathBuf;
use tempfile::TempDir;

#[test]
fn test_complete_pipeline() -> Result<()> {
    let code = r#"
        var QB9 = Object.create;
        const systemPrompt = "You are Claude Code, Anthropic's official CLI for Claude. You help users with software engineering tasks and provide comprehensive assistance.";
        const tool = {
            name: "Bash",
            description: "Execute bash commands in a persistent shell session",
            parameters: {
                "$schema": "http://json-schema.org/draft-07/schema#",
                type: "object",
                properties: {
                    command: { type: "string", description: "The command to execute" }
                }
            }
        };
        const config = "claude-sonnet-4-5-20250929";
        const apiUrl = "https://api.anthropic.com/v1/messages";

        function process(data) {
            return data.map(x => x * 2);
        }
    "#;

    let allocator = Allocator::default();
    let temp_dir = TempDir::new()
        .map_err(|e| claude_code_decypher::error::DecypherError::Other(e.into()))?;

    // PHASE 1: Parse
    let parser = Parser::new(code.to_string());
    let parse_result = parser.parse(&allocator)?;
    assert!(parse_result.is_success());

    // PHASE 2: Extract
    let analyzer = Analyzer::new(parse_result.program());
    let extractor = Extractor::new(analyzer);

    let prompts = extractor.extract_prompts()?;
    let tools = extractor.extract_tools()?;
    let configs = extractor.extract_configs()?;
    let strings = extractor.extract_strings()?;

    assert!(!prompts.is_empty(), "Should extract prompts");
    assert!(!tools.is_empty(), "Should extract tools");
    assert!(!configs.is_empty(), "Should extract configs");

    // Write extraction results
    let writer = OutputWriter::new(temp_dir.path());
    writer.create_structure()?;
    writer.write_prompts(&prompts)?;
    writer.write_tools(&tools)?;
    writer.write_configs(&configs)?;
    writer.write_strings(&strings)?;

    // PHASE 3: Transform
    let transformer = Transformer::new(parse_result.program());

    // Beautify
    let beautified = transformer.beautify(&allocator)?;
    assert!(!beautified.is_empty());

    // Rename
    let rename_map = transformer.generate_rename_map()?;
    assert!(!rename_map.is_empty());

    // Split
    let modules = transformer.split_into_modules(SplitStrategy::Hybrid)?;
    assert!(!modules.is_empty());

    // Verify all outputs
    assert!(temp_dir.path().join("extracted/system-prompts.json").exists());
    assert!(temp_dir.path().join("extracted/tool-definitions.json").exists());

    println!("\n=== Full Pipeline Test Results ===");
    println!("Phase 1: Parsed successfully");
    println!("Phase 2: Extracted {} prompts, {} tools, {} configs", prompts.len(), tools.len(), configs.len());
    println!("Phase 3: Generated {} lines, {} renames, {} modules", beautified.lines().count(), rename_map.len(), modules.len());

    Ok(())
}

#[test]
fn test_vendors_claude_parsing() -> Result<()> {
    let vendors_path = PathBuf::from("./vendors/claude");

    // Only run if file exists
    if !vendors_path.exists() {
        println!("Skipping vendors/claude test - file not found");
        return Ok(());
    }

    let allocator = Allocator::default();
    let parser = Parser::from_file(&vendors_path)?;
    let parse_result = parser.parse(&allocator)?;

    assert!(parse_result.is_success(), "Should parse vendors/claude successfully");

    println!("Successfully parsed vendors/claude");

    Ok(())
}
