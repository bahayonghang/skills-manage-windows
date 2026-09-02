"""List production @/types barrel importers for a git tree-ish or the worktree."""

from __future__ import annotations

import re
import subprocess
import sys
from pathlib import Path

PATTERN = re.compile(r"""from\s+['"]@/types['"]""")
ROOT = Path(__file__).resolve().parents[4]


def from_git(rev: str) -> list[str]:
    raw = subprocess.check_output(
        ["git", "grep", "-l", "-E", r"""from\s+['"]@/types['"]""", rev, "--", "src"],
        cwd=ROOT,
        text=True,
        encoding="utf-8",
    )
    paths: set[str] = set()
    prefix = f"{rev}:"
    for line in raw.splitlines():
        path = line[len(prefix) :] if line.startswith(prefix) else line
        path = path.replace("\\", "/")
        if path.startswith("src/test/") or path.startswith("src/types/"):
            continue
        if not (path.endswith(".ts") or path.endswith(".tsx")):
            continue
        paths.add(path)
    return sorted(paths)


def from_worktree() -> list[str]:
    src = ROOT / "src"
    skip_roots = {(src / "test").resolve(), (src / "types").resolve()}
    paths: list[str] = []
    for file in src.rglob("*"):
        if not file.is_file() or file.suffix not in {".ts", ".tsx"}:
            continue
        resolved = file.resolve()
        if any(resolved == skip or skip in resolved.parents for skip in skip_roots):
            continue
        text = file.read_text(encoding="utf-8")
        if PATTERN.search(text):
            paths.append(file.relative_to(ROOT).as_posix())
    return sorted(set(paths))


def main() -> None:
    mode = sys.argv[1] if len(sys.argv) > 1 else "HEAD"
    paths = from_worktree() if mode == "worktree" else from_git(mode)
    for path in paths:
        print(path)
    print(f"COUNT {len(paths)}")


if __name__ == "__main__":
    main()
