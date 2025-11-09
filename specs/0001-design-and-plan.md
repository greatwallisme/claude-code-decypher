# Design and Implementation Plan: Claude Code Decypher

## Executive Summary

This document outlines a comprehensive design and implementation plan for `claude-code-decypher`, a Rust-based tool that analyzes, deobfuscates, and restructures the minified Claude Code JavaScript bundle (~10MB) into readable, well-organized code that enables deep architectural analysis.

## 1. Problem Analysis

### 1.1 Current State
- **Input**: `./vendors/claude` - A 10MB heavily minified JavaScript bundle (4094 lines, max line length: 40,849 characters)
- **Characteristics**:
  - Bundled with webpack/esbuild-like bundler
  - Minified variable names (e.g., `YB9`, `QB9`, `IB9`)
  - No whitespace or formatting
  - Contains system prompts, tool definitions, configuration, and business logic
  - Mixed ES6 imports, CommonJS patterns, and module system

### 1.2 Key Content Patterns Identified
From code analysis, we identified:
- **System prompts**: `"You are Claude Code, Anthropic's official CLI for Claude"`
- **Tool definitions**: References to `Bash`, `Read`, `Write`, `Edit`, `Task`, `Grep`, `Glob`, etc.
- **AWS Bedrock integration**: Tool schemas, API clients
- **Telemetry/metrics**: `claude_code.session.count`, token usage tracking
- **Hook system**: `PreToolUse`, `PostToolUse`, `UserPromptSubmit`, etc.

### 1.3 Objectives
1. Parse and deobfuscate the minified JavaScript
2. Extract structured data (prompts, tool schemas, configurations)
3. Reorganize code into logical modules/components
4. Generate readable, well-formatted output for analysis
5. Enable understanding of Claude Code's architecture and behavior

## 2. Technology Stack

### 2.1 Core Parsing & AST Manipulation

**Primary Choice: Oxc (The JavaScript Oxidation Compiler)**

Rationale:
- **Performance**: 3x faster than SWC, 5x faster than Biome
- **Conformance**: Most spec-compliant JS/TS parser in Rust
- **Ecosystem**: Complete toolchain with parser, codegen, transformer
- **Production-ready**: Used by Rolldown, Biome, swc-node, Shopify, ByteDance

**Crates**:
```toml
oxc_parser = "0.56"      # Fast JS/TS parser
oxc_ast = "0.56"         # AST definitions and visitor patterns
oxc_codegen = "0.56"     # Pretty printing AST to source
oxc_allocator = "0.56"   # Memory arena for fast allocation
oxc_span = "0.56"        # Source location tracking
oxc_ast_visit = "0.56"   # Visitor pattern for AST traversal
```

### 2.2 Auxiliary Libraries

```toml
# Core functionality
anyhow = "1.0"           # Error handling
thiserror = "2.0"        # Custom error types
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"       # JSON serialization
clap = { version = "4.5", features = ["derive"] } # CLI parsing

# String/Text processing
regex = "1.11"           # Pattern matching for extraction
aho-corasick = "1.1"     # Multi-pattern string search

# File I/O
walkdir = "2.5"          # Directory traversal
ignore = "0.4"           # .gitignore-style filtering

# Formatting (optional, for additional beautification)
dprint-core = "0.67"     # Additional formatting capabilities

# Logging & Diagnostics
tracing = "0.1"          # Structured logging
tracing-subscriber = "0.3"
```

## 3. Architecture Design

### 3.1 High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         CLI Interface                            │
│                    (clap-based argument parsing)                 │
└────────────────────────────┬────────────────────────────────────┘
                             │
                             ▼
┌─────────────────────────────────────────────────────────────────┐
│                      Orchestrator/Pipeline                       │
│  - Coordinates all phases                                        │
│  - Manages output directory structure                            │
│  - Progress reporting                                            │
└────────────────┬────────────────────────────────────────────────┘
                 │
                 ▼
    ┌────────────┴──────────┐
    │                       │
    ▼                       ▼
┌─────────────┐    ┌─────────────────┐
│   Parser    │    │   Analyzer      │
│   Module    │───▶│    Module       │
└─────────────┘    └─────────────────┘
                            │
                            ▼
                   ┌────────────────────┐
                   │  Extractor Module  │
                   │ - System Prompts   │
                   │ - Tool Definitions │
                   │ - Config Values    │
                   │ - Strings/Literals │
                   └────────────────────┘
                            │
                            ▼
                   ┌────────────────────┐
                   │ Transformer Module │
                   │ - Variable Rename  │
                   │ - Code Splitting   │
                   │ - Module Grouping  │
                   └────────────────────┘
                            │
                            ▼
                   ┌────────────────────┐
                   │  Codegen Module    │
                   │ - Pretty Print     │
                   │ - File Generation  │
                   │ - Documentation    │
                   └────────────────────┘
```

### 3.2 Module Breakdown

#### 3.2.1 Parser Module (`src/parser/mod.rs`)
**Responsibilities**:
- Load and parse the minified JavaScript file
- Create AST using Oxc parser
- Handle parsing errors gracefully
- Provide AST navigation utilities

**Key Components**:
```rust
pub struct Parser {
    allocator: Allocator,
    source_text: String,
}

pub struct ParseResult {
    pub program: Program<'_>,
    pub errors: Vec<OxcDiagnostic>,
    pub comments: Vec<Comment>,
}

impl Parser {
    pub fn new(source_text: String) -> Self;
    pub fn parse(&self) -> Result<ParseResult>;
}
```

#### 3.2.2 Analyzer Module (`src/analyzer/mod.rs`)
**Responsibilities**:
- Traverse AST to identify patterns
- Build symbol tables and call graphs
- Detect module boundaries
- Identify export/import relationships

**Sub-modules**:
- `src/analyzer/scope.rs` - Scope analysis
- `src/analyzer/bindings.rs` - Variable binding tracking
- `src/analyzer/callgraph.rs` - Function call relationships
- `src/analyzer/modules.rs` - Module detection

**Key Components**:
```rust
pub struct Analyzer<'a> {
    program: &'a Program<'a>,
    scope_tree: ScopeTree,
    bindings: HashMap<SymbolId, Binding>,
}

pub struct AnalysisResult {
    pub module_groups: Vec<ModuleGroup>,
    pub exports: Vec<Export>,
    pub global_scope: Scope,
}
```

#### 3.2.3 Extractor Module (`src/extractor/mod.rs`)
**Responsibilities**:
- Extract system prompts (multi-line strings)
- Extract tool definitions (JSON Schema-like objects)
- Extract configuration values
- Extract string literals and patterns

**Sub-modules**:
- `src/extractor/prompts.rs` - System prompt extraction
- `src/extractor/tools.rs` - Tool definition extraction
- `src/extractor/config.rs` - Configuration extraction
- `src/extractor/strings.rs` - String literal extraction

**Key Components**:
```rust
pub struct PromptExtractor;
impl PromptExtractor {
    pub fn extract(&self, program: &Program) -> Vec<SystemPrompt>;
}

pub struct ToolExtractor;
impl ToolExtractor {
    pub fn extract(&self, program: &Program) -> Vec<ToolDefinition>;
}

#[derive(Serialize, Deserialize)]
pub struct SystemPrompt {
    pub id: String,
    pub content: String,
    pub context: CodeLocation,
}

#[derive(Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
    pub context: CodeLocation,
}
```

#### 3.2.4 Transformer Module (`src/transformer/mod.rs`)
**Responsibilities**:
- Rename minified variables to meaningful names
- Split monolithic file into logical modules
- Group related functions/classes
- Add comments and documentation

**Sub-modules**:
- `src/transformer/rename.rs` - Variable renaming logic
- `src/transformer/split.rs` - Code splitting strategies
- `src/transformer/comments.rs` - Comment injection
- `src/transformer/organize.rs` - Module organization

**Key Components**:
```rust
pub struct VariableRenamer {
    rename_map: HashMap<String, String>,
}

impl VariableRenamer {
    pub fn analyze_and_rename(&mut self, program: &mut Program);
    fn suggest_name(&self, binding: &Binding) -> String;
}

pub struct ModuleSplitter {
    pub strategy: SplitStrategy,
}

pub enum SplitStrategy {
    ByExport,
    ByNamespace,
    ByFeature,
    Hybrid,
}
```

#### 3.2.5 Codegen Module (`src/codegen/mod.rs`)
**Responsibilities**:
- Pretty-print AST back to JavaScript
- Generate individual module files
- Create index files for module exports
- Generate documentation files

**Key Components**:
```rust
pub struct CodeGenerator {
    options: CodegenOptions,
}

impl CodeGenerator {
    pub fn generate(&self, program: &Program) -> String;
    pub fn generate_with_sourcemap(&self, program: &Program)
        -> (String, SourceMap);
}

pub struct FileWriter {
    output_dir: PathBuf,
}

impl FileWriter {
    pub fn write_module(&self, module: &Module, code: String)
        -> Result<()>;
    pub fn write_metadata(&self, metadata: &Metadata)
        -> Result<()>;
}
```

### 3.3 Output Structure

```
output/
├── extracted/
│   ├── system-prompts.json          # All system prompts
│   ├── tool-definitions.json        # All tool definitions
│   ├── configurations.json          # Config values
│   └── strings.json                 # String literals
├── modules/
│   ├── core/
│   │   ├── index.js
│   │   ├── main-loop.js
│   │   ├── message-processing.js
│   │   └── api-client.js
│   ├── tools/
│   │   ├── index.js
│   │   ├── bash.js
│   │   ├── read.js
│   │   ├── write.js
│   │   ├── edit.js
│   │   └── task.js
│   ├── prompts/
│   │   ├── index.js
│   │   ├── system-prompt.js
│   │   └── prompt-builder.js
│   ├── telemetry/
│   │   ├── index.js
│   │   ├── metrics.js
│   │   └── usage-tracking.js
│   ├── hooks/
│   │   ├── index.js
│   │   └── hook-system.js
│   └── utils/
│       ├── index.js
│       ├── git.js
│       └── fs.js
├── docs/
│   ├── architecture.md              # Generated architecture doc
│   ├── modules.md                   # Module documentation
│   ├── call-graph.md               # Function call relationships
│   └── data-flow.md                # Data flow analysis
└── analysis/
    ├── statistics.json              # Code statistics
    ├── complexity.json              # Complexity metrics
    └── dependencies.json            # Module dependencies
```

## 4. Implementation Strategy

### 4.1 Phase 1: Foundation (Week 1)

**Goals**: Set up project structure, implement basic parsing

**Tasks**:
1. Initialize Cargo project with all dependencies
2. Implement CLI interface with clap
3. Implement Parser module:
   - Load JavaScript file
   - Parse with Oxc
   - Handle errors gracefully
4. Implement basic AST visitor
5. Add unit tests for parser

**Deliverables**:
- Working CLI that can parse the input file
- Basic error reporting
- Unit tests with >80% coverage

### 4.2 Phase 2: Analysis & Extraction (Week 2)

**Goals**: Analyze AST and extract key information

**Tasks**:
1. Implement Analyzer module:
   - Scope analysis
   - Symbol table construction
   - Module boundary detection
2. Implement Extractor module:
   - System prompt extraction (pattern matching for large strings)
   - Tool definition extraction (JSON schema objects)
   - Configuration extraction
3. Output extracted data to JSON files
4. Add integration tests

**Deliverables**:
- `extracted/` directory with JSON files
- Analyzer with scope/symbol analysis
- Integration tests

**Key Extraction Patterns**:

```rust
// Pattern for system prompts
fn extract_prompts(program: &Program) -> Vec<SystemPrompt> {
    // Look for:
    // 1. String literals > 100 chars containing "You are Claude"
    // 2. Template literals with multiple lines
    // 3. String concatenations with "system" keywords
}

// Pattern for tool definitions
fn extract_tools(program: &Program) -> Vec<ToolDefinition> {
    // Look for objects with structure:
    // {
    //   name: "ToolName",
    //   description: "...",
    //   parameters: { $schema: "...", properties: {...} }
    // }
}
```

### 4.3 Phase 3: Transformation (Week 3)

**Goals**: Transform and organize code

**Tasks**:
1. Implement VariableRenamer:
   - Heuristic-based name suggestion
   - Context-aware renaming
   - Collision prevention
2. Implement ModuleSplitter:
   - Identify module boundaries
   - Group related functions
   - Extract cohesive modules
3. Implement code organization:
   - Group by feature/namespace
   - Create index files
4. Add transformation tests

**Deliverables**:
- Renamed variables with meaningful names
- Split code into modules
- Organized directory structure

**Renaming Heuristics**:

```rust
fn suggest_name(binding: &Binding, usage: &UsageAnalysis) -> String {
    // Heuristics:
    // 1. If assigned a function, use function purpose: "handle", "process", "create"
    // 2. If assigned a string literal, use content hint
    // 3. If used in return position, use "result", "output"
    // 4. If parameter, use position: "first", "second", or infer from call sites
    // 5. If constant, use SCREAMING_SNAKE_CASE
    // 6. Default: use position in scope + type hint
}
```

### 4.4 Phase 4: Code Generation (Week 4)

**Goals**: Generate readable output

**Tasks**:
1. Implement CodeGenerator:
   - Use oxc_codegen for pretty printing
   - Add formatting options
   - Generate sourcemaps (optional)
2. Implement FileWriter:
   - Write modules to files
   - Create directory structure
   - Generate metadata files
3. Implement documentation generator:
   - Architecture overview
   - Module documentation
   - Call graph visualization
4. Polish and optimize

**Deliverables**:
- Complete `output/` directory
- Readable, well-formatted code
- Documentation files
- Performance optimization

### 4.5 Phase 5: Testing & Refinement (Week 5)

**Goals**: Comprehensive testing and refinement

**Tasks**:
1. End-to-end testing
2. Performance profiling and optimization
3. Handle edge cases
4. Documentation and examples
5. User feedback and iteration

**Deliverables**:
- Production-ready tool
- Comprehensive documentation
- Performance benchmarks
- Release v1.0

## 5. Key Algorithms & Techniques

### 5.1 Variable Renaming Algorithm

**Challenge**: Minified variables like `QB9`, `IB9` → meaningful names

**Approach**: Multi-pass contextual analysis

```rust
pub struct RenameContext {
    // Pass 1: Collect usage patterns
    usage_sites: Vec<UsageSite>,

    // Pass 2: Analyze assignments
    assignments: Vec<Assignment>,

    // Pass 3: Infer types
    inferred_types: HashMap<SymbolId, InferredType>,

    // Pass 4: Generate names
    suggestions: HashMap<SymbolId, String>,
}

impl RenameContext {
    pub fn analyze(&mut self, program: &Program) {
        // Pass 1: Collect all usages
        self.collect_usages(program);

        // Pass 2: Analyze assignments and patterns
        self.analyze_assignments(program);

        // Pass 3: Type inference
        self.infer_types();

        // Pass 4: Generate meaningful names
        self.generate_names();
    }

    fn generate_names(&mut self) {
        for (symbol_id, usage) in &self.usage_sites {
            let name = match usage.context {
                UsageContext::FunctionCall(ref func) =>
                    format!("{}_function", func.purpose),
                UsageContext::PropertyAccess(ref prop) =>
                    prop.name.clone(),
                UsageContext::StringAssignment(ref str) =>
                    self.extract_hint_from_string(str),
                UsageContext::ObjectPattern =>
                    "config".to_string(),
                _ => format!("var_{}", symbol_id.0),
            };

            self.suggestions.insert(*symbol_id, name);
        }
    }
}
```

### 5.2 Module Splitting Algorithm

**Challenge**: 10MB single file → organized modules

**Approach**: Cluster analysis based on coupling/cohesion

```rust
pub struct ModuleSplitter {
    call_graph: CallGraph,
    coupling_matrix: CouplingMatrix,
}

impl ModuleSplitter {
    pub fn split(&self, program: &Program) -> Vec<Module> {
        // 1. Build call graph
        let call_graph = self.build_call_graph(program);

        // 2. Calculate coupling between functions
        let coupling = self.calculate_coupling(&call_graph);

        // 3. Apply clustering algorithm (Louvain community detection)
        let clusters = self.detect_communities(&coupling);

        // 4. Create modules from clusters
        let modules = self.create_modules(clusters);

        // 5. Optimize boundaries (move functions to reduce coupling)
        self.optimize_boundaries(modules)
    }

    fn calculate_coupling(&self, graph: &CallGraph) -> CouplingMatrix {
        // Coupling metrics:
        // - Direct calls
        // - Shared data access
        // - Common parameters
        // - Namespace hints (function name prefixes)
    }
}
```

### 5.3 System Prompt Extraction Algorithm

**Challenge**: Find multi-line system prompts in minified code

**Approach**: Pattern matching with context analysis

```rust
pub struct PromptExtractor {
    patterns: Vec<PromptPattern>,
}

impl PromptExtractor {
    pub fn extract(&self, program: &Program) -> Vec<SystemPrompt> {
        let mut prompts = Vec::new();

        // Visit all string literals
        for string_lit in self.find_string_literals(program) {
            if self.is_likely_prompt(&string_lit) {
                let prompt = SystemPrompt {
                    id: self.generate_id(&string_lit),
                    content: string_lit.value.clone(),
                    context: string_lit.span.into(),
                };
                prompts.push(prompt);
            }
        }

        prompts
    }

    fn is_likely_prompt(&self, string: &StringLiteral) -> bool {
        // Heuristics:
        // 1. Length > 100 chars
        // 2. Contains keywords: "You are Claude", "system", "prompt"
        // 3. Has instruction-like language
        // 4. Contains formatting markers

        string.value.len() > 100 &&
        (string.value.contains("You are Claude") ||
         string.value.contains("answer the user") ||
         string.value.contains("tool_use") ||
         string.value.contains("# "))
    }
}
```

### 5.4 Tool Definition Extraction Algorithm

**Challenge**: Find tool schema objects

**Approach**: AST pattern matching for object literals

```rust
pub struct ToolExtractor;

impl ToolExtractor {
    pub fn extract(&self, program: &Program) -> Vec<ToolDefinition> {
        let mut tools = Vec::new();

        // Look for object patterns matching tool definition schema
        for obj in self.find_object_literals(program) {
            if self.is_tool_definition(obj) {
                let tool = self.parse_tool_definition(obj);
                tools.push(tool);
            }
        }

        tools
    }

    fn is_tool_definition(&self, obj: &ObjectExpression) -> bool {
        // Must have: name, description, parameters
        // Parameters must have: $schema, properties

        obj.has_property("name") &&
        obj.has_property("description") &&
        obj.has_property("parameters") &&
        obj.get_property("parameters")
           .and_then(|p| p.as_object())
           .map(|p| p.has_property("$schema"))
           .unwrap_or(false)
    }

    fn parse_tool_definition(&self, obj: &ObjectExpression)
        -> ToolDefinition {
        ToolDefinition {
            name: obj.get_string_property("name").unwrap(),
            description: obj.get_string_property("description").unwrap(),
            parameters: obj.get_json_property("parameters"),
            context: obj.span.into(),
        }
    }
}
```

## 6. CLI Design

### 6.1 Command Structure

```bash
# Basic usage
claude-code-decypher ./vendors/claude -o ./output

# With options
claude-code-decypher ./vendors/claude \
  --output ./output \
  --extract-only \
  --format pretty \
  --rename-variables \
  --split-modules \
  --verbose

# Subcommands
claude-code-decypher parse ./vendors/claude         # Parse only
claude-code-decypher extract ./vendors/claude       # Extract data
claude-code-decypher transform ./vendors/claude     # Transform code
claude-code-decypher analyze ./vendors/claude       # Analyze only
```

### 6.2 CLI Implementation

```rust
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "claude-code-decypher")]
#[command(about = "Deobfuscate and analyze Claude Code JavaScript bundle")]
struct Cli {
    /// Input JavaScript file
    input: PathBuf,

    /// Output directory
    #[arg(short, long, default_value = "./output")]
    output: PathBuf,

    /// Verbosity level
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Parse JavaScript and output AST
    Parse {
        /// Output format (json, debug)
        #[arg(short, long, default_value = "json")]
        format: String,
    },

    /// Extract system prompts and tool definitions
    Extract {
        /// Extract only prompts
        #[arg(long)]
        prompts_only: bool,

        /// Extract only tools
        #[arg(long)]
        tools_only: bool,
    },

    /// Transform and organize code
    Transform {
        /// Enable variable renaming
        #[arg(long)]
        rename: bool,

        /// Split into modules
        #[arg(long)]
        split: bool,

        /// Module split strategy
        #[arg(long, default_value = "hybrid")]
        strategy: String,
    },

    /// Analyze code structure
    Analyze {
        /// Generate call graph
        #[arg(long)]
        call_graph: bool,

        /// Calculate complexity metrics
        #[arg(long)]
        complexity: bool,
    },
}
```

## 7. Testing Strategy

### 7.1 Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_minified_code() {
        let code = r#"var a=1;function b(){return a}"#;
        let parser = Parser::new(code.to_string());
        let result = parser.parse().unwrap();
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_extract_system_prompt() {
        let code = r#"
            const prompt = "You are Claude Code, Anthropic's official CLI...";
        "#;
        let extractor = PromptExtractor::new();
        let prompts = extractor.extract(&parse(code));
        assert_eq!(prompts.len(), 1);
        assert!(prompts[0].content.contains("You are Claude"));
    }

    #[test]
    fn test_variable_renaming() {
        let code = "var QB9=function(){return 42}";
        let renamer = VariableRenamer::new();
        let renamed = renamer.rename(&parse(code));
        assert!(!renamed.contains("QB9"));
    }
}
```

### 7.2 Integration Tests

```rust
#[test]
fn test_full_pipeline() {
    let input = PathBuf::from("./vendors/claude");
    let output = PathBuf::from("./test-output");

    let result = run_pipeline(input, output).unwrap();

    // Verify output structure
    assert!(output.join("extracted/system-prompts.json").exists());
    assert!(output.join("extracted/tool-definitions.json").exists());
    assert!(output.join("modules/core/index.js").exists());
    assert!(output.join("docs/architecture.md").exists());

    // Verify extracted data
    let prompts: Vec<SystemPrompt> =
        read_json(output.join("extracted/system-prompts.json"));
    assert!(!prompts.is_empty());
}
```

### 7.3 Benchmark Tests

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_parsing(c: &mut Criterion) {
    let code = std::fs::read_to_string("./vendors/claude").unwrap();

    c.bench_function("parse_10mb_bundle", |b| {
        b.iter(|| {
            let parser = Parser::new(code.clone());
            black_box(parser.parse())
        })
    });
}

criterion_group!(benches, benchmark_parsing);
criterion_main!(benches);
```

## 8. Performance Considerations

### 8.1 Memory Management

**Challenge**: 10MB file, large AST in memory

**Solutions**:
1. **Arena allocation**: Use `oxc_allocator` for fast, bulk deallocation
2. **Streaming extraction**: Process and write data incrementally
3. **Lazy evaluation**: Parse once, traverse multiple times without re-parsing

```rust
pub struct Pipeline {
    allocator: Allocator,
    // Reuse allocator across operations
}

impl Pipeline {
    pub fn run(&self, input: PathBuf) -> Result<()> {
        // Single parse
        let parser = Parser::new_with_allocator(&self.allocator, input);
        let ast = parser.parse()?;

        // Multiple passes without re-parsing
        self.extract_prompts(&ast)?;
        self.extract_tools(&ast)?;
        self.analyze_structure(&ast)?;
        self.transform_and_generate(&ast)?;

        Ok(())
    }
}
```

### 8.2 Performance Targets

- **Parsing**: < 100ms for 10MB file
- **Full pipeline**: < 5 seconds end-to-end
- **Memory usage**: < 500MB peak
- **Output generation**: < 1 second

### 8.3 Optimization Techniques

1. **Parallel processing**: Use `rayon` for parallel AST traversal
2. **Incremental updates**: Cache analysis results
3. **Smart visitor**: Skip irrelevant branches early
4. **String interning**: Reduce memory for repeated strings

## 9. Error Handling

### 9.1 Error Types

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DecypherError {
    #[error("Failed to read input file: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Failed to parse JavaScript: {0}")]
    ParseError(String),

    #[error("Failed to extract {0}: {1}")]
    ExtractionError(String, String),

    #[error("Failed to transform code: {0}")]
    TransformError(String),

    #[error("Failed to generate output: {0}")]
    CodegenError(String),
}

pub type Result<T> = std::result::Result<T, DecypherError>;
```

### 9.2 Error Recovery

- **Parsing errors**: Continue with partial AST
- **Extraction failures**: Log warning, continue with other extractions
- **Transformation errors**: Fallback to original code
- **Codegen errors**: Write error comments in output

## 10. Documentation Requirements

### 10.1 User Documentation

1. **README.md**: Quick start, installation, basic usage
2. **USAGE.md**: Detailed CLI reference
3. **ARCHITECTURE.md**: Tool architecture
4. **CONTRIBUTING.md**: Development guide

### 10.2 Generated Documentation

1. **output/docs/architecture.md**: Claude Code architecture analysis
2. **output/docs/modules.md**: Module documentation
3. **output/docs/call-graph.md**: Function relationships
4. **output/docs/tool-reference.md**: Extracted tool documentation

## 11. Future Enhancements

### 11.1 Phase 2 Features

1. **Interactive mode**: TUI for exploring code
2. **Diff analysis**: Compare versions of Claude Code
3. **Behavior simulation**: Test tool behaviors
4. **AI-assisted analysis**: Use Claude API to explain patterns
5. **Plugin system**: Custom extractors/transformers

### 11.2 Advanced Analysis

1. **Data flow analysis**: Track how data moves through system
2. **Security analysis**: Identify potential vulnerabilities
3. **Performance profiling**: Estimate runtime characteristics
4. **Compliance checking**: Verify against specifications

## 12. Success Criteria

The tool is successful if:

1. ✅ Parses the entire 10MB bundle without errors
2. ✅ Extracts all system prompts accurately
3. ✅ Extracts all tool definitions with complete schemas
4. ✅ Generates readable, well-formatted code
5. ✅ Organizes code into logical, coherent modules
6. ✅ Enables human understanding of Claude Code architecture
7. ✅ Performs within performance targets
8. ✅ Provides comprehensive documentation

## 13. Timeline

| Phase | Duration | Deliverables |
|-------|----------|-------------|
| Phase 1: Foundation | Week 1 | Parser, CLI, basic tests |
| Phase 2: Extraction | Week 2 | Extracted data, analyzer |
| Phase 3: Transformation | Week 3 | Renamed code, modules |
| Phase 4: Code Generation | Week 4 | Complete output, docs |
| Phase 5: Testing & Polish | Week 5 | Production release |

**Total Estimated Time**: 5 weeks

## 14. Dependencies & Prerequisites

### 14.1 Development Environment

- Rust 1.75+ (edition 2024)
- Cargo
- Git

### 14.2 Crate Dependencies

All dependencies are well-maintained, stable crates:
- **Oxc ecosystem**: Active development, strong community
- **Serde**: Industry standard for serialization
- **Clap**: De facto CLI framework
- **Anyhow/Thiserror**: Standard error handling

### 14.3 External Tools (Optional)

- `cargo-criterion`: For benchmarking
- `cargo-tarpaulin`: For coverage
- `cargo-audit`: For security audits

## 15. Risk Assessment

| Risk | Probability | Impact | Mitigation |
|------|------------|--------|------------|
| Oxc API changes | Low | Medium | Pin versions, monitor releases |
| Parsing errors | Medium | High | Robust error handling, fallbacks |
| Memory issues | Low | Medium | Arena allocation, streaming |
| Incorrect extraction | Medium | High | Extensive testing, validation |
| Performance issues | Low | Low | Profiling, optimization |

## 16. Conclusion

This design provides a comprehensive, production-ready approach to deobfuscating and analyzing the Claude Code JavaScript bundle. By leveraging the Oxc parser ecosystem and implementing sophisticated analysis algorithms, the tool will transform a 10MB minified file into readable, well-organized, documented code suitable for deep architectural analysis.

The modular architecture ensures maintainability and extensibility, while the phased implementation plan provides clear milestones and deliverables. Performance targets are achievable with proper use of arena allocation and efficient algorithms.

Upon completion, developers will have a powerful tool for understanding Claude Code's internal architecture, design decisions, and implementation patterns.
