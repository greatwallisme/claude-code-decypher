//! Code transformation module for beautifying and organizing code.

pub mod advanced_split;
pub mod codegen;
pub mod docs;
pub mod rename;
pub mod sourcemap;
pub mod split;

use crate::analyzer::Analyzer;
use crate::Result;
use oxc_allocator::Allocator;
use oxc_ast::ast::Program;
use std::collections::HashMap;

/// Main transformer that coordinates all transformation operations.
pub struct Transformer<'a> {
    program: &'a Program<'a>,
    analyzer: Analyzer<'a>,
}

impl<'a> Transformer<'a> {
    /// Create a new transformer.
    pub fn new(program: &'a Program<'a>) -> Self {
        let analyzer = Analyzer::new(program);
        Self { program, analyzer }
    }

    /// Generate a rename map for minified variables.
    pub fn generate_rename_map(&self) -> Result<HashMap<String, String>> {
        rename::VariableRenamer::new(&self.analyzer).generate_rename_map()
    }

    /// Split code into logical modules.
    pub fn split_into_modules(&self, strategy: split::SplitStrategy) -> Result<Vec<split::Module>> {
        split::ModuleSplitter::new(&self.analyzer, strategy).split()
    }

    /// Generate beautified code.
    pub fn beautify(&self, allocator: &'a Allocator) -> Result<String> {
        codegen::CodeGenerator::new(allocator, self.program).generate()
    }

    /// Get the program reference.
    pub fn program(&self) -> &'a Program<'a> {
        self.program
    }
}
