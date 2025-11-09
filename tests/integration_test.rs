//! Integration tests for claude-code-decypher

use claude_code_decypher::{
    parser::{visitor::StatsVisitor, Parser},
    Result,
};
use oxc_allocator::Allocator;
use std::fs;
use tempfile::NamedTempFile;

#[test]
fn test_parse_simple_file() -> Result<()> {
    let code = r#"
        var x = 1;
        function hello() {
            return "world";
        }
    "#;

    let allocator = Allocator::default();
    let parser = Parser::new(code.to_string());
    let result = parser.parse(&allocator)?;

    assert!(result.is_success());
    assert_eq!(result.error_count(), 0);

    Ok(())
}

#[test]
fn test_parse_from_file() -> Result<()> {
    // Create a temporary file
    let temp_file = NamedTempFile::new().unwrap();
    let code = "const x = 42; console.log(x);";
    fs::write(temp_file.path(), code).unwrap();

    // Parse from file
    let allocator = Allocator::default();
    let parser = Parser::from_file(temp_file.path())?;
    let result = parser.parse(&allocator)?;

    assert!(result.is_success());

    Ok(())
}

#[test]
fn test_stats_collection() -> Result<()> {
    let code = r#"
        const obj = { name: "test", value: 42 };
        const arr = [1, 2, 3, 4, 5];

        function process(data) {
            return data.map(x => x * 2);
        }

        const result = process(arr);
        console.log(result);
    "#;

    let allocator = Allocator::default();
    let parser = Parser::new(code.to_string());
    let parse_result = parser.parse(&allocator)?;

    let mut visitor = StatsVisitor::new();
    let stats = visitor.visit_program(parse_result.program());

    // Verify statistics
    assert!(stats.variable_count >= 3); // obj, arr, result
    assert!(stats.function_count >= 1); // process (arrow functions might not be counted separately)
    assert!(stats.object_count >= 1);
    assert!(stats.array_count >= 1);
    assert!(stats.call_count >= 1); // at least one call

    Ok(())
}

#[test]
fn test_parse_minified_code() -> Result<()> {
    let code = "var a=1,b=2;function c(){return a+b}var d=c();";

    let allocator = Allocator::default();
    let parser = Parser::new(code.to_string());
    let result = parser.parse(&allocator)?;

    assert!(result.is_success());

    Ok(())
}

#[test]
fn test_parse_es6_features() -> Result<()> {
    let code = r#"
        const arrow = (x) => x * 2;
        const { name, age } = person;
        const [...rest] = items;
        const template = `Hello ${name}`;

        class MyClass {
            constructor() {
                this.value = 42;
            }

            async method() {
                const result = await fetch('/api');
                return result;
            }
        }
    "#;

    let allocator = Allocator::default();
    let parser = Parser::new(code.to_string());
    let result = parser.parse(&allocator)?;

    // Should parse without fatal errors
    assert!(result.is_success());

    Ok(())
}

#[test]
fn test_longest_string_detection() -> Result<()> {
    let code = r#"
        const short = "hi";
        const medium = "this is a medium string";
        const long = "this is a very long string that should be detected as the longest one in the code";
    "#;

    let allocator = Allocator::default();
    let parser = Parser::new(code.to_string());
    let parse_result = parser.parse(&allocator)?;

    let mut visitor = StatsVisitor::new();
    let stats = visitor.visit_program(parse_result.program());

    assert_eq!(stats.string_literal_count, 3);
    assert!(stats.longest_string > 50); // The long string

    Ok(())
}

#[test]
fn test_error_recovery() -> Result<()> {
    // Intentionally malformed code
    let code = "var x = ;";

    let allocator = Allocator::default();
    let parser = Parser::new(code.to_string());
    let result = parser.parse(&allocator)?;

    // Should not panic, but should report errors
    assert!(!result.is_success());
    assert!(result.error_count() > 0);

    Ok(())
}

#[test]
fn test_nested_structures() -> Result<()> {
    let code = r#"
        const nested = {
            level1: {
                level2: {
                    level3: {
                        value: 42
                    }
                }
            }
        };

        function outer() {
            function inner() {
                function deepest() {
                    return "deep";
                }
                return deepest();
            }
            return inner();
        }
    "#;

    let allocator = Allocator::default();
    let parser = Parser::new(code.to_string());
    let parse_result = parser.parse(&allocator)?;

    let mut visitor = StatsVisitor::new();
    let stats = visitor.visit_program(parse_result.program());

    assert!(stats.max_depth > 1);
    assert!(stats.function_count >= 3);

    Ok(())
}
