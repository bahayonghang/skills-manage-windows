"""Read-only preview for repository provenance in a startup recovery database."""

from __future__ import annotations

import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path
import sqlite3
from typing import Any


REPOSITORY_COLUMNS = (
    "id",
    "name",
    "source_type",
    "owner",
    "repo",
    "branch",
    "url",
    "pinned",
    "is_unknown",
    "created_at",
    "updated_at",
    "last_synced_at",
)
MEMBERSHIP_COLUMNS = (
    "skill_id",
    "repository_id",
    "source_path",
    "added_at",
    "updated_at",
    "resolved_commit_sha",
    "content_digest",
)


def open_read_only(path: Path) -> sqlite3.Connection:
    connection = sqlite3.connect(f"{path.resolve().as_uri()}?mode=ro", uri=True)
    connection.row_factory = sqlite3.Row
    connection.execute("PRAGMA query_only=ON")
    return connection


def begin_read_snapshot(connection: sqlite3.Connection) -> None:
    connection.execute("BEGIN")
    # BEGIN is deferred in SQLite. This read pins the WAL snapshot before any
    # health, classification, or digest query can observe a later commit.
    connection.execute("SELECT COUNT(*) FROM sqlite_schema").fetchone()


def database_health(connection: sqlite3.Connection) -> dict[str, Any]:
    quick_check = [row[0] for row in connection.execute("PRAGMA quick_check")]
    foreign_key_violations = connection.execute(
        "SELECT COUNT(*) FROM pragma_foreign_key_check"
    ).fetchone()[0]
    return {
        "quickCheck": quick_check,
        "foreignKeyViolations": foreign_key_violations,
    }


def central_skill_ids(connection: sqlite3.Connection) -> set[str]:
    return {
        row[0]
        for row in connection.execute("SELECT id FROM skills WHERE is_central = 1")
    }


def memberships(connection: sqlite3.Connection) -> dict[str, tuple[str, str | None]]:
    return {
        row[0]: (row[1], row[2])
        for row in connection.execute(
            "SELECT skill_id, repository_id, source_path "
            "FROM skill_repository_members"
        )
    }


def repository_rows(connection: sqlite3.Connection) -> dict[str, tuple[Any, ...]]:
    columns = ", ".join(REPOSITORY_COLUMNS)
    return {
        row[0]: tuple(row)
        for row in connection.execute(
            f"SELECT {columns} FROM skill_repositories ORDER BY id"
        )
    }


def membership_rows(connection: sqlite3.Connection) -> dict[str, tuple[Any, ...]]:
    columns = ", ".join(MEMBERSHIP_COLUMNS)
    return {
        row[0]: tuple(row)
        for row in connection.execute(
            f"SELECT {columns} FROM skill_repository_members ORDER BY skill_id"
        )
    }


def classify_provenance(
    current: sqlite3.Connection,
    recovery: sqlite3.Connection,
) -> dict[str, Any]:
    current_skills = central_skill_ids(current)
    recovery_skills = central_skill_ids(recovery)
    current_members = memberships(current)
    recovery_members = memberships(recovery)

    addable: list[str] = []
    already_same: list[str] = []
    conflict: list[str] = []
    missing_parent: list[str] = []
    for skill_id, assignment in recovery_members.items():
        if skill_id not in current_skills:
            missing_parent.append(skill_id)
        elif skill_id not in current_members:
            addable.append(skill_id)
        elif current_members[skill_id] == assignment:
            already_same.append(skill_id)
        else:
            conflict.append(skill_id)

    unresolved = sorted(current_skills - current_members.keys() - recovery_members.keys())
    referenced_repository_ids = {
        recovery_members[skill_id][0] for skill_id in addable
    }
    current_repositories = repository_rows(current)
    recovery_repositories = repository_rows(recovery)
    missing_recovery_repositories = sorted(
        referenced_repository_ids - recovery_repositories.keys()
    )
    if missing_recovery_repositories:
        raise RuntimeError("recovery memberships reference missing repository metadata")
    repositories_to_insert = sorted(
        referenced_repository_ids - current_repositories.keys()
    )
    repository_conflicts = sorted(
        repository_id
        for repository_id in referenced_repository_ids & current_repositories.keys()
        if current_repositories[repository_id] != recovery_repositories[repository_id]
    )

    return {
        "currentSkills": current_skills,
        "recoverySkills": recovery_skills,
        "currentMembers": current_members,
        "recoveryMembers": recovery_members,
        "addable": sorted(addable),
        "alreadySame": sorted(already_same),
        "conflict": sorted(conflict),
        "missingParent": sorted(missing_parent),
        "unresolved": unresolved,
        "repositoriesToInsert": repositories_to_insert,
        "repositoryConflicts": repository_conflicts,
    }


def relevant_state_marker(connection: sqlite3.Connection) -> dict[str, str]:
    state = {
        "schemaMigrations": [
            list(row)
            for row in connection.execute(
                "SELECT version, checksum, applied_at "
                "FROM schema_migrations ORDER BY version"
            )
        ],
        "centralSkillIds": sorted(central_skill_ids(connection)),
        "repositories": [
            list(row)
            for row in connection.execute(
                f"SELECT {', '.join(REPOSITORY_COLUMNS)} "
                "FROM skill_repositories ORDER BY id"
            )
        ],
        "memberships": [
            list(row)
            for row in connection.execute(
                f"SELECT {', '.join(MEMBERSHIP_COLUMNS)} "
                "FROM skill_repository_members ORDER BY skill_id"
            )
        ],
    }
    encoded = json.dumps(
        state,
        ensure_ascii=False,
        separators=(",", ":"),
        sort_keys=True,
    ).encode("utf-8")
    return {"algorithm": "sha256", "digest": hashlib.sha256(encoded).hexdigest()}


def build_preview_from_connections(
    current: sqlite3.Connection,
    recovery: sqlite3.Connection,
) -> dict[str, Any]:
    current_health = database_health(current)
    recovery_health = database_health(recovery)
    classified = classify_provenance(current, recovery)
    current_members = classified["currentMembers"]
    recovery_members = classified["recoveryMembers"]
    addable = classified["addable"]
    addable_by_repository = Counter(
        recovery_members[skill_id][0] for skill_id in addable
    )
    current_repository_count = len(
        {repository_id for repository_id, _ in current_members.values()}
    )
    recovery_repository_count = len(
        {repository_id for repository_id, _ in recovery_members.values()}
    )
    preview = {
        "current": {
            **current_health,
            "snapshot": relevant_state_marker(current),
            "centralSkills": len(classified["currentSkills"]),
            "memberships": len(current_members),
            "repositoriesWithMembers": current_repository_count,
        },
        "recovery": {
            **recovery_health,
            "snapshot": relevant_state_marker(recovery),
            "centralSkills": len(classified["recoverySkills"]),
            "memberships": len(recovery_members),
            "repositoriesWithMembers": recovery_repository_count,
        },
        "classification": {
            "addable": len(addable),
            "alreadySame": len(classified["alreadySame"]),
            "conflict": len(classified["conflict"]),
            "missingParent": len(classified["missingParent"]),
            "unresolved": len(classified["unresolved"]),
            "repositoriesToInsert": len(classified["repositoriesToInsert"]),
            "repositoryConflicts": len(classified["repositoryConflicts"]),
            "addableByRepository": dict(sorted(addable_by_repository.items())),
        },
    }
    preview["readyForApprovedApply"] = (
        current_health == {"quickCheck": ["ok"], "foreignKeyViolations": 0}
        and recovery_health
        == {"quickCheck": ["ok"], "foreignKeyViolations": 0}
        and not classified["conflict"]
        and not classified["missingParent"]
        and not classified["repositoryConflicts"]
        and bool(addable)
    )
    return preview


def build_preview(current_path: Path, recovery_path: Path) -> dict[str, Any]:
    current = open_read_only(current_path)
    recovery = open_read_only(recovery_path)
    try:
        begin_read_snapshot(current)
        begin_read_snapshot(recovery)
        preview = build_preview_from_connections(current, recovery)
        if current.total_changes != 0 or recovery.total_changes != 0:
            raise RuntimeError("read-only preview unexpectedly changed a database")
        return preview
    finally:
        if current.in_transaction:
            current.rollback()
        if recovery.in_transaction:
            recovery.rollback()
        current.close()
        recovery.close()


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Preview startup-recovery repository provenance without writes."
    )
    parser.add_argument("--current", type=Path, required=True)
    parser.add_argument("--recovery", type=Path, required=True)
    args = parser.parse_args()

    preview = build_preview(args.current, args.recovery)
    print(json.dumps(preview, indent=2, sort_keys=True))
    return 0 if preview["readyForApprovedApply"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
