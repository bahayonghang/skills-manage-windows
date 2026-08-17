#!/usr/bin/env python3
"""Describe how to rebuild SkillPort platform icons.

The tracked master is `src-tauri/icons/icon-source.png` (B2 routing hub).
This script does not draw a new icon and does not overwrite that file.

Rebuild platform sizes with:

    pnpm tauri icon src-tauri/icons/icon-source.png --ios-color "#1e1e2e"
"""

from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
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
