//! Analysis report generation.

use super::{callgraph::CallGraph, complexity::ComplexityMetrics, metrics::CodeMetrics};
use crate::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use tracing::info;

/// Comprehensive analysis report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisReport {
    /// Call graph analysis.
    pub call_graph: CallGraph,

    /// Complexity metrics.
    pub complexity: ComplexityMetrics,

    /// Code metrics.
    pub metrics: CodeMetrics,
}

impl AnalysisReport {
    /// Write the report to JSON file.
    pub fn write_json(&self, output_dir: &Path) -> Result<()> {
        let analysis_dir = output_dir.join("analysis");
        fs::create_dir_all(&analysis_dir)
            .map_err(|e| crate::error::DecypherError::io(&analysis_dir, e))?;

        // Write call graph
        let cg_path = analysis_dir.join("call-graph.json");
        let cg_json = serde_json::to_string_pretty(&self.call_graph)
            .map_err(|e| crate::error::DecypherError::Other(e.into()))?;
        fs::write(&cg_path, cg_json)
            .map_err(|e| crate::error::DecypherError::io(&cg_path, e))?;
        info!("Wrote call graph to {}", cg_path.display());

        // Write complexity metrics
        let complexity_path = analysis_dir.join("complexity.json");
        let complexity_json = serde_json::to_string_pretty(&self.complexity)
            .map_err(|e| crate::error::DecypherError::Other(e.into()))?;
        fs::write(&complexity_path, complexity_json)
            .map_err(|e| crate::error::DecypherError::io(&complexity_path, e))?;
        info!("Wrote complexity metrics to {}", complexity_path.display());

        // Write code metrics
        let metrics_path = analysis_dir.join("metrics.json");
        let metrics_json = serde_json::to_string_pretty(&self.metrics)
            .map_err(|e| crate::error::DecypherError::Other(e.into()))?;
        fs::write(&metrics_path, metrics_json)
            .map_err(|e| crate::error::DecypherError::io(&metrics_path, e))?;
        info!("Wrote code metrics to {}", metrics_path.display());

        Ok(())
    }

    /// Generate a markdown report.
    pub fn generate_markdown(&self, output_dir: &Path) -> Result<()> {
        let docs_dir = output_dir.join("docs");
        fs::create_dir_all(&docs_dir)
            .map_err(|e| crate::error::DecypherError::io(&docs_dir, e))?;

        let mut content = String::from("# Code Analysis Report\n\n");

        // Call Graph Section
        content.push_str("## Call Graph Analysis\n\n");
        content.push_str(&format!("- **Total Functions**: {}\n", self.call_graph.unique_functions));
        content.push_str(&format!("- **Total Calls**: {}\n", self.call_graph.total_calls));
        content.push_str(&format!(
            "- **Average Calls per Function**: {:.2}\n\n",
            if self.call_graph.unique_functions > 0 {
                self.call_graph.total_calls as f32 / self.call_graph.unique_functions as f32
            } else {
                0.0
            }
        ));

        // Complexity Section
        content.push_str("## Complexity Metrics\n\n");
        content.push_str(&format!("- **Average Cyclomatic Complexity**: {:.2}\n", self.complexity.avg_cyclomatic));
        content.push_str(&format!("- **Max Cyclomatic Complexity**: {}\n", self.complexity.max_cyclomatic));
        content.push_str(&format!("- **Most Complex Function**: {}\n", self.complexity.most_complex_function));
        content.push_str(&format!("- **Total Decision Points**: {}\n", self.complexity.total_decision_points));
        content.push_str(&format!("- **Average Nesting Depth**: {:.2}\n", self.complexity.avg_nesting_depth));
        content.push_str(&format!("- **Max Nesting Depth**: {}\n\n", self.complexity.max_nesting_depth));

        // Code Metrics Section
        content.push_str("## Code Metrics\n\n");
        content.push_str(&format!("- **Total Lines of Code**: {}\n", self.metrics.total_loc));
        content.push_str(&format!("- **Functions**: {}\n", self.metrics.function_count));
        content.push_str(&format!("- **Classes**: {}\n", self.metrics.class_count));
        content.push_str(&format!("- **Variables**: {}\n", self.metrics.variable_count));
        content.push_str(&format!("- **Imports**: {}\n", self.metrics.import_count));
        content.push_str(&format!("- **Exports**: {}\n", self.metrics.export_count));
        content.push_str(&format!("- **Avg Function Length**: {:.1} lines\n", self.metrics.avg_function_length));
        content.push_str(&format!("- **Max Function Length**: {} lines\n\n", self.metrics.max_function_length));

        // Top Complex Functions
        if !self.complexity.function_complexity.is_empty() {
            content.push_str("## Most Complex Functions\n\n");
            content.push_str("| Function | Cyclomatic | Nesting | Params | Statements |\n");
            content.push_str("|----------|------------|---------|--------|------------|\n");

            let mut sorted = self.complexity.function_complexity.clone();
            sorted.sort_by(|a, b| b.cyclomatic.cmp(&a.cyclomatic));

            for func in sorted.iter().take(10) {
                content.push_str(&format!(
                    "| {} | {} | {} | {} | {} |\n",
                    func.name, func.cyclomatic, func.nesting_depth, func.param_count, func.statement_count
                ));
            }
        }

        let path = docs_dir.join("analysis-report.md");
        fs::write(&path, content)
            .map_err(|e| crate::error::DecypherError::io(&path, e))?;
        info!("Generated analysis report at {}", path.display());

        Ok(())
    }

    /// Print a summary to stdout.
    pub fn print_summary(&self) {
        println!("\n=== Analysis Report ===\n");

        println!("Call Graph:");
        println!("  Functions:       {}", self.call_graph.unique_functions);
        println!("  Total Calls:     {}", self.call_graph.total_calls);

        println!("\nComplexity:");
        println!("  Avg Cyclomatic:  {:.2}", self.complexity.avg_cyclomatic);
        println!("  Max Cyclomatic:  {}", self.complexity.max_cyclomatic);
        println!("  Most Complex:    {}", self.complexity.most_complex_function);
        println!("  Decision Points: {}", self.complexity.total_decision_points);

        println!("\nCode Metrics:");
        println!("  Total LOC:       {}", self.metrics.total_loc);
        println!("  Functions:       {}", self.metrics.function_count);
        println!("  Variables:       {}", self.metrics.variable_count);
        println!("  Avg Func Length: {:.1} lines", self.metrics.avg_function_length);
    }
}
