//! Mermaid diagram generation.

use crate::analysis::callgraph::CallGraph;
use crate::transformer::split::Module;
use crate::Result;

/// Generator for Mermaid diagrams.
pub struct MermaidGenerator;

impl MermaidGenerator {
    /// Generate a Mermaid call graph diagram.
    pub fn generate_callgraph(call_graph: &CallGraph, limit: usize) -> Result<String> {
        let mut diagram = String::from("```mermaid\ngraph TD\n");

        // Add top functions by call count
        let mut functions = call_graph.functions.clone();
        functions.sort_by(|a, b| b.calls_out.cmp(&a.calls_out));

        for (idx, func) in functions.iter().take(limit).enumerate() {
            let node_id = format!("F{}", idx);
            diagram.push_str(&format!("    {}[\"{}\"]\n", node_id, func.name));

            // Add calls if available
            if let Some(callees) = call_graph.calls.get(&func.name) {
                for (callee_idx, callee) in callees.iter().take(3).enumerate() {
                    let callee_id = format!("F{}C{}", idx, callee_idx);
                    diagram.push_str(&format!("    {}[\"{}\"]\n", callee_id, callee));
                    diagram.push_str(&format!("    {} --> {}\n", node_id, callee_id));
                }
            }
        }

        diagram.push_str("```\n");
        Ok(diagram)
    }

    /// Generate a Mermaid module architecture diagram.
    pub fn generate_modules(modules: &[Module]) -> Result<String> {
        let mut diagram = String::from("```mermaid\ngraph LR\n");

        for module in modules {
            let node_id = module.name.replace('-', "_");

            diagram.push_str(&format!(
                "    {}[\"{}\\n{:?}\\n{} lines\"]\n",
                node_id, module.name, module.category, module.estimated_lines
            ));

            // Style by category
            let style = match module.category {
                crate::transformer::split::ModuleCategory::Core => "fill:#f9f,stroke:#333,stroke-width:4px",
                crate::transformer::split::ModuleCategory::Tools => "fill:#bbf,stroke:#333,stroke-width:2px",
                crate::transformer::split::ModuleCategory::ApiClient => "fill:#bfb,stroke:#333,stroke-width:2px",
                _ => "fill:#fbb,stroke:#333,stroke-width:2px",
            };

            diagram.push_str(&format!("    style {} {}\n", node_id, style));
        }

        // Add relationships based on common patterns
        diagram.push_str("\n    %% Module Dependencies\n");
        if modules.iter().any(|m| m.name == "core") && modules.iter().any(|m| m.name == "tools") {
            diagram.push_str("    core --> tools\n");
        }
        if modules.iter().any(|m| m.name == "core") && modules.iter().any(|m| m.name == "apiclient") {
            diagram.push_str("    core --> apiclient\n");
        }
        if modules.iter().any(|m| m.name == "tools") && modules.iter().any(|m| m.name == "utils") {
            diagram.push_str("    tools --> utils\n");
        }

        diagram.push_str("```\n");
        Ok(diagram)
    }

    /// Generate a flowchart of the transformation pipeline.
    pub fn generate_pipeline_flowchart() -> String {
        r#"```mermaid
flowchart LR
    A[Minified JS] --> B[Parse]
    B --> C[AST]
    C --> D[Extract]
    C --> E[Transform]
    C --> F[Analyze]
    D --> G[Prompts/Tools/Config]
    E --> H[Beautified Code]
    E --> I[Modules]
    F --> J[Call Graph]
    F --> K[Complexity]
    F --> L[Metrics]

    style A fill:#f9f,stroke:#333,stroke-width:2px
    style C fill:#bbf,stroke:#333,stroke-width:2px
    style G fill:#bfb,stroke:#333,stroke-width:2px
    style H fill:#bfb,stroke:#333,stroke-width:2px
    style I fill:#bfb,stroke:#333,stroke-width:2px
    style J fill:#fbb,stroke:#333,stroke-width:2px
    style K fill:#fbb,stroke:#333,stroke-width:2px
    style L fill:#fbb,stroke:#333,stroke-width:2px
```
"#.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transformer::split::ModuleCategory;

    #[test]
    fn test_generate_modules_diagram() {
        let modules = vec![
            Module {
                name: "core".to_string(),
                category: ModuleCategory::Core,
                estimated_lines: 1000,
                functions: vec!["main".to_string()],
                keywords: vec![],
            },
            Module {
                name: "tools".to_string(),
                category: ModuleCategory::Tools,
                estimated_lines: 500,
                functions: vec!["bash".to_string()],
                keywords: vec![],
            },
        ];

        let diagram = MermaidGenerator::generate_modules(&modules).unwrap();

        assert!(diagram.contains("```mermaid"));
        assert!(diagram.contains("graph LR"));
        assert!(diagram.contains("core"));
        assert!(diagram.contains("tools"));
    }

    #[test]
    fn test_generate_pipeline() {
        let diagram = MermaidGenerator::generate_pipeline_flowchart();

        assert!(diagram.contains("```mermaid"));
        assert!(diagram.contains("flowchart"));
        assert!(diagram.contains("Parse"));
        assert!(diagram.contains("Extract"));
    }
}
