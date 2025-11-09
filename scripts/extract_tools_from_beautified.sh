#!/bin/bash

echo "Extracting tools from beautified.js..."
echo ""

# Find all tool name constants
grep '^var [a-zA-Z0-9_]* = "[A-Z][a-zA-Z]*"' ./output/beautified.js | \
  grep -E '"Bash"|"Read"|"Write"|"Edit"|"Grep"|"Glob"|"Task"|"TodoWrite"|"NotebookEdit"|"WebFetch"|"WebSearch"|"Skill"|"SlashCommand"|"AskUserQuestion"|"ExitPlanMode"|"BashOutput"|"KillShell"' | \
  sed 's/var \([a-zA-Z0-9_]*\) = "\([^"]*\)".*/Tool: \2 (constant: \1)/'

echo ""
echo "Total tools found with this pattern"
