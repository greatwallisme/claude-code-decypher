//! DOT (Graphviz) diagram generation.

use crate::analysis::callgraph::CallGraph;
use crate::transformer::split::Module;
use crate::Result;

/// Generator for DOT/Graphviz diagrams.
pub struct DotGenerator;

impl DotGenerator {
    /// Generate a DOT call graph diagram.
    pub fn generate_callgraph(call_graph: &CallGraph, limit: usize) -> Result<String> {
        let mut diagram = String::from("digraph CallGraph {\n");
        diagram.push_str("    rankdir=LR;\n");
        diagram.push_str("    node [shape=box, style=rounded];\n\n");

        // Add top functions
        let mut functions = call_graph.functions.clone();
        functions.sort_by(|a, b| b.calls_out.cmp(&a.calls_out));

        for func in functions.iter().take(limit) {
            let node_name = func.name.replace('-', "_").replace('.', "_");

            // Color by complexity (if we could infer it)
            let color = if func.calls_out > 5 {
                "lightcoral"
            } else if func.calls_out > 2 {
                "lightyellow"
            } else {
                "lightgreen"
            };

            diagram.push_str(&format!(
                "    {} [label=\"{}\\n{} calls\", fillcolor={}, style=filled];\n",
                node_name, func.name, func.calls_out, color
            ));

            // Add call relationships
            if let Some(callees) = call_graph.calls.get(&func.name) {
                for callee in callees.iter().take(3) {
                    let callee_name = callee.replace('-', "_").replace('.', "_");
                    diagram.push_str(&format!("    {} -> {};\n", node_name, callee_name));
                }
            }
        }

        diagram.push_str("}\n");
        Ok(diagram)
    }

    /// Generate a DOT module architecture diagram.
    pub fn generate_modules(modules: &[Module]) -> Result<String> {
        let mut diagram = String::from("digraph Modules {\n");
        diagram.push_str("    rankdir=TB;\n");
        diagram.push_str("    node [shape=component, style=filled];\n\n");

        for module in modules {
            let node_id = module.name.replace('-', "_");

            let color = match module.category {
                crate::transformer::split::ModuleCategory::Core => "lightblue",
                crate::transformer::split::ModuleCategory::Tools => "lightgreen",
                crate::transformer::split::ModuleCategory::ApiClient => "lightyellow",
                _ => "lightgray",
            };

            diagram.push_str(&format!(
                "    {} [label=\"{}\\n{:?}\\n{} lines\", fillcolor={}];\n",
                node_id, module.name, module.category, module.estimated_lines, color
            ));
        }

        // Add dependencies
        diagram.push_str("\n    // Dependencies\n");
        if modules.iter().any(|m| m.name == "core") && modules.iter().any(|m| m.name == "tools") {
            diagram.push_str("    core -> tools;\n");
        }
        if modules.iter().any(|m| m.name == "core") && modules.iter().any(|m| m.name == "apiclient") {
            diagram.push_str("    core -> apiclient;\n");
        }

        diagram.push_str("}\n");
        Ok(diagram)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transformer::split::ModuleCategory;

    #[test]
    fn test_generate_dot_modules() {
        let modules = vec![
            Module {
                name: "core".to_string(),
                category: ModuleCategory::Core,
                estimated_lines: 1000,
                functions: vec![],
                keywords: vec![],
            },
        ];

        let dot = DotGenerator::generate_modules(&modules).unwrap();

        assert!(dot.contains("digraph Modules"));
        assert!(dot.contains("core"));
        assert!(dot.contains("lightblue"));
    }
}
