"""Apply one explicitly approved repository-provenance recovery transaction."""

from __future__ import annotations

import argparse
from dataclasses import dataclass
from datetime import datetime, timezone
import json
from pathlib import Path
import sqlite3
import sys
import uuid
from typing import Any

from preview_startup_recovery import (
    MEMBERSHIP_COLUMNS,
    REPOSITORY_COLUMNS,
    begin_read_snapshot,
    build_preview_from_connections,
    central_skill_ids,
    classify_provenance,
    database_health,
    membership_rows,
    memberships,
    open_read_only,
    relevant_state_marker,
    repository_rows,
)


OPERATION_LOG_COLUMNS = (
    "id",
    "created_at",
    "level",
    "target_kind",
    "target_id",
    "target_label",
    "category",
    "action",
    "status",
    "subject_type",
    "subject_id",
    "subject_label",
    "summary",
    "error_summary",
    "details_json",
    "duration_ms",
    "batch_id",
)


@dataclass(frozen=True)
class RecoveryExpectations:
    current_digest: str
    recovery_digest: str
    current_central_skills: int
    current_memberships: int
    recovery_memberships: int
    addable: int
    unresolved: int
    repositories_to_insert: int
    final_memberships: int
    final_populated_github_repositories: int


def require_columns(
    connection: sqlite3.Connection,
    table: str,
    required: tuple[str, ...],
) -> None:
    actual = {row[1] for row in connection.execute(f"PRAGMA table_info('{table}')")}
    missing = set(required) - actual
    if missing:
        raise RuntimeError(f"required columns are missing from {table}")


def validate_preview(
    preview: dict[str, Any],
    expected: RecoveryExpectations,
) -> None:
    healthy = {"quickCheck": ["ok"], "foreignKeyViolations": 0}
    if {
        key: preview["current"][key] for key in healthy
    } != healthy or {key: preview["recovery"][key] for key in healthy} != healthy:
        raise RuntimeError("database health precondition changed")
    if preview["current"]["snapshot"]["digest"] != expected.current_digest:
        raise RuntimeError("current database snapshot digest changed")
    if preview["recovery"]["snapshot"]["digest"] != expected.recovery_digest:
        raise RuntimeError("recovery database snapshot digest changed")
    if preview["current"]["centralSkills"] != expected.current_central_skills:
        raise RuntimeError("current Central skill count changed")
    if preview["current"]["memberships"] != expected.current_memberships:
        raise RuntimeError("current membership count changed")
    if preview["recovery"]["memberships"] != expected.recovery_memberships:
        raise RuntimeError("recovery membership count changed")

    classification = preview["classification"]
    exact_counts = {
        "addable": expected.addable,
        "alreadySame": 0,
        "conflict": 0,
        "missingParent": 0,
        "unresolved": expected.unresolved,
        "repositoriesToInsert": expected.repositories_to_insert,
        "repositoryConflicts": 0,
    }
    for key, value in exact_counts.items():
        if classification[key] != value:
            raise RuntimeError(f"recovery classification changed: {key}")
    if not preview["readyForApprovedApply"]:
        raise RuntimeError("recovery preview is not ready for the approved apply")


def validate_backup(
    backup_path: Path,
    expected: RecoveryExpectations,
) -> None:
    backup = open_read_only(backup_path)
    try:
        begin_read_snapshot(backup)
        if database_health(backup) != {
            "quickCheck": ["ok"],
            "foreignKeyViolations": 0,
        }:
            raise RuntimeError("verified backup health changed")
        if relevant_state_marker(backup)["digest"] != expected.current_digest:
            raise RuntimeError("verified backup does not match the approved current snapshot")
        if len(central_skill_ids(backup)) != expected.current_central_skills:
            raise RuntimeError("verified backup Central skill count changed")
        if len(memberships(backup)) != expected.current_memberships:
            raise RuntimeError("verified backup membership count changed")
        if backup.total_changes != 0:
            raise RuntimeError("backup validation unexpectedly changed the database")
    finally:
        if backup.in_transaction:
            backup.rollback()
        backup.close()


def open_read_write(path: Path) -> sqlite3.Connection:
    connection = sqlite3.connect(path, isolation_level=None, timeout=5.0)
    connection.row_factory = sqlite3.Row
    connection.execute("PRAGMA busy_timeout=5000")
    connection.execute("PRAGMA foreign_keys=ON")
    if connection.execute("PRAGMA foreign_keys").fetchone()[0] != 1:
        connection.close()
        raise RuntimeError("foreign key enforcement could not be enabled")
    return connection


def count_populated_github_repositories(connection: sqlite3.Connection) -> int:
    return connection.execute(
        "SELECT COUNT(DISTINCT m.repository_id) "
        "FROM skill_repository_members m "
        "JOIN skill_repositories r ON r.id = m.repository_id "
        "WHERE r.source_type = 'github' AND r.is_unknown = 0"
    ).fetchone()[0]


def count_unresolved_central_skills(connection: sqlite3.Connection) -> int:
    return connection.execute(
        "SELECT COUNT(*) FROM skills s "
        "LEFT JOIN skill_repository_members m ON m.skill_id = s.id "
        "WHERE s.is_central = 1 AND m.skill_id IS NULL"
    ).fetchone()[0]


def insert_audit_log(
    connection: sqlite3.Connection,
    memberships_restored: int,
    repositories_inserted: int,
    unresolved_preserved: int,
) -> str:
    audit_id = str(uuid.uuid4())
    created_at = (
        datetime.now(timezone.utc)
        .isoformat(timespec="milliseconds")
        .replace("+00:00", "Z")
    )
    details = json.dumps(
        {
            "membershipsRestored": memberships_restored,
            "repositoriesInserted": repositories_inserted,
            "source": "startup_recovery",
            "unresolvedSkillsPreserved": unresolved_preserved,
        },
        separators=(",", ":"),
        sort_keys=True,
    )
    values = (
        audit_id,
        created_at,
        "info",
        "local",
        "local",
        "Local",
        "maintenance",
        "database.provenance_restore",
        "succeeded",
        None,
        None,
        None,
        f"Restored {memberships_restored} repository provenance relationships.",
        None,
        details,
        None,
        None,
    )
    placeholders = ", ".join("?" for _ in OPERATION_LOG_COLUMNS)
    cursor = connection.execute(
        f"INSERT INTO operation_logs ({', '.join(OPERATION_LOG_COLUMNS)}) "
        f"VALUES ({placeholders})",
        values,
    )
    if cursor.rowcount != 1:
        raise RuntimeError("operation log audit row was not inserted exactly once")
    return audit_id


def verify_post_commit(
    current_path: Path,
    recovery_path: Path,
    backup_path: Path,
    expected: RecoveryExpectations,
    audit_id: str,
) -> dict[str, Any]:
    current = open_read_only(current_path)
    recovery = open_read_only(recovery_path)
    try:
        begin_read_snapshot(current)
        begin_read_snapshot(recovery)
        if database_health(current) != {
            "quickCheck": ["ok"],
            "foreignKeyViolations": 0,
        }:
            raise RuntimeError("post-commit current database health check failed")
        if database_health(recovery) != {
            "quickCheck": ["ok"],
            "foreignKeyViolations": 0,
        }:
            raise RuntimeError("post-commit recovery database health check failed")
        if relevant_state_marker(recovery)["digest"] != expected.recovery_digest:
            raise RuntimeError("recovery source changed during apply")
        final_memberships = len(memberships(current))
        populated_repositories = count_populated_github_repositories(current)
        unresolved = count_unresolved_central_skills(current)
        if final_memberships != expected.final_memberships:
            raise RuntimeError("post-commit membership count is not exact")
        if populated_repositories != expected.final_populated_github_repositories:
            raise RuntimeError("post-commit populated GitHub repository count is not exact")
        if unresolved != expected.unresolved:
            raise RuntimeError("post-commit unresolved skill count changed")
        audit = current.execute(
            "SELECT action, status, details_json FROM operation_logs WHERE id = ?",
            (audit_id,),
        ).fetchone()
        if audit is None or audit[0:2] != (
            "database.provenance_restore",
            "succeeded",
        ):
            raise RuntimeError("post-commit operation log audit row is missing")
        result = {
            "memberships": final_memberships,
            "populatedGithubRepositories": populated_repositories,
            "unresolved": unresolved,
            "quickCheck": "ok",
            "foreignKeyViolations": 0,
            "operationLogRecorded": True,
            "postSnapshot": relevant_state_marker(current),
            "recoverySnapshotUnchanged": True,
        }
    finally:
        if current.in_transaction:
            current.rollback()
        if recovery.in_transaction:
            recovery.rollback()
        current.close()
        recovery.close()
    validate_backup(backup_path, expected)
    return result


def apply_recovery(
    current_path: Path,
    recovery_path: Path,
    backup_path: Path,
    expected: RecoveryExpectations,
) -> dict[str, Any]:
    resolved_paths = {
        current_path.resolve(),
        recovery_path.resolve(),
        backup_path.resolve(),
    }
    if len(resolved_paths) != 3:
        raise RuntimeError("current, recovery, and backup databases must be distinct")
    validate_backup(backup_path, expected)

    current = open_read_write(current_path)
    recovery = open_read_only(recovery_path)
    committed = False
    audit_id = ""
    try:
        require_columns(current, "skill_repositories", REPOSITORY_COLUMNS)
        require_columns(recovery, "skill_repositories", REPOSITORY_COLUMNS)
        require_columns(current, "skill_repository_members", MEMBERSHIP_COLUMNS)
        require_columns(recovery, "skill_repository_members", MEMBERSHIP_COLUMNS)
        require_columns(current, "operation_logs", OPERATION_LOG_COLUMNS)
        current.execute("BEGIN IMMEDIATE")
        begin_read_snapshot(recovery)

        preview = build_preview_from_connections(current, recovery)
        validate_preview(preview, expected)
        classified = classify_provenance(current, recovery)
        original_memberships = membership_rows(current)
        recovery_memberships = membership_rows(recovery)
        current_repositories = repository_rows(current)
        recovery_repositories = repository_rows(recovery)

        repository_rows_to_insert = [
            recovery_repositories[repository_id]
            for repository_id in classified["repositoriesToInsert"]
        ]
        repository_placeholders = ", ".join("?" for _ in REPOSITORY_COLUMNS)
        repository_cursor = current.executemany(
            f"INSERT INTO skill_repositories ({', '.join(REPOSITORY_COLUMNS)}) "
            f"VALUES ({repository_placeholders})",
            repository_rows_to_insert,
        )
        if repository_cursor.rowcount != expected.repositories_to_insert:
            raise RuntimeError("repository insert count is not exact")

        membership_rows_to_insert = [
            recovery_memberships[skill_id] for skill_id in classified["addable"]
        ]
        membership_placeholders = ", ".join("?" for _ in MEMBERSHIP_COLUMNS)
        membership_cursor = current.executemany(
            f"INSERT INTO skill_repository_members ({', '.join(MEMBERSHIP_COLUMNS)}) "
            f"VALUES ({membership_placeholders})",
            membership_rows_to_insert,
        )
        if membership_cursor.rowcount != expected.addable:
            raise RuntimeError("membership insert count is not exact")

        final_rows = membership_rows(current)
        if any(final_rows.get(skill_id) != row for skill_id, row in original_memberships.items()):
            raise RuntimeError("an existing membership changed during recovery")
        if len(final_rows) != expected.final_memberships:
            raise RuntimeError("transaction membership count is not exact")
        if count_populated_github_repositories(current) != (
            expected.final_populated_github_repositories
        ):
            raise RuntimeError("transaction populated GitHub repository count is not exact")
        if count_unresolved_central_skills(current) != expected.unresolved:
            raise RuntimeError("transaction unresolved skill count changed")
        if database_health(current) != {
            "quickCheck": ["ok"],
            "foreignKeyViolations": 0,
        }:
            raise RuntimeError("transaction database health check failed")

        audit_id = insert_audit_log(
            current,
            memberships_restored=expected.addable,
            repositories_inserted=expected.repositories_to_insert,
            unresolved_preserved=expected.unresolved,
        )
        current.commit()
        committed = True
    except Exception:
        if current.in_transaction:
            current.rollback()
        raise
    finally:
        if recovery.in_transaction:
            recovery.rollback()
        recovery.close()
        current.close()

    if not committed:
        raise RuntimeError("recovery transaction did not commit")
    post = verify_post_commit(
        current_path,
        recovery_path,
        backup_path,
        expected,
        audit_id,
    )
    return {
        "status": "applied",
        "membershipsRestored": expected.addable,
        "repositoriesInserted": expected.repositories_to_insert,
        **post,
    }


def require_backup_companions(backup_path: Path) -> None:
    for suffix in ("-wal", "-shm"):
        if not Path(f"{backup_path}{suffix}").is_file():
            raise RuntimeError("verified backup database set is incomplete")


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Apply the explicitly approved startup provenance recovery."
    )
    parser.add_argument("--current", type=Path, required=True)
    parser.add_argument("--recovery", type=Path, required=True)
    parser.add_argument("--verified-backup", type=Path, required=True)
    parser.add_argument("--expected-current-digest", required=True)
    parser.add_argument("--expected-recovery-digest", required=True)
    parser.add_argument("--expected-current-central-skills", type=int, default=141)
    parser.add_argument("--expected-current-memberships", type=int, default=7)
    parser.add_argument("--expected-recovery-memberships", type=int, default=111)
    parser.add_argument("--expected-addable", type=int, default=111)
    parser.add_argument("--expected-unresolved", type=int, default=23)
    parser.add_argument("--expected-repositories-to-insert", type=int, default=23)
    parser.add_argument("--expected-final-memberships", type=int, default=118)
    parser.add_argument(
        "--expected-final-populated-github-repositories", type=int, default=24
    )
    args = parser.parse_args()
    expected = RecoveryExpectations(
        current_digest=args.expected_current_digest,
        recovery_digest=args.expected_recovery_digest,
        current_central_skills=args.expected_current_central_skills,
        current_memberships=args.expected_current_memberships,
        recovery_memberships=args.expected_recovery_memberships,
        addable=args.expected_addable,
        unresolved=args.expected_unresolved,
        repositories_to_insert=args.expected_repositories_to_insert,
        final_memberships=args.expected_final_memberships,
        final_populated_github_repositories=(
            args.expected_final_populated_github_repositories
        ),
    )
    try:
        require_backup_companions(args.verified_backup)
        result = apply_recovery(
            args.current,
            args.recovery,
            args.verified_backup,
            expected,
        )
    except (RuntimeError, ValueError) as error:
        print(f"Recovery apply aborted: {error}", file=sys.stderr)
        return 1
    except sqlite3.Error:
        print("Recovery apply aborted because SQLite rejected the operation.", file=sys.stderr)
        return 1
    print(json.dumps(result, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
