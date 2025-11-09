//! Error types for the claude-code-decypher tool.

use std::path::PathBuf;
use thiserror::Error;

/// Main error type for the decypher tool.
#[derive(Error, Debug)]
pub enum DecypherError {
    /// I/O error when reading or writing files.
    #[error("Failed to read file '{path}': {source}")]
    IoError {
        path: PathBuf,
        source: std::io::Error,
    },

    /// Error during JavaScript parsing.
    #[error("Failed to parse JavaScript: {0}")]
    ParseError(String),

    /// Error during AST analysis.
    #[error("Failed to analyze AST: {0}")]
    AnalysisError(String),

    /// Error during code extraction.
    #[error("Failed to extract {item}: {message}")]
    ExtractionError { item: String, message: String },

    /// Error during code transformation.
    #[error("Failed to transform code: {0}")]
    TransformError(String),

    /// Error during code generation.
    #[error("Failed to generate output: {0}")]
    CodegenError(String),

    /// Invalid CLI arguments.
    #[error("Invalid arguments: {0}")]
    InvalidArguments(String),

    /// Other errors.
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// Result type alias for the decypher tool.
pub type Result<T> = std::result::Result<T, DecypherError>;

impl DecypherError {
    /// Create a new I/O error with context.
    pub fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::IoError {
            path: path.into(),
            source,
        }
    }

    /// Create a new parse error.
    pub fn parse(message: impl Into<String>) -> Self {
        Self::ParseError(message.into())
    }

    /// Create a new extraction error.
    pub fn extraction(item: impl Into<String>, message: impl Into<String>) -> Self {
        Self::ExtractionError {
            item: item.into(),
            message: message.into(),
        }
    }
}
