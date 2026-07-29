---
name: run-tests
description: "Runs the repo's test suite via the one canonical invocation — whatever `config.json` declares (`cargo test`, `dotnet test`, `pytest`, …) — with an optional filter mode for a subset. Use when you need to execute the project's tests to verify a change, confirm a fix, or check for regressions."
arguments: Optional. Filter flags for a subset run — the configured selectors (repeatable), plus `--name`/`--exact`. No arguments runs the whole suite.
---

# Run Tests

Canonical test-runner for this repo. One job: execute the suite correctly and surface results.

## Why this skill exists

Every repo has exactly one invocation that reliably runs everything, and it is rarely the
obvious one — a targeting flag that silently runs nothing, a filter syntax only that runner
understands. This skill pins that invocation, and the filter vocabulary that goes with it, at
a single stable location so the agent never rediscovers it. **Which** invocation is a repo
property, so it lives in `config.json`, not in this file or in the script.

## Configuration

`config.json` declares the runner. Nothing repo-specific belongs anywhere else:

| Key | Meaning |
| --- | ------- |
| `test_command` | `{"command": …, "args": […]}` — the whole-suite invocation |
| `selectors` | `{"<cli-name>": [arg templates]}` — each key becomes a repeatable `--<cli-name> VALUE` option; `{}` in a template is replaced by the value |
| `name_filter` | arg template for `--name`, or `[]` if the runner has no free-text name filter |
| `exact_args` | appended after the name filter when `--exact` is passed |
| `ignore_exit_codes_when_filtered` | exit codes to report as success **in filter mode only**, for runners that signal "no test matched" with a non-zero code; `[]` when the runner doesn't |

The script builds its own CLI from `selectors`, so adding a filter dimension is a config edit,
never a code change. A Rust workspace declares `package`/`test`; a .NET solution declares
`class`/`method`/`namespace`/`trait` and `ignore_exit_codes_when_filtered: [8]` for the MTP
"zero tests" code.

## Process

### 1. Run the bundled script

Execute from the repo root:
```powershell
uv run .claude/skills/run-tests/scripts/run-tests.py                     # whole suite
uv run .claude/skills/run-tests/scripts/run-tests.py --help              # the configured selectors
uv run .claude/skills/run-tests/scripts/run-tests.py --<selector> VALUE  # a configured selector
uv run .claude/skills/run-tests/scripts/run-tests.py --name PATTERN      # by test name
uv run .claude/skills/run-tests/scripts/run-tests.py --name X --exact    # that exact test only
```

Do not call the underlying runner directly — always go through this script, so the one
canonical invocation stays canonical. It runs from the repo root (harness guarantee) and
forwards the exit code unchanged. Selector options are repeatable and combine the way the
underlying runner combines them; `--name` is a single filter, and `--exact` requires it.

Prefer a filter when you only need to check a specific area — it is far faster than the full
suite. Use the no-argument form to confirm the whole suite is green. `--help` lists exactly the
selectors this repo configured, so start there rather than guessing a flag.

### 2. Surface results

After the script exits:
- **Exit 0** — print the pass/skip counts from the output and continue.
- **Exit ≠ 0** — paste failing test names and error lines verbatim; stop.

The raw runner output is sufficient — do not reformat or summarise it beyond the final count
line.

### 3. Exit code contract

- `0` → tests passed; continue with whatever triggered this skill.
- `≠ 0` → tests failed; stop and surface failures before proceeding.

In filter mode the script maps any `ignore_exit_codes_when_filtered` code to `0`, so a runner
that reports "nothing matched this filter" as a non-zero code is not mistaken for a failure. A
real test failure is never in that list, so the contract above holds. One caveat, and it is the
reason to read the output rather than trust the code alone: unless the runner has such a
dedicated code, **a filter that matches nothing still exits 0** — check the count.

## Done criterion

Script exits. Exit code and pass/fail summary reported to the user.
If exit code ≠ 0, failing tests surfaced inline.
