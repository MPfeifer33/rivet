use std::path::Path;
use std::process::Command;
use serde::{Deserialize, Serialize};

use crate::RivetError;

#[derive(Debug, Serialize, Deserialize)]
pub struct DiffData {
    pub base: String,
    pub files: Vec<DiffFile>,
    pub total_additions: usize,
    pub total_deletions: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DiffFile {
    pub path: String,
    pub additions: usize,
    pub deletions: usize,
    pub is_new: bool,
    pub is_deleted: bool,
    pub is_rename: bool,
    pub added_lines: Vec<String>,
}

pub fn parse_diff(repo: &Path, base: Option<&str>) -> Result<DiffData, RivetError> {
    let base = base.unwrap_or("HEAD");

    // Try staged first
    let staged_files = get_diff_stat(repo, &["--cached", "--numstat"])?;
    let files = if !staged_files.is_empty() {
        let added_lines = get_added_lines(repo, &["--cached"])?;
        merge_stats_and_lines(staged_files, added_lines)
    } else {
        // Try unstaged
        let unstaged_files = get_diff_stat(repo, &["--numstat", base])?;
        let added_lines = get_added_lines(repo, &[base])?;
        merge_stats_and_lines(unstaged_files, added_lines)
    };

    let total_additions = files.iter().map(|f| f.additions).sum();
    let total_deletions = files.iter().map(|f| f.deletions).sum();

    Ok(DiffData {
        base: base.to_string(),
        files,
        total_additions,
        total_deletions,
    })
}

fn get_diff_stat(repo: &Path, args: &[&str]) -> Result<Vec<(String, usize, usize)>, RivetError> {
    let mut cmd_args = vec!["diff"];
    cmd_args.extend(args);

    let output = Command::new("git")
        .args(&cmd_args)
        .current_dir(repo)
        .output()?;

    if !output.status.success() {
        return Ok(Vec::new());
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let mut results = Vec::new();

    for line in text.lines() {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() >= 3 {
            let additions = parts[0].parse::<usize>().unwrap_or(0);
            let deletions = parts[1].parse::<usize>().unwrap_or(0);
            let path = parts[2].to_string();
            results.push((path, additions, deletions));
        }
    }

    Ok(results)
}

fn get_added_lines(repo: &Path, args: &[&str]) -> Result<std::collections::HashMap<String, Vec<String>>, RivetError> {
    let mut cmd_args = vec!["diff", "-U0"];
    cmd_args.extend(args);

    let output = Command::new("git")
        .args(&cmd_args)
        .current_dir(repo)
        .output()?;

    let mut map: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
    let mut current_file = String::new();

    if output.status.success() {
        let text = String::from_utf8_lossy(&output.stdout);
        for line in text.lines() {
            if line.starts_with("+++ b/") {
                current_file = line.trim_start_matches("+++ b/").to_string();
            } else if line.starts_with('+') && !line.starts_with("+++") && !current_file.is_empty() {
                map.entry(current_file.clone())
                    .or_default()
                    .push(line[1..].to_string());
            }
        }
    }

    Ok(map)
}

fn merge_stats_and_lines(
    stats: Vec<(String, usize, usize)>,
    added_lines: std::collections::HashMap<String, Vec<String>>,
) -> Vec<DiffFile> {
    stats.into_iter().map(|(path, additions, deletions)| {
        let lines = added_lines.get(&path).cloned().unwrap_or_default();
        DiffFile {
            is_new: deletions == 0 && additions > 0,
            is_deleted: additions == 0 && deletions > 0,
            is_rename: path.contains(" => "),
            path,
            additions,
            deletions,
            added_lines: lines,
        }
    }).collect()
}
