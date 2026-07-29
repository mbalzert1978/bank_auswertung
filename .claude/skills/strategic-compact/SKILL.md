---
name: strategic-compact
description: Suggests manual context compaction at logical intervals to preserve context through task phases rather than arbitrary auto-compaction.
origin: ECC
---

# Strategic Compact Skill

Suggests manual `/compact` at strategic points in your workflow rather than relying on arbitrary auto-compaction.

## When to Activate

- Running long sessions that approach context limits (200K+ tokens)
- Working on multi-phase tasks (research → plan → implement → test)
- Switching between unrelated tasks within the same session
- After completing a major milestone and starting new work
- When responses slow down or become less coherent (context pressure)

## Why Strategic Compaction?

Auto-compaction triggers at arbitrary points:
- Often mid-task, losing important context
- No awareness of logical task boundaries
- Can interrupt complex multi-step operations

Strategic compaction at logical boundaries:
- **After exploration, before execution** — Compact research context, keep implementation plan
- **After completing a milestone** — Fresh start for next phase
- **Before major context shifts** — Clear exploration context before different task

## How It Works

The bundled `scripts/suggest_compact.py` script runs on PreToolUse (Edit/Write) and combines two signals:

1. **Context size (primary)** — Reads the latest `usage` record from the session transcript (`transcript_path` in the hook payload) and sums `input_tokens + cache_read_input_tokens + cache_creation_input_tokens` (the true context size of the turn). Suggests `/compact` at a window-scaled threshold — 160k tokens on a 200k window, 250k on a 1M window (detected from a `[1m]` model marker, or inferred when observed tokens already exceed 200k) — and re-reminds after every additional 60k tokens of context growth
2. **Tool-call count (secondary)** — Counts tool invocations in session; suggests at a configurable threshold (default: 50 calls), then every 25 calls after

Tool count alone is a weak proxy for window pressure: a few large file reads or MCP responses can fill the window in very few calls, while many tiny calls can cross 50 with a near-empty window. The context-size signal fires when it actually matters.

## Hook Setup

Add to your `~/.claude/settings.json`:

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Edit",
        "hooks": [{ "type": "command", "command": "python3 .claude/skills/strategic-compact/scripts/suggest_compact.py" }]
      },
      {
        "matcher": "Write",
        "hooks": [{ "type": "command", "command": "python3 .claude/skills/strategic-compact/scripts/suggest_compact.py" }]
      }
    ]
  }
}
```

## Configuration

The numeric defaults live in the sibling `config.json`, so you can tune them without editing the script:

| `config.json` key | Default | Meaning |
|-------------------|---------|---------|
| `compact_threshold` | 50 | Tool calls before the first tool-count suggestion |
| `compact_context_threshold_200k` | 160000 | Context-token threshold on a 200k window |
| `compact_context_threshold_1m` | 250000 | Context-token threshold on a 1M window |
| `compact_context_interval` | 60000 | Additional context tokens before the context suggestion repeats |
| `compact_state_ttl_days` | 14 | Days before stale per-session state files in the temp dir are swept |
| `compact_tool_repeat_interval` | 25 | Tool calls between repeat tool-count suggestions after the first |

The script loads `config.json` with the standard library only and falls back to these built-in defaults if the file is missing or unreadable.

Each setting can also be overridden per session via an environment variable, which takes precedence over `config.json` (env var > `config.json` > built-in fallback):
- `COMPACT_THRESHOLD` — overrides `compact_threshold`
- `COMPACT_CONTEXT_THRESHOLD` — overrides the window-scaled context threshold (`0` disables the context signal)
- `COMPACT_CONTEXT_INTERVAL` — overrides `compact_context_interval`
- `COMPACT_STATE_TTL_DAYS` — overrides `compact_state_ttl_days`

## Compaction Decision Guide

Use this table to decide when to compact:

| Phase Transition | Compact? | Why |
|-----------------|----------|-----|
| Research → Planning | Yes | Research context is bulky; plan is the distilled output |
| Planning → Implementation | Yes | Plan is in TodoWrite or a file; free up context for code |
| Implementation → Testing | Maybe | Keep if tests reference recent code; compact if switching focus |
| Debugging → Next feature | Yes | Debug traces pollute context for unrelated work |
| Mid-implementation | No | Losing variable names, file paths, and partial state is costly |
| After a failed approach | Yes | Clear the dead-end reasoning before trying a new approach |

## What Survives Compaction

Understanding what persists helps you compact with confidence:

| Persists | Lost |
|----------|------|
| CLAUDE.md instructions | Intermediate reasoning and analysis |
| TodoWrite task list | File contents you previously read |
| Memory files (`~/.claude/memory/`) | Multi-step conversation context |
| Git state (commits, branches) | Tool call history and counts |
| Files on disk | Nuanced user preferences stated verbally |

## Best Practices

1. **Compact after planning** — Once plan is finalized in TodoWrite, compact to start fresh
2. **Compact after debugging** — Clear error-resolution context before continuing
3. **Don't compact mid-implementation** — Preserve context for related changes
4. **Read the suggestion** — The hook tells you *when*, you decide *if*
5. **Write before compacting** — Save important context to files or memory before compacting
6. **Use `/compact` with a summary** — Add a custom message: `/compact Focus on implementing auth middleware next`

## Related

- Memory persistence hooks — For state that survives compaction
- `continuous-learning` skill — Extracts patterns before session ends