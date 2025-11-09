# Instructions

## generate a design and implementation plan for claude-code-decypher

I want to use rust to build a tool to decypher claude code. It is a uglified js code in ./vendors/claude. Find a best js parser / unuglify crates to make the code more meaningful and readable. The ultimate goal is to:

1. extract all the system prompts, tool definitions, and other relevant information from the jscode.
2. the js code should be clear enough and separated in different files regarding different components, sub systems, etc. so that I could further use claude code to understand its architecture, design decisions, behavior, and so on.

Think this ultra hard, and generate a detailed design and implementation plan in ./specs/0001-design-and-plan.md.
