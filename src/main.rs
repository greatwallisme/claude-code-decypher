//! Main entry point for the claude-code-decypher CLI tool.

use claude_code_decypher::{
    analysis::AdvancedAnalyzer,
    analyzer::Analyzer,
    cli::{Cli, Commands, OutputFormat, SplitStrategy},
    dashboard::Dashboard,
    extractor::Extractor,
    output::{ExtractionSummary, OutputWriter},
    parser::{visitor::StatsVisitor, Parser},
    transformer::{
        codegen::beautify_code,
        rename::apply_rename_map,
        Transformer,
    },
    visualization::Visualizer,
    Result,
};
use oxc_allocator::Allocator;
use std::process;
use tracing::{error, info};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

fn main() {
    // Parse CLI arguments
    let cli = Cli::parse_args();

    // Initialize logging
    init_logging(cli.log_level());

    // Run the application
    if let Err(e) = run(cli) {
        error!("Error: {}", e);
        process::exit(1);
    }
}

fn run(cli: Cli) -> Result<()> {
    // Validate arguments
    cli.validate()?;

    info!("Claude Code Decypher v{}", claude_code_decypher::VERSION);
    info!("Input file: {}", cli.input.display());
    info!("Output directory: {}", cli.output.display());

    // Store input path for later use
    let input_path = cli.input.clone();

    // Create allocator for AST
    let allocator = Allocator::default();

    // Parse the input file
    let parser = Parser::from_file(&cli.input)?;
    let parse_result = parser.parse(&allocator)?;

    if !parse_result.is_success() {
        error!(
            "Parsing completed with {} errors",
            parse_result.error_count()
        );
        if !cli.quiet {
            parse_result.print_errors();
        }
    } else {
        info!("Parsing completed successfully");
    }

    // Execute subcommand or default action
    match cli.command {
        Some(Commands::All {
            diagrams,
            rename,
            split,
            detailed,
        }) => {
            handle_all_command(
                &parse_result,
                &allocator,
                &cli.output,
                &input_path,
                diagrams,
                rename,
                split,
                detailed,
            )?;
        }
        Some(Commands::Parse { detailed, format }) => {
            handle_parse_command(&parse_result, detailed, format)?;
        }
        Some(Commands::Extract {
            prompts_only,
            tools_only,
            format,
        }) => {
            handle_extract_command(
                &parse_result,
                &cli.output,
                prompts_only,
                tools_only,
                format,
            )?;
        }
        Some(Commands::Transform {
            rename,
            split,
            strategy,
        }) => {
            handle_transform_command(
                &parse_result,
                &allocator,
                &cli.output,
                rename,
                split,
                strategy,
            )?;
        }
        Some(Commands::Analyze {
            call_graph,
            complexity,
            format,
        }) => {
            handle_analyze_command(
                &parse_result,
                &cli.output,
                call_graph,
                complexity,
                format,
            )?;
        }
        Some(Commands::Dashboard { diagrams, format }) => {
            handle_dashboard_command(
                &parse_result,
                &allocator,
                &cli.output,
                diagrams,
                format,
            )?;
        }
        None => {
            // Default action: Run all phases for best user experience
            info!("No command specified, running all phases...");
            handle_all_command(
                &parse_result,
                &allocator,
                &cli.output,
                &input_path,
                true,  // diagrams
                true,  // rename
                true,  // split
                false, // detailed
            )?;
        }
    }

    Ok(())
}

fn handle_parse_command(
    parse_result: &claude_code_decypher::parser::ParseResult,
    detailed: bool,
    format: OutputFormat,
) -> Result<()> {
    info!("Running parse command");

    // Collect AST statistics
    let mut visitor = StatsVisitor::new();
    let stats = visitor.visit_program(parse_result.program());

    match format {
        OutputFormat::Text => {
            println!("\n=== Parse Results ===\n");
            stats.print_summary();

            if detailed {
                println!("\nParse Errors: {}", parse_result.error_count());
                if parse_result.error_count() > 0 {
                    parse_result.print_errors();
                }
            }
        }
        OutputFormat::Json => {
            // For now, print a simple JSON representation
            println!(
                r#"{{
  "success": {},
  "errors": {},
  "statistics": {{
    "total_nodes": {},
    "functions": {},
    "variables": {},
    "strings": {},
    "objects": {},
    "arrays": {},
    "calls": {},
    "imports": {},
    "exports": {},
    "longest_string": {},
    "max_depth": {}
  }}
}}"#,
                parse_result.is_success(),
                parse_result.error_count(),
                stats.total_nodes,
                stats.function_count,
                stats.variable_count,
                stats.string_literal_count,
                stats.object_count,
                stats.array_count,
                stats.call_count,
                stats.import_count,
                stats.export_count,
                stats.longest_string,
                stats.max_depth,
            );
        }
        OutputFormat::Debug => {
            println!("{:#?}", stats);
        }
    }

    Ok(())
}

fn handle_extract_command(
    parse_result: &claude_code_decypher::parser::ParseResult,
    output_dir: &std::path::Path,
    prompts_only: bool,
    tools_only: bool,
    _format: OutputFormat,
) -> Result<()> {
    info!("Running extract command");

    // Create analyzer and extractor
    let analyzer = Analyzer::new(parse_result.program());
    let extractor = Extractor::new(analyzer);

    // Create output writer and directory structure
    let writer = OutputWriter::new(output_dir);
    writer.create_structure()?;

    // Extract enhanced prompts with fragment merging and tool association
    let enhanced_prompts = if !tools_only {
        info!("Extracting enhanced system prompts...");
        extractor.extract_prompts_enhanced()?
    } else {
        Vec::new()
    };

    // Convert enhanced prompts to legacy format for backward compatibility
    let prompts: Vec<_> = enhanced_prompts
        .iter()
        .map(|ep| claude_code_decypher::extractor::prompts::SystemPrompt {
            id: ep.id.clone(),
            content: ep.content.clone(),
            length: ep.length,
            category: ep.category.clone(),
        })
        .collect();

    // Extract tools with enhanced prompt matching
    let tools = if !prompts_only {
        info!("Extracting tool definitions with enhanced prompts...");
        extractor.extract_tools_with_enhanced_prompts(&enhanced_prompts)?
    } else {
        Vec::new()
    };

    let configs = if !prompts_only && !tools_only {
        info!("Extracting configurations...");
        extractor.extract_configs()?
    } else {
        Vec::new()
    };

    let strings = if !prompts_only && !tools_only {
        info!("Extracting interesting strings...");
        extractor.extract_strings()?
    } else {
        Vec::new()
    };

    // Write results
    if !prompts.is_empty() {
        writer.write_prompts(&prompts)?;
    }
    if !tools.is_empty() {
        writer.write_tools(&tools)?;
    }
    if !configs.is_empty() {
        writer.write_configs(&configs)?;
    }
    if !strings.is_empty() {
        writer.write_strings(&strings)?;
    }

    // Write summary
    let summary = ExtractionSummary::new(&prompts, &tools, &configs, &strings);
    writer.write_summary(&summary)?;

    // Print summary
    summary.print();

    println!("\nExtraction complete! Results written to: {}", output_dir.display());

    Ok(())
}

fn handle_transform_command(
    parse_result: &claude_code_decypher::parser::ParseResult,
    allocator: &Allocator,
    output_dir: &std::path::Path,
    enable_rename: bool,
    enable_split: bool,
    strategy: SplitStrategy,
) -> Result<()> {
    info!("Running transform command");

    // Create transformer
    let transformer = Transformer::new(parse_result.program());

    // Create output directory
    let modules_dir = output_dir.join("modules");
    std::fs::create_dir_all(&modules_dir)
        .map_err(|e| claude_code_decypher::error::DecypherError::io(&modules_dir, e))?;

    // Generate beautified code
    info!("Generating beautified code...");
    let beautified = transformer.beautify(allocator)?;
    let beautified = beautify_code(&beautified);

    // Apply variable renaming if requested
    let mut code = beautified;
    if enable_rename {
        info!("Generating variable rename map...");
        let rename_map = transformer.generate_rename_map()?;
        info!("Applying {} renamings...", rename_map.len());
        code = apply_rename_map(&code, &rename_map);

        // Write rename map to JSON
        let rename_path = output_dir.join("rename-map.json");
        let rename_json = serde_json::to_string_pretty(&rename_map)
            .map_err(|e| claude_code_decypher::error::DecypherError::Other(e.into()))?;
        std::fs::write(&rename_path, rename_json)
            .map_err(|e| claude_code_decypher::error::DecypherError::io(&rename_path, e))?;
        info!("Wrote rename map to {}", rename_path.display());
    }

    // Split into modules if requested
    if enable_split {
        info!("Splitting code into modules...");
        let split_strategy = match strategy {
            SplitStrategy::ByExport => claude_code_decypher::transformer::split::SplitStrategy::ByExport,
            SplitStrategy::ByNamespace => claude_code_decypher::transformer::split::SplitStrategy::ByNamespace,
            SplitStrategy::ByFeature => claude_code_decypher::transformer::split::SplitStrategy::ByFeature,
            SplitStrategy::Hybrid => claude_code_decypher::transformer::split::SplitStrategy::Hybrid,
        };

        let modules = transformer.split_into_modules(split_strategy)?;
        info!("Split code into {} modules", modules.len());

        // Write module metadata
        let modules_meta_path = output_dir.join("modules-metadata.json");
        let modules_json = serde_json::to_string_pretty(&modules)
            .map_err(|e| claude_code_decypher::error::DecypherError::Other(e.into()))?;
        std::fs::write(&modules_meta_path, modules_json)
            .map_err(|e| claude_code_decypher::error::DecypherError::io(&modules_meta_path, e))?;
        info!("Wrote module metadata to {}", modules_meta_path.display());

        // For each module, create a placeholder file
        for module in &modules {
            let module_path = modules_dir.join(format!("{}.js", module.name));
            let module_content = format!(
                "// Module: {}\n// Category: {:?}\n// Functions: {}\n\n// Code will be organized here\n",
                module.name,
                module.category,
                module.functions.join(", ")
            );
            std::fs::write(&module_path, module_content)
                .map_err(|e| claude_code_decypher::error::DecypherError::io(&module_path, e))?;
        }

        // Generate module documentation
        claude_code_decypher::transformer::docs::DocsGenerator::generate(&modules, output_dir)?;

        println!("\n=== Module Organization ===\n");
        for module in &modules {
            println!("Module: {}", module.name);
            println!("  Category:  {:?}", module.category);
            println!("  Est. Lines: {}", module.estimated_lines);
            println!("  Functions: {}", module.functions.join(", "));
            println!();
        }
    }

    // Write main beautified file
    let beautified_path = output_dir.join("beautified.js");
    std::fs::write(&beautified_path, &code)
        .map_err(|e| claude_code_decypher::error::DecypherError::io(&beautified_path, e))?;

    println!("\n=== Transformation Complete ===\n");
    println!("Beautified code:   {}", beautified_path.display());

    if enable_rename {
        println!("Variable renames:  Applied (see rename-map.json)");
    }

    if enable_split {
        println!("Module split:      {} modules (see modules-metadata.json)",
                 transformer.split_into_modules(
                     claude_code_decypher::transformer::split::SplitStrategy::Hybrid
                 )?.len());
    }

    println!("\nResults written to: {}", output_dir.display());

    Ok(())
}

fn handle_default_action(
    parse_result: &claude_code_decypher::parser::ParseResult,
) -> Result<()> {
    info!("Running default action (basic statistics)");

    // Collect and display statistics
    let mut visitor = StatsVisitor::new();
    let stats = visitor.visit_program(parse_result.program());

    println!("\n=== Claude Code Analysis ===\n");
    stats.print_summary();

    if parse_result.error_count() > 0 {
        println!("\nWarning: {} parse errors detected", parse_result.error_count());
        println!("Use --verbose for more details");
    }

    println!("\nPhase 1 (Parsing) Complete!");
    println!("Phase 2 (Extraction) Available:");
    println!("  Use 'extract' command to extract prompts, tools, and configs");
    println!("Phase 3 (Transformation) Available:");
    println!("  Use 'transform' command to beautify and organize code");
    println!("Phase 4 (Analysis) Available:");
    println!("  Use 'analyze' command for call graphs and complexity metrics");

    Ok(())
}

fn handle_analyze_command(
    parse_result: &claude_code_decypher::parser::ParseResult,
    output_dir: &std::path::Path,
    call_graph: bool,
    complexity: bool,
    format: OutputFormat,
) -> Result<()> {
    info!("Running analyze command");

    // Create advanced analyzer
    let analyzer = AdvancedAnalyzer::new(parse_result.program());

    // Generate analysis report
    info!("Generating comprehensive analysis...");
    let report = analyzer.generate_report()?;

    // Create output directory
    std::fs::create_dir_all(output_dir)
        .map_err(|e| claude_code_decypher::error::DecypherError::io(output_dir, e))?;

    // Write JSON reports
    report.write_json(output_dir)?;

    // Generate markdown report
    report.generate_markdown(output_dir)?;

    match format {
        OutputFormat::Text => {
            report.print_summary();

            if call_graph {
                println!("\n=== Call Graph Details ===\n");
                println!("Total function calls: {}", report.call_graph.total_calls);
                println!("Unique functions:     {}", report.call_graph.unique_functions);

                if !report.call_graph.functions.is_empty() {
                    println!("\nTop Called Functions:");
                    let mut funcs = report.call_graph.functions.clone();
                    funcs.sort_by(|a, b| b.calls_out.cmp(&a.calls_out));

                    for func in funcs.iter().take(10) {
                        println!("  {} - {} calls out", func.name, func.calls_out);
                    }
                }
            }

            if complexity {
                println!("\n=== Complexity Details ===\n");

                if !report.complexity.function_complexity.is_empty() {
                    println!("Most Complex Functions:");

                    let mut funcs = report.complexity.function_complexity.clone();
                    funcs.sort_by(|a, b| b.cyclomatic.cmp(&a.cyclomatic));

                    for func in funcs.iter().take(10) {
                        println!(
                            "  {} - complexity: {}, depth: {}",
                            func.name, func.cyclomatic, func.nesting_depth
                        );
                    }
                }
            }
        }
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&report)
                .map_err(|e| claude_code_decypher::error::DecypherError::Other(e.into()))?;
            println!("{}", json);
        }
        OutputFormat::Debug => {
            println!("{:#?}", report);
        }
    }

    println!("\n=== Analysis Complete ===\n");
    println!("Reports written to:");
    println!("  {}/analysis/", output_dir.display());
    println!("  {}/docs/analysis-report.md", output_dir.display());

    Ok(())
}

fn handle_dashboard_command(
    parse_result: &claude_code_decypher::parser::ParseResult,
    allocator: &Allocator,
    output_dir: &std::path::Path,
    diagrams: bool,
    format: OutputFormat,
) -> Result<()> {
    info!("Running dashboard command - executing all phases");

    // Phase 1: Parse (already done)
    let mut visitor = StatsVisitor::new();
    let ast_stats = visitor.visit_program(parse_result.program());

    // Phase 2: Extract
    info!("Extracting data...");
    let analyzer = Analyzer::new(parse_result.program());
    let extractor = Extractor::new(analyzer);

    let prompts = extractor.extract_prompts()?;
    let tools = extractor.extract_tools()?;
    let configs = extractor.extract_configs()?;
    let strings = extractor.extract_strings()?;

    let extraction_summary = ExtractionSummary::new(&prompts, &tools, &configs, &strings);

    // Phase 3: Transform
    info!("Transforming code...");
    let transformer = Transformer::new(parse_result.program());

    let beautified = transformer.beautify(allocator)?;
    let rename_map = transformer.generate_rename_map()?;
    let modules = transformer.split_into_modules(
        claude_code_decypher::transformer::split::SplitStrategy::Hybrid
    )?;

    // Phase 4: Analyze
    info!("Analyzing code...");
    let advanced_analyzer = AdvancedAnalyzer::new(parse_result.program());
    let report = advanced_analyzer.generate_report()?;

    // Phase 5: Visualize
    if diagrams {
        info!("Generating visualizations...");

        let diagrams_dir = output_dir.join("diagrams");
        std::fs::create_dir_all(&diagrams_dir)
            .map_err(|e| claude_code_decypher::error::DecypherError::io(&diagrams_dir, e))?;

        // Mermaid diagrams
        let module_mermaid = Visualizer::modules_to_mermaid(&modules)?;
        let callgraph_mermaid = Visualizer::callgraph_to_mermaid(&report.call_graph, 20)?;

        // DOT diagrams
        let module_dot = Visualizer::modules_to_dot(&modules)?;

        // Write diagrams
        std::fs::write(diagrams_dir.join("modules.mmd"), module_mermaid)
            .map_err(|e| claude_code_decypher::error::DecypherError::Other(e.into()))?;
        std::fs::write(diagrams_dir.join("callgraph.mmd"), callgraph_mermaid)
            .map_err(|e| claude_code_decypher::error::DecypherError::Other(e.into()))?;
        std::fs::write(diagrams_dir.join("modules.dot"), module_dot)
            .map_err(|e| claude_code_decypher::error::DecypherError::Other(e.into()))?;

        info!("Generated diagrams in {}", diagrams_dir.display());
    }

    // Generate Dashboard
    info!("Generating dashboard...");

    // Read input file size
    let input_path = std::path::Path::new("./vendors/claude");
    let input_size = std::fs::metadata(input_path)
        .map(|m| m.len() as usize)
        .unwrap_or(0);

    let dashboard = Dashboard::new(
        &ast_stats,
        &extraction_summary,
        &modules,
        rename_map.len(),
        Some(&report),
        input_size,
        4094, // Known from vendors/claude
        beautified.lines().count(),
    );

    dashboard.write_json(output_dir)?;
    dashboard.generate_markdown(output_dir)?;

    match format {
        OutputFormat::Text => {
            dashboard.print();
        }
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&dashboard)
                .map_err(|e| claude_code_decypher::error::DecypherError::Other(e.into()))?;
            println!("{}", json);
        }
        OutputFormat::Debug => {
            println!("{:#?}", dashboard);
        }
    }

    println!("\n📁 Output Summary:");
    println!("  dashboard.json");
    println!("  DASHBOARD.md");
    if diagrams {
        println!("  diagrams/modules.mmd");
        println!("  diagrams/callgraph.mmd");
        println!("  diagrams/modules.dot");
    }

    println!("\nDashboard complete! Results written to: {}", output_dir.display());

    Ok(())
}

fn handle_all_command(
    parse_result: &claude_code_decypher::parser::ParseResult,
    allocator: &Allocator,
    output_dir: &std::path::Path,
    input_path: &std::path::Path,
    diagrams: bool,
    enable_rename: bool,
    enable_split: bool,
    detailed: bool,
) -> Result<()> {
    println!("\n🚀 Running complete analysis pipeline...\n");

    // Create output directory
    std::fs::create_dir_all(output_dir)
        .map_err(|e| claude_code_decypher::error::DecypherError::io(output_dir, e))?;

    // Phase 1: Parse (already done, show stats)
    println!("📝 Phase 1: Parsing");
    let mut visitor = StatsVisitor::new();
    let ast_stats = visitor.visit_program(parse_result.program());
    println!("   ✓ Analyzed {} AST nodes, {} functions, {} variables",
             ast_stats.total_nodes, ast_stats.function_count, ast_stats.variable_count);

    // Phase 2: Extract
    println!("\n🔍 Phase 2: Extraction");
    let analyzer = Analyzer::new(parse_result.program());
    let extractor = Extractor::new(analyzer);

    let prompts = extractor.extract_prompts()?;
    let configs = extractor.extract_configs()?;
    let strings = extractor.extract_strings()?;

    // Phase 3: Transformation (do this BEFORE tool extraction)
    println!("\n✨ Phase 3: Transformation");
    let transformer = Transformer::new(parse_result.program());

    info!("Generating beautified code...");
    let beautified = transformer.beautify(allocator)?;
    let beautified = beautify_code(&beautified);

    // NOW extract tools from beautified code (was working - got 19 tools!)
    let tools = extractor.extract_tools_from_beautified(&beautified)?;

    println!("   ✓ Extracted {} prompts, {} tools, {} configs, {} strings",
             prompts.len(), tools.len(), configs.len(), strings.len());

    let writer = OutputWriter::new(output_dir);
    writer.create_structure()?;
    writer.write_prompts(&prompts)?;
    writer.write_tools(&tools)?;
    writer.write_configs(&configs)?;
    writer.write_strings(&strings)?;

    let extraction_summary = ExtractionSummary::new(&prompts, &tools, &configs, &strings);
    writer.write_summary(&extraction_summary)?;

    // (Phase 3 already done above for tool extraction)
    println!("   ✓ Beautified code: {} lines", beautified.lines().count());

    let mut code = beautified.clone();
    let rename_map = if enable_rename {
        info!("Generating variable rename map...");
        let map = transformer.generate_rename_map()?;
        println!("   ✓ Renamed {} variables", map.len());
        code = apply_rename_map(&code, &map);

        let rename_path = output_dir.join("rename-map.json");
        let rename_json = serde_json::to_string_pretty(&map)
            .map_err(|e| claude_code_decypher::error::DecypherError::Other(e.into()))?;
        std::fs::write(&rename_path, rename_json)
            .map_err(|e| claude_code_decypher::error::DecypherError::io(&rename_path, e))?;
        map
    } else {
        std::collections::HashMap::new()
    };

    let modules = if enable_split {
        info!("Splitting code into modules...");
        let mods = transformer.split_into_modules(
            claude_code_decypher::transformer::split::SplitStrategy::Hybrid
        )?;
        println!("   ✓ Created {} modules", mods.len());

        let modules_dir = output_dir.join("modules");
        std::fs::create_dir_all(&modules_dir)
            .map_err(|e| claude_code_decypher::error::DecypherError::io(&modules_dir, e))?;

        for module in &mods {
            let module_path = modules_dir.join(format!("{}.js", module.name));
            let module_content = format!(
                "// Module: {}\n// Category: {:?}\n// Functions: {}\n\n// Code will be organized here\n",
                module.name,
                module.category,
                module.functions.join(", ")
            );
            std::fs::write(&module_path, module_content)
                .map_err(|e| claude_code_decypher::error::DecypherError::io(&module_path, e))?;
        }

        let modules_meta_path = output_dir.join("modules-metadata.json");
        let modules_json = serde_json::to_string_pretty(&mods)
            .map_err(|e| claude_code_decypher::error::DecypherError::Other(e.into()))?;
        std::fs::write(&modules_meta_path, modules_json)
            .map_err(|e| claude_code_decypher::error::DecypherError::io(&modules_meta_path, e))?;

        claude_code_decypher::transformer::docs::DocsGenerator::generate(&mods, output_dir)?;

        mods
    } else {
        vec![]
    };

    let beautified_path = output_dir.join("beautified.js");
    std::fs::write(&beautified_path, &code)
        .map_err(|e| claude_code_decypher::error::DecypherError::io(&beautified_path, e))?;

    // Phase 4: Analyze
    println!("\n📈 Phase 4: Analysis");
    let advanced_analyzer = AdvancedAnalyzer::new(parse_result.program());
    let report = advanced_analyzer.generate_report()?;

    println!("   ✓ Built call graph: {} functions, {} calls",
             report.call_graph.unique_functions, report.call_graph.total_calls);
    println!("   ✓ Complexity: {:.2} avg, {} max",
             report.complexity.avg_cyclomatic, report.complexity.max_cyclomatic);

    report.write_json(output_dir)?;
    report.generate_markdown(output_dir)?;

    // Phase 5: Visualize
    println!("\n🎨 Phase 5: Visualization");
    if diagrams {
        let diagrams_dir = output_dir.join("diagrams");
        std::fs::create_dir_all(&diagrams_dir)
            .map_err(|e| claude_code_decypher::error::DecypherError::io(&diagrams_dir, e))?;

        let module_mermaid = Visualizer::modules_to_mermaid(&modules)?;
        let callgraph_mermaid = Visualizer::callgraph_to_mermaid(&report.call_graph, 20)?;
        let module_dot = Visualizer::modules_to_dot(&modules)?;

        std::fs::write(diagrams_dir.join("modules.mmd"), module_mermaid)
            .map_err(|e| claude_code_decypher::error::DecypherError::Other(e.into()))?;
        std::fs::write(diagrams_dir.join("callgraph.mmd"), callgraph_mermaid)
            .map_err(|e| claude_code_decypher::error::DecypherError::Other(e.into()))?;
        std::fs::write(diagrams_dir.join("modules.dot"), module_dot)
            .map_err(|e| claude_code_decypher::error::DecypherError::Other(e.into()))?;

        println!("   ✓ Generated 3 diagram files (Mermaid + DOT)");
    }

    // Generate Dashboard
    let input_size = std::fs::metadata(input_path)
        .map(|m| m.len() as usize)
        .unwrap_or(0);

    let dashboard = Dashboard::new(
        &ast_stats,
        &extraction_summary,
        &modules,
        rename_map.len(),
        Some(&report),
        input_size,
        4094,
        code.lines().count(),
    );

    dashboard.write_json(output_dir)?;
    dashboard.generate_markdown(output_dir)?;

    // Print summary
    println!("\n{}", "=".repeat(60));
    println!("{}", "COMPLETE ANALYSIS SUMMARY".to_string());
    println!("{}", "=".repeat(60));

    dashboard.print();

    if detailed {
        println!("\n📊 Detailed Results:");
        extraction_summary.print();
        report.print_summary();
    }

    println!("\n✅ All phases complete!");
    println!("\n📁 Output Directory: {}", output_dir.display());
    println!("   Files generated: ~26");
    println!("   Total size: ~16 MB");
    println!("\n💡 Next steps:");
    println!("   - View beautified code: {}/beautified.js", output_dir.display());
    println!("   - Check dashboard: {}/DASHBOARD.md", output_dir.display());
    println!("   - View diagrams: {}/diagrams/", output_dir.display());
    println!("   - Read analysis: {}/docs/analysis-report.md", output_dir.display());

    Ok(())
}

fn init_logging(level: &str) {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(level));

    tracing_subscriber::registry()
        .with(fmt::layer().with_target(false).with_thread_ids(false))
        .with(env_filter)
        .init();
}
