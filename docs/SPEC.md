# rivet spec

Status: MVP implementation contract

`rivet` is a patch intent verifier. It reads the current git diff, checks for
common pre-commit risks, and reports whether the patch looks clean, cautious,
or blocked.

## Goals

- Catch high-risk patch issues before commit.
- Preserve machine-readable findings for agent workflows.
- Provide concise action items for each finding.
- Support an optional stated intent to flag possibly unrelated edits.

## Non-Goals

- Full static analysis.
- Replacing human review.
- Running tests automatically.
- Perfect natural-language understanding of intent.

## Commands

### check

```sh
rivet check
rivet check --intent "add recommendations output"
rivet check --base main
rivet check --format json
```

Checks staged changes first. If nothing is staged, checks unstaged changes
against `--base` or `HEAD`.

### diff

```sh
rivet diff
rivet diff --base main
rivet diff --format json
```

Shows parsed diff metadata without running checks.

## Verdicts

- `clean`: no findings
- `caution`: warning findings exist, but no errors
- `blocked`: one or more error findings exist

## Checks

| Check | Severity | Meaning |
| ----- | -------- | ------- |
| `secret_detected` | error | Secret-like value appears in added lines |
| `generated_churn` | info | Large generated/lockfile change |
| `large_diff` | warning | Patch is large enough to consider splitting |
| `large_file_change` | info | One file has a large change |
| `missing_tests` | warning | Source changed but tests did not |
| `risky_file` | info | Operational/config file changed |
| `formatting_churn` | warning | Changes appear mostly whitespace-only |
| `todo_added` | info | New task/follow-up marker |
| `possibly_unrelated` | warning | File may not match stated intent |

## JSON Schema

`rivet check --format json` returns:

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
  "action_items": [
    {
      "severity": "warning",
      "check": "missing_tests",
      "file": null,
      "action": "Add targeted tests, run an existing relevant test, or document why test changes are unnecessary."
    }
  ],
  "findings": [
    {
      "check": "missing_tests",
      "severity": "warning",
      "file": null,
      "line": null,
      "message": "1 source file(s) changed but no test files in diff"
    }
  ]
}
```

`findings` are raw check results. `action_items` is the recommended next-step
view for agents.

## Text Output

Text output is optimized for terminal review:

```text
rivet check: caution
  1 file(s), +4 -1

  Warnings (1):
    [missing_tests]  1 source file(s) changed but no test files in diff

  Action items:
    [missing_tests] warning: Add targeted tests, run an existing relevant test, or document why test changes are unnecessary.
```

## Exit Codes

| Code | Meaning |
| ---- | ------- |
| `0` | Command completed |
| `1` | Validation or JSON error |
| `2` | IO error |

`blocked` is data, not a process failure, in the MVP. Consumers should inspect
the JSON verdict.
