#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.10"
# dependencies = []
# ///
"""Run this repo's configured linter and formatter — command + args come from
config.json, not from this script. lint-and-format-check works for any
language/toolchain this way: a C# repo configures `dotnet build`/`dotnet
format`, a Python repo configures `ruff`/`mypy`, a Rust repo configures
`cargo clippy`/`cargo fmt`, and so on.

Usage:
  lint_format_check.py [--json]

Requires config.json's `linter` and `formatter`, each `{"command": str, "args":
[str, ...]}`. If either is missing, exits 1 naming which key is missing — the
SKILL.md's preflight step is responsible for proposing a default based on the
repo's detected language and asking the user to confirm it, not this script.

A tool-agnostic script can't reliably parse every linter's own output format
for an exact violation count, so `findings` per step is a coarse `1` if that
step's exit code is non-zero, `0` if zero — read `output_tail` for the detail.
"""

import argparse
import json
import subprocess
import sys
from dataclasses import asdict, dataclass
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent.parent / "_shared"))
from skill_config import load_config

HERE = Path(__file__).resolve().parent
CONFIG = HERE.parent / "config.json"
TAIL_LINES = 200


@dataclass(frozen=True)
class StepResult:
    command: str
    exit_code: int
    ok: bool
    findings: int
    output_tail: str


def run(cmd: list[str]) -> subprocess.CompletedProcess[str]:
    return subprocess.run(cmd, capture_output=True, text=True)


def tail(text: str, n: int = TAIL_LINES) -> str:
    lines = text.splitlines()
    return "\n".join(lines[-n:])


def run_step(cmd: list[str]) -> StepResult:
    proc = run(cmd)
    combined = proc.stdout + proc.stderr
    findings = 0 if proc.returncode == 0 else 1
    return StepResult(" ".join(cmd), proc.returncode, proc.returncode == 0, findings, tail(combined))


def configured_tool(cfg: dict, key: str) -> list[str] | None:
    spec = cfg.get(key)
    if not spec or not spec.get("command"):
        return None
    return [spec["command"], *spec.get("args", [])]


def main() -> int:
    ap = argparse.ArgumentParser(description="Run the configured linter and formatter.")
    ap.add_argument("--json", action="store_true", help="emit JSON instead of a human-readable report")
    args = ap.parse_args()

    as_json = args.json
    cfg = load_config(CONFIG)
    linter_cmd = configured_tool(cfg, "linter")
    formatter_cmd = configured_tool(cfg, "formatter")

    if linter_cmd is None or formatter_cmd is None:
        missing = [n for n, v in (("linter", linter_cmd), ("formatter", formatter_cmd)) if v is None]
        print(f"error: {', '.join(missing)} not configured in config.json", file=sys.stderr)
        return 1

    lint = run_step(linter_cmd)
    fmt = run_step(formatter_cmd)
    passed = lint.ok and fmt.ok

    result = {
        "pass": passed,
        "findings": lint.findings + fmt.findings,
        "linter": asdict(lint),
        "formatter": asdict(fmt),
    }

    if as_json:
        print(json.dumps(result, indent=2))
    else:
        print(f"linter:    {lint.command}  exit={lint.exit_code} findings={lint.findings}")
        if not lint.ok:
            print(lint.output_tail)
        print(f"formatter: {fmt.command}  exit={fmt.exit_code} findings={fmt.findings}")
        if not fmt.ok:
            print(fmt.output_tail)
        print(f"\npass: {passed}")
        print(f"Findings: {result['findings']}")

    return 0 if passed else 1


if __name__ == "__main__":
    sys.exit(main())
