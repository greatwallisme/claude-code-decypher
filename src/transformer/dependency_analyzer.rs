//! Dependency analysis for functions.

use crate::transformer::function_extractor::FunctionInfo;
use std::collections::{HashMap, HashSet};
use tracing::debug;

/// Dependency graph for functions.
#[derive(Debug, Clone)]
pub struct DependencyGraph {
    /// Function dependencies (function -> [functions it calls]).
    pub dependencies: HashMap<String, Vec<String>>,

    /// Reverse dependencies (function -> [functions that call it]).
    pub reverse_dependencies: HashMap<String, Vec<String>>,

    /// All unique functions.
    pub all_functions: HashSet<String>,

    /// Circular dependency chains detected.
    pub circular_deps: Vec<Vec<String>>,
}

/// Analyzes dependencies between functions.
pub struct DependencyAnalyzer;

impl DependencyAnalyzer {
    /// Analyze dependencies from function information.
    pub fn analyze(functions: &[FunctionInfo]) -> DependencyGraph {
        debug!("Analyzing dependencies for {} functions", functions.len());

        let mut dependencies: HashMap<String, Vec<String>> = HashMap::new();
        let mut reverse_dependencies: HashMap<String, Vec<String>> = HashMap::new();
        let mut all_functions: HashSet<String> = HashSet::new();

        // Collect all functions and their dependencies
        for func in functions {
            all_functions.insert(func.name.clone());

            if !func.dependencies.is_empty() {
                // Filter dependencies to only include functions we know about
                let known_deps: Vec<String> = func
                    .dependencies
                    .iter()
                    .filter(|dep| functions.iter().any(|f| &f.name == *dep))
                    .cloned()
                    .collect();

                dependencies.insert(func.name.clone(), known_deps.clone());

                // Build reverse dependencies
                for dep in &known_deps {
                    reverse_dependencies
                        .entry(dep.clone())
                        .or_default()
                        .push(func.name.clone());
                }
            } else {
                dependencies.insert(func.name.clone(), Vec::new());
            }
        }

        // Detect circular dependencies
        let circular_deps = Self::detect_circular_dependencies(&dependencies);

        debug!(
            "Found {} functions, {} dependency edges, {} circular dependencies",
            all_functions.len(),
            dependencies.values().map(|v| v.len()).sum::<usize>(),
            circular_deps.len()
        );

        DependencyGraph {
            dependencies,
            reverse_dependencies,
            all_functions,
            circular_deps,
        }
    }

    /// Detect circular dependencies using depth-first search.
    fn detect_circular_dependencies(
        dependencies: &HashMap<String, Vec<String>>,
    ) -> Vec<Vec<String>> {
        let mut cycles: Vec<Vec<String>> = Vec::new();
        let mut visited: HashSet<String> = HashSet::new();
        let mut rec_stack: HashSet<String> = HashSet::new();
        let mut path: Vec<String> = Vec::new();

        for func in dependencies.keys() {
            if !visited.contains(func) {
                Self::dfs(
                    func,
                    dependencies,
                    &mut visited,
                    &mut rec_stack,
                    &mut path,
                    &mut cycles,
                );
            }
        }

        cycles
    }

    fn dfs(
        node: &str,
        dependencies: &HashMap<String, Vec<String>>,
        visited: &mut HashSet<String>,
        rec_stack: &mut HashSet<String>,
        path: &mut Vec<String>,
        cycles: &mut Vec<Vec<String>>,
    ) {
        visited.insert(node.to_string());
        rec_stack.insert(node.to_string());
        path.push(node.to_string());

        if let Some(deps) = dependencies.get(node) {
            for dep in deps {
                if !visited.contains(dep) {
                    Self::dfs(dep, dependencies, visited, rec_stack, path, cycles);
                } else if rec_stack.contains(dep) {
                    // Found a cycle
                    let cycle_start = path.iter().position(|p| p == dep).unwrap();
                    let cycle: Vec<String> = path[cycle_start..].to_vec();
                    if !cycles.contains(&cycle) {
                        cycles.push(cycle);
                    }
                }
            }
        }

        path.pop();
        rec_stack.remove(node);
    }
}

impl DependencyGraph {
    /// Find shared utilities (functions called by multiple modules).
    pub fn find_shared_functions(
        &self,
        function_modules: &HashMap<String, String>,
        threshold: usize,
    ) -> Vec<String> {
        let mut shared: Vec<String> = Vec::new();

        for (func, callers) in &self.reverse_dependencies {
            if callers.len() >= threshold {
                // Check if called by different modules
                let modules: HashSet<&str> = callers
                    .iter()
                    .filter_map(|caller| function_modules.get(caller).map(|s| s.as_str()))
                    .collect();

                if modules.len() >= 2 {
                    shared.push(func.clone());
                }
            }
        }

        shared
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transformer::function_extractor::SpanInfo;

    #[test]
    fn test_analyze_dependencies() {
        let functions = vec![
            FunctionInfo {
                name: "foo".to_string(),
                span: SpanInfo { start: 0, end: 100 },
                param_count: 0,
                is_anonymous: false,
                dependencies: vec!["bar".to_string()],
                outer_variables: vec![],
                assigned_module: None,
                is_exported: false,
            },
            FunctionInfo {
                name: "bar".to_string(),
                span: SpanInfo { start: 100, end: 200 },
                param_count: 0,
                is_anonymous: false,
                dependencies: vec!["baz".to_string()],
                outer_variables: vec![],
                assigned_module: None,
                is_exported: false,
            },
            FunctionInfo {
                name: "baz".to_string(),
                span: SpanInfo { start: 200, end: 300 },
                param_count: 0,
                is_anonymous: false,
                dependencies: vec![],
                outer_variables: vec![],
                assigned_module: None,
                is_exported: false,
            },
        ];

        let graph = DependencyAnalyzer::analyze(&functions);

        assert_eq!(graph.all_functions.len(), 3);
        assert_eq!(graph.dependencies.get("foo").unwrap().len(), 1);
        assert_eq!(graph.dependencies.get("foo").unwrap()[0], "bar");
        assert_eq!(graph.reverse_dependencies.get("bar").unwrap().len(), 1);
    }

    #[test]
    fn test_detect_circular_dependencies() {
        let functions = vec![
            FunctionInfo {
                name: "a".to_string(),
                span: SpanInfo { start: 0, end: 100 },
                param_count: 0,
                is_anonymous: false,
                dependencies: vec!["b".to_string()],
                outer_variables: vec![],
                assigned_module: None,
                is_exported: false,
            },
            FunctionInfo {
                name: "b".to_string(),
                span: SpanInfo { start: 100, end: 200 },
                param_count: 0,
                is_anonymous: false,
                dependencies: vec!["c".to_string()],
                outer_variables: vec![],
                assigned_module: None,
                is_exported: false,
            },
            FunctionInfo {
                name: "c".to_string(),
                span: SpanInfo { start: 200, end: 300 },
                param_count: 0,
                is_anonymous: false,
                dependencies: vec!["a".to_string()],
                outer_variables: vec![],
                assigned_module: None,
                is_exported: false,
            },
        ];

        let graph = DependencyAnalyzer::analyze(&functions);

        assert!(!graph.circular_deps.is_empty());
    }

    #[test]
    fn test_find_shared_functions() {
        let functions = vec![
            FunctionInfo {
                name: "util".to_string(),
                span: SpanInfo { start: 0, end: 100 },
                param_count: 0,
                is_anonymous: false,
                dependencies: vec![],
                outer_variables: vec![],
                assigned_module: None,
                is_exported: false,
            },
            FunctionInfo {
                name: "foo".to_string(),
                span: SpanInfo { start: 100, end: 200 },
                param_count: 0,
                is_anonymous: false,
                dependencies: vec!["util".to_string()],
                outer_variables: vec![],
                assigned_module: Some("module1".to_string()),
                is_exported: false,
            },
            FunctionInfo {
                name: "bar".to_string(),
                span: SpanInfo { start: 200, end: 300 },
                param_count: 0,
                is_anonymous: false,
                dependencies: vec!["util".to_string()],
                outer_variables: vec![],
                assigned_module: Some("module2".to_string()),
                is_exported: false,
            },
        ];

        let graph = DependencyAnalyzer::analyze(&functions);

        let mut function_modules = HashMap::new();
        function_modules.insert("foo".to_string(), "module1".to_string());
        function_modules.insert("bar".to_string(), "module2".to_string());

        let shared = graph.find_shared_functions(&function_modules, 2);

        assert!(shared.contains(&"util".to_string()));
    }
}
