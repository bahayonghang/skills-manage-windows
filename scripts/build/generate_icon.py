#!/usr/bin/env python3
"""Describe how to rebuild SkillPort platform icons.

The tracked master is `src-tauri/icons/icon-source.png` (B2 routing hub).
This script does not draw a new icon and does not overwrite that file.

Rebuild platform sizes with:

    pnpm tauri icon src-tauri/icons/icon-source.png --ios-color "#1e1e2e"
"""

from __future__ import annotations

import json
import sys
from pathlib import Path


def resolve_repo_root(start: Path) -> Path:
    current = start.resolve().parent
    while True:
        package_json = current / "package.json"
        if package_json.is_file():
            try:
                payload = json.loads(package_json.read_text(encoding="utf-8"))
            except (OSError, json.JSONDecodeError):
                payload = None
            if isinstance(payload, dict) and payload.get("name") == "skillport":
                return current
        parent = current.parent
        if parent == current:
            raise SystemExit(
                'Could not find repository root. Expected a package.json with name "skillport".'
            )
        current = parent


ROOT = resolve_repo_root(Path(__file__))
MASTER = ROOT / "src-tauri" / "icons" / "icon-source.png"


def main() -> int:
    if not MASTER.is_file():
        print(f"Missing tracked icon master: {MASTER}", file=sys.stderr)
        return 1
    print(f"Tracked icon master: {MASTER.relative_to(ROOT)}")
    print(
        'Rebuild platform icons with: '
        'pnpm tauri icon src-tauri/icons/icon-source.png --ios-color "#1e1e2e"'
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
