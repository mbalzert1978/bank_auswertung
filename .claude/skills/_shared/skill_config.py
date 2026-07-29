"""Shared config.json loader for scripts under .claude/skills/*/scripts/.

Every skill's config.json is optional and best-effort: a missing or malformed
file falls back to {} so callers supply their own per-key default via
.get(key, default) instead of duplicating this try/except per skill.
"""

import json
from pathlib import Path


def load_config(path: Path) -> dict:
    try:
        return json.loads(path.read_text())
    except (OSError, ValueError):
        return {}
