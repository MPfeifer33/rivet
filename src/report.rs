use crate::checks::Finding;
use crate::diff::DiffData;
use crate::RivetError;
use serde::Serialize;

#[derive(Debug, Serialize)]
struct ActionItem {
    severity: String,
    check: String,
    file: Option<String>,
    action: String,
}

pub fn print_report(
    diff: &DiffData,
    findings: &[Finding],
    is_json: bool,
) -> Result<(), RivetError> {
    let verdict = compute_verdict(findings);
    let action_items = build_action_items(findings);

    if is_json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "ok": true,
                "verdict": verdict,
                "summary": {
                    "files_changed": diff.files.len(),
                    "additions": diff.total_additions,
                    "deletions": diff.total_deletions,
                    "errors": findings.iter().filter(|f| f.severity == "error").count(),
                    "warnings": findings.iter().filter(|f| f.severity == "warning").count(),
                    "info": findings.iter().filter(|f| f.severity == "info").count(),
                },
                "action_items": action_items,
                "findings": findings,
            }))?
        );
    } else {
        print_text(diff, findings, &action_items, &verdict);
    }

    Ok(())
}

fn compute_verdict(findings: &[Finding]) -> String {
    let errors = findings.iter().filter(|f| f.severity == "error").count();
    let warnings = findings.iter().filter(|f| f.severity == "warning").count();

    if errors > 0 {
        "blocked".into()
    } else if warnings > 0 {
        "caution".into()
    } else {
        "clean".into()
    }
}

fn build_action_items(findings: &[Finding]) -> Vec<ActionItem> {
    findings
        .iter()
        .map(|finding| ActionItem {
            severity: finding.severity.clone(),
            check: finding.check.clone(),
            file: finding.file.clone(),
            action: action_for(finding),
        })
        .collect()
}

fn action_for(finding: &Finding) -> String {
    match finding.check.as_str() {
        "secret_detected" => {
            "Remove the secret-like value, rotate it if real, and use environment/config indirection.".into()
        }
        "generated_churn" => {
            "Verify generated or lockfile churn is intentional and reproducible.".into()
        }
        "large_diff" | "large_file_change" => {
            "Consider splitting the patch or documenting why the large change is cohesive.".into()
        }
        "missing_tests" => {
            "Add targeted tests, run an existing relevant test, or document why test changes are unnecessary.".into()
        }
        "risky_file" => {
            "Double-check operational impact and mention the risky file in the handoff or commit notes.".into()
        }
        "formatting_churn" => {
            "Separate formatting-only churn from behavioral changes when practical.".into()
        }
        "todo_added" => {
            "Resolve the marker or link it to a tracked follow-up before merging.".into()
        }
        "possibly_unrelated" => {
            "Confirm this file belongs to the stated intent or split it into another change.".into()
        }
        _ => "Review this finding before committing.".into(),
    }
}

fn print_text(diff: &DiffData, findings: &[Finding], action_items: &[ActionItem], verdict: &str) {
    let icon = match verdict {
        "blocked" => "✗",
        "caution" => "⚠",
        _ => "✓",
    };

    println!("rivet check: {icon} {verdict}");
    println!(
        "  {} file(s), +{} -{}",
        diff.files.len(),
        diff.total_additions,
        diff.total_deletions
    );
    println!();

    if findings.is_empty() {
        println!("  No issues found. Ready to commit.");
        return;
    }

    let errors: Vec<_> = findings.iter().filter(|f| f.severity == "error").collect();
    let warnings: Vec<_> = findings
        .iter()
        .filter(|f| f.severity == "warning")
        .collect();
    let infos: Vec<_> = findings.iter().filter(|f| f.severity == "info").collect();

    if !errors.is_empty() {
        println!("  Errors ({}):", errors.len());
        for f in &errors {
            print_finding(f);
        }
        println!();
    }

    if !warnings.is_empty() {
        println!("  Warnings ({}):", warnings.len());
        for f in &warnings {
            print_finding(f);
        }
        println!();
    }

    if !infos.is_empty() {
        println!("  Info ({}):", infos.len());
        for f in &infos {
            print_finding(f);
        }
        println!();
    }

    if !action_items.is_empty() {
        println!("  Action items:");
        for item in action_items {
            let file = item.file.as_deref().unwrap_or("");
            if file.is_empty() {
                println!("    [{}] {}: {}", item.check, item.severity, item.action);
            } else {
                println!(
                    "    [{}] {} {}: {}",
                    item.check, item.severity, file, item.action
                );
            }
        }
    }
}

fn print_finding(f: &Finding) {
    let file_str = f.file.as_deref().unwrap_or("");
    println!("    [{}] {} {}", f.check, file_str, f.message);
    if let Some(ref line) = f.line {
        println!("      → {line}");
    }
}
