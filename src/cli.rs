use clap::{Parser, Subcommand};
use std::path::PathBuf;

use crate::RivetError;

#[derive(Parser, Debug)]
#[command(name = "rivet", version, about = "Patch intent verifier — checks diffs for quality and intent alignment")]
pub struct Cli {
    /// Project root override
    #[arg(long, global = true)]
    pub repo: Option<PathBuf>,

    /// Output format
    #[arg(long, global = true, default_value = "text")]
    pub format: OutputFormat,

    #[command(subcommand)]
    pub command: Command,
}

impl Cli {
    pub fn resolve_repo(&self) -> Result<PathBuf, RivetError> {
        if let Some(ref repo) = self.repo {
            return Ok(repo.clone());
        }
        if let Ok(output) = std::process::Command::new("git")
            .args(["rev-parse", "--show-toplevel"])
            .output()
        {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                return Ok(PathBuf::from(path));
            }
        }
        std::env::current_dir().map_err(RivetError::Io)
    }

    pub fn is_json(&self) -> bool {
        matches!(self.format, OutputFormat::Json)
    }
}

#[derive(Debug, Clone, clap::ValueEnum)]
pub enum OutputFormat {
    Json,
    Text,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Check current diff for quality issues
    Check {
        /// Stated intent/task description (used to detect unrelated edits)
        #[arg(long)]
        intent: Option<String>,
        /// Base revision (default: HEAD)
        #[arg(long)]
        base: Option<String>,
    },
    /// Show parsed diff summary (debugging)
    Diff {
        /// Base revision
        #[arg(long)]
        base: Option<String>,
    },
}
