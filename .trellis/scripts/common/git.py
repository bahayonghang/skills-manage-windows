"""
Git command execution utility.

Single source of truth for running git commands across all Trellis scripts.
"""

from __future__ import annotations

import subprocess
from pathlib import Path


def run_git(
    args: list[str],
    cwd: Path | None = None,
    timeout: float | None = None,
) -> tuple[int, str, str]:
    """Run a git command and return (returncode, stdout, stderr).

    Uses UTF-8 encoding with -c i18n.logOutputEncoding=UTF-8 to ensure
    consistent output across all platforms (Windows, macOS, Linux). Callers
    may provide a timeout for best-effort probes; normal Git operations remain
    unbounded by default.
    """
    try:
        git_args = ["git", "-c", "i18n.logOutputEncoding=UTF-8"] + args
        result = subprocess.run(
            git_args,
            cwd=cwd,
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            timeout=timeout,
        )
        return result.returncode, result.stdout, result.stderr
    except Exception as e:
        return 1, "", str(e)


def resolve_default_branch(repo_root: Path) -> str | None:
    """Resolve the repository's default branch from local refs/config only.

    Reads ``refs/remotes/origin/HEAD`` then repo-local ``init.defaultBranch``.
    Does not call ``git remote show`` or any other network Git command.
    Returns None when neither local source resolves so callers can use an
    explicit ``--base-branch`` or the checked-out branch fallback.
    """
    rc, out, _ = run_git(
        ["symbolic-ref", "--quiet", "refs/remotes/origin/HEAD"],
        cwd=repo_root,
        timeout=5,
    )
    if rc == 0 and out.strip():
        return out.strip().rsplit("/", 1)[-1]

    rc, out, _ = run_git(
        ["config", "--local", "--get", "init.defaultBranch"],
        cwd=repo_root,
        timeout=5,
    )
    if rc == 0 and out.strip():
        return out.strip()

    return None


def branch_exists_locally(branch: str, repo_root: Path) -> bool:
    """Check whether a local branch ref exists in the repository."""
    if not branch:
        return False
    rc, _, _ = run_git(
        ["rev-parse", "--verify", "--quiet", f"refs/heads/{branch}"],
        cwd=repo_root,
    )
    return rc == 0
