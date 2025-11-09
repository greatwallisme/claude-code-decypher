//! Claude Code Decypher
//!
//! A Rust tool to deobfuscate and analyze minified JavaScript code from Claude Code.
//!
//! This library provides functionality to:
//! - Parse JavaScript code using the Oxc parser
//! - Analyze and traverse the AST
//! - Extract system prompts, tool definitions, and configuration
//! - Transform and organize code into readable modules
//! - Generate documentation and analysis reports

pub mod analysis;
pub mod analyzer;
pub mod cli;
pub mod dashboard;
pub mod error;
pub mod extractor;
pub mod output;
pub mod parser;
pub mod transformer;
pub mod visualization;

pub use error::{DecypherError, Result};

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Library name
pub const NAME: &str = env!("CARGO_PKG_NAME");
