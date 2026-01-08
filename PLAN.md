# AST-Based Code Splitting Implementation Plan

**Objective**: Implement real code splitting for JavaScript bundles while preserving beautified.js

**Status**: Planning Phase
**Priority**: High

## Executive Summary

Currently, the code splitting feature only generates placeholder files. This plan implements AST-based code extraction to create actual modular code with proper imports/exports while keeping the complete beautified.js file.

## Problem Statement

### Current State
- `output/beautified.js`: 465,350 lines of complete code ✅
- `output/modules/*.js`: Placeholder comments only ❌
- Module metadata exists (names, functions, categories) ✅
- No actual code distribution to modules ❌

### Desired State
- `output/beautified.js`: Complete code (preserved) ✅
- `output/modules/*.js`: Actual code segments with imports/exports ✅
- All functions accounted for in modules ✅
- No broken references or syntax errors ✅

## Technical Approach

### Strategy: AST-Based Extraction

Instead of line-based extraction, we use the parsed AST (Oxc) for accurate code extraction.

#### Why AST-Based?
- Preserves code structure and semantics
- Handles nested functions and closures correctly
- Manages minified variable names properly
- Generates syntactically correct code
- Works with obfuscated code

### Key Insights

1. **We have the call graph** from Phase 4 (5,321 functions, 12,482 calls)
   - Use this to guide module assignment
   - Detect dependencies automatically
   - Handle circular references

2. **Existing module metadata is valuable**
   - 8 modules already identified (core, tools, git, etc.)
   - Function lists provide "seeds" for assignment
   - Categories guide the affinity algorithm

3. **Oxc codegen is available**
   - Already used in Phase 3
   - Converts AST → code reliably
   - Handles all edge cases

## Architecture Design

### New Components

```
src/transformer/
├── function_extractor.rs    # Extract function AST nodes
├── dependency_analyzer.rs   # Build dependency graph
├── module_assigner.rs       # Assign functions to modules
├── import_generator.rs      # Generate ES6 imports/exports
└── code_assembler.rs        # Assemble final module code
```

### Data Structures

```rust
/// Information extracted for each function
pub struct FunctionInfo {
    pub name: String,
    pub ast_node: AstNode<'a>,
    pub dependencies: Vec<String>,      // Functions this calls
    pub variables_accessed: Vec<String>, // Outer scope variables
    pub start_pos: usize,
    pub end_pos: usize,
    pub assigned_module: Option<String>,
    pub is_exported: bool,
}

/// Complete module with generated code
pub struct ModuleCode {
    pub name: String,
    pub category: ModuleCategory,
    pub imports: Vec<ImportStatement>,
    pub exports: Vec<String>,
    pub functions: Vec<FunctionInfo>,
    pub code: String,
    pub line_count: usize,
}

/// Import statement structure
pub struct ImportStatement {
    pub module: String,          // './apiclient.js'
    pub imports: Vec<String>,    // ['api_request', 'handle_response']
    pub is_dynamic: bool,        // For dynamic imports
}
```

### Pipeline Integration

```
Phase 1: Parse (existing)
  ├─ Parse JavaScript with Oxc
  └─ Collect AST statistics

Phase 2: Extract (existing)
  ├─ Extract prompts, tools, configs
  └─ Extract strings

Phase 3: Transform (enhanced)
  ├─ Beautify code (existing)
  ├─ Extract function AST nodes (NEW)
  ├─ Build call graph (moved from Phase 4)
  ├─ Analyze dependencies (NEW)
  ├─ Assign functions to modules (NEW)
  ├─ Generate imports/exports (NEW)
  ├─ Assemble module code (NEW)
  └─ Write modules/*.js (enhanced)

Phase 4: Analyze (reordered)
  ├─ Build complexity metrics
  └─ Generate reports

Phase 5: Visualize (existing)
  └─ Generate diagrams
```

## Implementation Plan

### Milestone 1: Core Infrastructure (Week 1)

**Objective**: Extract functions from AST and build dependency graph

**Tasks**:
1. Create `src/transformer/function_extractor.rs`
   - Traverse AST to find all functions
   - Extract: FunctionDeclaration, FunctionExpression, ArrowFunctionExpression
   - Map function name → AST node + position
   - Handle anonymous functions (assign synthetic names)
   - **Test**: Extract functions from test fixtures

2. Create `src/transformer/dependency_analyzer.rs`
   - Use existing call graph from Phase 4
   - Build dependency matrix: function → [dependencies]
   - Detect circular dependencies
   - Identify shared utilities
   - **Test**: Verify dependencies on known call patterns

3. Add unit tests
   - Test with `fixtures/simple-function.js`
   - Test with `fixtures/nested-functions.js`
   - Test with `fixtures/class-methods.js`

**Deliverables**:
- Function extraction working
- Dependency graph built
- Unit tests passing
- Performance: <500ms for 11MB file

### Milestone 2: Module Assignment (Week 1-2)

**Objective**: Intelligently assign functions to modules

**Tasks**:
1. Create `src/transformer/module_assigner.rs`
   - Implement module affinity scoring algorithm
   - Use existing module metadata as seeds
   - Assign orphaned functions to best-fit modules
   - Handle edge cases (orphans, cycles)

2. Assignment Algorithm:
```rust
fn calculate_affinity(
    function: &FunctionInfo,
    module: &Module,
    call_graph: &CallGraph
) -> f64 {
    let mut score = 0.0;

    // Factor 1: Calls to functions in this module (40% weight)
    let calls_in_module = function.dependencies
        .iter()
        .filter(|dep| module.has_function(dep))
        .count();
    score += (calls_in_module as f64) / (function.dependencies.len() as f64) * 0.4;

    // Factor 2: String/keyword patterns (30% weight)
    let keyword_match = matches_keywords(&function, &module.keywords);
    score += if keyword_match { 0.3 } else { 0.0 };

    // Factor 3: Existing assignment hints (30% weight)
    if module.metadata.contains(&function.name) {
        score += 0.3;
    }

    score
}
```

3. Handle special cases:
   - Functions called by multiple modules → create "utils" module
   - Circular dependencies → allow circular imports
   - Very large functions → keep intact
   - Built-in functions → mark as external

**Deliverables**:
- All 6,630 functions assigned to modules
- Assignment quality >90% (manual verification on sample)
- Integration tests passing

### Milestone 3: Code Generation (Week 2)

**Objective**: Generate module code with imports/exports

**Tasks**:
1. Create `src/transformer/import_generator.rs`
   - Analyze each module's dependencies
   - Generate ES6 import statements
   - Handle default vs named exports
   - Resolve circular imports
   - **Test**: Import syntax validation

2. Create `src/transformer/code_assembler.rs`
   - Collect function AST nodes for each module
   - Add import statements at top
   - Add export statements for public interface
   - Use oxc_codegen to generate code
   - **Test**: Output syntax validation

3. Module format:
```javascript
// Module: core
// Auto-generated from AST analysis

// Imports
import { api_request } from './apiclient.js';
import { log_message, format_error } from './utils.js';

// Exports
export { main_loop, process_message, handle_request };

// === Function: main_loop ===
function main_loop(config) {
    // [Actual extracted code from AST]
    // ... 50+ lines ...
}

// === Function: process_message ===
function process_message(msg) {
    // [Actual extracted code]
    api_request(msg);  // Calls imported function
    log_message(msg);  // Calls imported function
}

// === Function: handle_request ===
function handle_request(req) {
    // [Actual extracted code]
}
```

**Deliverables**:
- All 8 modules generate valid code
- No syntax errors (verified by eslint/parser)
- Imports/exports correctly formatted
- Line counts sum to ≈ beautified.js

### Milestone 4: Integration (Week 2-3)

**Objective**: Integrate into main pipeline

**Tasks**:
1. Reorder call graph computation
   - Move from Phase 4 to early Phase 3
   - Or compute in Phase 1 as part of parsing
   - Ensure available before code splitting

2. Modify `src/transformer/mod.rs`
   - Add new modules to exports
   - Create unified `split_code()` function
   - Replace placeholder generation

3. Update `src/main.rs`
   - Modify Phase 3 handler
   - Remove placeholder generation code
   - Add progress indicators

4. Add CLI flags:
```rust
All {
    #[arg(long, default_value_t = true)]
    diagrams: bool,

    #[arg(long, default_value_t = true)]
    rename: bool,

    #[arg(long, default_value_t = true)]
    split: bool,

    #[arg(long, default_value = "ast")]
    split_strategy: String,  // NEW: "ast" | "placeholder"

    #[arg(long)]
    split_aggressiveness: Option<String>,  // NEW: "conservative" | "moderate" | "aggressive"

    #[arg(long, default_value_t = true)]
    keep_beautified: bool,  // NEW: always keep beautified.js

    #[arg(long)]
    detailed: bool,
}
```

**Deliverables**:
- End-to-end pipeline working
- All modules/*.js contain actual code
- Performance impact <3s additional
- Integration tests passing

### Milestone 5: Polish (Week 3)

**Objective**: Performance, error handling, documentation

**Tasks**:
1. Performance optimization
   - Profile bottlenecks
   - Optimize AST traversal
   - Cache call graph results
   - Parallelize code generation if needed

2. Error handling improvements
   - Graceful degradation (fallback to beautified.js)
   - Detailed error messages
   - Validation warnings

3. Documentation
   - Update README.md with new behavior
   - Create docs/CODE_SPLITTING.md
   - Update CLAUDE.md architecture
   - Add examples

4. Testing
   - Syntax validation (eslint)
   - Reference validation (imports → exports)
   - Completeness check (all functions accounted)
   - Round-trip test (if possible)

**Deliverables**:
- Performance budget met (<3s additional)
- Comprehensive documentation
- All tests passing
- Example outputs generated

## Technical Challenges & Solutions

### Challenge 1: Minified Variable Names

**Problem**: Function A in module1 calls function B in module2, both use minified names

**Solution**:
- Keep variable renaming (Phase 3) as-is
- Generate imports/exports using renamed names
- No cross-module variable references, only function calls
- Trust the renaming to be consistent

### Challenge 2: Closures and Nested Scopes

**Problem**: Functions access variables from outer scopes

**Solutions**:
- **Conservative** (MVP): Keep closure-heavy functions together
- **Advanced** (future): Analyze scope chains, extract state objects
- Decision: Start with conservative approach

### Challenge 3: IIFE and Bundled Structure

**Problem**: Code wrapped in IIFE or closures

**Solution**:
- Detect IIFE boundaries
- Split inside IIFE for better organization
- Maintain closure integrity
- Handle nested IIFEs carefully

### Challenge 4: Circular Dependencies

**Problem**: Module A depends on B, B depends on A

**Solution**:
- ES6 supports circular imports ✅
- Allow circular imports in generated code
- Alternative: merge tightly-coupled modules
- Detect during assignment phase

### Challenge 5: Shared Utilities

**Problem**: Function used by multiple modules

**Solutions**:
- Create "utils.js" or "common.js" module
- Duplicate code (if small)
- Attach to primary caller's module
- Decision: Create utils module for shared functions

### Challenge 6: Very Large Functions

**Problem**: 1000+ line functions, shouldn't be split

**Solution**:
- Detect size threshold
- Keep large functions intact
- Assign to single module
- Don't try to further decompose

## Testing Strategy

### Unit Tests

1. **Function Extraction Tests**
   - Test file: `tests/function_extractor_test.rs`
   - Fixtures: `fixtures/extraction/*.js`
   - Cover: simple functions, nested, classes, arrows

2. **Dependency Analysis Tests**
   - Test file: `tests/dependency_test.rs`
   - Verify: call graph, dependency detection
   - Cover: simple calls, nested calls, cycles

3. **Module Assignment Tests**
   - Test file: `tests/module_assignment_test.rs`
   - Verify: affinity scoring, orphan handling
   - Cover: edge cases, conflicts

### Integration Tests

1. **Small Fixture Test**
   - Input: `fixtures/test-bundle.js` (~100 lines)
   - Verify: all modules generated, valid syntax

2. **Medium Fixture Test**
   - Input: `fixtures/medium-bundle.js` (~1000 lines)
   - Verify: all functions accounted, no broken refs

3. **Full Pipeline Test**
   - Input: `./vendors/claude` (11MB)
   - Verify: performance, completeness

### Validation Tests

1. **Syntax Validation**
   ```bash
   for f in output/modules/*.js; do
     node -c $f || echo "Syntax error in $f"
   done
   ```

2. **Import/Export Validation**
   ```rust
   // Verify all imports resolve to exports
   for import in imports {
       assert!(exporting_module.exports.contains(import));
   }
   ```

3. **Completeness Check**
   ```rust
   assert_eq!(sum(modules.line_counts), beautified.line_count);
   assert_eq!(all_functions.len(), 6630);
   ```

## Performance Budget

| Component | Target | Notes |
|-----------|--------|-------|
| Function extraction | <500ms | Single AST traversal |
| Dependency analysis | <500ms | Reuse Phase 4 data |
| Module assignment | <500ms | Scoring algorithm |
| Code generation | <1000ms | oxc_codegen |
| **Total** | **<2.5s** | Additional to current 14s |

### Optimization Techniques

1. **Reuse existing work**
   - Call graph from Phase 4
   - Module metadata from Phase 3
   - Beautified code from Phase 3

2. **Efficient data structures**
   - HashMap for O(1) lookups
   - Arena allocation (Oxc)
   - Avoid cloning large ASTs

3. **Parallel processing** (if needed)
   - Module code generation is independent
   - Use Rayon for parallelism

4. **Lazy evaluation**
   - Don't compute what's not needed
   - Cache results

## Backward Compatibility

### Breaking Changes

The new behavior is a **breaking change** (improvement):
- Old: `modules/*.js` are placeholders
- New: `modules/*.js` contain actual code

### Migration Path

1. **Default behavior**: Use new AST-based splitting
2. **Legacy flag**: `--split-strategy placeholder` for old behavior
3. **Documentation**: Explain improvement in changelog

### Configuration

New CLI flags (all optional):
- `--split-strategy ast|placeholder` (default: ast)
- `--split-aggressiveness conservative|moderate|aggressive` (default: moderate)
- `--keep-beautified true|false` (default: true)

Future: `.decypher.toml` configuration file

## Success Criteria

### Functional Requirements

✅ All `modules/*.js` files contain actual code (not placeholders)
✅ Generated code has no syntax errors
✅ All imports resolve to valid exports
✅ All 6,630 functions accounted for
✅ No orphaned code segments
✅ `beautified.js` preserved unchanged

### Quality Requirements

✅ Assignment quality >90% (manual verification)
✅ Module size balanced (no 100K line single modules)
✅ Imports/exports properly formatted
✅ Code comments preserved
✅ Readable and maintainable output

### Performance Requirements

✅ Total pipeline time <20s (currently 14s)
✅ Code splitting adds <3s
✅ Memory usage reasonable (<2GB peak)

### Test Requirements

✅ Unit tests for all new modules
✅ Integration tests for end-to-end
✅ Validation tests for syntax/imports
✅ Performance benchmarks met

## Timeline

| Week | Milestone | Deliverables |
|------|-----------|--------------|
| 1 | Core Infrastructure | Function extraction, dependency analysis |
| 1-2 | Module Assignment | Affinity algorithm, integration tests |
| 2 | Code Generation | Import/export generation, code assembly |
| 2-3 | Integration | Pipeline integration, CLI flags |
| 3 | Polish | Optimization, documentation, testing |

**Critical Path**: Milestone 1 → Milestone 2 → Milestone 3 → Milestone 4

**Parallel Work**:
- Documentation can start in Week 2
- Testing runs throughout
- Performance optimization in Week 3

## Risks & Mitigations

| Risk | Impact | Probability | Mitigation |
|------|--------|-------------|------------|
| AST extraction fails | High | Medium | Fallback to placeholders, log errors |
| Circular dependencies | Medium | High | Allow circular imports, merge modules |
| Performance exceeds budget | Medium | Low | Optimization, parallel processing |
| Broken references | High | Medium | Validation tests, conservative assignment |
| Closures mishandled | High | Medium | Keep closures together, mark for future |

## Future Enhancements (Post-MVP)

1. **Advanced scope analysis**
   - Extract state objects from closures
   - Split closure-heavy modules
   - Handle prototype chains

2. **Smart de-duplication**
   - Detect duplicate utility functions
   - Consolidate into common modules
   - Reduce code redundancy

3. **TypeScript support**
   - Preserve type annotations
   - Generate .d.ts files
   - Handle TS-specific constructs

4. **Interactive refinement**
   - Manual module assignment overrides
   - Visual module editor
   - Real-time preview

5. **Optimization**
   - Dead code elimination
   - Tree shaking
   - Minification of modules

## References

- Oxc Parser: https://oxc.rs
- ES6 Modules: https://developer.mozilla.org/en-US/docs/Web/JavaScript/Guide/Modules
- Call Graph Analysis: Existing Phase 4 implementation
- Module metadata: Existing Phase 3 metadata

## Appendix

### Example: Before and After

**Before (Current Output)**:
```javascript
// output/modules/core.js
// Module: core
// Category: Core
// Functions: main_loop, message_processing, api_client

// Code will be organized here
```

**After (New Implementation)**:
```javascript
// output/modules/core.js
// Module: core
// Category: Core
// Auto-generated from AST analysis

// Imports
import { api_request, handle_response } from './apiclient.js';
import { log_message, format_error } from './utils.js';
import { parse_config } from './prompts.js';

// Exports
export { main_loop, message_processing, process_queue };

// === Function: main_loop ===
function main_loop(config) {
    const queue = [];
    let running = true;

    while (running) {
        const msg = queue.shift();
        if (msg) {
            message_processing(msg);
        }
    }
}

// === Function: message_processing ===
function message_processing(msg) {
    try {
        api_request(msg);  // Imported function
        log_message(msg);  // Imported function
    } catch (error) {
        format_error(error);  // Imported function
    }
}

// === Function: process_queue ===
function process_queue(items) {
    items.forEach(item => {
        message_processing(item);
    });
}
```

### Module Assignment Algorithm (Pseudocode)

```
function assign_functions_to_modules(functions, modules, call_graph):
    # Step 1: Assign seeded functions (from metadata)
    for module in modules:
        for func_name in module.metadata.function_list:
            if functions.has(func_name):
                functions[func_name].assigned_module = module.name

    # Step 2: Assign remaining functions by affinity
    unassigned = [f for f in functions if f.assigned_module is None]

    for function in unassigned:
        best_module = None
        best_score = -1

        for module in modules:
            score = calculate_affinity(function, module, call_graph)
            if score > best_score:
                best_score = score
                best_module = module

        if best_score > THRESHOLD:
            function.assigned_module = best_module.name
        else:
            # Orphan - assign to utils or core
            function.assigned_module = decide_orphan_module(function)

    # Step 3: Handle shared functions
    shared_functions = identify_shared_functions(call_graph)
    for func in shared_functions:
        if should_create_utils_module(shared_functions):
            create_utils_module(shared_functions)
        else:
            duplicate_to_calling_modules(func)

    return modules
```

---

**Document Version**: 1.0
**Last Updated**: 2025-01-05
**Author**: Claude Code Decypher Team
**Status**: Ready for Implementation
