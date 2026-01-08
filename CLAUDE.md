# CLAUDE.md - Claude Code Decypher

## Project Overview

**Claude Code Decypher** is a production-ready Rust tool (v1.0) that deobfuscates and analyzes minified JavaScript code from Claude Code. It provides comprehensive parsing, extraction, transformation, analysis, and visualization capabilities through a 5-phase pipeline.

**Project Status**: Production-ready v1.0 - All 5 phases complete
**Primary Language**: Rust (Edition 2024)
**Domain**: Development tools, JavaScript deobfuscation, AST analysis

### Key Achievements
- 10MB minified JavaScript analyzed in ~14 seconds
- 417K lines of beautified code generated from 4K minified lines
- 93% test coverage with 69 tests
- 3x faster parsing than SWC using Oxc parser

## Project Structure

```
claude-code-decypher/
├── src/
│   ├── main.rs                 # CLI entry point
│   ├── lib.rs                  # Library root
│   ├── cli.rs                  # Command-line interface (clap derive)
│   ├── error.rs                # Error types (thiserror)
│   ├── dashboard.rs            # Statistics dashboard
│   ├── output.rs               # JSON output and file writing
│   │
│   ├── parser/                 # JavaScript parsing with Oxc
│   │   ├── mod.rs              # Oxc parser wrapper
│   │   └── visitor.rs          # AST visitor and statistics
│   │
│   ├── analyzer/               # AST analysis and pattern matching
│   │   ├── mod.rs              # Main analyzer
│   │   └── symbols.rs          # Symbol table management
│   │
│   ├── extractor/              # Data extraction from AST
│   │   ├── mod.rs              # Main extractor orchestration
│   │   ├── prompts.rs          # System prompt extraction
│   │   ├── prompts_enhanced.rs # Enhanced prompt patterns
│   │   ├── tools.rs            # Tool definition extraction
│   │   ├── tools_old.rs        # Legacy tool extraction
│   │   ├── beautified_tools.rs # Beautified tool patterns
│   │   ├── schemas.rs          # JSON schemas for validation
│   │   ├── config.rs           # Configuration value extraction
│   │   └── strings.rs          # String literal extraction
│   │
│   ├── transformer/            # Code transformation
│   │   ├── mod.rs              # Main transformer orchestration
│   │   ├── codegen.rs          # Code generation with Oxc codegen
│   │   ├── rename.rs           # Variable renaming (29 patterns)
│   │   ├── split.rs            # Module splitting strategies
│   │   ├── advanced_split.rs   # AST-aware splitting
│   │   ├── docs.rs             # Documentation generation
│   │   └── sourcemap.rs        # Source map generation
│   │
│   ├── analysis/               # Advanced code analysis
│   │   ├── mod.rs              # Main analyzer
│   │   ├── callgraph.rs        # Call graph construction
│   │   ├── complexity.rs       # Cyclomatic complexity
│   │   ├── metrics.rs          # Code metrics (LOC, functions)
│   │   └── report.rs           # Report generation
│   │
│   └── visualization/          # Graph and diagram generation
│       ├── mod.rs              # Main visualizer
│       ├── mermaid.rs          # Mermaid diagrams
│       └── dot.rs              # DOT/Graphviz diagrams
│
├── tests/                      # Integration tests
│   ├── integration_test.rs     # Phase 1 integration tests
│   ├── phase2_integration_test.rs  # Phase 2 extraction tests
│   ├── phase3_integration_test.rs  # Phase 3 transformation tests
│   ├── phase4_integration_test.rs  # Phase 4 analysis tests
│   └── full_pipeline_test.rs   # Complete end-to-end tests
│
├── benches/                    # Performance benchmarks
├── examples/                   # Example usage
├── fixtures/                   # Test fixtures
│
├── docs/                       # Project documentation
│   ├── QUICKSTART.md           # Quick start guide
│   ├── ULTIMATE-GUIDE.md       # Comprehensive guide
│   ├── FINAL-PROJECT-SUMMARY.md # Project completion summary
│   └── PHASE*.md               # Phase completion reports
│
├── specs/                      # Design specifications
│   ├── 0001-design-and-plan.md # Original design
│   ├── 0002-extraction-improvement.md # Extraction improvements
│   └── README.md               # Spec overview
│
├── scripts/                    # Utility scripts
├── vendors/                    # Git submodules (claude code)
│
├── Cargo.toml                  # Dependencies and project config
├── Cargo.lock                  # Locked dependency versions
├── Makefile                    # Build and test automation
├── cliff.toml                  # Git-cliff changelog config
├── deny.toml                   # Cargo-deny policies
├── .pre-commit-config.yaml     # Pre-commit hooks
├── _typos.toml                 # Typos checker config
└── README.md                   # User-facing documentation
```

## Core Concepts

### 5-Phase Pipeline

The tool operates in 5 distinct phases that can be run individually or as a complete pipeline:

**Phase 1: Parse (AST Statistics)**
- Parse minified JavaScript using Oxc parser
- Collect AST statistics: nodes, functions, variables, literals
- Measure nesting depth and complexity
- Performance: 10MB in ~800ms

**Phase 2: Extract (Data Extraction)**
- System prompts extraction with pattern matching
- Tool definition extraction with confidence scores
- Configuration values (models, APIs, paths)
- Interesting string literals (URLs, file paths)
- Output: Categorized JSON files

**Phase 3: Transform (Code Beautification)**
- Code beautification using Oxc codegen
- Variable renaming with 29 common patterns
- Module splitting with 4 strategies (by-export, by-namespace, by-feature, hybrid)
- Automatic documentation generation
- Output: 417K lines from 4K minified lines

**Phase 4: Analyze (Deep Analysis)**
- Call graph construction (3,391 functions, 9,347 calls)
- Cyclomatic complexity calculation (avg 2.08, max 36)
- Comprehensive code metrics (LOC, function length)
- Module relationship analysis
- Output: JSON reports + Markdown documentation

**Phase 5: Visualize (Dashboard & Diagrams)**
- Comprehensive dashboard with all metrics
- Mermaid diagrams for modules and call graphs
- DOT/Graphviz format for advanced visualization
- Single-command pipeline integration
- Output: 21 files in ~14 seconds

### Oxc Parser Ecosystem

This project uses Oxc (formerly Oxidation) for JavaScript parsing:
- **oxc_parser**: Fast JavaScript/TypeScript parser
- **oxc_ast**: AST definitions
- **oxc_codegen**: Code generation from AST
- **oxc_allocator**: Arena-based memory allocation
- **oxc_span**: Source text span tracking
- **oxc_diagnostics**: Error diagnostics

**Why Oxc?** 3x faster than SWC, memory-efficient with arena allocation, actively maintained.

### Module Splitting Strategies

The transformer supports 4 module splitting strategies:

1. **by-export**: Split based on export statements
2. **by-namespace**: Group by namespace patterns
3. **by-feature**: Cluster by related functionality
4. **hybrid**: Combined approach (default)

Each strategy creates organized module structure with metadata.

## Development Workflow

### Quick Start

```bash
# Build the project
cargo build --release

# Run complete analysis pipeline
cargo run --release -- ./vendors/claude all

# Or run individual phases
cargo run --release -- ./vendors/claude parse
cargo run --release -- ./vendors/claude extract
cargo run --release -- ./vendors/claude transform --rename --split
cargo run --release -- ./vendors/claude analyze --call-graph --complexity
cargo run --release -- ./vendors/claude dashboard --diagrams
```

### Build Commands

```bash
# Development build
cargo build

# Release build (optimized)
cargo build --release

# Check compilation without building
cargo check --all

# Format code
cargo fmt

# Run linter
cargo clippy --all-targets --all-features -- -D warnings
```

### Testing

```bash
# Run all tests (using cargo-nextest)
make test
cargo nextest run --all-features

# Run with output
cargo test -- --nocapture

# Run specific test file
cargo test --test phase3_integration_test

# Run specific test
cargo test test_beautify_minified_code

# Run tests with coverage (requires tarpaulin)
cargo tarpaulin --out Html
```

### Pre-commit Hooks

The project uses pre-commit hooks for code quality:
- **cargo-fmt**: Code formatting check
- **cargo-deny**: Dependency validation
- **typos**: Spell checking
- **cargo-check**: Compilation check
- **cargo-clippy**: Linting with warnings as errors
- **cargo-test**: Full test suite with nextest

Install hooks:
```bash
pre-commit install
```

Run manually:
```bash
pre-commit run --all-files
```

### Release Process

```bash
# Automated release (uses cargo-release and git-cliff)
make release

# Manual steps:
# 1. Bump version
# 2. Create git tag
# 3. Generate CHANGELOG.md with git-cliff
# 4. Push to origin
```

## Key Modules

### CLI Module (`src/cli.rs`)

Command-line interface using clap derive macros. Defines all subcommands:
- `parse`: Phase 1 - AST statistics
- `extract`: Phase 2 - Data extraction
- `transform`: Phase 3 - Code beautification
- `analyze`: Phase 4 - Deep analysis
- `dashboard`: Phase 5 - Visualization
- `all`: Complete pipeline

### Parser Module (`src/parser/`)

**Purpose**: Fast JavaScript parsing using Oxc

**Key Files**:
- `mod.rs`: Oxc parser wrapper with error handling
- `visitor.rs`: AST visitor pattern for statistics collection

**Important**: Always use arena allocation for performance. The visitor collects statistics during traversal without modifying the AST.

### Extractor Module (`src/extractor/`)

**Purpose**: Extract meaningful data from AST

**Key Files**:
- `prompts.rs` / `prompts_enhanced.rs`: System prompt extraction with pattern matching
- `tools.rs` / `beautified_tools.rs`: Tool definition extraction
- `config.rs`: Configuration values (models, APIs, paths)
- `strings.rs`: Interesting string literals
- `schemas.rs`: JSON schemas for validation

**Note**: Multiple tool extraction implementations exist (tools.rs, tools_old.rs, beautified_tools.rs) representing iterative improvements.

### Transformer Module (`src/transformer/`)

**Purpose**: Beautify and reorganize code

**Key Files**:
- `codegen.rs`: Code generation using Oxc codegen
- `rename.rs`: Variable renaming with 29 patterns
- `split.rs`: Basic module splitting
- `advanced_split.rs`: AST-aware splitting with 4 strategies
- `docs.rs`: Documentation generation

**Important**: The rename map contains common minified variable patterns. Add new patterns by updating the hashmap.

### Analysis Module (`src/analysis/`)

**Purpose**: Deep code analysis

**Key Files**:
- `callgraph.rs`: Build function call relationships
- `complexity.rs`: Calculate cyclomatic complexity
- `metrics.rs`: Collect code metrics (LOC, function length)
- `report.rs`: Generate JSON and Markdown reports

**Key Metrics**:
- Functions: 3,391
- Total calls: 9,347
- Average complexity: 2.08
- Max complexity: 36

### Visualization Module (`src/visualization/`)

**Purpose**: Generate diagrams and graphs

**Key Files**:
- `mermaid.rs`: Mermaid diagram generation
- `dot.rs`: DOT/Graphviz format

**Output**: Module diagrams, call graphs in both formats

### Error Handling (`src/error.rs`)

Uses thiserror for strongly-typed errors:
- `ParseError`: JavaScript parsing failures
- `AnalysisError`: AST analysis failures
- `ExtractionError`: Data extraction failures
- `TransformError`: Code transformation failures
- `IoError`: File I/O failures

## Output Structure

The tool generates comprehensive output:

```
output/
├── beautified.js              # Beautified JavaScript (417K lines)
├── rename-map.json            # Variable rename mapping
├── modules-metadata.json      # Module organization metadata
├── dashboard.json             # Comprehensive dashboard
├── DASHBOARD.md               # Dashboard in markdown
│
├── extracted/                 # Phase 2 extraction
│   ├── system-prompts.json
│   ├── tool-definitions.json
│   ├── configurations.json
│   ├── strings.json
│   └── summary.json
│
├── modules/                   # Phase 3 transformation
│   ├── core.js
│   ├── tools.js
│   ├── utils.js
│   ├── apiclient.js
│   ├── git.js
│   ├── hooks.js
│   └── prompts.js
│
├── analysis/                  # Phase 4 analysis
│   ├── call-graph.json        # 561 KB
│   ├── complexity.json        # 458 KB
│   └── metrics.json
│
├── diagrams/                  # Phase 5 visualization
│   ├── modules.mmd
│   ├── callgraph.mmd
│   └── modules.dot
│
└── docs/                      # Auto-generated documentation
    ├── modules.md
    ├── architecture.md
    └── analysis-report.md
```

## Testing Strategy

### Test Organization

**Unit Tests**: Located in individual source files (inline tests)
**Integration Tests**: Located in `tests/` directory
- `integration_test.rs`: 8 tests for Phase 1
- `phase2_integration_test.rs`: 5 tests for Phase 2
- `phase3_integration_test.rs`: 7 tests for Phase 3
- `phase4_integration_test.rs`: 5 tests for Phase 4
- `full_pipeline_test.rs`: Complete pipeline tests

### Coverage

- **Total Tests**: 69
- **Coverage**: 93%
- **Test Framework**: cargo-nextest (faster than cargo test)

### Writing Tests

Follow these conventions:
1. Use descriptive test names: `test_<functionality>_<scenario>`
2. Use fixtures from `fixtures/` directory
3. Clean up temporary files with tempfile crate
4. Test error paths, not just success paths
5. Use `--include-ignored` to run ignored tests

Example:
```rust
#[test]
fn test_beautify_minified_code() {
    let input = "var a=1;";
    let result = beautify(input).unwrap();
    assert!(result.contains("var a = 1;"));
}
```

## Development Guidelines

### Code Style

- **Formatting**: Use `cargo fmt` (Rust standard formatting)
- **Linting**: All clippy warnings must be fixed (`-D warnings`)
- **Comments**: Only comment complex logic, keep comments concise
- **No emojis**: Do not use emoji symbols in code or comments

### Error Handling

- Use `thiserror` for strongly-typed errors
- Use `anyhow::Error` for application-level errors
- Always include context with `.context()` or `.expect()`
- Never use `.unwrap()` in production code except in tests

### Performance Considerations

- Use arena allocation (Oxc allocator) for AST parsing
- Prefer iterators over collections for large datasets
- Avoid unnecessary clones
- Use `cargo bench` to verify performance changes

### Adding New Features

1. **New extraction pattern**: Add to `src/extractor/prompts.rs` or `tools.rs`
2. **New transformation**: Add to `src/transformer/mod.rs`
3. **New analysis metric**: Add to `src/analysis/metrics.rs`
4. **New diagram format**: Add to `src/visualization/mod.rs`

Always add:
- Unit tests for the new feature
- Integration test if it affects pipeline
- Documentation in README.md
- Update CLAUDE.md if architecture changes

### Dependencies Management

- The project uses `cargo-deny` for dependency auditing
- Check for advisories: `cargo deny check`
- Check licenses: `cargo deny check licenses`
- Update dependencies in `Cargo.toml` after review

## Common Tasks

### Add a New Variable Rename Pattern

Edit `src/transformer/rename.rs`:
```rust
let rename_map = hashmap! {
    // ... existing patterns
    "NEW_PATTERN" => "descriptive_name",
};
```

### Add a New Extraction Pattern

Edit `src/extractor/prompts.rs`:
```rust
pub fn extract_custom_pattern(ast: &Program) -> Vec<CustomData> {
    // Implementation
}
```

### Add a New Module Splitting Strategy

Edit `src/transformer/advanced_split.rs`:
```rust
pub enum SplitStrategy {
    ByExport,
    ByNamespace,
    ByFeature,
    Hybrid,
    YourNewStrategy,  // Add here
}
```

### Run Benchmarks

```bash
# Run all benchmarks
cargo bench

# Run specific benchmark
cargo bench --bench parse_benchmark

# Compare performance
cargo bench -- --save-baseline main
# Make changes
cargo bench -- --baseline main
```

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                        CLI (clap)                           │
│  parse | extract | transform | analyze | dashboard | all    │
└──────────────────────┬──────────────────────────────────────┘
                       │
       ┌───────────────┼───────────────┐
       │               │               │
       ▼               ▼               ▼
┌──────────┐    ┌──────────┐    ┌──────────┐
│  Parser  │    │Extractor │    │Analysis  │
│   (Oxc)  │───▶│ (Pattern │───▶│ (Call    │
│          │    │  Match)  │    │  Graph)  │
└──────────┘    └────┬─────┘    └────┬─────┘
                     │               │
                     ▼               ▼
              ┌──────────┐    ┌──────────┐
              │Transformer│    │Visualizer│
              │(Beautify │    │(Mermaid/ │
              │  Rename) │    │   DOT)   │
              └─────┬────┘    └────┬─────┘
                    │              │
                    ▼              ▼
              ┌─────────────────────────────┐
              │        Output Files         │
              │  JSON | JS | MD | Diagrams  │
              └─────────────────────────────┘
```

## Important Notes

1. **Git Submodules**: The `vendors/claude` directory is a git submodule. Update with `make update-submodule`

2. **Performance**: The tool is optimized for 10MB JavaScript files. For larger files, consider streaming or chunking.

3. **Memory Usage**: Arena allocation in Oxc means memory is freed in one operation. Monitor with `valgrind` or `heaptrack` if needed.

4. **JSON Schema**: All JSON outputs follow schemas in `src/extractor/schemas.rs`. Update schemas when changing output format.

5. **Test Data**: Keep test fixtures in `fixtures/` directory. Do not commit large test files.

6. **Documentation**: Auto-generated documentation in `output/docs/` is created on every run. Do not edit manually.

## Troubleshooting

### Build Errors

**Error**: "Failed to parse JavaScript"
- Check input file is valid JavaScript/TypeScript
- Verify file encoding is UTF-8
- Try with `--detailed` flag for more info

**Error**: "Out of memory"
- Reduce input file size
- Run phases individually instead of `all`
- Increase system swap space

### Test Failures

**Error**: Integration test timeout
- Check that `vendors/claude` submodule is initialized
- Verify test fixtures exist in `fixtures/`
- Run with `--nocapture` to see where it hangs

### Pre-commit Hook Failures

**Error**: `cargo fmt` check failed
- Run `cargo fmt` to fix formatting
- Commit the formatted files

**Error**: `cargo clippy` warnings
- Fix the specific warnings
- Use `cargo clippy --fix` to auto-fix simple issues

## Resources

- **Oxc Documentation**: https://oxc.rs
- **Clap Documentation**: https://docs.rs/clap
- **Thiserror Documentation**: https://docs.rs/thiserror
- **Project README**: See README.md for user-facing documentation
- **Design Specs**: See `specs/0001-design-and-plan.md` for detailed design

## License

MIT License - See LICENSE.md for details.
