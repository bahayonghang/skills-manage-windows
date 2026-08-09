from __future__ import annotations

import argparse
import json
import os
import re
import sqlite3
from pathlib import Path


def default_database() -> Path:
    return Path(os.environ["USERPROFILE"]) / ".skillsmanage" / "db.sqlite"


def default_runtime_log() -> Path:
    return (
        Path(os.environ["USERPROFILE"])
        / ".skillsmanage"
        / "logs"
        / "skillport-2026-08-09.log"
    )


def latest_runtime_apply_fields(path: Path) -> set[str]:
    if not path.exists():
        return set()

    latest = ""
    for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
        if 'action="update_center.apply"' in line:
            latest = line

    return set(re.findall(r"\b([a-z][a-z0-9_]*)=", latest))


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Read-only probe for actionable Update Center apply diagnostics."
    )
    parser.add_argument("--db", type=Path, default=default_database())
    parser.add_argument("--runtime-log", type=Path, default=default_runtime_log())
    args = parser.parse_args()

    uri = f"file:{args.db.resolve().as_posix()}?mode=ro"
    connection = sqlite3.connect(uri, uri=True)
    connection.row_factory = sqlite3.Row
    latest = connection.execute(
        """
        SELECT created_at, status, details_json
        FROM operation_logs
        WHERE action = 'update_center.apply'
        ORDER BY created_at DESC
        LIMIT 1
        """
    ).fetchone()

    if latest is None:
        print("INCONCLUSIVE: no update_center.apply row exists")
        return 2

    details = json.loads(latest["details_json"] or "{}")
    failures = int(details.get("failures") or 0)
    safe_item_diagnostics = details.get("failureItems")
    stable_codes = sorted(set(details.get("failureCodes") or []))
    stable_categories = sorted(set(details.get("failureCategories") or []))
    runtime_fields = sorted(latest_runtime_apply_fields(args.runtime_log))

    print(
        {
            "created_at": latest["created_at"],
            "status": latest["status"],
            "failures": failures,
            "failure_codes": stable_codes,
            "failure_categories": stable_categories,
            "failure_item_count": (
                len(safe_item_diagnostics)
                if isinstance(safe_item_diagnostics, list)
                else 0
            ),
            "runtime_fields": runtime_fields,
        }
    )

    if failures == 0:
        print("INCONCLUSIVE: latest apply has no item failures")
        return 2

    required_runtime_fields = {
        "failure_categories",
        "failure_codes",
        "failure_count",
        "phase_counts",
        "success_count",
    }
    item_diagnostics_complete = (
        isinstance(safe_item_diagnostics, list)
        and len(safe_item_diagnostics) == failures
        and all(
            isinstance(item, dict)
            and all(
                isinstance(item.get(key), str) and item[key]
                for key in ("step", "identifier", "errorCode", "errorCategory", "phase")
            )
            for item in safe_item_diagnostics
        )
    )
    runtime_diagnostics_complete = required_runtime_fields.issubset(runtime_fields)

    if not item_diagnostics_complete or not runtime_diagnostics_complete:
        print(
            "RED: failed update items do not retain bounded per-item and runtime "
            "diagnostic context"
        )
        return 1

    print("GREEN: failed update items retain actionable, bounded diagnostics")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
