//! Module assignment using affinity scoring.

use crate::transformer::{
    dependency_analyzer::DependencyGraph,
    function_extractor::FunctionInfo,
    split::{Module, ModuleCategory},
};
use std::collections::HashMap;
use tracing::debug;

/// Configuration for module assignment.
#[derive(Debug, Clone)]
pub struct AssignmentConfig {
    /// Minimum affinity score to assign to a module.
    pub min_affinity_threshold: f64,

    /// Whether to create a utils module for shared functions.
    pub create_utils_module: bool,

    /// Threshold for considering a function as shared (called by N+ modules).
    pub shared_function_threshold: usize,
}

impl Default for AssignmentConfig {
    fn default() -> Self {
        Self {
            min_affinity_threshold: 0.3,
            create_utils_module: true,
            shared_function_threshold: 2,
        }
    }
}

/// Assigns functions to modules based on affinity scoring.
pub struct ModuleAssigner;

impl ModuleAssigner {
    /// Assign functions to modules.
    pub fn assign(
        functions: &mut [FunctionInfo],
        modules: &[Module],
        dep_graph: &DependencyGraph,
        config: &AssignmentConfig,
    ) -> HashMap<String, Vec<String>> {
        debug!("Assigning {} functions to modules", functions.len());

        // Build module keyword mappings
        let module_keywords = Self::build_module_keywords(modules);

        // Build module function mappings from metadata
        let mut module_functions: HashMap<String, Vec<String>> = HashMap::new();
        let mut function_to_module: HashMap<String, String> = HashMap::new();

        // Step 1: Assign seeded functions from module metadata
        for module in modules {
            for func_name in &module.functions {
                if let Some(func) = functions.iter().find(|f| &f.name == func_name) {
                    function_to_module.insert(func.name.clone(), module.name.clone());
                    module_functions
                        .entry(module.name.clone())
                        .or_default()
                        .push(func.name.clone());
                }
            }
        }

        // Step 2: Assign remaining functions by affinity
        for func in functions.iter_mut() {
            if func.assigned_module.is_none() && !function_to_module.contains_key(&func.name) {
                let best_module = Self::find_best_module(
                    func,
                    modules,
                    &dep_graph,
                    &module_keywords,
                    &function_to_module,
                );

                if let Some((module_name, score)) = best_module {
                    if score >= config.min_affinity_threshold {
                        func.assigned_module = Some(module_name.clone());
                        function_to_module.insert(func.name.clone(), module_name.clone());
                        module_functions
                            .entry(module_name)
                            .or_default()
                            .push(func.name.clone());
                    }
                }
            }
        }

        // Step 3: Handle shared functions
        if config.create_utils_module {
            let _shared = dep_graph.find_shared_functions(&function_to_module, config.shared_function_threshold);
            // TODO: Implement shared function handling
            // For now, shared functions will be assigned to orphans in Step 4
        }

        // Step 4: Assign orphans to best available module
        for func in functions.iter_mut() {
            if func.assigned_module.is_none() {
                let best = Self::find_best_module(
                    func,
                    modules,
                    &dep_graph,
                    &module_keywords,
                    &function_to_module,
                );

                if let Some((module_name, _)) = best {
                    func.assigned_module = Some(module_name.clone());
                    module_functions
                        .entry(module_name)
                        .or_default()
                        .push(func.name.clone());
                } else {
                    // Fallback to core or utils
                    let fallback = if module_functions.contains_key("core") {
                        "core".to_string()
                    } else if module_functions.contains_key("utils") {
                        "utils".to_string()
                    } else {
                        modules.first().map(|m| m.name.clone()).unwrap_or_else(|| {
                            "unknown".to_string()
                        })
                    };
                    func.assigned_module = Some(fallback.clone());
                    module_functions
                        .entry(fallback)
                        .or_default()
                        .push(func.name.clone());
                }
            }
        }

        debug!(
            "Assigned {} functions to {} modules",
            functions.len(),
            module_functions.len()
        );

        module_functions
    }

    /// Build keyword mappings for each module.
    fn build_module_keywords(modules: &[Module]) -> HashMap<ModuleCategory, Vec<String>> {
        modules
            .iter()
            .map(|m| (m.category.clone(), m.keywords.clone()))
            .collect()
    }

    /// Find the best module for a function using affinity scoring.
    fn find_best_module(
        func: &FunctionInfo,
        modules: &[Module],
        dep_graph: &DependencyGraph,
        module_keywords: &HashMap<ModuleCategory, Vec<String>>,
        function_to_module: &HashMap<String, String>,
    ) -> Option<(String, f64)> {
        let mut best_module: Option<(String, f64)> = None;

        for module in modules {
            let score = Self::calculate_affinity(
                func,
                module,
                dep_graph,
                module_keywords,
                function_to_module,
            );

            if best_module.is_none() || score > best_module.as_ref().unwrap().1 {
                best_module = Some((module.name.clone(), score));
            }
        }

        best_module
    }

    /// Calculate affinity score for a function to a module.
    fn calculate_affinity(
        func: &FunctionInfo,
        module: &Module,
        dep_graph: &DependencyGraph,
        module_keywords: &HashMap<ModuleCategory, Vec<String>>,
        function_to_module: &HashMap<String, String>,
    ) -> f64 {
        let mut score = 0.0;

        // Factor 1: Calls to functions in this module (40% weight)
        if !func.dependencies.is_empty() {
            let calls_in_module = func
                .dependencies
                .iter()
                .filter(|dep| {
                    function_to_module
                        .get(*dep)
                        .map(|m| m == &module.name)
                        .unwrap_or(false)
                })
                .count();

            let calls_ratio = calls_in_module as f64 / func.dependencies.len() as f64;
            score += calls_ratio * 0.4;
        }

        // Factor 2: String/keyword patterns (30% weight)
        let _dep_graph = dep_graph; // Mark as intentionally unused for now
        if let Some(keywords) = module_keywords.get(&module.category) {
            let keyword_match = keywords.iter().any(|keyword| {
                func.name.to_lowercase().contains(keyword.to_lowercase().as_str())
            });
            if keyword_match {
                score += 0.3;
            }
        }

        // Factor 3: Existing assignment hints (30% weight)
        if module.functions.contains(&func.name) {
            score += 0.3;
        }

        score
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transformer::function_extractor::SpanInfo;
    use std::collections::HashSet;

    #[test]
    fn test_assign_to_modules() {
        let mut functions = vec![
            FunctionInfo {
                name: "main_loop".to_string(),
                span: SpanInfo { start: 0, end: 100 },
                param_count: 0,
                is_anonymous: false,
                dependencies: vec!["process_message".to_string()],
                outer_variables: vec![],
                assigned_module: None,
                is_exported: false,
            },
            FunctionInfo {
                name: "process_message".to_string(),
                span: SpanInfo { start: 100, end: 200 },
                param_count: 0,
                is_anonymous: false,
                dependencies: vec![],
                outer_variables: vec![],
                assigned_module: None,
                is_exported: false,
            },
            FunctionInfo {
                name: "api_request".to_string(),
                span: SpanInfo { start: 200, end: 300 },
                param_count: 0,
                is_anonymous: false,
                dependencies: vec![],
                outer_variables: vec![],
                assigned_module: None,
                is_exported: false,
            },
        ];

        let modules = vec![
            Module {
                name: "core".to_string(),
                category: ModuleCategory::Core,
                estimated_lines: 100,
                functions: vec!["main_loop".to_string()],
                keywords: vec!["main".to_string(), "loop".to_string()],
            },
            Module {
                name: "apiclient".to_string(),
                category: ModuleCategory::ApiClient,
                estimated_lines: 100,
                functions: vec!["api_request".to_string()],
                keywords: vec!["api".to_string()],
            },
        ];

        let dep_graph = DependencyGraph {
            dependencies: HashMap::new(),
            reverse_dependencies: HashMap::new(),
            all_functions: HashSet::new(),
            circular_deps: vec![],
        };

        let config = AssignmentConfig::default();

        let assignment = ModuleAssigner::assign(&mut functions, &modules, &dep_graph, &config);

        assert!(assignment.contains_key("core"));
        assert!(assignment.contains_key("apiclient"));
    }

    #[test]
    fn test_affinity_scoring() {
        let modules = vec![
            Module {
                name: "core".to_string(),
                category: ModuleCategory::Core,
                estimated_lines: 100,
                functions: vec![],
                keywords: vec!["main".to_string()],
            },
            Module {
                name: "utils".to_string(),
                category: ModuleCategory::Utils,
                estimated_lines: 100,
                functions: vec![],
                keywords: vec!["util".to_string()],
            },
        ];

        let func = FunctionInfo {
            name: "main_loop".to_string(),
            span: SpanInfo { start: 0, end: 100 },
            param_count: 0,
            is_anonymous: false,
            dependencies: vec![],
            outer_variables: vec![],
            assigned_module: None,
            is_exported: false,
        };

        let dep_graph = DependencyGraph {
            dependencies: HashMap::new(),
            reverse_dependencies: HashMap::new(),
            all_functions: HashSet::new(),
            circular_deps: vec![],
        };

        let module_keywords = ModuleAssigner::build_module_keywords(&modules);
        let function_to_module = HashMap::new();

        let core_score = ModuleAssigner::calculate_affinity(
            &func,
            &modules[0],
            &dep_graph,
            &module_keywords,
            &function_to_module,
        );

        let utils_score = ModuleAssigner::calculate_affinity(
            &func,
            &modules[1],
            &dep_graph,
            &module_keywords,
            &function_to_module,
        );

        assert!(core_score > utils_score);
    }
}
