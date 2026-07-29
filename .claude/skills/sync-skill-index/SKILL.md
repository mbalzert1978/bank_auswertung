---
name: sync-skill-index
description: Regenerate the skill-inventory region of CLAUDE.md from the filesystem — the directory tree (with each skill's bundled scripts/ assets/ config.json) and the four bucket lists — so the docs never drift from the skills on disk. Reads the name→bucket map and summaries from config.json and splices the result between markers, leaving the rest of CLAUDE.md untouched. Use when a skill was added, removed, or renamed and CLAUDE.md must be updated, when you want to regenerate or sync the skill inventory / index / list, or after running skill-linker.
arguments: Optional. Path to a config.json with the skills_dir, CLAUDE.md path, bucket map, and summaries. If omitted, the bundled config.json next to this skill is used.
---

# Sync Skill Index

Keep the **inventory region of CLAUDE.md** — the directory tree and the four
bucket lists — in sync with the skills actually on disk. One deterministic
Python script scans the skills dir, fills a template, and splices it between
markers. Don't regenerate the tree or re-classify skills by hand.

## When to use

- A skill was added, removed, or renamed (often right after `skill-linker`).
- A skill gained or lost a `scripts/`, `assets/`, or `config.json` bundle.
- A skill's bucket or one-line summary changed.
- You want to confirm CLAUDE.md is current without writing (`--check`).

## Config

`config.json` (bundled next to the skill, or pass a path as the argument) holds
everything the filesystem can't tell us. Paths may use `~`.

```json
{
  "skills_dir": ".claude/skills",
  "claude_md": ".claude/skills/CLAUDE.md",
  "tree_root_label": ".claude/skills/",
  "begin_marker": "<!-- BEGIN GENERATED: sync-skill-index -->",
  "end_marker": "<!-- END GENERATED: sync-skill-index -->",
  "buckets": [ { "name": "Utility", "tagline": "one small reusable thing, …" } ],
  "skills": {
    "skill-linker": { "bucket": "Utility", "summary": "syncs skill symlinks …" }
  }
}
```

Each skill needs one entry under `skills` giving its **bucket** (which the
filesystem can't derive) and an optional **summary** (the one-line description
in the bucket list). Omit `summary` to fall back to the first sentence of the
skill's own `description`.

## Run

```bash
uv run .claude/skills/sync-skill-index/scripts/sync_index.py                  # use bundled config.json, write CLAUDE.md
uv run .claude/skills/sync-skill-index/scripts/sync_index.py /path/cfg.json   # use a specific config
uv run .claude/skills/sync-skill-index/scripts/sync_index.py --check          # don't write; exit 1 if stale
```

## What it does

A **skill** is any immediate subdirectory of `skills_dir` containing a `SKILL.md`.

1. **Scan** — discover every skill, read its `name`/`description`, and detect
   which of `scripts/`, `assets/`, `config.json` it bundles.
2. **Tree** — render the column-aligned directory-tree block from that scan.
3. **Buckets** — group skills by the `config.json` map and render one summary
   line each, in the configured bucket order.
4. **Splice** — fill `assets/inventory.template.md` and replace only the text
   between the begin/end markers in CLAUDE.md.

**Safety:** only the region between the two markers is rewritten — the prose
sections of CLAUDE.md are never touched. A skill on disk but **missing from the
map is a hard error** (classify it first); a map entry with no skill on disk is
a warning and is skipped. `--check` writes nothing and exits non-zero when the
region is stale, so it can gate a commit.

## After running

The markers must already exist in CLAUDE.md. If they don't, the script prints
the two lines to add around the inventory region and exits.
