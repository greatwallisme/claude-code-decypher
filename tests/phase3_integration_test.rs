//! Phase 3 integration tests - Transformation functionality

use claude_code_decypher::{
    parser::Parser,
    transformer::{
        codegen::{beautify_code, CodeGenerator},
        rename::{apply_rename_map, VariableRenamer},
        split::{ModuleSplitter, SplitStrategy},
        Transformer,
    },
    Result,
};
use oxc_allocator::Allocator;
use std::collections::HashMap;

#[test]
fn test_beautify_minified_code() -> Result<()> {
    let code = "var x=1;var y=2;function f(){return x+y}";

    let allocator = Allocator::default();
    let parser = Parser::new(code.to_string());
    let parse_result = parser.parse(&allocator)?;

    let generator = CodeGenerator::new(&allocator, parse_result.program());
    let generated = generator.generate()?;

    // Should generate valid code
    assert!(!generated.is_empty());
    assert!(generated.contains("var x"));

    // Apply beautification
    let beautified = beautify_code(&generated);
    assert!(beautified.contains('\n'));

    Ok(())
}

#[test]
fn test_variable_renaming() -> Result<()> {
    let code = "var QB9 = 1; function test() { return QB9; }";

    let mut map = HashMap::new();
    map.insert("QB9".to_string(), "create_object".to_string());

    let renamed = apply_rename_map(code, &map);

    assert!(renamed.contains("create_object"));
    assert!(!renamed.contains("QB9"));

    Ok(())
}

#[test]
fn test_generate_rename_map() -> Result<()> {
    let code = "var QB9 = 1; var IB9 = 2;";

    let allocator = Allocator::default();
    let parser = Parser::new(code.to_string());
    let parse_result = parser.parse(&allocator)?;

    let transformer = Transformer::new(parse_result.program());
    let rename_map = transformer.generate_rename_map()?;

    // Should generate at least some renames
    assert!(!rename_map.is_empty());

    Ok(())
}

#[test]
fn test_module_splitting() -> Result<()> {
    let code = r#"
        const tool = "Bash";
        const api = "https://api.anthropic.com";
        const prompt = "You are Claude Code";
    "#;

    let allocator = Allocator::default();
    let parser = Parser::new(code.to_string());
    let parse_result = parser.parse(&allocator)?;

    let transformer = Transformer::new(parse_result.program());
    let modules = transformer.split_into_modules(SplitStrategy::Hybrid)?;

    assert!(!modules.is_empty());

    Ok(())
}

#[test]
fn test_split_strategies() -> Result<()> {
    let code = "var x = 1;";

    let allocator = Allocator::default();
    let parser = Parser::new(code.to_string());
    let parse_result = parser.parse(&allocator)?;

    let transformer = Transformer::new(parse_result.program());

    // Test different strategies
    let by_export = transformer.split_into_modules(SplitStrategy::ByExport)?;
    let by_namespace = transformer.split_into_modules(SplitStrategy::ByNamespace)?;
    let by_feature = transformer.split_into_modules(SplitStrategy::ByFeature)?;
    let hybrid = transformer.split_into_modules(SplitStrategy::Hybrid)?;

    // All should return some modules
    assert!(!by_export.is_empty());
    assert!(!by_namespace.is_empty());
    assert!(!by_feature.is_empty());
    assert!(!hybrid.is_empty());

    Ok(())
}

#[test]
fn test_full_transformation_pipeline() -> Result<()> {
    let code = r#"
        var QB9 = 1;
        var IB9 = function() { return QB9; };
        const tool = {
            name: "Bash",
            description: "Execute commands"
        };
    "#;

    let allocator = Allocator::default();
    let parser = Parser::new(code.to_string());
    let parse_result = parser.parse(&allocator)?;

    let transformer = Transformer::new(parse_result.program());

    // Generate beautified code
    let beautified = transformer.beautify(&allocator)?;
    assert!(!beautified.is_empty());

    // Generate rename map
    let rename_map = transformer.generate_rename_map()?;
    assert!(!rename_map.is_empty());

    // Apply renaming
    let renamed = apply_rename_map(&beautified, &rename_map);
    assert!(!renamed.is_empty());

    // Split into modules
    let modules = transformer.split_into_modules(SplitStrategy::Hybrid)?;
    assert!(!modules.is_empty());

    Ok(())
}

#[test]
fn test_beautify_complex_code() -> Result<()> {
    let code = r#"
        const obj={name:"test",value:42};
        const arr=[1,2,3];
        function process(data){return data.map(x=>x*2)}
        const result=process(arr);
    "#;

    let allocator = Allocator::default();
    let parser = Parser::new(code.to_string());
    let parse_result = parser.parse(&allocator)?;

    let generator = CodeGenerator::new(&allocator, parse_result.program());
    let generated = generator.generate()?;

    // Should contain all elements
    assert!(generated.contains("const obj"));
    assert!(generated.contains("function process"));

    Ok(())
}
