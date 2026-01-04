# Instructions

## generate a design and implementation plan for claude-code-decypher

I want to use rust to build a tool to decypher claude code. It is a uglified js code in ./vendors/claude. Find a best js parser / unuglify crates to make the code more meaningful and readable. The ultimate goal is to:

1. extract all the system prompts, tool definitions, and other relevant information from the jscode.
2. the js code should be clear enough and separated in different files regarding different components, sub systems, etc. so that I could further use claude code to understand its architecture, design decisions, behavior, and so on.

Think this ultra hard, and generate a detailed design and implementation plan in ./specs/0001-design-and-plan.md.

## improve extraction

Read the ./output/beautified.js file progressively to understand how things are structured and core components. Looking at existing code in ./src carefully, and think ultra hard to forma new design to extract system prompts organize them entirely (not partial ones), and extract tool definitions with correct descriptions and parameters. Looks at how they are extracted currently in ./output/extracted/system-prompts.json and ./output/extracted/tool-definitions.json. With all these knowledge, create a new design and implementation plan in ./specs/0002-extraction-improvement.md. Make sure you prefer to AST extraction rather than regex extraction.
