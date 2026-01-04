# Extraction Improvement Design

## Executive Summary

This document outlines a comprehensive redesign of the extraction system to accurately extract complete system prompts and tool definitions with full descriptions and parameters from minified/beautified JavaScript bundles. The current implementation has significant limitations:

1. **Incomplete Prompts**: System prompts are fragmented - we're getting partial strings instead of complete multi-paragraph documentation
2. **Missing Tool Descriptions**: Tool definitions lack their actual descriptions and only show generic placeholders
3. **No Parameter Extraction**: Input/output schemas are not being extracted despite being present in the code
4. **Over-reliance on Regex**: Current approach uses regex patterns which are brittle for complex nested structures

## Current State Analysis

### Current Extraction Results

**System Prompts (`system-prompts.json`)**:
- Contains 374 entries but many are fragments
- Example issues:
  - `prompt_368`: " tool has been optimized for correct permissions..." (incomplete, missing beginning)
  - `prompt_816`: " characters, output will be truncated..." (fragment)
  - `prompt_826`: Middle section of instructions without context

**Tool Definitions (`tool-definitions.json`)**:
- Contains 17 entries but with incomplete information
- Example issues:
  - All tools show generic descriptions like "A powerful search tool built on ripgrep" (incorrect)
  - Missing actual descriptions like "Reads a file from the local filesystem..."
  - No parameter schemas extracted
  - Low confidence scores (0.6-0.8)

### Code Structure Analysis

After analyzing `beautified.js` (417,477 lines), the structure is:

```javascript
// 1. Description variables are assigned first (short summary)
var MHB = "Update the todo list for the current session. To be used proactively...";

// 2. Lazy initialization blocks contain full prompts in template literals
var OHB = lazy_init(() => {
    LHB = `Use this tool to create and manage a structured task list...

    ## When to Use This Tool
    Use this tool proactively in these scenarios:
    ...
    ` // Multi-paragraph, markdown-formatted documentation
});

// 3. Schema definitions use a builder pattern (k.object, k.strictObject, etc.)
var Rr4, Tr4, pG;
var wn = lazy_init(() => {
    C9();
    OHB();
    Wz1();
    Rr4 = k.strictObject({
        todos: eQA.describe("The updated todo list")
    }),
    Tr4 = k.object({
        oldTodos: eQA.describe("The todo list before the update"),
        newTodos: eQA.describe("The todo list after the update")
    }),

    // 4. Tool definition objects
    pG = {
        name: UvA,                    // Variable reference: "TodoWrite"
        strict: !0,
        async description() {
            return MHB;                // Returns description variable
        },
        async prompt() {
            return LHB;                // Returns full prompt variable
        },
        inputSchema: Rr4,              // References schema object
        outputSchema: Tr4,
        userFacingName() { return ""; },
        isEnabled() { return !0; },
        // ... other methods
    };
});

// Example for Read tool:
var G7 = "Read", oWA = 2e3, zr4 = 2e3,
    FHB = "Read a file from the local filesystem.",
    CHB; // Declared but assigned later

var wD = lazy_init(() => {
    Gz1();
    CHB = `Reads a file from the local filesystem. You can access any file directly by using this tool.
Assume this tool is able to read all files on the machine...

Usage:
- The file_path parameter must be an absolute path, not a relative path
...
`;
});
```

**Key Patterns**:
1. Tool name: Simple string variable (`var UvA = "TodoWrite"`)
2. Short description: Variable assignment (`var MHB = "short desc"`)
3. Full prompt: Template literal in lazy_init block (`LHB = \`full docs...\``)
4. Input schema: Object built with `k.object()` or `k.strictObject()`
5. Tool object: Object literal with async methods returning variables

## Root Cause Analysis

### Why Current Extraction Fails

1. **String Literal Extraction Limitations**:
   - AST's `StringLiteral` nodes only capture individual string values
   - Multi-line template literals are stored as single nodes but we're not properly extracting their full content
   - Variable references in tool objects (like `return MHB`) can't be resolved without symbol tracking

2. **Lazy Initialization Pattern**:
   - Prompts are inside `lazy_init(() => { ... })` blocks
   - These are CallExpression nodes containing arrow functions
   - Content is assigned inside function body, not directly in declarations

3. **Variable Reference Chain**:
   - Tool object has `async description() { return MHB }`
   - `MHB` is a variable that needs to be resolved
   - Current code doesn't track variable-to-value mappings

4. **Schema Builder Pattern**:
   - Schemas use `k.object({ ... })` with chained `.describe()` calls
   - This creates a CallExpression tree, not a simple ObjectExpression
   - Current extraction doesn't handle this builder pattern

## Proposed Solution: AST-Based Multi-Pass Extraction

### Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                    Extraction Pipeline                       │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
        ┌───────────────────────────────────────┐
        │   Pass 1: Symbol Table Builder        │
        │   - Map variable names to values      │
        │   - Extract string literals           │
        │   - Extract template literals         │
        │   - Track lazy_init blocks            │
        └───────────────────────────────────────┘
                            │
                            ▼
        ┌───────────────────────────────────────┐
        │   Pass 2: Tool Definition Extractor   │
        │   - Find tool objects                 │
        │   - Resolve name/description refs     │
        │   - Extract inputSchema/outputSchema  │
        │   - Build complete tool definitions   │
        └───────────────────────────────────────┘
                            │
                            ▼
        ┌───────────────────────────────────────┐
        │   Pass 3: Prompt Extractor            │
        │   - Identify complete prompts         │
        │   - Merge fragments from same context │
        │   - Categorize by content             │
        │   - Associate with tools if applicable│
        └───────────────────────────────────────┘
                            │
                            ▼
        ┌───────────────────────────────────────┐
        │   Pass 4: Schema Extractor            │
        │   - Parse k.object() call chains      │
        │   - Extract property definitions      │
        │   - Resolve .describe() annotations   │
        │   - Convert to JSON Schema format     │
        └───────────────────────────────────────┘
```

### Pass 1: Enhanced Symbol Table Builder

**Purpose**: Build a comprehensive mapping of variable names to their actual values.

**Implementation**:

```rust
pub struct SymbolTableBuilder<'a> {
    program: &'a Program<'a>,
    symbols: HashMap<String, SymbolValue>,
    lazy_init_blocks: Vec<LazyInitBlock>,
}

#[derive(Debug, Clone)]
pub enum SymbolValue {
    String(String),
    TemplateLiteral(String),
    Number(f64),
    Boolean(bool),
    ObjectRef(String),  // Reference to another symbol
    Unknown,
}

#[derive(Debug)]
pub struct LazyInitBlock {
    pub function_name: Option<String>,
    pub assignments: Vec<(String, SymbolValue)>,
    pub span: Span,
}

impl<'a> SymbolTableBuilder<'a> {
    /// Main extraction method
    pub fn build(&mut self) -> SymbolTable {
        // Visit all top-level variable declarations
        self.visit_program();

        // Process lazy_init blocks
        self.process_lazy_init_blocks();

        // Resolve indirect references
        self.resolve_references();

        SymbolTable {
            symbols: self.symbols.clone(),
            lazy_blocks: self.lazy_init_blocks.clone(),
        }
    }

    /// Visit variable declarations
    fn visit_variable_declaration(&mut self, decl: &VariableDeclaration) {
        for declarator in &decl.declarations {
            if let Some(id) = declarator.id.get_binding_identifier() {
                let name = id.name.to_string();

                if let Some(init) = &declarator.init {
                    let value = self.extract_value(init);
                    self.symbols.insert(name, value);
                }
            }
        }
    }

    /// Extract value from any expression
    fn extract_value(&self, expr: &Expression) -> SymbolValue {
        match expr {
            Expression::StringLiteral(s) => {
                SymbolValue::String(s.value.to_string())
            }
            Expression::TemplateLiteral(tmpl) => {
                // Extract complete template literal content
                let mut content = String::new();
                for (i, quasi) in tmpl.quasis.iter().enumerate() {
                    content.push_str(&quasi.value.raw);
                    if i < tmpl.expressions.len() {
                        // For expressions, try to resolve or use placeholder
                        if let Some(resolved) = self.try_resolve_expr(&tmpl.expressions[i]) {
                            content.push_str(&resolved);
                        } else {
                            content.push_str("${...}");
                        }
                    }
                }
                SymbolValue::TemplateLiteral(content)
            }
            Expression::CallExpression(call) => {
                // Check if this is a lazy_init call
                if self.is_lazy_init_call(call) {
                    self.extract_lazy_init_block(call);
                }
                SymbolValue::Unknown
            }
            Expression::Identifier(id) => {
                SymbolValue::ObjectRef(id.name.to_string())
            }
            _ => SymbolValue::Unknown,
        }
    }

    /// Check if a call is to lazy_init
    fn is_lazy_init_call(&self, call: &CallExpression) -> bool {
        if let Expression::Identifier(id) = &call.callee {
            id.name == "lazy_init"
        } else {
            false
        }
    }

    /// Extract assignments from lazy_init block
    fn extract_lazy_init_block(&mut self, call: &CallExpression) {
        if let Some(arg) = call.arguments.first() {
            if let Argument::ArrowFunctionExpression(arrow) = arg {
                // Parse function body for assignments
                let assignments = self.extract_assignments_from_block(&arrow.body);

                self.lazy_init_blocks.push(LazyInitBlock {
                    function_name: None,
                    assignments,
                    span: arrow.span,
                });
            }
        }
    }

    /// Extract variable assignments from a function block
    fn extract_assignments_from_block(&self, body: &FunctionBody) -> Vec<(String, SymbolValue)> {
        let mut assignments = Vec::new();

        for stmt in &body.statements {
            if let Statement::ExpressionStatement(expr_stmt) = stmt {
                if let Expression::AssignmentExpression(assign) = &expr_stmt.expression {
                    // Extract: variableName = value
                    if let AssignmentTarget::SimpleAssignmentTarget(
                        SimpleAssignmentTarget::AssignmentTargetIdentifier(id)
                    ) = &assign.left {
                        let name = id.name.to_string();
                        let value = self.extract_value(&assign.right);
                        assignments.push((name, value));
                    }
                }
            }
        }

        assignments
    }

    /// Process lazy_init blocks and merge into symbol table
    fn process_lazy_init_blocks(&mut self) {
        for block in &self.lazy_init_blocks {
            for (name, value) in &block.assignments {
                self.symbols.insert(name.clone(), value.clone());
            }
        }
    }

    /// Resolve indirect references (variable references to other variables)
    fn resolve_references(&mut self) {
        let max_iterations = 10;

        for _ in 0..max_iterations {
            let mut changed = false;
            let symbols_copy = self.symbols.clone();

            for (name, value) in self.symbols.iter_mut() {
                if let SymbolValue::ObjectRef(ref_name) = value {
                    if let Some(resolved) = symbols_copy.get(ref_name) {
                        if !matches!(resolved, SymbolValue::ObjectRef(_)) {
                            *value = resolved.clone();
                            changed = true;
                        }
                    }
                }
            }

            if !changed {
                break;
            }
        }
    }
}
```

### Pass 2: Tool Definition Extractor

**Purpose**: Extract complete tool definitions with resolved names, descriptions, and schemas.

```rust
pub struct ToolDefinitionExtractor<'a> {
    program: &'a Program<'a>,
    symbol_table: &'a SymbolTable,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub short_description: String,
    pub full_prompt: String,
    pub input_schema: Option<JsonValue>,
    pub output_schema: Option<JsonValue>,
    pub properties: ToolProperties,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolProperties {
    pub is_strict: bool,
    pub is_enabled: bool,
    pub is_read_only: bool,
    pub is_concurrency_safe: bool,
    pub user_facing_name: Option<String>,
}

impl<'a> ToolDefinitionExtractor<'a> {
    pub fn extract(&self) -> Result<Vec<ToolDefinition>> {
        let mut tools = Vec::new();

        // Find all object expressions that match tool pattern
        for obj in self.find_tool_objects() {
            if let Some(tool) = self.extract_tool_from_object(obj) {
                tools.push(tool);
            }
        }

        Ok(tools)
    }

    /// Find objects that look like tool definitions
    fn find_tool_objects(&self) -> Vec<&'a ObjectExpression<'a>> {
        let mut objects = Vec::new();

        // Visit all object expressions
        self.visit_objects(&mut objects);

        // Filter to those with tool-like structure
        objects.into_iter()
            .filter(|obj| self.is_tool_object(obj))
            .collect()
    }

    /// Check if an object matches tool pattern
    fn is_tool_object(&self, obj: &ObjectExpression) -> bool {
        let has_name = self.has_property(obj, "name");
        let has_async_description = self.has_async_method(obj, "description");
        let has_async_prompt = self.has_async_method(obj, "prompt");
        let has_input_schema = self.has_property(obj, "inputSchema");

        // Must have name + (description OR prompt) to be a tool
        has_name && (has_async_description || has_async_prompt || has_input_schema)
    }

    /// Extract complete tool definition from object
    fn extract_tool_from_object(&self, obj: &ObjectExpression) -> Option<ToolDefinition> {
        // Extract name (resolving variable reference)
        let name = self.extract_property_value(obj, "name")?;
        let name_str = self.resolve_to_string(&name)?;

        // Extract descriptions
        let short_desc = self.extract_method_return_value(obj, "description")
            .and_then(|v| self.resolve_to_string(&v))
            .unwrap_or_default();

        let full_prompt = self.extract_method_return_value(obj, "prompt")
            .and_then(|v| self.resolve_to_string(&v))
            .unwrap_or_default();

        // Extract schemas
        let input_schema = self.extract_property_value(obj, "inputSchema")
            .and_then(|v| self.resolve_to_schema(&v));

        let output_schema = self.extract_property_value(obj, "outputSchema")
            .and_then(|v| self.resolve_to_schema(&v));

        // Extract properties
        let properties = self.extract_tool_properties(obj);

        // Calculate confidence
        let confidence = self.calculate_confidence(
            &name_str,
            &short_desc,
            &full_prompt,
            &input_schema
        );

        Some(ToolDefinition {
            name: name_str,
            short_description: short_desc,
            full_prompt,
            input_schema,
            output_schema,
            properties,
            confidence,
        })
    }

    /// Extract return value from async method
    fn extract_method_return_value(&self, obj: &ObjectExpression, method_name: &str)
        -> Option<Expression>
    {
        for prop in &obj.properties {
            if let ObjectPropertyKind::ObjectProperty(p) = prop {
                // Check if this is the right method
                if !self.matches_key(&p.key, method_name) {
                    continue;
                }

                // Extract from function body
                if let Expression::FunctionExpression(func) = &p.value {
                    // Look for return statement
                    if let Some(body) = &func.body {
                        for stmt in &body.statements {
                            if let Statement::ReturnStatement(ret) = stmt {
                                if let Some(arg) = &ret.argument {
                                    return Some(arg.clone());
                                }
                            }
                        }
                    }
                }
                // Also handle arrow functions: async description() => MHB
                else if let Expression::ArrowFunctionExpression(arrow) = &p.value {
                    if let FunctionBody::Expression(expr) = &arrow.body {
                        return Some((**expr).clone());
                    }
                }
            }
        }
        None
    }

    /// Resolve an expression to a string using symbol table
    fn resolve_to_string(&self, expr: &Expression) -> Option<String> {
        match expr {
            Expression::StringLiteral(s) => {
                Some(s.value.to_string())
            }
            Expression::TemplateLiteral(_) => {
                // Already extracted in symbol table
                self.symbol_table.resolve_template_expr(expr)
            }
            Expression::Identifier(id) => {
                // Look up in symbol table
                self.symbol_table.get_string_value(&id.name.to_string())
            }
            _ => None,
        }
    }

    /// Resolve schema reference to JSON
    fn resolve_to_schema(&self, expr: &Expression) -> Option<JsonValue> {
        // Schema might be:
        // 1. Direct identifier (variable name)
        // 2. Call expression (k.object(...))

        match expr {
            Expression::Identifier(id) => {
                // Look up schema in symbol table and convert
                self.symbol_table.get_schema(&id.name.to_string())
            }
            Expression::CallExpression(call) => {
                // Parse k.object() call
                self.parse_schema_builder_call(call)
            }
            _ => None,
        }
    }

    /// Calculate confidence score
    fn calculate_confidence(&self, name: &str, short: &str, full: &str, schema: &Option<JsonValue>)
        -> f32
    {
        let mut score = 0.0;

        if !name.is_empty() { score += 0.2; }
        if !short.is_empty() && short.len() > 20 { score += 0.2; }
        if !full.is_empty() && full.len() > 100 { score += 0.4; }
        if schema.is_some() { score += 0.2; }

        score.min(1.0)
    }
}
```

### Pass 3: Enhanced Prompt Extractor

**Purpose**: Extract complete, unfragmented system prompts.

```rust
pub struct EnhancedPromptExtractor<'a> {
    symbol_table: &'a SymbolTable,
    tool_definitions: &'a [ToolDefinition],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemPrompt {
    pub id: String,
    pub content: String,
    pub length: usize,
    pub category: PromptCategory,
    pub context: PromptContext,
    pub associated_tool: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PromptContext {
    /// Standalone system prompt
    Standalone,
    /// Part of tool documentation
    ToolDocumentation(String),
    /// Part of larger multi-section prompt
    Section { parent_id: String, section_name: String },
}

impl<'a> EnhancedPromptExtractor<'a> {
    pub fn extract(&self) -> Result<Vec<SystemPrompt>> {
        let mut prompts = Vec::new();

        // Strategy 1: Extract tool-associated prompts
        for tool in self.tool_definitions {
            if !tool.full_prompt.is_empty() {
                prompts.push(SystemPrompt {
                    id: format!("tool_prompt_{}", tool.name),
                    content: tool.full_prompt.clone(),
                    length: tool.full_prompt.len(),
                    category: PromptCategory::Tool,
                    context: PromptContext::ToolDocumentation(tool.name.clone()),
                    associated_tool: Some(tool.name.clone()),
                });
            }
        }

        // Strategy 2: Extract standalone template literals
        for (name, value) in &self.symbol_table.symbols {
            if let SymbolValue::TemplateLiteral(content) = value {
                if self.is_system_prompt(content) {
                    // Check if already extracted as tool prompt
                    let is_duplicate = prompts.iter()
                        .any(|p| p.content == *content);

                    if !is_duplicate {
                        prompts.push(SystemPrompt {
                            id: format!("standalone_prompt_{}", prompts.len()),
                            content: content.clone(),
                            length: content.len(),
                            category: self.categorize_prompt(content),
                            context: PromptContext::Standalone,
                            associated_tool: None,
                        });
                    }
                }
            }
        }

        // Strategy 3: Merge related fragments
        prompts = self.merge_fragments(prompts);

        Ok(prompts)
    }

    /// Check if content is a system prompt
    fn is_system_prompt(&self, content: &str) -> bool {
        // Minimum length threshold
        if content.len() < 150 {
            return false;
        }

        // Must have prompt-like characteristics
        let indicators = [
            "You are Claude",
            "Usage notes:",
            "IMPORTANT:",
            "## ",
            "When to use",
            "This tool",
            "Example:",
            "Parameters:",
        ];

        indicators.iter().any(|ind| content.contains(ind))
    }

    /// Merge related prompt fragments
    fn merge_fragments(&self, prompts: Vec<SystemPrompt>) -> Vec<SystemPrompt> {
        // Look for prompts that should be merged based on:
        // 1. Sequential variable names (e.g., LHB, MHB)
        // 2. Content continuation (one ends, another begins)
        // 3. Same lazy_init block

        // For now, return as-is (can be enhanced later)
        prompts
    }
}
```

### Pass 4: Schema Extractor

**Purpose**: Parse builder pattern schemas into JSON Schema format.

```rust
pub struct SchemaExtractor<'a> {
    symbol_table: &'a SymbolTable,
}

impl<'a> SchemaExtractor<'a> {
    /// Parse k.object() or k.strictObject() call
    pub fn parse_schema_builder_call(&self, call: &CallExpression) -> Option<JsonValue> {
        // Check if this is k.object() or k.strictObject()
        if !self.is_schema_builder_call(call) {
            return None;
        }

        // Get the argument (should be object expression)
        let arg = call.arguments.first()?;

        if let Argument::ObjectExpression(obj_expr) = arg {
            let mut properties = serde_json::Map::new();

            for prop in &obj_expr.properties {
                if let ObjectPropertyKind::ObjectProperty(p) = prop {
                    let key = self.extract_key(&p.key)?;

                    // Value might be:
                    // 1. Another schema builder call (nested)
                    // 2. Variable reference
                    // 3. Call to .describe()

                    let value = self.parse_schema_value(&p.value)?;
                    properties.insert(key, value);
                }
            }

            Some(JsonValue::Object(properties))
        } else {
            None
        }
    }

    /// Parse schema value (might have .describe() call)
    fn parse_schema_value(&self, expr: &Expression) -> Option<JsonValue> {
        match expr {
            // Handle: k.string().describe("...")
            Expression::CallExpression(call) => {
                // Check if this is a .describe() call
                if self.is_describe_call(call) {
                    // Extract the description
                    let description = self.extract_describe_arg(call)?;

                    // Get the base type from callee
                    let base_type = self.extract_base_schema_type(&call.callee)?;

                    let mut schema = base_type;
                    if let JsonValue::Object(ref mut map) = schema {
                        map.insert("description".to_string(), JsonValue::String(description));
                    }

                    Some(schema)
                } else {
                    // Might be k.string(), k.number(), etc.
                    self.parse_type_call(call)
                }
            }
            // Handle: variable reference
            Expression::Identifier(id) => {
                self.symbol_table.get_schema(&id.name.to_string())
            }
            _ => None,
        }
    }

    /// Check if call is to .describe()
    fn is_describe_call(&self, call: &CallExpression) -> bool {
        if let Expression::StaticMemberExpression(member) = &call.callee {
            if let Expression::Identifier(id) = &member.property {
                return id.name == "describe";
            }
        }
        false
    }

    /// Extract base schema type from k.string(), k.number(), etc.
    fn parse_type_call(&self, call: &CallExpression) -> Option<JsonValue> {
        // Check if this is k.string(), k.number(), k.boolean(), etc.
        if let Expression::StaticMemberExpression(member) = &call.callee {
            if let Expression::Identifier(obj) = &member.object {
                if obj.name == "k" {
                    if let Expression::Identifier(prop) = &member.property {
                        return Some(self.type_name_to_schema(&prop.name.to_string()));
                    }
                }
            }
        }
        None
    }

    /// Convert type name to JSON schema
    fn type_name_to_schema(&self, type_name: &str) -> JsonValue {
        let mut schema = serde_json::Map::new();

        match type_name {
            "string" => {
                schema.insert("type".to_string(), JsonValue::String("string".to_string()));
            }
            "number" => {
                schema.insert("type".to_string(), JsonValue::String("number".to_string()));
            }
            "boolean" => {
                schema.insert("type".to_string(), JsonValue::String("boolean".to_string()));
            }
            "object" | "strictObject" => {
                schema.insert("type".to_string(), JsonValue::String("object".to_string()));
            }
            "array" => {
                schema.insert("type".to_string(), JsonValue::String("array".to_string()));
            }
            _ => {}
        }

        JsonValue::Object(schema)
    }
}
```

## Implementation Plan

### Phase 1: Core Infrastructure (Week 1)

**Tasks**:
1. Implement enhanced `SymbolTableBuilder`
   - Variable declaration extraction
   - Template literal extraction
   - Lazy init block parsing
   - Reference resolution

2. Extend `SymbolTable` with new methods
   - `get_string_value(name: &str) -> Option<String>`
   - `get_schema(name: &str) -> Option<JsonValue>`
   - `resolve_template_expr(expr: &Expression) -> Option<String>`

3. Add comprehensive unit tests
   - Test variable resolution
   - Test lazy_init extraction
   - Test reference chain resolution

**Files to modify**:
- `src/analyzer/symbols.rs` - Enhance symbol table
- `src/analyzer/mod.rs` - Add new analyzer methods

### Phase 2: Tool Extraction (Week 1-2)

**Tasks**:
1. Implement `ToolDefinitionExtractor`
   - Object pattern matching
   - Property extraction
   - Method return value resolution
   - Schema extraction

2. Implement `SchemaExtractor`
   - Builder pattern parsing
   - `.describe()` call handling
   - Nested schema support

3. Integration tests
   - Test against real beautified.js snippets
   - Validate extracted tool definitions

**Files to modify**:
- `src/extractor/tools.rs` - Replace with new implementation
- Add `src/extractor/schemas.rs` - New schema extractor

### Phase 3: Prompt Extraction (Week 2)

**Tasks**:
1. Implement `EnhancedPromptExtractor`
   - Tool prompt association
   - Standalone prompt extraction
   - Fragment merging

2. Improve categorization
   - Better heuristics
   - Context tracking

3. Validation
   - Compare with current extraction
   - Ensure no regression

**Files to modify**:
- `src/extractor/prompts.rs` - Enhance implementation

### Phase 4: Integration & Testing (Week 2-3)

**Tasks**:
1. Update `Extractor` coordinator
   - Wire up new extractors
   - Maintain backward compatibility

2. Add end-to-end tests
   - Test with full beautified.js
   - Validate output quality

3. Performance optimization
   - Profile extraction
   - Optimize hot paths

4. Documentation
   - Update README
   - Add extraction examples

**Files to modify**:
- `src/extractor/mod.rs` - Update coordinator
- `tests/` - Add integration tests

### Phase 5: Output Validation (Week 3)

**Tasks**:
1. Create validation tools
   - Compare old vs new output
   - Metrics: completeness, accuracy

2. Manual review
   - Spot-check extracted prompts
   - Verify tool definitions

3. Fix issues
   - Address edge cases
   - Improve extraction quality

## Expected Improvements

### Quantitative Goals

1. **System Prompts**:
   - Current: 374 entries, ~30% complete
   - Target: ~50-80 entries, >95% complete
   - Reduce fragmentation by merging related content

2. **Tool Definitions**:
   - Current: 17 entries, missing descriptions
   - Target: 15-20 entries with complete documentation
   - Include full prompts (multi-paragraph)
   - Include parameter schemas

3. **Accuracy**:
   - Current confidence: 0.6-0.8
   - Target confidence: 0.9-1.0
   - Validation: Manual review of top 10 tools

### Qualitative Improvements

1. **Complete Documentation**:
   - Full tool prompts with usage examples
   - Complete parameter descriptions
   - Proper categorization

2. **Structured Data**:
   - JSON Schema format for parameters
   - Proper type information
   - Constraint documentation

3. **Maintainability**:
   - Pure AST-based extraction (no regex)
   - Clear separation of concerns
   - Extensible architecture

## Risk Mitigation

### Potential Issues

1. **Incomplete Symbol Resolution**:
   - Risk: Some variables can't be resolved
   - Mitigation: Fallback to heuristic extraction
   - Mitigation: Log unresolved references for debugging

2. **Schema Builder Complexity**:
   - Risk: Complex chained calls might fail
   - Mitigation: Incremental implementation
   - Mitigation: Extensive test coverage

3. **Performance**:
   - Risk: Multi-pass extraction might be slow
   - Mitigation: Profile and optimize
   - Mitigation: Parallel processing where possible

4. **Edge Cases**:
   - Risk: Unexpected code patterns
   - Mitigation: Comprehensive test suite
   - Mitigation: Graceful degradation

## Success Criteria

### Minimum Viable Product (MVP)

1. Extract at least 15 tools with:
   - Complete names
   - Full documentation (>500 chars)
   - Input schemas in JSON format
   - Confidence >0.9

2. Extract at least 50 complete prompts:
   - No fragments
   - Proper categorization
   - Tool association where applicable

3. Pass all existing tests
4. No performance regression (< 2x slower)

### Stretch Goals

1. Extract 100% of tools from beautified.js
2. Auto-generate TypeScript definitions from schemas
3. Create interactive documentation browser
4. Extract function call graphs

## Conclusion

This redesign addresses the fundamental limitations of the current extraction approach by:

1. **Understanding the actual code structure** (lazy_init, variable references)
2. **Using pure AST traversal** instead of fragile regex patterns
3. **Building a symbol table** to resolve variable references
4. **Multi-pass extraction** for proper dependency handling
5. **Complete prompt extraction** without fragmentation

The result will be high-quality, structured extraction of system prompts and tool definitions that accurately represents the Claude Code implementation.
