//! Advanced analysis module for call graphs, complexity, and metrics.

pub mod callgraph;
pub mod complexity;
pub mod metrics;
pub mod report;

use crate::Result;
use oxc_ast::ast::Program;
use serde::{Deserialize, Serialize};

/// Main analyzer that performs deep code analysis.
pub struct AdvancedAnalyzer<'a> {
    program: &'a Program<'a>,
}

impl<'a> AdvancedAnalyzer<'a> {
    /// Create a new advanced analyzer.
    pub fn new(program: &'a Program<'a>) -> Self {
        Self { program }
    }

    /// Build a call graph.
    pub fn build_call_graph(&self) -> Result<callgraph::CallGraph> {
        callgraph::CallGraphBuilder::new(self.program).build()
    }

    /// Calculate complexity metrics.
    pub fn calculate_complexity(&self) -> Result<complexity::ComplexityMetrics> {
        complexity::ComplexityCalculator::new(self.program).calculate()
    }

    /// Calculate code metrics.
    pub fn calculate_metrics(&self) -> Result<metrics::CodeMetrics> {
        metrics::MetricsCalculator::new(self.program).calculate()
    }

    /// Generate a comprehensive analysis report.
    pub fn generate_report(&self) -> Result<report::AnalysisReport> {
        let call_graph = self.build_call_graph()?;
        let complexity = self.calculate_complexity()?;
        let metrics = self.calculate_metrics()?;

        Ok(report::AnalysisReport {
            call_graph,
            complexity,
            metrics,
        })
    }
}
