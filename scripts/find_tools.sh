#!/bin/bash
# Extract tool definitions from beautified code

echo "Searching for tool name constants..."

# Find tool name constants (capitalized words in quotes)
grep -n '^var [a-zA-Z0-9_]* = "[A-Z][a-zA-Z]*";$' ./output/beautified.js | \
  grep -v 'var.*= "SYSRES\|var.*= "ERROR\|var.*= "SIG' | \
  head -30

echo ""
echo "Found tool constants"
