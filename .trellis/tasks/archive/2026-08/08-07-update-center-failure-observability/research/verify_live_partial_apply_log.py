from __future__ import annotations

import argparse
import json
import os
import sqlite3
from collections import Counter
from pathlib import Path


def default_database() -> Path:
    return Path(os.environ["USERPROFILE"]) / ".skillsmanage" / "db.sqlite"


def normalized_path(value: str) -> str:
    return os.path.normcase(os.path.normpath(value))


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Read-only probe for partial Update Center applies logged as success."
    )
    parser.add_argument("--db", type=Path, default=default_database())
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
    safe_details = {
        key: details.get(key)
        for key in (
            "updates",
            "deleteMissing",
            "failures",
            "updated",
            "deleted",
            "imported",
        )
        if key in details
    }
    print(
        {
            "created_at": latest["created_at"],
            "status": latest["status"],
            "details": safe_details,
        }
    )

    pending = connection.execute(
        """
        SELECT operation_kind, skill_id, phase, last_error_code, manifest_json
        FROM fs_db_operations
        WHERE phase NOT IN ('completed', 'rolled_back')
        ORDER BY updated_at DESC
        LIMIT 1
        """
    ).fetchone()
    if pending is not None:
        manifest = json.loads(pending["manifest_json"])
        payload = manifest.get("payload", {})
        paths = payload.get("paths", [])
        originals = Counter(normalized_path(item["original"]) for item in paths)
        duplicate_counts = sorted(count for count in originals.values() if count > 1)
        state_counts = Counter()
        for item in paths:
            original_exists = os.path.lexists(item["original"])
            backup_exists = os.path.lexists(item["backup"])
            state_counts[(original_exists, backup_exists)] += 1
        print(
            {
                "pending": {
                    "operation_kind": pending["operation_kind"],
                    "skill_id": pending["skill_id"],
                    "phase": pending["phase"],
                    "last_error_code": pending["last_error_code"],
                    "path_count": len(paths),
                    "duplicate_group_sizes": duplicate_counts,
                    "path_state_counts": {
                        "original_and_backup": state_counts[(True, True)],
                        "original_only": state_counts[(True, False)],
                        "backup_only": state_counts[(False, True)],
                        "neither": state_counts[(False, False)],
                    },
                }
            }
        )

    failure_count = int(details.get("failures") or 0)
    if failure_count == 0:
        print("INCONCLUSIVE: latest apply has no item failures")
        return 2
    if latest["status"] == "succeeded":
        print("RED: item failures were persisted as a successful operation")
        return 1

    print("GREEN: item failures are not persisted as a successful operation")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
