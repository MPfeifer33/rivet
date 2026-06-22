use crate::checks::Finding;
use crate::diff::DiffData;
use crate::RivetError;

pub fn print_report(diff: &DiffData, findings: &[Finding], is_json: bool) -> Result<(), RivetError> {
    let verdict = compute_verdict(findings);

    if is_json {
        println!("{}", serde_json::to_string_pretty(&serde_json::json!({
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
            "findings": findings,
        }))?);
    } else {
        print_text(diff, findings, &verdict);
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

fn print_text(diff: &DiffData, findings: &[Finding], verdict: &str) {
    let icon = match verdict {
        "blocked" => "✗",
        "caution" => "⚠",
        _ => "✓",
    };

    println!("rivet check: {icon} {verdict}");
    println!("  {} file(s), +{} -{}", diff.files.len(), diff.total_additions, diff.total_deletions);
    println!();

    if findings.is_empty() {
        println!("  No issues found. Ready to commit.");
        return;
    }

    let errors: Vec<_> = findings.iter().filter(|f| f.severity == "error").collect();
    let warnings: Vec<_> = findings.iter().filter(|f| f.severity == "warning").collect();
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
}

fn print_finding(f: &Finding) {
    let file_str = f.file.as_deref().unwrap_or("");
    println!("    [{}] {} {}", f.check, file_str, f.message);
    if let Some(ref line) = f.line {
        println!("      → {line}");
    }
}
