# rivet

`rivet` is a patch intent verifier. It checks the current git diff for common
pre-commit risks: secrets, missing tests, generated churn, large diffs,
formatting-only churn, risky files, new task markers, and edits that may not
match the stated intent.

It answers:

```text
Does this patch look ready to commit, and what should I fix first?
```

## Quickstart

```sh
cargo build

# Check staged changes, or unstaged changes if nothing is staged.
cargo run -- check

# Include stated intent for unrelated-edit detection.
cargo run -- check --intent "add ranked recommendations"

# Machine-readable report.
cargo run -- check --format json
```

After installation, replace `cargo run --` with `rivet`.

## Commands

### check

```sh
rivet check
rivet check --intent "fix login validation"
rivet check --base main
rivet check --format json
```

Verdicts:

- `clean`: no findings
- `caution`: warnings exist
- `blocked`: error findings exist

### diff

```sh
rivet diff
rivet diff --base main
rivet diff --format json
```

Shows parsed diff metadata without running the checks.

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

## JSON Output

```json
{
  "ok": true,
  "verdict": "caution",
  "summary": {
    "files_changed": 1,
    "additions": 4,
    "deletions": 1,
    "errors": 0,
    "warnings": 1,
    "info": 0
  },
  "action_items": [],
  "findings": []
}
```

`findings` are raw check results. `action_items` are the recommended next
steps.

## Typical Agent Flow

```sh
probe doctor
latch claim acquire src/report.rs --intent "report output"

# Edit files.

sieve analyze
rivet check --intent "report output"
witness run --tag test -- cargo test
```

## Design

The implementation contract is in [docs/SPEC.md](docs/SPEC.md).
