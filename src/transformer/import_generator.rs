//! Generate ES6 import/export statements.

use crate::transformer::dependency_analyzer::DependencyGraph;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use tracing::debug;

/// Import statement structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportStatement {
    /// Module path (e.g., './apiclient.js').
    pub module: String,

    /// Imported functions/variables.
    pub imports: Vec<String>,

    /// Is this a dynamic import?
    pub is_dynamic: bool,
}

/// Export information for a module.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportInfo {
    /// Exported function names.
    pub exports: Vec<String>,

    /// Re-exported modules.
    pub re_exports: Vec<String>,
}

/// Generates import and export statements.
pub struct ImportGenerator;

impl ImportGenerator {
    /// Generate imports for all modules.
    pub fn generate_imports(
        module_functions: &HashMap<String, Vec<String>>,
        dep_graph: &DependencyGraph,
    ) -> HashMap<String, Vec<ImportStatement>> {
        debug!("Generating imports for {} modules", module_functions.len());

        let mut imports: HashMap<String, Vec<ImportStatement>> = HashMap::new();

        for (module_name, functions) in module_functions {
            let mut module_imports: Vec<ImportStatement> = Vec::new();
            let mut imports_by_module: HashMap<String, Vec<String>> = HashMap::new();

            // Collect all imports needed by functions in this module
            for func in functions {
                if let Some(func_deps) = dep_graph.dependencies.get(func) {
                    for dep in func_deps {
                        // Find which module this dependency is in
                        if let Some(dep_module) = Self::find_function_module(
                            dep,
                            module_functions,
                        ) {
                            if &dep_module != module_name {
                                imports_by_module
                                    .entry(dep_module)
                                    .or_default()
                                    .push(dep.clone());
                            }
                        }
                    }
                }
            }

            // Convert to ImportStatement structures
            for (dep_module, imported_functions) in &imports_by_module {
                let module_path = format!("./{}.js", dep_module);
                let mut import_set: HashSet<String> = imported_functions.iter().cloned().collect();
                let import_list: Vec<String> = import_set.into_iter().collect();

                module_imports.push(ImportStatement {
                    module: module_path,
                    imports: import_list,
                    is_dynamic: false,
                });
            }

            imports.insert(module_name.clone(), module_imports);
        }

        debug!("Generated imports for {} modules", imports.len());

        imports
    }

    /// Generate exports for all modules.
    pub fn generate_exports(
        module_functions: &HashMap<String, Vec<String>>,
        _all_modules: &[String],
    ) -> HashMap<String, ExportInfo> {
        debug!("Generating exports for {} modules", module_functions.len());

        let mut exports: HashMap<String, ExportInfo> = HashMap::new();

        for (module_name, functions) in module_functions {
            // Export all functions in the module
            let export_info = ExportInfo {
                exports: functions.clone(),
                re_exports: vec![],
            };

            exports.insert(module_name.clone(), export_info);
        }

        exports
    }

    /// Find which module a function belongs to.
    fn find_function_module(
        func_name: &str,
        module_functions: &HashMap<String, Vec<String>>,
    ) -> Option<String> {
        for (module_name, functions) in module_functions {
            if functions.contains(&func_name.to_string()) {
                return Some(module_name.clone());
            }
        }
        None
    }

    /// Format import statements as code.
    pub fn format_imports(import_statements: &[ImportStatement]) -> String {
        let mut code = String::new();

        for stmt in import_statements {
            if stmt.is_dynamic {
                // Dynamic import
                for import in &stmt.imports {
                    code.push_str(&format!("const {} = await import('{}');\n", import, stmt.module));
                }
            } else {
                // Static ES6 import
                if stmt.imports.len() == 1 {
                    code.push_str(&format!(
                        "import {{ {} }} from '{}';\n",
                        stmt.imports[0], stmt.module
                    ));
                } else {
                    code.push_str(&format!(
                        "import {{\n  {},\n}} from '{}';\n",
                        stmt.imports.join(",\n  "),
                        stmt.module
                    ));
                }
            }
        }

        code
    }

    /// Format export statements as code.
    pub fn format_exports(export_info: &ExportInfo) -> String {
        let mut code = String::new();

        if !export_info.exports.is_empty() {
            if export_info.exports.len() == 1 {
                code.push_str(&format!("export {{ {} }};\n", export_info.exports[0]));
            } else {
                code.push_str(&format!(
                    "export {{\n  {},\n}};\n",
                    export_info.exports.join(",\n  ")
                ));
            }
        }

        for re_export in &export_info.re_exports {
            code.push_str(&format!("export * from '{}';\n", re_export));
        }

        code
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transformer::function_extractor::SpanInfo;

    #[test]
    fn test_generate_imports() {
        let mut module_functions: HashMap<String, Vec<String>> = HashMap::new();
        module_functions.insert("core".to_string(), vec!["foo".to_string()]);
        module_functions.insert("utils".to_string(), vec!["bar".to_string()]);

        let mut dependencies = HashMap::new();
        dependencies.insert("foo".to_string(), vec!["bar".to_string()]);
        dependencies.insert("bar".to_string(), vec![]);

        let dep_graph = DependencyGraph {
            dependencies,
            reverse_dependencies: HashMap::new(),
            all_functions: HashSet::new(),
            circular_deps: vec![],
        };

        let imports = ImportGenerator::generate_imports(&module_functions, &dep_graph);

        assert!(imports.contains_key("core"));
    }

    #[test]
    fn test_generate_exports() {
        let mut module_functions: HashMap<String, Vec<String>> = HashMap::new();
        module_functions.insert("core".to_string(), vec!["foo".to_string(), "bar".to_string()]);

        let exports = ImportGenerator::generate_exports(&module_functions, &vec!["core".to_string()]);

        assert!(exports.contains_key("core"));
        assert_eq!(exports.get("core").unwrap().exports.len(), 2);
    }

    #[test]
    fn test_format_imports() {
        let imports = vec![ImportStatement {
            module: "./utils.js".to_string(),
            imports: vec!["helper".to_string(), "format".to_string()],
            is_dynamic: false,
        }];

        let code = ImportGenerator::format_imports(&imports);

        assert!(code.contains("import"));
        assert!(code.contains("helper"));
        assert!(code.contains("format"));
        assert!(code.contains("./utils.js"));
    }

    #[test]
    fn test_format_exports() {
        let export_info = ExportInfo {
            exports: vec!["foo".to_string(), "bar".to_string()],
            re_exports: vec![],
        };

        let code = ImportGenerator::format_exports(&export_info);

        assert!(code.contains("export"));
        assert!(code.contains("foo"));
        assert!(code.contains("bar"));
    }
}
