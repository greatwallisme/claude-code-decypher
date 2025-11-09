//! Phase 4 integration tests - Analysis functionality

use claude_code_decypher::{
    analysis::AdvancedAnalyzer,
    parser::Parser,
    transformer::advanced_split::AdvancedSplitter,
    Result,
};
use oxc_allocator::Allocator;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_build_call_graph() -> Result<()> {
    let code = r#"
        function foo() {
            return bar() + baz();
        }

        function bar() {
            return 42;
        }

        function baz() {
            return foo();
        }
    "#;

    let allocator = Allocator::default();
    let parser = Parser::new(code.to_string());
    let parse_result = parser.parse(&allocator)?;

    let analyzer = AdvancedAnalyzer::new(parse_result.program());
    let call_graph = analyzer.build_call_graph()?;

    assert!(call_graph.unique_functions >= 1);
    assert!(call_graph.total_calls >= 1);

    Ok(())
}

#[test]
fn test_calculate_complexity() -> Result<()> {
    let code = r#"
        function simple() {
            return 42;
        }

        function complex(x) {
            if (x > 0) {
                for (let i = 0; i < x; i++) {
                    if (i % 2 === 0) {
                        console.log(i);
                    }
                }
            }
            return x;
        }
    "#;

    let allocator = Allocator::default();
    let parser = Parser::new(code.to_string());
    let parse_result = parser.parse(&allocator)?;

    let analyzer = AdvancedAnalyzer::new(parse_result.program());
    let complexity = analyzer.calculate_complexity()?;

    assert!(complexity.avg_cyclomatic >= 1.0);
    assert!(complexity.max_cyclomatic >= 1); // At least base complexity
    assert!(complexity.total_decision_points >= 1);

    Ok(())
}

#[test]
fn test_calculate_metrics() -> Result<()> {
    let code = r#"
        function foo() { return 1; }
        function bar() { return 2; }
        const x = 1;
        const y = 2;
        const z = 3;
    "#;

    let allocator = Allocator::default();
    let parser = Parser::new(code.to_string());
    let parse_result = parser.parse(&allocator)?;

    let analyzer = AdvancedAnalyzer::new(parse_result.program());
    let metrics = analyzer.calculate_metrics()?;

    assert_eq!(metrics.function_count, 2);
    assert_eq!(metrics.variable_count, 3);
    assert!(metrics.avg_function_length > 0.0);

    Ok(())
}

#[test]
fn test_generate_report() -> Result<()> {
    let code = r#"
        function process(data) {
            if (data) {
                return transform(data);
            }
            return null;
        }

        function transform(x) {
            return x * 2;
        }
    "#;

    let allocator = Allocator::default();
    let parser = Parser::new(code.to_string());
    let parse_result = parser.parse(&allocator)?;

    let analyzer = AdvancedAnalyzer::new(parse_result.program());
    let report = analyzer.generate_report()?;

    // Verify all components are present
    assert!(report.call_graph.unique_functions >= 2);
    assert!(report.complexity.avg_cyclomatic >= 1.0);
    assert!(report.metrics.function_count >= 2);

    Ok(())
}

#[test]
fn test_write_analysis_reports() -> Result<()> {
    let code = r#"
        function foo() { return bar(); }
        function bar() { return 42; }
    "#;

    let allocator = Allocator::default();
    let parser = Parser::new(code.to_string());
    let parse_result = parser.parse(&allocator)?;

    let analyzer = AdvancedAnalyzer::new(parse_result.program());
    let report = analyzer.generate_report()?;

    let temp_dir = TempDir::new()
        .map_err(|e| claude_code_decypher::error::DecypherError::Other(e.into()))?;

    // Write JSON reports
    report.write_json(temp_dir.path())?;

    // Verify files were created
    assert!(temp_dir.path().join("analysis/call-graph.json").exists());
    assert!(temp_dir.path().join("analysis/complexity.json").exists());
    assert!(temp_dir.path().join("analysis/metrics.json").exists());

    // Verify JSON is valid
    let cg_json = fs::read_to_string(temp_dir.path().join("analysis/call-graph.json"))
        .map_err(|e| claude_code_decypher::error::DecypherError::Other(e.into()))?;
    let _: serde_json::Value = serde_json::from_str(&cg_json)
        .map_err(|e| claude_code_decypher::error::DecypherError::Other(e.into()))?;

    Ok(())
}

#[test]
fn test_generate_markdown_report() -> Result<()> {
    let code = "function test() { return 42; }";

    let allocator = Allocator::default();
    let parser = Parser::new(code.to_string());
    let parse_result = parser.parse(&allocator)?;

    let analyzer = AdvancedAnalyzer::new(parse_result.program());
    let report = analyzer.generate_report()?;

    let temp_dir = TempDir::new()
        .map_err(|e| claude_code_decypher::error::DecypherError::Other(e.into()))?;

    report.generate_markdown(temp_dir.path())?;

    // Verify markdown report was created
    assert!(temp_dir.path().join("docs/analysis-report.md").exists());

    Ok(())
}

#[test]
fn test_advanced_splitter() -> Result<()> {
    let code = r#"
        function toolHandler() {}
        function apiRequest() {}
        function gitCommit() {}
    "#;

    let allocator = Allocator::default();
    let parser = Parser::new(code.to_string());
    let parse_result = parser.parse(&allocator)?;

    let splitter = AdvancedSplitter::new(parse_result.program(), code);
    let modules = splitter.split()?;

    assert!(!modules.is_empty());

    // Should categorize functions appropriately
    let categories: Vec<_> = modules.iter().map(|m| m.category.as_str()).collect();
    assert!(categories.contains(&"tools") || categories.contains(&"core"));

    Ok(())
}
