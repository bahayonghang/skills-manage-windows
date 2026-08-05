from pathlib import Path
import sqlite3
import tempfile
import unittest

from preview_startup_recovery import (
    begin_read_snapshot,
    build_preview,
    memberships,
    open_read_only,
    relevant_state_marker,
)


def create_database(
    path: Path,
    skill_ids: list[str],
    assignments: dict[str, tuple[str, str]],
) -> None:
    connection = sqlite3.connect(path)
    connection.executescript(
        """
        PRAGMA foreign_keys=ON;
        CREATE TABLE schema_migrations (
            version INTEGER PRIMARY KEY,
            checksum TEXT NOT NULL,
            applied_at TEXT NOT NULL
        );
        CREATE TABLE skills (
            id TEXT PRIMARY KEY,
            is_central INTEGER NOT NULL
        );
        CREATE TABLE skill_repositories (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            source_type TEXT NOT NULL,
            owner TEXT,
            repo TEXT,
            branch TEXT,
            url TEXT,
            pinned INTEGER NOT NULL DEFAULT 0,
            is_unknown INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            last_synced_at TEXT
        );
        CREATE TABLE skill_repository_members (
            skill_id TEXT PRIMARY KEY REFERENCES skills(id),
            repository_id TEXT NOT NULL REFERENCES skill_repositories(id),
            source_path TEXT,
            added_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            resolved_commit_sha TEXT,
            content_digest TEXT
        );
        CREATE TABLE operation_logs (
            id TEXT PRIMARY KEY,
            created_at TEXT NOT NULL,
            level TEXT NOT NULL,
            target_kind TEXT NOT NULL,
            target_id TEXT NOT NULL,
            target_label TEXT,
            category TEXT NOT NULL,
            action TEXT NOT NULL,
            status TEXT NOT NULL,
            subject_type TEXT,
            subject_id TEXT,
            subject_label TEXT,
            summary TEXT NOT NULL,
            error_summary TEXT,
            details_json TEXT,
            duration_ms INTEGER,
            batch_id TEXT
        );
        INSERT INTO schema_migrations VALUES (4, 'canonical-v4', '2026-08-03T00:00:00Z');
        """
    )
    connection.executemany(
        "INSERT INTO skills (id, is_central) VALUES (?, 1)",
        [(skill_id,) for skill_id in skill_ids],
    )
    repository_ids = sorted({repository_id for repository_id, _ in assignments.values()})
    connection.executemany(
            "INSERT INTO skill_repositories "
            "(id, name, source_type, owner, repo, branch, url, pinned, is_unknown, "
            "created_at, updated_at, last_synced_at) "
            "VALUES (?, ?, 'github', 'owner', ?, 'main', NULL, 0, 0, ?, ?, NULL)",
        [
            (
                repository_id,
                repository_id,
                repository_id,
                "2026-08-03T00:00:00Z",
                "2026-08-03T00:00:00Z",
            )
            for repository_id in repository_ids
        ],
    )
    connection.executemany(
        "INSERT INTO skill_repository_members "
        "(skill_id, repository_id, source_path, added_at, updated_at) "
        "VALUES (?, ?, ?, ?, ?)",
        [
            (
                skill_id,
                repository_id,
                source_path,
                "2026-08-03T00:00:00Z",
                "2026-08-03T00:00:00Z",
            )
            for skill_id, (repository_id, source_path) in assignments.items()
        ],
    )
    connection.commit()
    connection.close()


class PreviewStartupRecoveryTests(unittest.TestCase):
    def test_preview_classifies_stable_ids_without_writing_either_database(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            current_path = root / "current.sqlite"
            recovery_path = root / "recovery.sqlite"
            create_database(
                current_path,
                ["addable", "same", "conflict", "unresolved"],
                {
                    "same": ("repo-same", "skills/same"),
                    "conflict": ("repo-current", "skills/conflict"),
                },
            )
            create_database(
                recovery_path,
                ["addable", "same", "conflict", "missing-current"],
                {
                    "addable": ("repo-add", "skills/addable"),
                    "same": ("repo-same", "skills/same"),
                    "conflict": ("repo-recovery", "skills/conflict"),
                    "missing-current": ("repo-missing", "skills/missing"),
                },
            )
            before = (current_path.read_bytes(), recovery_path.read_bytes())

            preview = build_preview(current_path, recovery_path)

            self.assertEqual(
                preview["classification"],
                {
                    "addable": 1,
                    "alreadySame": 1,
                    "conflict": 1,
                    "missingParent": 1,
                    "unresolved": 1,
                    "repositoriesToInsert": 1,
                    "repositoryConflicts": 0,
                    "addableByRepository": {"repo-add": 1},
                },
            )
            self.assertFalse(preview["readyForApprovedApply"])
            for side in ("current", "recovery"):
                self.assertEqual(preview[side]["snapshot"]["algorithm"], "sha256")
                self.assertEqual(len(preview[side]["snapshot"]["digest"]), 64)
            self.assertEqual(before, (current_path.read_bytes(), recovery_path.read_bytes()))
            self.assertNotIn(str(root), str(preview))

    def test_read_transaction_is_stable_and_digest_detects_wal_only_drift(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "current.sqlite"
            create_database(
                path,
                ["skill"],
                {"skill": ("repo", "skills/before")},
            )
            writer = sqlite3.connect(path)
            self.assertEqual(writer.execute("PRAGMA journal_mode=WAL").fetchone()[0], "wal")
            writer.execute("PRAGMA wal_autocheckpoint=0")
            writer.execute("PRAGMA wal_checkpoint(TRUNCATE)")

            reader = open_read_only(path)
            begin_read_snapshot(reader)
            before_memberships = memberships(reader)
            before_marker = relevant_state_marker(reader)
            main_file_marker = (path.stat().st_size, path.stat().st_mtime_ns)

            writer.execute(
                "UPDATE skill_repository_members "
                "SET source_path = 'skills/after', updated_at = '2026-08-03T01:00:00Z' "
                "WHERE skill_id = 'skill'"
            )
            writer.commit()

            self.assertEqual(main_file_marker, (path.stat().st_size, path.stat().st_mtime_ns))
            self.assertEqual(memberships(reader), before_memberships)
            self.assertEqual(relevant_state_marker(reader), before_marker)
            reader.rollback()
            reader.close()

            fresh_reader = open_read_only(path)
            begin_read_snapshot(fresh_reader)
            self.assertNotEqual(relevant_state_marker(fresh_reader), before_marker)
            fresh_reader.rollback()
            fresh_reader.close()
            writer.close()


if __name__ == "__main__":
    unittest.main()
