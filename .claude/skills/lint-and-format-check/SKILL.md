---
name: lint-and-format-check
description: Run this repo's configured linter and formatter (command + args from `config.json` — not hardcoded to any one language or toolchain) and return an objective PASS/FAIL plus a machine-checkable `Findings: <n>` count. If `linter`/`formatter` aren't configured, proposes a pair based on the repo's detected language and stops (`Verdict: CONFIG ERROR`) for the user to confirm or override — never guesses and runs something anyway. Use as one leg of a validator loop, or standalone when checking lint/formatting cleanliness — "ist der Code sauber formatiert", "lint check", "check build and format cleanliness".
arguments: none — the skill always uses config.json's `linter`/`formatter`; to check a different target/solution/project, change the configured `args`.
---

# Lint and Format Check

Two deterministic checks, nothing to judge: whatever `linter` and `formatter` this repo's
`config.json` names, run unmodified. Nothing about a specific language or toolchain
belongs in this file — a C# repo configures `dotnet build`/`dotnet format`, a Python repo
configures `ruff`/`mypy`, a Rust repo configures `cargo clippy`/`cargo fmt`; the script
only knows "run command with args, check the exit code."

## Process

0. **Preflight.** Read `config.json`. Both `linter` and `formatter` are required, each an
   object `{"command": "...", "args": [...]}`. If either is missing, **do not guess and
   run something anyway** — a silently-picked tool could be entirely wrong for this repo's
   stack, and running the wrong linter is worse than running none. Instead:

   1. Look at the repo root for language markers — e.g. `*.csproj`/`*.sln`/`*.slnx` → .NET,
      `pyproject.toml`/`setup.py`/`requirements.txt` → Python, `Cargo.toml` → Rust,
      `package.json` → JS/TS, `go.mod` → Go.
   2. Propose one sensible `linter`/`formatter` pair for the detected language, e.g.:
      - Python: linter `ruff check .`, formatter `ruff format --check .` (or `mypy .` /
        `black --check .`, if that's what the repo already uses elsewhere)
      - Rust: linter `cargo clippy -- -D warnings`, formatter `cargo fmt -- --check`
      - .NET/C#: linter `dotnet build <solution>`, formatter `dotnet format <solution>
        --verify-no-changes`
   3. **Stop here without running anything** and report:

      ```
      Verdict: CONFIG ERROR
      `linter`/`formatter` sind in .claude/skills/lint-and-format-check/config.json nicht
      gesetzt. Erkannt: <language markers found, e.g. "*.slnx (.NET)">. Vorschlag:
        "linter":    { "command": "...", "args": [...] }
        "formatter": { "command": "...", "args": [...] }
      Bitte bestaetigen ("ok, wie vorgeschlagen") oder eigene Werte in config.json setzen.
      ```

      Running the proposed commands before the user confirms would defeat the point of
      asking — this skill only ever runs what `config.json` actually names.

1. Run the bundled script from the repo root:
   ```powershell
   uv run .claude/skills/lint-and-format-check/scripts/lint_format_check.py [--json]
   ```
   It runs `config.json`'s `linter` then `formatter` (each `command` + `args`, unmodified)
   and prints a `Findings: <n>` line. A tool-agnostic script can't reliably parse every
   linter's own output format, so the finding count per step is a **coarse** `1` if that
   step's exit code is non-zero, `0` if it's zero — not an exact violation count.

2. **Do not re-run the checks by hand or reformat the output** — paste the script's own
   `output_tail` for any failing step verbatim, the same rule `run-tests` follows for test
   output. Read the actual tail for detail; the `findings` number only tells you which
   step(s) failed, not how many issues each one has.

## Report format

**Verdict: PASS** or **Verdict: FAIL**, then:

| Step | Command | Exit | Findings |
| ----- | ------- | ---- | -------- |
| linter | `<command> <args>` | 0 | 0 |
| formatter | `<command> <args>` | 0 | 0 |

For any non-zero step, the verbatim output tail underneath its row. End with:

```
Findings: <n>
```

`<n>` is the script's total (`0` iff both steps passed). For a step-0 abort, the
`Verdict: CONFIG ERROR` block instead, per `_shared/validator-contract.md`
("`Verdict: CONFIG ERROR` abort").
