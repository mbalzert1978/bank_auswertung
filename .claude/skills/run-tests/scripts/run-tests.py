#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.10"
# dependencies = []
# ///
"""Canonical test runner - runs whatever `config.json` declares.

The one invocation that works for a repo is a repo property, not this script's:
a Rust workspace runs `cargo test`, a .NET solution runs `dotnet test`, a Python
project runs `pytest`. This script locks in *that* invocation at a single stable
location and layers a controlled filter mode on top, so the agent never has to
rediscover the working command.

config.json:

  test_command    {"command": ..., "args": [...]}  the whole-suite invocation
  selectors       {"<cli-name>": [arg-template, ...]}  repeatable selector flags;
                  each key becomes a `--<cli-name> VALUE` option and "{}" in its
                  templates is replaced by the value
  name_filter     [arg-template, ...] for --name, or [] if the runner has no
                  free-text name filter
  exact_args      appended after the name filter when --exact is passed
  ignore_exit_codes_when_filtered
                  exit codes to report as success in filter mode only, for
                  runners that signal "no test matched" with a non-zero code
                  (e.g. .NET MTP's 8); [] when the runner doesn't do that

So a Rust repo declares selectors `package`/`test`, and a .NET one declares
`class`/`method`/`namespace`/`trait` with `ignore_exit_codes_when_filtered: [8]`
-- same script, same CLI shape, no code change.

  run-tests.py                       # whole suite
  run-tests.py --package io          # a configured selector (repeatable)
  run-tests.py --name storage        # free-text name filter
  run-tests.py --name a::b --exact   # that exact test only

Note what filter mode cannot tell you: unless the runner has a dedicated
"zero tests" exit code, a filter that matches nothing still exits 0. Read the
count in the output, not just the exit code.

Working dir is the repo root (harness guarantee); the process exit code is
forwarded unchanged, except for the configured ignore list in filter mode.
"""

from __future__ import annotations

import argparse
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import NoReturn

sys.path.insert(0, str(Path(__file__).resolve().parent.parent.parent / "_shared"))
from skill_config import load_config

HERE = Path(__file__).resolve().parent
CONFIG = HERE.parent / "config.json"

PLACEHOLDER = "{}"


def die(msg: str) -> NoReturn:
    print(f"run-tests: error: {msg}", file=sys.stderr)
    sys.exit(1)


@dataclass(frozen=True)
class Runner:
    """The repo's test invocation and the filter vocabulary it understands.
    Constructed only by `load_runner`, so every template is already validated."""

    command: tuple[str, ...]
    selectors: dict[str, tuple[str, ...]]
    name_filter: tuple[str, ...]
    exact_args: tuple[str, ...]
    ignore_when_filtered: frozenset[int]

    def supports_name_filter(self) -> bool:
        return bool(self.name_filter)


@dataclass(frozen=True)
class Selection:
    """What to run. Constructed only by `parse_selection`, so `exact` can never
    be set without the `name` it qualifies, and no unsupported filter reaches
    the command line."""

    selected: tuple[tuple[str, str], ...]  # (selector name, value), in CLI order
    name: str | None
    exact: bool

    def is_filtered(self) -> bool:
        return bool(self.selected) or self.name is not None


def expand(template: tuple[str, ...], value: str) -> tuple[str, ...]:
    return tuple(arg.replace(PLACEHOLDER, value) for arg in template)


def load_runner(path: Path) -> Runner:
    cfg = load_config(path)

    match cfg.get("test_command"):
        case {"command": str(command), **rest} if command.strip():
            args = rest.get("args", [])
        case _:
            die(
                f"config test_command must be an object with a non-empty `command` "
                f"(and optional `args`), set it in {path}"
            )
    if not isinstance(args, (list, tuple)):
        die(f"config test_command.args must be a list of strings, got {args!r}")
    if not isinstance(selectors := cfg.get("selectors", {}), dict):
        die(f"config selectors must be an object of name -> arg templates, got {selectors!r}")

    return Runner(
        command=(command, *(str(a) for a in args)),
        selectors={
            str(name): tuple(str(a) for a in template)
            for name, template in selectors.items()
            if isinstance(template, (list, tuple)) and template
        },
        name_filter=tuple(str(a) for a in cfg.get("name_filter", [])),
        exact_args=tuple(str(a) for a in cfg.get("exact_args", [])),
        ignore_when_filtered=frozenset(
            int(code) for code in cfg.get("ignore_exit_codes_when_filtered", [])
        ),
    )


def parse_selection(args: argparse.Namespace, runner: Runner) -> Selection:
    if args.name and not runner.supports_name_filter():
        die("this repo's configured runner has no name filter (`name_filter` is empty)")
    if args.exact and not args.name:
        die("--exact qualifies a test name: pass --name NAME with it")
    if args.exact and not runner.exact_args:
        die("this repo's configured runner has no exact-match mode (`exact_args` is empty)")
    return Selection(
        selected=tuple(
            (name, value) for name in runner.selectors for value in getattr(args, name)
        ),
        name=args.name,
        exact=args.exact,
    )


def build_command(selection: Selection, runner: Runner) -> tuple[str, ...]:
    selectors = tuple(
        arg
        for name, value in selection.selected
        for arg in expand(runner.selectors[name], value)
    )
    match (selection.name, selection.exact):
        case (None, _):
            filter_args: tuple[str, ...] = ()
        case (name, False):
            filter_args = expand(runner.name_filter, name)
        case (name, True):
            filter_args = (*expand(runner.name_filter, name), *runner.exact_args)
    return (*runner.command, *selectors, *filter_args)


def main() -> int:
    runner = load_runner(CONFIG)

    parser = argparse.ArgumentParser(
        description="Canonical test runner; the invocation and filters come from config.json."
    )
    for name, template in runner.selectors.items():
        parser.add_argument(
            f"--{name}",
            dest=name,
            action="append",
            default=[],
            metavar="VALUE",
            help=f"repeatable selector -> {' '.join(template)}",
        )
    parser.add_argument("--name", metavar="NAME", help="filter tests by name")
    parser.add_argument(
        "--exact", action="store_true", help="match --name exactly instead of as a substring"
    )

    selection = parse_selection(parser.parse_args(), runner)
    code = subprocess.run(build_command(selection, runner)).returncode
    if selection.is_filtered() and code in runner.ignore_when_filtered:
        return 0  # the runner's "nothing matched this filter" code, not a failure
    return code


if __name__ == "__main__":
    sys.exit(main())
