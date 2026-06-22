# PROJECT.md — rivet

**What:** Patch intent verifier. Checks git diffs for pre-commit risks and
reports a `clean`, `caution`, or `blocked` verdict with action items.

**Status:** MVP implemented. Diff parsing, checks, report rendering, docs, and
integration tests are complete.

**Tech:** Rust 2021, clap 4, serde/serde_json, regex, thiserror.

## Module Ownership

| Module | Owner | Status |
| ------ | ----- | ------ |
| cli.rs | Nix | Done |
| main.rs | Nix | Done |
| diff.rs | Nix | Done |
| checks.rs | Nix | Done |
| report.rs | Bjarn | Done |
| docs/SPEC.md | Bjarn | Done |
| README.md | Bjarn | Done |

## Build

```sh
cargo build
cargo check
cargo test
```

## Usage

```sh
rivet check
rivet check --intent "describe the patch"
rivet check --format json
rivet diff
```

## Checks

- `secret_detected`
- `generated_churn`
- `large_diff`
- `large_file_change`
- `missing_tests`
- `risky_file`
- `formatting_churn`
- `todo_added`
- `possibly_unrelated`

## Last Updated

2026-06-22 — MVP complete; `cargo test` passes with 4 integration tests.
