#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.10"
# dependencies = []
# ///
"""check_dag: the deterministic half of criterion 3 (sound dependency graph).

This handles only the *mechanical* check the agent must never eyeball — building
the blocked-by graph from the drafted slices and proving it is a DAG. The verdict
(and the other five criteria) stays in SKILL.md.

Requires Python 3.10+ (match statements, PEP 604 `X | None` unions). The PEP 723
metadata block above lets `uv run` provision a suitable interpreter regardless of
the system default — invoke via `uv run scripts/check_dag.py …` (see SKILL.md).

Input is a JSON list of slices, each `{ "id": ..., "blocked_by": [...] }`, read
from a file path argument or from stdin with `-`:

  [ { "id": "S1", "blocked_by": [] },
    { "id": "S2", "blocked_by": ["S1"] },
    { "id": "S3", "blocked_by": ["S4"] } ]   # S4 dangles

`id` is required and must be unique; `blocked_by` is optional (defaults to []).

Output is a single JSON object on stdout:

  { "acyclic": bool,           # true iff no cycle was found
    "cycles": [["S2","S3",…]], # each cycle as the slice ids on it, in order
    "dangling": [["S3","S4"]] } # each [referrer, missing-blocker] pair

Exit code is 0 when the graph is a clean DAG (acyclic and no dangling refs), 1 on
a usage / validation error, 2 when the graph parsed but has cycles and/or dangling
references — so a caller can gate on the exit code alone.
"""

import json
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any, NoReturn


def die(msg: str) -> NoReturn:
    print(f"check_dag: error: {msg}", file=sys.stderr)
    sys.exit(1)


# --- Domain model: a validated graph, illegal states unrepresentable ----------


@dataclass(frozen=True)
class Slice:
    """One drafted slice: a unique id and the ids it is blocked by."""

    id: str
    blocked_by: tuple[str, ...]


@dataclass(frozen=True)
class Graph:
    """The blocked-by graph. Built only by build_graph, so every Slice id is
    unique and `edges` maps each id to its (possibly dangling) blockers."""

    order: tuple[str, ...]  # slice ids in input order, for stable output
    edges: dict[str, tuple[str, ...]]

    def known(self, node: str) -> bool:
        return node in self.edges


@dataclass(frozen=True)
class Report:
    acyclic: bool
    cycles: tuple[tuple[str, ...], ...]
    dangling: tuple[tuple[str, str], ...]  # (referrer, missing blocker)

    def to_json(self) -> dict[str, Any]:
        return {
            "acyclic": self.acyclic,
            "cycles": [list(c) for c in self.cycles],
            "dangling": [list(d) for d in self.dangling],
        }


# --- Parsing ------------------------------------------------------------------


def load_slices(spec: str) -> list[Any]:
    match spec:
        case "-":
            raw = sys.stdin.read()
        case _:
            p = Path(spec)
            if not p.is_file():
                die(f"slices file not found: {spec}")
            raw = p.read_text()
    try:
        data = json.loads(raw)
    except json.JSONDecodeError as e:
        die(f"could not parse slices JSON: {e}")
    if not isinstance(data, list):
        die("slices must be a JSON list of objects")
    return data


def parse_slice(raw: Any, idx: int) -> Slice:
    if not isinstance(raw, dict):
        die(f"slice #{idx} must be a JSON object")
    if not (sid := str(raw.get("id") or "").strip()):
        die(f"slice #{idx} needs a non-empty `id`")
    match raw.get("blocked_by", []):
        case list() as blockers:
            return Slice(sid, tuple(str(b).strip() for b in blockers if str(b).strip()))
        case other:
            die(f"slice {sid!r}: `blocked_by` must be a list, got {other!r}")


def build_graph(raw: list[Any]) -> Graph:
    slices = [parse_slice(item, i) for i, item in enumerate(raw, 1)]
    edges: dict[str, tuple[str, ...]] = {}
    for s in slices:
        if s.id in edges:
            die(f"duplicate slice id: {s.id!r}")
        edges[s.id] = s.blocked_by
    return Graph(tuple(s.id for s in slices), edges)


# --- Checks -------------------------------------------------------------------


def find_dangling(graph: Graph) -> tuple[tuple[str, str], ...]:
    """Every blocked_by id that is not itself a slice in the set."""
    return tuple(
        (node, blocker)
        for node in graph.order
        for blocker in graph.edges[node]
        if not graph.known(blocker)
    )


def find_cycles(graph: Graph) -> tuple[tuple[str, ...], ...]:
    """Detect cycles via DFS with three-colour marking (white/grey/black).

    A grey node reached again closes a cycle; the cycle is the grey stack from
    that node onward. Dangling blockers are skipped here — find_dangling owns
    them — so only edges within the known set are traversed."""
    WHITE, GREY, BLACK = 0, 1, 2
    colour = {node: WHITE for node in graph.order}
    cycles: list[tuple[str, ...]] = []
    seen: set[frozenset[str]] = set()

    def visit(node: str, stack: list[str]) -> None:
        colour[node] = GREY
        stack.append(node)
        for blocker in graph.edges[node]:
            if not graph.known(blocker):
                continue
            match colour[blocker]:
                case c if c == GREY:
                    cycle = tuple(stack[stack.index(blocker):])
                    if (key := frozenset(cycle)) not in seen:
                        seen.add(key)
                        cycles.append(cycle)
                case c if c == WHITE:
                    visit(blocker, stack)
        stack.pop()
        colour[node] = BLACK

    for node in graph.order:
        if colour[node] == WHITE:
            visit(node, [])
    return tuple(cycles)


def check(graph: Graph) -> Report:
    cycles = find_cycles(graph)
    return Report(acyclic=not cycles, cycles=cycles, dangling=find_dangling(graph))


def main() -> int:
    args = sys.argv[1:]
    if len(args) != 1:
        die("usage: check_dag.py <slices.json | ->  (- reads JSON from stdin)")
    report = check(build_graph(load_slices(args[0])))
    print(json.dumps(report.to_json(), indent=2))
    return 0 if report.acyclic and not report.dangling else 2


if __name__ == "__main__":
    sys.exit(main())
