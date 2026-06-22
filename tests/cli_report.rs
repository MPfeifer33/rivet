//! Integration tests for rivet report output.

use std::fs;
use std::path::Path;
use std::process::{Command, Output};
use tempfile::TempDir;

fn rivet(dir: &Path) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_rivet"));
    cmd.arg("--repo").arg(dir);
    cmd
}

fn assert_success(output: &Output, label: &str) {
    assert!(
        output.status.success(),
        "{label} failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn json_output(output: Output, label: &str) -> serde_json::Value {
    assert_success(&output, label);
    serde_json::from_slice(&output.stdout).unwrap_or_else(|err| {
        panic!(
            "{label} returned invalid json: {err}\nstdout:\n{}",
            String::from_utf8_lossy(&output.stdout)
        )
    })
}

fn init_repo(dir: &Path) {
    fs::write(
        dir.join("Cargo.toml"),
        r#"[package]
name = "sample"
version = "0.1.0"
edition = "2021"
"#,
    )
    .unwrap();
    fs::create_dir_all(dir.join("src")).unwrap();
    fs::create_dir_all(dir.join("tests")).unwrap();
    fs::write(dir.join("src/lib.rs"), "pub fn value() -> u8 { 1 }\n").unwrap();
    fs::write(
        dir.join("tests/cli_value.rs"),
        "#[test] fn value_cli() {}\n",
    )
    .unwrap();

    assert_success(
        &Command::new("git")
            .arg("init")
            .current_dir(dir)
            .output()
            .unwrap(),
        "git init",
    );
    assert_success(
        &Command::new("git")
            .args(["config", "user.email", "rivet@example.test"])
            .current_dir(dir)
            .output()
            .unwrap(),
        "git config email",
    );
    assert_success(
        &Command::new("git")
            .args(["config", "user.name", "Rivet Test"])
            .current_dir(dir)
            .output()
            .unwrap(),
        "git config name",
    );
    assert_success(
        &Command::new("git")
            .args(["add", "-A"])
            .current_dir(dir)
            .output()
            .unwrap(),
        "git add",
    );
    assert_success(
        &Command::new("git")
            .args(["commit", "-m", "baseline"])
            .current_dir(dir)
            .output()
            .unwrap(),
        "git commit",
    );
}

#[test]
fn json_report_includes_action_items_for_missing_tests() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    init_repo(dir);

    fs::write(dir.join("src/lib.rs"), "pub fn value() -> u8 { 2 }\n").unwrap();

    let json = json_output(
        rivet(dir)
            .args(["--format", "json", "check", "--intent", "update value"])
            .output()
            .unwrap(),
        "rivet check json",
    );

    assert_eq!(json["verdict"], "caution");
    assert!(json["findings"]
        .as_array()
        .unwrap()
        .iter()
        .any(|finding| finding["check"] == "missing_tests"));
    assert!(json["action_items"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["check"] == "missing_tests"));
}

#[test]
fn json_report_blocks_on_secret_like_added_line() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    init_repo(dir);

    fs::write(
        dir.join("src/lib.rs"),
        "pub fn key() { let api_key = \"abcdefghijklmnop\"; }\n",
    )
    .unwrap();

    let json = json_output(
        rivet(dir)
            .args(["--format", "json", "check"])
            .output()
            .unwrap(),
        "rivet check json",
    );

    assert_eq!(json["verdict"], "blocked");
    assert!(json["action_items"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["check"] == "secret_detected" && item["severity"] == "error"));
}

#[test]
fn text_report_highlights_action_items() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    init_repo(dir);

    fs::write(dir.join("src/lib.rs"), "pub fn value() -> u8 { 3 }\n").unwrap();

    let output = rivet(dir).args(["check"]).output().unwrap();
    assert_success(&output, "rivet check text");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("rivet check"));
    assert!(stdout.contains("Action items"));
    assert!(stdout.contains("missing_tests"));
}

#[test]
fn untracked_test_files_count_as_test_changes() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    init_repo(dir);

    fs::write(dir.join("src/lib.rs"), "pub fn value() -> u8 { 4 }\n").unwrap();
    fs::write(
        dir.join("tests/cli_new_value.rs"),
        "#[test] fn new_value_cli() {}\n",
    )
    .unwrap();

    let json = json_output(
        rivet(dir)
            .args(["--format", "json", "check"])
            .output()
            .unwrap(),
        "rivet check json",
    );

    assert!(!json["findings"]
        .as_array()
        .unwrap()
        .iter()
        .any(|finding| finding["check"] == "missing_tests"));
}
