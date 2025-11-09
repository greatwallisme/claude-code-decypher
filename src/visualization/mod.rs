//! Visualization module for generating graphs and diagrams.

pub mod mermaid;
pub mod dot;

use crate::analysis::callgraph::CallGraph;
use crate::transformer::split::Module;
use crate::Result;

/// Main visualizer for generating various diagram formats.
pub struct Visualizer;

impl Visualizer {
    /// Generate a Mermaid diagram from call graph.
    pub fn callgraph_to_mermaid(call_graph: &CallGraph, limit: usize) -> Result<String> {
        mermaid::MermaidGenerator::generate_callgraph(call_graph, limit)
    }

    /// Generate a Mermaid diagram from modules.
    pub fn modules_to_mermaid(modules: &[Module]) -> Result<String> {
        mermaid::MermaidGenerator::generate_modules(modules)
    }

    /// Generate a DOT diagram from call graph.
    pub fn callgraph_to_dot(call_graph: &CallGraph, limit: usize) -> Result<String> {
        dot::DotGenerator::generate_callgraph(call_graph, limit)
    }

    /// Generate a DOT diagram from modules.
    pub fn modules_to_dot(modules: &[Module]) -> Result<String> {
        dot::DotGenerator::generate_modules(modules)
    }
}
