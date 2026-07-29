#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.10"
# dependencies = []
# ///
"""validate-fix-loop: turn one iteration's validator reports into a dispatch plan.

Requires Python 3.10+ (match statements, PEP 604 `X | None` unions). The PEP 723
metadata block above lets `uv run` provision a suitable interpreter regardless of
the system default — invoke via `uv run scripts/plan_iteration.py` (see SKILL.md).

Everything mechanical about an iteration lives here so the orchestrating model
never re-derives it by hand:

  * parse each report's `Verdict: CONFIG ERROR` marker and `Findings: <n>` trailer
  * sum this iteration's findings and compare against the previous iteration
    (clean / plateau / cap / continue)
  * route each verifier with findings to its own fixer via config's `fixer_map`
  * extract the file paths each verifier's findings point at, and group the fixer
    dispatches into WAVES whose file sets are pairwise disjoint — so the
    orchestrator can run one wave in parallel without two fixers racing on the
    same file, and run the waves themselves in order.

A dispatch unit whose files could not be determined from its report is placed in
a wave of its own: unknown reach is treated as "might touch anything", which is
the conservative choice.

Input (stdin, JSON):

  { "config_path": ".claude/skills/validate-fix-loop/config.json",
    "iteration": 1,
    "previous_total": null,
    "reports": [ { "validator": "verifier-long-method",
                   "report": "<that validator's full report text, verbatim>" },
                 ... one per configured validator ] }

Output (stdout, JSON):

  { "stop": null | "config_error" | "clean" | "plateau" | "cap",
    "current_total": 4,
    "config_error_validators": [],
    "waves": [ [ { "type": "fixer", "validator": "...", "skill": "...",
                   "files": ["src/a.rs"] }, ... ], ... ] }

Exit code is 0 on success, 1 on a usage/validation error.
"""

import json
import re
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any, NoReturn, TypeAlias

CONFIG_ERROR_RE = re.compile(r"^\s*Verdict:\s*CONFIG ERROR\b", re.MULTILINE)
FINDINGS_RE = re.compile(r"^\s*Findings:\s*(\d+)\s*$", re.MULTILINE)
BACKTICKED_RE = re.compile(r"`([^`\n]+)`")
PATHLIKE_RE = re.compile(r"^[\w./\\-]+\.[A-Za-z0-9_]+$")


def die(msg: str) -> NoReturn:
    print(f"plan_iteration: {msg}", file=sys.stderr)
    raise SystemExit(1)


@dataclass(frozen=True)
class Report:
    """One validator's verbatim report, as handed over by its subagent."""

    validator: str
    text: str

    @property
    def is_config_error(self) -> bool:
        return CONFIG_ERROR_RE.search(self.text) is not None

    @property
    def findings(self) -> int:
        """The LAST `Findings: <n>` trailer — earlier ones may be quoted examples."""
        matches = FINDINGS_RE.findall(self.text)
        return int(matches[-1]) if matches else 0

    @property
    def files(self) -> frozenset[str]:
        """Paths this report's findings point at, read from its table rows only.

        Every verifier reports findings as a markdown table with a location
        column, so restricting extraction to table rows keeps prose mentions of
        other skills and documents out of the file sets.
        """
        found: set[str] = set()
        for line in self.text.splitlines():
            if not line.lstrip().startswith("|"):
                continue
            for token in BACKTICKED_RE.findall(line):
                if (path := _as_path(token)) is not None:
                    found.add(path)
        return frozenset(found)


def _as_path(token: str) -> str | None:
    """Reduce a backticked table token to a normalized path, or None."""
    candidate = token.strip().split(":", 1)[0].strip()
    if not candidate or not PATHLIKE_RE.match(candidate):
        return None
    return candidate.replace("\\", "/")


@dataclass(frozen=True)
class Unit:
    """One fixer dispatch: exactly one verifier's findings to its own fixer."""

    validator: str
    skill: str
    files: frozenset[str]

    def as_json(self) -> dict[str, Any]:
        return {
            "type": "fixer",
            "validator": self.validator,
            "skill": self.skill,
            "files": sorted(self.files),
        }


@dataclass(frozen=True)
class ConfigError:
    validators: tuple[str, ...]


@dataclass(frozen=True)
class Clean:
    pass


@dataclass(frozen=True)
class Plateau:
    pass


@dataclass(frozen=True)
class Cap:
    pass


@dataclass(frozen=True)
class Proceed:
    waves: tuple[tuple[Unit, ...], ...]


Outcome: TypeAlias = ConfigError | Clean | Plateau | Cap | Proceed


def load_config(path: Path) -> dict[str, Any]:
    if not path.is_file():
        die(f"config not found: {path}")
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as exc:
        die(f"config is not valid JSON: {exc}")
    if not isinstance(data, dict):
        die("config must be a JSON object")
    return data


def read_request() -> dict[str, Any]:
    raw = sys.stdin.read().strip()
    if not raw:
        die("no JSON on stdin — see the module docstring for the expected shape")
    try:
        data = json.loads(raw)
    except json.JSONDecodeError as exc:
        die(f"stdin is not valid JSON: {exc}")
    if not isinstance(data, dict):
        die("stdin must be a JSON object")
    return data


def partition_into_waves(units: tuple[Unit, ...]) -> tuple[tuple[Unit, ...], ...]:
    """Greedily pack units into waves that are internally file-disjoint.

    Units keep their configured order, so the packing is deterministic: each unit
    joins the first wave that shares no file with it, otherwise it opens a new
    one. A unit with an empty file set never joins an existing wave — its reach
    is unknown, so it gets a wave to itself.
    """
    waves: list[list[Unit]] = []
    claimed: list[set[str]] = []
    for unit in units:
        placed = False
        if unit.files:
            for wave, taken in zip(waves, claimed):
                if taken.isdisjoint(unit.files) and all(u.files for u in wave):
                    wave.append(unit)
                    taken.update(unit.files)
                    placed = True
                    break
        if not placed:
            waves.append([unit])
            claimed.append(set(unit.files))
    return tuple(tuple(wave) for wave in waves)


def decide(
    reports: tuple[Report, ...],
    fixer_map: dict[str, str],
    iteration: int,
    previous_total: int | None,
    max_iterations: int,
) -> tuple[Outcome, int]:
    broken = tuple(r.validator for r in reports if r.is_config_error)
    if broken:
        return ConfigError(broken), 0

    total = sum(r.findings for r in reports)
    if total == 0:
        return Clean(), 0
    if previous_total is not None and total >= previous_total:
        return Plateau(), total
    if iteration >= max_iterations:
        return Cap(), total

    units = tuple(
        Unit(r.validator, fixer_map[r.validator], r.files)
        for r in reports
        if r.findings > 0 and r.validator in fixer_map
    )
    return Proceed(partition_into_waves(units)), total


def render(outcome: Outcome, total: int) -> dict[str, Any]:
    base: dict[str, Any] = {
        "stop": None,
        "current_total": total,
        "config_error_validators": [],
        "waves": [],
    }
    match outcome:
        case ConfigError(validators):
            return base | {"stop": "config_error", "config_error_validators": list(validators)}
        case Clean():
            return base | {"stop": "clean"}
        case Plateau():
            return base | {"stop": "plateau"}
        case Cap():
            return base | {"stop": "cap"}
        case Proceed(waves):
            return base | {"waves": [[u.as_json() for u in wave] for wave in waves]}


def main() -> int:
    request = read_request()
    config = load_config(Path(request.get("config_path", "")))

    configured = config.get("validators")
    fixer_map = config.get("fixer_map")
    if not isinstance(configured, list) or not configured:
        die("config's `validators` must be a non-empty list")
    if not isinstance(fixer_map, dict):
        die("config's `fixer_map` must be an object")

    order = {name: i for i, name in enumerate(configured)}
    raw_reports = request.get("reports")
    if not isinstance(raw_reports, list) or not raw_reports:
        die("`reports` must be a non-empty list")

    reports = tuple(
        sorted(
            (Report(str(r.get("validator", "")), str(r.get("report", ""))) for r in raw_reports),
            key=lambda r: order.get(r.validator, len(order)),
        )
    )
    missing = [name for name in configured if name not in {r.validator for r in reports}]
    if missing:
        die(f"no report handed over for: {', '.join(missing)}")

    previous = request.get("previous_total")
    outcome, total = decide(
        reports,
        fixer_map,
        int(request.get("iteration", 1)),
        None if previous is None else int(previous),
        int(config.get("max_iterations", 3)),
    )
    json.dump(render(outcome, total), sys.stdout, indent=2)
    sys.stdout.write("\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
