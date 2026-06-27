use std::path::Path;
use std::sync::LazyLock;
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::diff::DiffData;
use crate::RivetError;

// --- Sentinel integration types ---

#[derive(Debug, Deserialize)]
struct SentinelMatrix {
    files: Vec<SentinelFileRisk>,
}

#[derive(Debug, Deserialize)]
struct SentinelFileRisk {
    path: String,
    risk_score: u32,
    level: String,
    bugfix_commits: usize,
    reasons: Vec<String>,
}

// --- Keystone integration types ---

#[derive(Debug, Deserialize)]
struct KeystoneContract {
    #[serde(default)]
    protected: Vec<KeystoneProtected>,
}

#[derive(Debug, Deserialize)]
struct KeystoneProtected {
    pattern: String,
    reason: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Finding {
    pub check: String,
    pub severity: String,  // "error", "warning", "info"
    pub file: Option<String>,
    pub line: Option<String>,
    pub message: String,
}

pub fn run_all_checks(
    repo: &Path,
    diff: &DiffData,
    intent: Option<&str>,
) -> Result<Vec<Finding>, RivetError> {
    let mut findings = Vec::new();

    findings.extend(check_secrets(diff));
    findings.extend(check_generated_churn(diff));
    findings.extend(check_large_diff(diff));
    findings.extend(check_missing_tests(repo, diff));
    findings.extend(check_risky_files(diff));
    findings.extend(check_sentinel_risk(repo, diff));
    findings.extend(check_keystone_protected(repo, diff));
    findings.extend(check_formatting_only(diff));
    findings.extend(check_todo_fixme(diff));

    if let Some(intent) = intent {
        findings.extend(check_unrelated_edits(diff, intent));
    }

    Ok(findings)
}

static SECRET_PATTERNS: LazyLock<Vec<(Regex, &'static str)>> = LazyLock::new(|| vec![
    (Regex::new(r#"(?i)(api[_-]?key|apikey)\s*[:=]\s*['"][^'"]{8,}"#).unwrap(), "API key"),
    (Regex::new(r#"(?i)(secret|password|passwd|pwd)\s*[:=]\s*['"][^'"]{8,}"#).unwrap(), "Secret/password"),
    (Regex::new(r"(?i)bearer\s+[a-zA-Z0-9\-._~+/]{20,}").unwrap(), "Bearer token"),
    (Regex::new(r"ghp_[a-zA-Z0-9]{36}").unwrap(), "GitHub personal access token"),
    (Regex::new(r"sk-[a-zA-Z0-9]{20,}").unwrap(), "API secret key"),
    (Regex::new(r"-----BEGIN (RSA |EC |DSA )?PRIVATE KEY-----").unwrap(), "Private key"),
    (Regex::new(r#"(?i)aws[_-]?secret[_-]?access[_-]?key\s*[:=]\s*['"]?[A-Za-z0-9/+=]{40}"#).unwrap(), "AWS secret"),
]);

fn check_secrets(diff: &DiffData) -> Vec<Finding> {
    let mut findings = Vec::new();

    for file in &diff.files {
        for line in &file.added_lines {
            for (re, name) in SECRET_PATTERNS.iter() {
                if re.is_match(line) {
                    findings.push(Finding {
                        check: "secret_detected".into(),
                        severity: "error".into(),
                        file: Some(file.path.clone()),
                        line: Some(truncate(line, 80)),
                        message: format!("Possible {name} in added code"),
                    });
                }
            }
        }
    }

    findings
}

fn check_generated_churn(diff: &DiffData) -> Vec<Finding> {
    let generated_patterns = [
        "package-lock.json",
        "pnpm-lock.yaml",
        "yarn.lock",
        "Cargo.lock",
        "go.sum",
        "poetry.lock",
        ".min.js",
        ".min.css",
        "dist/",
        "build/",
        "node_modules/",
    ];

    let mut findings = Vec::new();

    for file in &diff.files {
        for pattern in &generated_patterns {
            if file.path.contains(pattern) && (file.additions + file.deletions) > 100 {
                findings.push(Finding {
                    check: "generated_churn".into(),
                    severity: "info".into(),
                    file: Some(file.path.clone()),
                    line: None,
                    message: format!("Large change in likely-generated file ({} lines changed)", file.additions + file.deletions),
                });
            }
        }
    }

    findings
}

fn check_large_diff(diff: &DiffData) -> Vec<Finding> {
    let mut findings = Vec::new();

    if diff.total_additions + diff.total_deletions > 500 {
        findings.push(Finding {
            check: "large_diff".into(),
            severity: "warning".into(),
            file: None,
            line: None,
            message: format!("Large diff: {} additions, {} deletions across {} files. Consider splitting.", diff.total_additions, diff.total_deletions, diff.files.len()),
        });
    }

    // Individual large files
    for file in &diff.files {
        if file.additions + file.deletions > 200 {
            findings.push(Finding {
                check: "large_file_change".into(),
                severity: "info".into(),
                file: Some(file.path.clone()),
                line: None,
                message: format!("{} lines changed in single file", file.additions + file.deletions),
            });
        }
    }

    findings
}

fn check_missing_tests(repo: &Path, diff: &DiffData) -> Vec<Finding> {
    let mut findings = Vec::new();

    let source_files: Vec<&str> = diff.files.iter()
        .map(|f| f.path.as_str())
        .filter(|p| is_source_file(p))
        .collect();

    let test_files: Vec<&str> = diff.files.iter()
        .map(|f| f.path.as_str())
        .filter(|p| is_test_file(p))
        .collect();

    if !source_files.is_empty() && test_files.is_empty() {
        // Check if any test files exist in the project at all
        let has_tests = repo.join("tests").exists()
            || repo.join("test").exists()
            || repo.join("__tests__").exists();

        if has_tests {
            findings.push(Finding {
                check: "missing_tests".into(),
                severity: "warning".into(),
                file: None,
                line: None,
                message: format!("{} source file(s) changed but no test files in diff", source_files.len()),
            });
        }
    }

    findings
}

fn check_risky_files(diff: &DiffData) -> Vec<Finding> {
    let risky_patterns = [
        (".env", "Environment file"),
        ("Dockerfile", "Container definition"),
        ("docker-compose", "Container orchestration"),
        (".github/workflows", "CI/CD pipeline"),
        ("Makefile", "Build system"),
        ("nginx.conf", "Web server config"),
        ("systemd", "System service"),
        (".service", "Systemd unit"),
    ];

    let mut findings = Vec::new();

    for file in &diff.files {
        for (pattern, description) in &risky_patterns {
            if file.path.contains(pattern) {
                findings.push(Finding {
                    check: "risky_file".into(),
                    severity: "info".into(),
                    file: Some(file.path.clone()),
                    line: None,
                    message: format!("Editing {description} — verify this is intentional"),
                });
            }
        }
    }

    findings
}

fn check_formatting_only(diff: &DiffData) -> Vec<Finding> {
    let mut findings = Vec::new();

    for file in &diff.files {
        if file.additions > 10 && file.deletions > 10 {
            // Check if changes are mostly whitespace
            let ws_changes = file.added_lines.iter()
                .filter(|l| l.trim().is_empty() || l.chars().all(|c| c.is_whitespace()))
                .count();

            if ws_changes as f64 / file.added_lines.len().max(1) as f64 > 0.7 {
                findings.push(Finding {
                    check: "formatting_churn".into(),
                    severity: "warning".into(),
                    file: Some(file.path.clone()),
                    line: None,
                    message: "Most changes appear to be formatting/whitespace. Consider separating formatting commits.".into(),
                });
            }
        }
    }

    findings
}

static TODO_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)(?://|#|/\*|\*)\s*(TODO|FIXME|HACK|XXX|TEMP)\b").unwrap());

fn check_todo_fixme(diff: &DiffData) -> Vec<Finding> {
    let re = &*TODO_RE;
    let mut findings = Vec::new();

    for file in &diff.files {
        for line in &file.added_lines {
            if re.is_match(line) {
                findings.push(Finding {
                    check: "todo_added".into(),
                    severity: "info".into(),
                    file: Some(file.path.clone()),
                    line: Some(truncate(line, 80)),
                    message: "New TODO/FIXME added — track or resolve before merge".into(),
                });
            }
        }
    }

    findings
}

fn check_unrelated_edits(diff: &DiffData, intent: &str) -> Vec<Finding> {
    let mut findings = Vec::new();

    // Simple heuristic: if the intent mentions specific files/modules,
    // flag files that seem unrelated
    let intent_lower = intent.to_lowercase();
    let intent_words: Vec<&str> = intent_lower.split_whitespace()
        .filter(|w| w.len() > 3)
        .collect();

    if intent_words.is_empty() {
        return findings;
    }

    for file in &diff.files {
        let file_lower = file.path.to_lowercase();
        let file_stem = Path::new(&file.path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();

        // Check if any intent word appears in the file path
        let related = intent_words.iter().any(|word| {
            file_lower.contains(word) || file_stem.contains(word)
        });

        if !related && !is_test_file(&file.path) && !is_config_file(&file.path) {
            findings.push(Finding {
                check: "possibly_unrelated".into(),
                severity: "warning".into(),
                file: Some(file.path.clone()),
                line: None,
                message: format!("File may be unrelated to stated intent: \"{}\"", truncate(intent, 60)),
            });
        }
    }

    findings
}

fn is_source_file(path: &str) -> bool {
    let ext = Path::new(path).extension().and_then(|e| e.to_str()).unwrap_or("");
    matches!(ext, "rs" | "js" | "ts" | "jsx" | "tsx" | "py" | "go" | "java" | "rb" | "c" | "cpp" | "h")
        && !is_test_file(path)
}

fn is_test_file(path: &str) -> bool {
    path.contains("test") || path.contains("spec") || path.ends_with("_test.go")
}

fn is_config_file(path: &str) -> bool {
    let name = Path::new(path).file_name().and_then(|n| n.to_str()).unwrap_or("");
    matches!(name,
        "Cargo.toml" | "package.json" | "tsconfig.json" | "pyproject.toml"
        | ".gitignore" | "PROJECT.md" | "README.md"
    )
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        let end = s.char_indices()
            .take_while(|(i, _)| *i < max)
            .last()
            .map(|(i, c)| i + c.len_utf8())
            .unwrap_or(0);
        format!("{}...", &s[..end])
    }
}

// --- Sentinel risk check ---

fn load_sentinel_matrix(repo: &Path) -> Option<SentinelMatrix> {
    let path = repo.join(".agent-sentinel").join("matrix.json");
    let content = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

fn check_sentinel_risk(repo: &Path, diff: &DiffData) -> Vec<Finding> {
    let mut findings = Vec::new();

    let matrix = match load_sentinel_matrix(repo) {
        Some(m) => m,
        None => return findings,
    };

    for file in &diff.files {
        if let Some(risk) = matrix.files.iter().find(|f| f.path == file.path) {
            if risk.level == "high" {
                findings.push(Finding {
                    check: "sentinel_high_risk".into(),
                    severity: "error".into(),
                    file: Some(file.path.clone()),
                    line: None,
                    message: format!(
                        "Sentinel risk score {} ({} bugfix commits) — {}",
                        risk.risk_score, risk.bugfix_commits,
                        risk.reasons.first().map(|s| s.as_str()).unwrap_or("historically fragile")
                    ),
                });
            } else if risk.level == "medium" {
                findings.push(Finding {
                    check: "sentinel_medium_risk".into(),
                    severity: "warning".into(),
                    file: Some(file.path.clone()),
                    line: None,
                    message: format!(
                        "Sentinel risk score {} — verify test coverage",
                        risk.risk_score
                    ),
                });
            }
        }
    }

    findings
}

// --- Keystone protected-zone check ---

fn load_keystone_contract(repo: &Path) -> Option<KeystoneContract> {
    let path = repo.join(".agent-contract.toml");
    let content = std::fs::read_to_string(path).ok()?;
    toml::from_str(&content).ok()
}

fn matches_keystone_pattern(path: &str, pattern: &str) -> bool {
    if pattern.ends_with("/**") {
        let prefix = &pattern[..pattern.len() - 3];
        path == prefix || path.starts_with(&format!("{prefix}/"))
    } else if pattern.contains('*') {
        static GLOB_DOT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\.").unwrap());
        static GLOB_STAR: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\*").unwrap());
        let escaped = GLOB_DOT.replace_all(pattern, r"\.");
        let regex_pattern = GLOB_STAR.replace_all(&escaped, ".*");
        Regex::new(&format!("^{regex_pattern}$"))
            .map(|re| re.is_match(path))
            .unwrap_or(false)
    } else {
        path == pattern
    }
}

fn check_keystone_protected(repo: &Path, diff: &DiffData) -> Vec<Finding> {
    let mut findings = Vec::new();

    let contract = match load_keystone_contract(repo) {
        Some(c) => c,
        None => return findings,
    };

    for file in &diff.files {
        for protected in &contract.protected {
            if matches_keystone_pattern(&file.path, &protected.pattern) {
                findings.push(Finding {
                    check: "keystone_protected".into(),
                    severity: "warning".into(),
                    file: Some(file.path.clone()),
                    line: None,
                    message: format!(
                        "Protected zone `{}` — {}",
                        protected.pattern, protected.reason
                    ),
                });
            }
        }
    }

    findings
}
