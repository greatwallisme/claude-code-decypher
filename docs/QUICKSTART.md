# Quick Start Guide

## Install

```bash
cargo build --release
```

## Fastest Start - One Command Does Everything!

```bash
# Just run this - it does ALL 5 phases automatically!
cargo run --release -- ./vendors/claude

# That's it! You'll get:
# - Beautified code (417K lines)
# - Extracted prompts, tools, configs
# - Module organization (7 modules)
# - Call graph analysis (3,391 functions)
# - Complexity metrics
# - Visual diagrams
# - Complete dashboard
# - 26 files in ./output/ in ~14 seconds
```

## Customize the Analysis

```bash
# Without renaming
cargo run -- ./vendors/claude all --split --diagrams

# Minimal (no diagrams, no rename, no split)
cargo run -- ./vendors/claude all

# Maximum (everything + detailed output)
cargo run -- ./vendors/claude all --diagrams --rename --split --detailed
```

## Commands

- `parse` - AST statistics
- `extract` - Extract data
- `transform` - Beautify code
- `analyze` - Deep analysis
- `dashboard` - Complete overview

## Output

- `beautified.js` - Readable code
- `extracted/` - Prompts, tools, configs
- `modules/` - Organized modules
- `analysis/` - Call graph, complexity
- `diagrams/` - Mermaid & DOT
- `docs/` - Documentation
- `dashboard.json` - All metrics

See README.md for complete documentation.
