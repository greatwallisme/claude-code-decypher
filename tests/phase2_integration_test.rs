//! Phase 2 integration tests - Extraction functionality

use claude_code_decypher::{
    analyzer::Analyzer,
    extractor::Extractor,
    output::OutputWriter,
    parser::Parser,
    Result,
};
use oxc_allocator::Allocator;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_extract_prompts() -> Result<()> {
    let code = r#"
        const prompt1 = "You are Claude Code, Anthropic's official CLI for Claude. You help users with software engineering tasks and provide comprehensive assistance.";
        const prompt2 = "This tool allows you to execute bash commands in a persistent shell session with optional timeout. Use this for terminal operations and system tasks.";
    "#;

    let allocator = Allocator::default();
    let parser = Parser::new(code.to_string());
    let parse_result = parser.parse(&allocator)?;

    let analyzer = Analyzer::new(parse_result.program());
    let extractor = Extractor::new(analyzer);

    let prompts = extractor.extract_prompts()?;

    assert!(!prompts.is_empty());
    Ok(())
}

#[test]
fn test_extract_tools() -> Result<()> {
    let code = r#"
        const tool = {
            name: "Bash",
            description: "Execute bash commands",
            parameters: { type: "object" }
        };
    "#;

    let allocator = Allocator::default();
    let parser = Parser::new(code.to_string());
    let parse_result = parser.parse(&allocator)?;

    let analyzer = Analyzer::new(parse_result.program());
    let extractor = Extractor::new(analyzer);

    let tools = extractor.extract_tools()?;

    assert!(!tools.is_empty());
    Ok(())
}

#[test]
fn test_extract_configs() -> Result<()> {
    let code = r#"
        const model = "claude-sonnet-4-5-20250929";
        const endpoint = "https://api.anthropic.com/v1/messages";
    "#;

    let allocator = Allocator::default();
    let parser = Parser::new(code.to_string());
    let parse_result = parser.parse(&allocator)?;

    let analyzer = Analyzer::new(parse_result.program());
    let extractor = Extractor::new(analyzer);

    let configs = extractor.extract_configs()?;

    assert!(!configs.is_empty());
    Ok(())
}

#[test]
fn test_output_writer() -> Result<()> {
    let temp_dir = TempDir::new()
        .map_err(|e| claude_code_decypher::error::DecypherError::Other(e.into()))?;
    let output_path = temp_dir.path();

    let writer = OutputWriter::new(output_path);
    writer.create_structure()?;

    // Verify directory was created
    let extracted_dir = output_path.join("extracted");
    assert!(extracted_dir.exists());

    Ok(())
}

#[test]
fn test_full_extraction_pipeline() -> Result<()> {
    let code = r#"
        const systemPrompt = "You are Claude Code, Anthropic's official CLI for Claude. You help users with software engineering tasks and provide comprehensive assistance with code analysis.";
        const tool = {
            name: "Read",
            description: "Read files from the filesystem",
            parameters: { type: "object", properties: {} }
        };
        const config = {
            model: "claude-sonnet-4",
            endpoint: "https://api.anthropic.com/v1",
            timeout: 30000
        };
    "#;

    let allocator = Allocator::default();
    let parser = Parser::new(code.to_string());
    let parse_result = parser.parse(&allocator)?;

    let analyzer = Analyzer::new(parse_result.program());
    let extractor = Extractor::new(analyzer);

    // Extract all data
    let prompts = extractor.extract_prompts()?;
    let tools = extractor.extract_tools()?;
    let configs = extractor.extract_configs()?;
    let strings = extractor.extract_strings()?;

    // Verify results
    assert!(!prompts.is_empty(), "Should extract at least one prompt");
    assert!(!tools.is_empty(), "Should extract at least one tool");
    assert!(!configs.is_empty(), "Should extract at least one config");

    // Create output directory and write results
    let temp_dir = TempDir::new()
        .map_err(|e| claude_code_decypher::error::DecypherError::Other(e.into()))?;
    let writer = OutputWriter::new(temp_dir.path());
    writer.create_structure()?;

    writer.write_prompts(&prompts)?;
    writer.write_tools(&tools)?;
    writer.write_configs(&configs)?;
    writer.write_strings(&strings)?;

    // Verify files were created
    assert!(temp_dir.path().join("extracted/system-prompts.json").exists());
    assert!(temp_dir.path().join("extracted/tool-definitions.json").exists());
    assert!(temp_dir.path().join("extracted/configurations.json").exists());
    assert!(temp_dir.path().join("extracted/strings.json").exists());

    // Verify JSON is valid
    let prompts_json = fs::read_to_string(temp_dir.path().join("extracted/system-prompts.json"))
        .map_err(|e| claude_code_decypher::error::DecypherError::Other(e.into()))?;
    let _: serde_json::Value = serde_json::from_str(&prompts_json)
        .map_err(|e| claude_code_decypher::error::DecypherError::Other(e.into()))?;

    Ok(())
}
