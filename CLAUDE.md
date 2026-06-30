# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project state

This is a freshly scaffolded `uv` project (the output of `uv init`). At present it contains only boilerplate: a hello-world `main.py`, an empty `README.md`, no third-party dependencies, and no tests. There is no real application code or architecture yet — expect to build it out.

The name `bank-auswertung` (German for "bank statement evaluation/analysis") signals the intended domain, but nothing in the code implements it so far.

## Tooling

Managed with [`uv`](https://docs.astral.sh/uv/). Python `>=3.13` is required (pinned to `3.13` in `.python-version`). There is no `uv.lock` yet — it will be created on the first `uv` command that resolves dependencies.

## Commands

```bash
uv run main.py            # run the entrypoint
uv run python -m <module> # run a module once real code exists
uv add <package>          # add a runtime dependency (updates pyproject.toml + uv.lock)
uv add --dev <package>    # add a dev dependency (e.g. pytest, ruff)
uv sync                   # install/sync the environment from pyproject + lockfile
uv lock                   # regenerate the lockfile
```

No test, lint, or format tooling is configured yet. If you add `pytest`, run tests with `uv run pytest` (single test: `uv run pytest path/to/test.py::test_name`). If you add `ruff`, use `uv run ruff check .` and `uv run ruff format .`.
