mod cli;
mod diff;
mod checks;
mod report;

use clap::Parser;
use cli::{Cli, Command};

fn main() {
    let cli = Cli::parse();
    let result = run(&cli);
    match result {
        Ok(()) => {}
        Err(e) => {
            let code = e.exit_code();
            if cli.is_json() {
                let err_json = serde_json::json!({
                    "ok": false,
                    "error": {
                        "code": e.error_code(),
                        "message": e.to_string(),
                    }
                });
                eprintln!("{}", serde_json::to_string_pretty(&err_json).unwrap_or_else(|_| format!("{{\"ok\":false,\"error\":{{\"message\":\"{e}\"}}}}")));
            } else {
                eprintln!("error: {e}");
            }
            std::process::exit(code);
        }
    }
}

fn run(cli: &Cli) -> Result<(), RivetError> {
    match &cli.command {
        Command::Check { intent, base } => {
            let repo = cli.resolve_repo()?;
            let diff_data = diff::parse_diff(&repo, base.as_deref())?;

            if diff_data.files.is_empty() {
                if cli.is_json() {
                    println!("{}", serde_json::to_string_pretty(&serde_json::json!({
                        "ok": true,
                        "message": "No changes to check",
                        "verdict": "clean",
                        "findings": [],
                    }))?);
                } else {
                    println!("No changes to check.");
                }
                return Ok(());
            }

            let findings = checks::run_all_checks(&repo, &diff_data, intent.as_deref())?;
            report::print_report(&diff_data, &findings, cli.is_json())?;
            Ok(())
        }
        Command::Diff { base } => {
            let repo = cli.resolve_repo()?;
            let diff_data = diff::parse_diff(&repo, base.as_deref())?;
            if cli.is_json() {
                println!("{}", serde_json::to_string_pretty(&serde_json::json!({
                    "ok": true,
                    "diff": diff_data,
                }))?);
            } else {
                for file in &diff_data.files {
                    println!("{:>4}+ {:>4}- {}", file.additions, file.deletions, file.path);
                }
                println!("---");
                println!("Total: {} files, +{} -{}", diff_data.files.len(), diff_data.total_additions, diff_data.total_deletions);
            }
            Ok(())
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RivetError {
    #[error("{0}")]
    Validation(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

impl RivetError {
    pub fn exit_code(&self) -> i32 {
        match self {
            RivetError::Validation(_) => 1,
            RivetError::Io(_) => 2,
            RivetError::Json(_) => 1,
        }
    }

    pub fn error_code(&self) -> &'static str {
        match self {
            RivetError::Validation(_) => "validation_error",
            RivetError::Io(_) => "io_error",
            RivetError::Json(_) => "json_error",
        }
    }
}
