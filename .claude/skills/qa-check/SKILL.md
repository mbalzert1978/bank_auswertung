---
name: qa-check
description: Run the repo's test suite via the canonical `run-tests` skill, optionally flag any changed production project whose co-located specs sibling didn't change alongside it, and optionally check every feature's test-facade against Fowler's Test-API pattern (blackbox facade, fluent arrange, encapsulated domain, no visibility-escape). All checks except the test suite are independently toggleable via `config.json` so this skill still makes sense in a repo with a different test topology. Returns PASS/FAIL plus a `Findings: <n>` count. Use as one leg of a validator loop, or standalone when checking test coverage or test-facade conformance — "sind Tests gruen", "fehlt ein Spec-Update", "haelt sich die TestApi an Fowlers Test-API-Pattern", "QA-Check", "did the tests keep up with this change".
arguments: Optional. Base ref to diff against for the coverage-gap check (default: same resolution as `thermo-nuclear-code-quality-review`'s size pass — `origin/HEAD`/`origin/main`/`main` in that order).
---

# QA Check

Three checks, the first mandatory and the other two independently toggleable via
`config.json`'s `checks` object: the test suite must be green; every changed production
project should have a change in its co-located specs project too (a repo-specific
test-colocation convention, named by `production_suffixes`/`specs_suffix` — another repo
might not have this convention at all, hence toggleable); and every feature's test-facade must
actually be the blackbox seam **Fowler's Test-API pattern** describes — not just present,
but *shaped* right: a fluent facade, an encapsulated domain, no compiler/runtime backdoor
that lets a spec bypass the facade.

That third check is judged, not scripted: fluent interfaces, encapsulation, and
dependency direction are semantic properties, not syntax you can regex for reliably
across languages (an earlier version tried a C#-specific regex script here and hit
word-boundary bugs on compound identifiers). The bundled script only finds candidate
files; **you**, running this skill, read them and judge — the same split
`architecture-adr-check` uses for ADRs.

This does **not** re-derive coverage percentages or re-implement test running — it
delegates both to existing, narrower tools. The test-facade check is scoped to *feature*
projects only, via `feature_project_prefix`/`feature_project_suffixes`/`specs_suffix` in
`config.json` — a repo's non-feature test projects (adapter tests against a mocked
external system, ViewModel tests, etc.) intentionally follow a different pattern and stay
out of scope.

Everything repo-specific — the colocation convention, the facade glob, which checks are
even active, the optional ADR that documents this repo's instance of the pattern — lives
in `config.json`. Nothing repo-specific belongs in this file.

## Process

1. **Read `config.json`'s `checks` object.** `coverage_gap` and `test_api_shape` each
   default to `true` if the key is absent, so an unmodified copy of this skill behaves
   exactly as today; set either to `false` in a repo where that convention doesn't apply.
   A disabled check gets one report row saying `disabled via config` — never silently
   folded into a `0`, so a reader can't mistake "not checked" for "checked and clean".

2. **Run the suite.** Use this repo's canonical test runner, unmodified:
   ```powershell
   uv run .claude/skills/run-tests/scripts/run-tests.py
   ```
   Exit 0 → suite green. Exit ≠ 0 → paste the failing test names/errors verbatim (same
   rule `run-tests` itself follows) — do not summarize them away.

3. **If `checks.coverage_gap` is enabled**, run:
   ```powershell
   uv run .claude/skills/qa-check/scripts/coverage_gap.py [base-ref]
   ```
   It lists every changed production project under `projects_root` — those matching a
   `production_suffixes` entry — as `covered` (its specs sibling also changed), `gap`
   (sibling exists on disk but didn't change), or `no-specs-sibling` (no such project
   exists — not a gap, nothing to require). The sibling is derived by stripping the matched
   production suffix and appending `specs_suffix`, so both `foo_domain` and `foo_adapter`
   point at `foo_specs`, while the default `production_suffixes` of `[""]` strips nothing
   and yields `Foo` → `Foo.Specs`. Judge `gap` rows, don't
   rubber-stamp them: a pure rename, a comment-only change, or a change already fully
   covered by an existing test can legitimately need no new test. Read the diff for each
   `gap` row before counting it as a finding; drop it (and say why) if it's a genuine
   no-test-needed change.

4. **If `checks.test_api_shape` is enabled**, run:
   ```powershell
   uv run .claude/skills/qa-check/scripts/test_api_shape.py
   ```
   For every project under `projects_root` matching `feature_project_prefix` **and**
   `feature_project_suffixes` (either may be empty for "any"; both exist because a repo
   may mark its features by a shared prefix, a suffix, or both — this is what keeps
   `Infrastructure`/`Presentation`-style non-feature projects out of scope), it lists
   whether a file exists matching `test_facade_glob`, resolved relative to the project
   directory. A `MISSING` row is an automatic finding — no judgment needed, there's
   nothing to read.

   For every feature **with** a candidate file: first read `reference_feature_project`'s
   own facade file once, to calibrate what "conformant" looks like in this codebase (if
   that reference feature itself doesn't conform, say so as a calibration concern — fix
   `config.json`'s `reference_feature_project` before trusting findings — and don't let it
   block judging the others). Then read each other feature's candidate file plus its
   handler/domain code, and judge against Fowler's Test-API checklist:

   - **Facade is the only supported entry point** — a public, discoverable type/module at
     the location the glob found, not one of several ways in.
   - **Arrange operations are fluent** — each returns the facade itself, so setup chains
     instead of scattering mutation across the test.
   - **At least one act operation returns the real response type** (DTO/DU), not a raw
     primitive or internal state leaking out.
   - **The domain/handler is not directly reachable from outside the facade** — no
     compiler/runtime visibility-escape defeats this (C#: `InternalsVisibleTo` granted to
     the feature's own `.Specs` assembly; other languages: whichever equivalent opens
     "internal" to the test code — reflection-based access, a leading-underscore import,
     etc.).
   - **Dependency direction**: the facade depends on application/domain, never the
     reverse; nothing outside the facade reaches into domain/handler internals directly.

   If `config.json` sets `adr_reference` (e.g. `"ADR-0007"`), cite it as a **footnote** in
   the report ("this repo additionally documents this as ADR-0007") — never as the reason
   the rule exists. The rule exists because it's Fowler's Test-API pattern; the ADR is
   just this repo's paperwork for it.

## Report format

**Verdict: PASS** or **Verdict: FAIL** (FAIL if the suite is red, any enabled `gap` row
survives judgement, or any enabled test-facade check finds a violation), then:

| Check | Result |
| ----- | ------ |
| Test suite | green / red (`N` gesamt, `M` fehlgeschlagen) |
| Coverage gaps | `<project>` — gap / dismissed (reason) / disabled via config |
| Test-facade shape | `<feature>` — conformant / violation (`<rule broken>`) / disabled via config |

For a red suite, the verbatim failing test names underneath. For a shape violation, the
exact rule broken and the `datei:zeile` (or file, if there's no single line to pin). End
with:

```
Findings: <n>
```

`<n>` = failing tests (0 if green) + surviving coverage-gap rows (0 if disabled) +
test-facade violations (0 if disabled). A disabled check contributes `0`, but its row
says `disabled via config`, not a bare `0`, so it's never mistaken for "checked, clean".
