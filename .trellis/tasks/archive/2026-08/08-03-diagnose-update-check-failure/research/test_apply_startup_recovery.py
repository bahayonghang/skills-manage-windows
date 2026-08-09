from pathlib import Path
import shutil
import sqlite3
import tempfile
import unittest

from apply_startup_recovery import RecoveryExpectations, apply_recovery
from preview_startup_recovery import build_preview, membership_rows
from test_preview_startup_recovery import create_database


def expectations_from_preview(preview: dict) -> RecoveryExpectations:
    return RecoveryExpectations(
        current_digest=preview["current"]["snapshot"]["digest"],
        recovery_digest=preview["recovery"]["snapshot"]["digest"],
        current_central_skills=preview["current"]["centralSkills"],
        current_memberships=preview["current"]["memberships"],
        recovery_memberships=preview["recovery"]["memberships"],
        addable=preview["classification"]["addable"],
        unresolved=preview["classification"]["unresolved"],
        repositories_to_insert=preview["classification"]["repositoriesToInsert"],
        final_memberships=(
            preview["current"]["memberships"]
            + preview["classification"]["addable"]
        ),
        final_populated_github_repositories=2,
    )


class ApplyStartupRecoveryTests(unittest.TestCase):
    def test_apply_restores_exact_rows_and_records_redacted_audit(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            current = root / "current.sqlite"
            recovery = root / "recovery.sqlite"
            backup = root / "backup.sqlite"
            create_database(
                current,
                ["addable", "existing", "unresolved"],
                {"existing": ("repo-existing", "skills/existing")},
            )
            create_database(
                recovery,
                ["addable"],
                {"addable": ("repo-add", "skills/addable")},
            )
            preview = build_preview(current, recovery)
            expected = expectations_from_preview(preview)
            before_current = sqlite3.connect(current)
            before_current.row_factory = sqlite3.Row
            existing_row = membership_rows(before_current)["existing"]
            before_current.close()
            shutil.copy2(current, backup)
            backup_bytes = backup.read_bytes()

            result = apply_recovery(current, recovery, backup, expected)

            self.assertEqual(result["status"], "applied")
            self.assertEqual(result["membershipsRestored"], 1)
            self.assertEqual(result["repositoriesInserted"], 1)
            connection = sqlite3.connect(current)
            connection.row_factory = sqlite3.Row
            rows = membership_rows(connection)
            self.assertEqual(rows["existing"], existing_row)
            self.assertEqual(rows["addable"][1:3], ("repo-add", "skills/addable"))
            audit = connection.execute(
                "SELECT action, status, details_json FROM operation_logs"
            ).fetchone()
            self.assertEqual(audit["action"], "database.provenance_restore")
            self.assertEqual(audit["status"], "succeeded")
            self.assertEqual(
                audit["details_json"],
                '{"membershipsRestored":1,"repositoriesInserted":1,'
                '"source":"startup_recovery","unresolvedSkillsPreserved":1}',
            )
            connection.close()
            self.assertEqual(backup.read_bytes(), backup_bytes)

    def test_snapshot_drift_aborts_without_writes(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            current = root / "current.sqlite"
            recovery = root / "recovery.sqlite"
            backup = root / "backup.sqlite"
            create_database(current, ["addable", "unresolved"], {})
            create_database(
                recovery,
                ["addable"],
                {"addable": ("repo-add", "skills/addable")},
            )
            preview = build_preview(current, recovery)
            expected = expectations_from_preview(preview)
            expected = RecoveryExpectations(
                **{
                    **expected.__dict__,
                    "final_populated_github_repositories": 1,
                }
            )
            shutil.copy2(current, backup)
            writer = sqlite3.connect(current)
            writer.execute(
                "UPDATE schema_migrations SET applied_at = '2026-08-03T01:00:00Z'"
            )
            writer.commit()
            writer.close()

            with self.assertRaisesRegex(RuntimeError, "snapshot digest changed"):
                apply_recovery(current, recovery, backup, expected)

            connection = sqlite3.connect(current)
            self.assertEqual(
                connection.execute(
                    "SELECT COUNT(*) FROM skill_repository_members"
                ).fetchone()[0],
                0,
            )
            self.assertEqual(
                connection.execute("SELECT COUNT(*) FROM operation_logs").fetchone()[0],
                0,
            )
            connection.close()

    def test_repository_metadata_conflict_aborts_without_writes(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            current = root / "current.sqlite"
            recovery = root / "recovery.sqlite"
            backup = root / "backup.sqlite"
            create_database(current, ["addable", "unresolved"], {})
            create_database(
                recovery,
                ["addable"],
                {"addable": ("repo-add", "skills/addable")},
            )
            writer = sqlite3.connect(current)
            writer.execute(
                "INSERT INTO skill_repositories "
                "(id, name, source_type, owner, repo, branch, url, pinned, "
                "is_unknown, created_at, updated_at, last_synced_at) "
                "VALUES ('repo-add', 'repo-add', 'github', 'different-owner', "
                "'repo-add', 'main', NULL, 0, 0, '2026-08-03T00:00:00Z', "
                "'2026-08-03T00:00:00Z', NULL)"
            )
            writer.commit()
            writer.close()
            preview = build_preview(current, recovery)
            expected = expectations_from_preview(preview)
            expected = RecoveryExpectations(
                **{
                    **expected.__dict__,
                    "repositories_to_insert": 0,
                    "final_populated_github_repositories": 1,
                }
            )
            shutil.copy2(current, backup)

            with self.assertRaisesRegex(RuntimeError, "repositoryConflicts"):
                apply_recovery(current, recovery, backup, expected)

            connection = sqlite3.connect(current)
            self.assertEqual(
                connection.execute(
                    "SELECT COUNT(*) FROM skill_repository_members"
                ).fetchone()[0],
                0,
            )
            self.assertEqual(
                connection.execute("SELECT COUNT(*) FROM operation_logs").fetchone()[0],
                0,
            )
            connection.close()


if __name__ == "__main__":
    unittest.main()
