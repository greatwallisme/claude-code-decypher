//! Command-line interface for the claude-code-decypher tool.

use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// Deobfuscate and analyze Claude Code JavaScript bundle
#[derive(Parser, Debug)]
#[command(name = "claude-code-decypher")]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Input JavaScript file to analyze
    #[arg(value_name = "INPUT")]
    pub input: PathBuf,

    /// Output directory for results
    #[arg(short, long, default_value = "./output")]
    pub output: PathBuf,

    /// Verbosity level (-v, -vv, -vvv)
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Don't print any output except errors
    #[arg(short, long)]
    pub quiet: bool,

    /// Subcommand to run
    #[command(subcommand)]
    pub command: Option<Commands>,
}

/// Available subcommands
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Run all phases: parse, extract, transform, analyze, visualize (recommended)
    All {
        /// Generate diagrams (Mermaid and DOT)
        #[arg(long)]
        diagrams: bool,

        /// Enable variable renaming
        #[arg(long)]
        rename: bool,

        /// Split into modules
        #[arg(long)]
        split: bool,

        /// Show detailed progress
        #[arg(long)]
        detailed: bool,
    },

    /// Parse JavaScript and display AST statistics
    Parse {
        /// Show detailed AST information
        #[arg(short, long)]
        detailed: bool,

        /// Output format (text, json)
        #[arg(short, long, default_value = "text")]
        format: OutputFormat,
    },

    /// Generate comprehensive dashboard (all phases)
    Dashboard {
        /// Generate diagrams (Mermaid and DOT)
        #[arg(long)]
        diagrams: bool,

        /// Output format
        #[arg(short, long, default_value = "text")]
        format: OutputFormat,
    },

    /// Extract system prompts and tool definitions (Phase 2)
    Extract {
        /// Extract only prompts
        #[arg(long)]
        prompts_only: bool,

        /// Extract only tools
        #[arg(long)]
        tools_only: bool,

        /// Output format (json)
        #[arg(short, long, default_value = "json")]
        format: OutputFormat,
    },

    /// Transform and organize code (Phase 3)
    Transform {
        /// Enable variable renaming
        #[arg(long)]
        rename: bool,

        /// Split into modules
        #[arg(long)]
        split: bool,

        /// Module split strategy
        #[arg(long, default_value = "hybrid", value_enum)]
        strategy: SplitStrategy,
    },

    /// Analyze code structure (Phase 2+)
    Analyze {
        /// Generate call graph
        #[arg(long)]
        call_graph: bool,

        /// Calculate complexity metrics
        #[arg(long)]
        complexity: bool,

        /// Output format
        #[arg(short, long, default_value = "text")]
        format: OutputFormat,
    },
}

/// Output format options
#[derive(Debug, Clone, clap::ValueEnum)]
pub enum OutputFormat {
    /// Human-readable text
    Text,
    /// JSON format
    Json,
    /// Debug format
    Debug,
}

/// Module splitting strategy
#[derive(Debug, Clone, clap::ValueEnum)]
pub enum SplitStrategy {
    /// Split by export statements
    ByExport,
    /// Split by namespace/prefix
    ByNamespace,
    /// Split by feature/functionality
    ByFeature,
    /// Hybrid approach (default)
    Hybrid,
}

impl Cli {
    /// Parse command-line arguments
    pub fn parse_args() -> Self {
        <Self as Parser>::parse()
    }

    /// Get the log level based on verbosity
    pub fn log_level(&self) -> &str {
        if self.quiet {
            "error"
        } else {
            match self.verbose {
                0 => "info",
                1 => "debug",
                _ => "trace",
            }
        }
    }

    /// Validate the CLI arguments
    pub fn validate(&self) -> crate::error::Result<()> {
        use crate::error::DecypherError;

        // Check if input file exists
        if !self.input.exists() {
            return Err(DecypherError::InvalidArguments(format!(
                "Input file does not exist: {}",
                self.input.display()
            )));
        }

        // Check if input is a file
        if !self.input.is_file() {
            return Err(DecypherError::InvalidArguments(format!(
                "Input must be a file, not a directory: {}",
                self.input.display()
            )));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_level() {
        let cli = Cli {
            input: PathBuf::from("test.js"),
            output: PathBuf::from("output"),
            verbose: 0,
            quiet: false,
            command: None,
        };

        assert_eq!(cli.log_level(), "info");
    }

    #[test]
    fn test_log_level_verbose() {
        let cli = Cli {
            input: PathBuf::from("test.js"),
            output: PathBuf::from("output"),
            verbose: 1,
            quiet: false,
            command: None,
        };

        assert_eq!(cli.log_level(), "debug");
    }

    #[test]
    fn test_log_level_very_verbose() {
        let cli = Cli {
            input: PathBuf::from("test.js"),
            output: PathBuf::from("output"),
            verbose: 2,
            quiet: false,
            command: None,
        };

        assert_eq!(cli.log_level(), "trace");
    }

    #[test]
    fn test_log_level_quiet() {
        let cli = Cli {
            input: PathBuf::from("test.js"),
            output: PathBuf::from("output"),
            verbose: 0,
            quiet: true,
            command: None,
        };

        assert_eq!(cli.log_level(), "error");
    }
}
