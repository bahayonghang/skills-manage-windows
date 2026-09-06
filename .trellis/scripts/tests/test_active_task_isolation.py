#!/usr/bin/env python3
"""SES-001: session identity must not read, clear, or mutate another session."""

from __future__ import annotations

import argparse
import contextlib
import io
import json
import os
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

_SCRIPTS_DIR = Path(__file__).resolve().parents[1]
if str(_SCRIPTS_DIR) not in sys.path:
    sys.path.insert(0, str(_SCRIPTS_DIR))

from common.active_task import (  # noqa: E402
    clear_active_task,
    resolve_active_task,
    set_active_task,
)
from common.paths import (  # noqa: E402
    get_current_task,
    get_current_task_abs,
    has_current_task,
)
from common.session_context import (  # noqa: E402
    get_context_json,
    get_context_record_json,
    get_context_text,
    get_context_text_record,
)
from task import cmd_current, cmd_workflow  # noqa: E402

REAL_REPO = Path(__file__).resolve().parents[3]
REAL_TASKS = REAL_REPO / ".trellis" / "tasks"
OLD_KEY = "codex_old"
NEW_KEY = "codex_new"
OTHER_KEY = "codex_other"
OLD_TASK = ".trellis/tasks/01-01-old"
OWN_TASK = ".trellis/tasks/01-01-own"


def _section(text: str, heading: str) -> str:
    marker = f"## {heading}"
    lines = text.splitlines()
    start = None
    for index, line in enumerate(lines):
        if line.strip() == marker:
            start = index + 1
            break
    if start is None:
        return ""
    end = len(lines)
    for index in range(start, len(lines)):
        if lines[index].startswith("## "):
            end = index
            break
    return "\n".join(lines[start:end]).strip()


class IsolationFixture(unittest.TestCase):
    def setUp(self) -> None:
        self._old_cwd = Path.cwd()
        self._real_task_names = (
            {path.name for path in REAL_TASKS.iterdir()} if REAL_TASKS.is_dir() else set()
        )
        self._temp = tempfile.TemporaryDirectory(prefix="trellis-ses001-")
        self.repo = Path(self._temp.name) / "repo"
        self.repo.mkdir()
        trellis = self.repo / ".trellis"
        trellis.mkdir()
        (trellis / ".developer").write_text("name=testdev\n", encoding="utf-8")
        (trellis / "tasks").mkdir()
        self.sessions = trellis / ".runtime" / "sessions"
        self.sessions.mkdir(parents=True)
        os.chdir(self.repo)

    def tearDown(self) -> None:
        os.chdir(self._old_cwd)
        self._temp.cleanup()
        after = {path.name for path in REAL_TASKS.iterdir()} if REAL_TASKS.is_dir() else set()
        self.assertEqual(
            after,
            self._real_task_names,
            "tests must not create or delete real .trellis/tasks/ entries",
        )

    def as_session(self, key: str | None):
        return patch("common.active_task.resolve_context_key", return_value=key)

    def write_session(self, key: str, task_ref: str | None) -> Path:
        path = self.sessions / f"{key}.json"
        payload: dict[str, object] = {}
        if task_ref is not None:
            payload["current_task"] = task_ref
        path.write_text(json.dumps(payload) + "\n", encoding="utf-8")
        return path

    def write_task(self, relative: str, *, status: str = "planning", extra: dict | None = None) -> Path:
        task_dir = self.repo / relative
        task_dir.mkdir(parents=True, exist_ok=True)
        data = {"name": Path(relative).name, "status": status, "title": Path(relative).name}
        if extra:
            data.update(extra)
        (task_dir / "task.json").write_text(
            json.dumps(data, indent=2) + "\n",
            encoding="utf-8",
        )
        return task_dir

    def archive_task(self, relative: str, month: str = "2026-09") -> str:
        source = self.repo / relative
        name = Path(relative).name
        dest = self.repo / ".trellis" / "tasks" / "archive" / month / name
        dest.parent.mkdir(parents=True, exist_ok=True)
        source.rename(dest)
        return dest.relative_to(self.repo).as_posix()

    def session_bytes(self, key: str) -> bytes:
        return (self.sessions / f"{key}.json").read_bytes()

    def task_bytes(self, relative: str) -> bytes:
        return (self.repo / relative / "task.json").read_bytes()

    def capture_current(self, *, as_json: bool = False, source: bool = False) -> tuple[str, int]:
        args = argparse.Namespace(json=as_json, source=source)
        buf = io.StringIO()
        with contextlib.redirect_stdout(buf):
            rc = cmd_current(args)
        return buf.getvalue(), rc

    def capture_workflow(self, workflow_id: str = "custom") -> tuple[str, int]:
        args = argparse.Namespace(clear=False, id=workflow_id)
        buf = io.StringIO()
        err = io.StringIO()
        with contextlib.redirect_stdout(buf), contextlib.redirect_stderr(err):
            rc = cmd_workflow(args)
        return buf.getvalue() + err.getvalue(), rc

    def presentations(self, key: str | None) -> dict[str, object]:
        with self.as_session(key):
            return {
                "text": get_context_text(self.repo),
                "json": get_context_json(self.repo),
                "record_text": get_context_text_record(self.repo),
                "record_json": get_context_record_json(self.repo),
                "current": self.capture_current(),
                "current_json": self.capture_current(as_json=True),
            }

    def assert_old_session_untouched(self, key: str, snapshot: bytes) -> None:
        path = self.sessions / f"{key}.json"
        self.assertTrue(path.is_file(), f"other session {key} must survive")
        self.assertEqual(path.read_bytes(), snapshot)

    def assert_no_executable_current(self, views: dict[str, object], pointer: str | None = None) -> None:
        text = str(views["text"])
        record_text = str(views["record_text"])
        for blob in (text, record_text):
            current = _section(blob, "CURRENT TASK")
            self.assertIn("(none)", current)
            if pointer:
                self.assertNotIn(f"Path: {pointer}", current)
        json_doc = views["json"]
        record_json = views["record_json"]
        assert isinstance(json_doc, dict)
        assert isinstance(record_json, dict)
        self.assertIsNone(json_doc.get("currentTask"))
        self.assertIsNone(record_json.get("currentTask"))
        output, rc = views["current"]
        assert isinstance(output, str)
        self.assertNotEqual(rc, 0)
        if pointer:
            self.assertNotEqual(output.strip(), pointer)
        self.assertRegex(output, r"(?i)no valid")
        json_out, json_rc = views["current_json"]
        assert isinstance(json_out, str)
        self.assertNotEqual(json_rc, 0)
        payload = json.loads(json_out)
        self.assertIsNone(payload.get("current_task"))

    def assert_invalid_pointer_shown(
        self,
        views: dict[str, object],
        pointer: str,
        reason: str,
        archive_path: str | None = None,
    ) -> None:
        self.assert_no_executable_current(views, pointer)
        for blob in (str(views["text"]), str(views["record_text"])):
            invalid = _section(blob, "INVALID POINTER")
            self.assertIn(pointer, invalid)
            self.assertIn(reason, invalid)
            if archive_path:
                self.assertIn(archive_path, invalid)
        for doc in (views["json"], views["record_json"]):
            assert isinstance(doc, dict)
            inv = doc.get("invalidPointer")
            self.assertIsInstance(inv, dict)
            self.assertEqual(inv.get("path"), pointer)
            self.assertEqual(inv.get("reason"), reason)
            if archive_path:
                self.assertEqual(inv.get("archivePath"), archive_path)
        output, _rc = views["current"]
        assert isinstance(output, str)
        self.assertIn("Invalid pointer:", output)
        self.assertIn(pointer, output)
        self.assertIn(reason, output)
        if archive_path:
            self.assertIn(archive_path, output)
        json_out, _rc = views["current_json"]
        payload = json.loads(str(json_out))
        inv = payload.get("invalid_pointer")
        self.assertIsInstance(inv, dict)
        self.assertEqual(inv.get("path"), pointer)
        self.assertEqual(inv.get("reason"), reason)
        if archive_path:
            self.assertEqual(inv.get("archive_path"), archive_path)
        self.assertTrue(payload.get("stale"))

    def assert_normal_current(self, views: dict[str, object], pointer: str) -> None:
        for blob in (str(views["text"]), str(views["record_text"])):
            current = _section(blob, "CURRENT TASK")
            self.assertIn(f"Path: {pointer}", current)
            self.assertEqual(_section(blob, "INVALID POINTER"), "")
        for doc in (views["json"], views["record_json"]):
            assert isinstance(doc, dict)
            current = doc.get("currentTask")
            self.assertIsInstance(current, dict)
            self.assertEqual(current.get("path"), pointer)
            self.assertIsNone(doc.get("invalidPointer"))
        output, rc = views["current"]
        self.assertEqual(rc, 0)
        self.assertEqual(str(output).strip(), pointer)
        json_out, json_rc = views["current_json"]
        self.assertEqual(json_rc, 0)
        payload = json.loads(str(json_out))
        current_task = payload.get("current_task")
        self.assertIsInstance(current_task, dict)
        self.assertEqual(current_task.get("dir"), pointer)
        self.assertFalse(payload.get("stale"))
        self.assertIsNone(payload.get("invalid_pointer"))


class TestReadIsolation(IsolationFixture):
    def test_new_identity_does_not_read_unique_live_old_task(self) -> None:
        self.write_task(OLD_TASK)
        old_path = self.write_session(OLD_KEY, OLD_TASK)
        snapshot = old_path.read_bytes()
        with self.as_session(NEW_KEY):
            active = resolve_active_task(self.repo, allow_single_session_fallback=True)
            self.assertIsNone(active.task_path)
            self.assertEqual(active.source_type, "none")
            self.assertEqual(active.context_key, NEW_KEY)
            self.assertNotEqual(active.source_type, "session-fallback")
            self.assertIsNone(get_current_task(self.repo))
            self.assertFalse(has_current_task(self.repo))
        self.assert_old_session_untouched(OLD_KEY, snapshot)

    def test_new_identity_does_not_read_missing_old_task(self) -> None:
        old_path = self.write_session(OLD_KEY, OLD_TASK)
        snapshot = old_path.read_bytes()
        with self.as_session(NEW_KEY):
            active = resolve_active_task(self.repo)
            self.assertIsNone(active.task_path)
            self.assertEqual(active.source_type, "none")
            self.assertEqual(active.context_key, NEW_KEY)
            self.assertFalse(active.stale)
        self.assert_old_session_untouched(OLD_KEY, snapshot)

    def test_no_identity_does_not_read_unique_old_task(self) -> None:
        self.write_task(OLD_TASK)
        old_path = self.write_session(OLD_KEY, OLD_TASK)
        snapshot = old_path.read_bytes()
        with self.as_session(None):
            active = resolve_active_task(self.repo)
            self.assertIsNone(active.task_path)
            self.assertEqual(active.source_type, "none")
            self.assertIsNone(active.context_key)
            self.assertNotEqual(active.source_type, "session-fallback")
            self.assertIsNone(get_current_task(self.repo))
            self.assertFalse(has_current_task(self.repo))
        self.assert_old_session_untouched(OLD_KEY, snapshot)

    def test_new_identity_does_not_read_when_two_old_sessions(self) -> None:
        self.write_task(OLD_TASK)
        self.write_session(OLD_KEY, OLD_TASK)
        self.write_session(OTHER_KEY, OLD_TASK)
        old_snapshot = self.session_bytes(OLD_KEY)
        other_snapshot = self.session_bytes(OTHER_KEY)
        with self.as_session(NEW_KEY):
            active = resolve_active_task(self.repo)
            self.assertIsNone(active.task_path)
            self.assertEqual(active.context_key, NEW_KEY)
            self.assertEqual(active.source_type, "none")
        self.assert_old_session_untouched(OLD_KEY, old_snapshot)
        self.assert_old_session_untouched(OTHER_KEY, other_snapshot)

    def test_parent_probe_new_session_does_not_borrow_or_clear_other(self) -> None:
        old_session = self.write_session(OLD_KEY, OLD_TASK)
        with self.as_session(NEW_KEY):
            stale = resolve_active_task(self.repo)
            self.assertIsNone(stale.task_path)
            self.assertNotEqual(stale.source_type, "session-fallback")
            self.write_task(OLD_TASK, extra={"status": "planning"})
            active = resolve_active_task(self.repo)
            self.assertIsNone(active.task_path)
            self.assertNotEqual(active.source_type, "session-fallback")
            self.assertNotEqual(active.context_key, OLD_KEY)
            cleared = clear_active_task(self.repo)
            self.assertNotIn("session-fallback", cleared.source)
            self.assertTrue(old_session.exists())
            self.assertTrue(old_session.read_text(encoding="utf-8"))


class TestMutationIsolation(IsolationFixture):
    def test_clear_new_identity_does_not_delete_old_session(self) -> None:
        self.write_task(OLD_TASK)
        old_path = self.write_session(OLD_KEY, OLD_TASK)
        snapshot = old_path.read_bytes()
        task_snapshot = self.task_bytes(OLD_TASK)
        with self.as_session(NEW_KEY):
            clear_active_task(self.repo)
        self.assert_old_session_untouched(OLD_KEY, snapshot)
        self.assertEqual(self.task_bytes(OLD_TASK), task_snapshot)

    def test_clear_no_identity_does_not_delete_old_session(self) -> None:
        self.write_task(OLD_TASK)
        old_path = self.write_session(OLD_KEY, OLD_TASK)
        snapshot = old_path.read_bytes()
        with self.as_session(None):
            cleared = clear_active_task(self.repo)
            self.assertIsNone(cleared.task_path)
            self.assertEqual(cleared.source_type, "none")
        self.assert_old_session_untouched(OLD_KEY, snapshot)

    def test_workflow_new_identity_does_not_write_old_task(self) -> None:
        self.write_task(OLD_TASK)
        self.write_session(OLD_KEY, OLD_TASK)
        snapshot = self.task_bytes(OLD_TASK)
        session_snapshot = self.session_bytes(OLD_KEY)
        with self.as_session(NEW_KEY):
            output, rc = self.capture_workflow()
        self.assertNotEqual(rc, 0)
        self.assertRegex(output, r"(?i)no current task")
        self.assertEqual(self.task_bytes(OLD_TASK), snapshot)
        self.assert_old_session_untouched(OLD_KEY, session_snapshot)

    def test_workflow_no_identity_does_not_write_old_task(self) -> None:
        self.write_task(OLD_TASK)
        self.write_session(OLD_KEY, OLD_TASK)
        snapshot = self.task_bytes(OLD_TASK)
        with self.as_session(None):
            output, rc = self.capture_workflow()
        self.assertNotEqual(rc, 0)
        self.assertRegex(output, r"(?i)no current task")
        self.assertEqual(self.task_bytes(OLD_TASK), snapshot)
        self.assertTrue((self.sessions / f"{OLD_KEY}.json").is_file())

    def test_same_session_read_clear_and_workflow_succeed(self) -> None:
        self.write_task(OWN_TASK)
        self.write_session(NEW_KEY, OWN_TASK)
        with self.as_session(NEW_KEY):
            active = resolve_active_task(self.repo)
            self.assertEqual(active.task_path, OWN_TASK)
            self.assertFalse(active.stale)
            self.assertEqual(active.source_type, "session")
            self.assertEqual(get_current_task(self.repo), OWN_TASK)
            output, rc = self.capture_workflow("custom")
            self.assertEqual(rc, 0, output)
            data = json.loads((self.repo / OWN_TASK / "task.json").read_text(encoding="utf-8"))
            self.assertEqual(data["workflow"], "custom")
            cleared = clear_active_task(self.repo)
            self.assertEqual(cleared.task_path, OWN_TASK)
            self.assertFalse((self.sessions / f"{NEW_KEY}.json").is_file())


class TestStaleAndPresentations(IsolationFixture):
    def test_missing_pointer_four_entry_points(self) -> None:
        session_path = self.write_session(NEW_KEY, OLD_TASK)
        snapshot = session_path.read_bytes()
        with self.as_session(NEW_KEY):
            active = resolve_active_task(self.repo)
            self.assertIsNone(active.task_path)
            self.assertTrue(active.stale)
            self.assertEqual(active.pointer, OLD_TASK)
            self.assertEqual(active.invalid_reason, "missing_directory")
            self.assertIsNone(get_current_task(self.repo))
            self.assertIsNone(get_current_task_abs(self.repo))
            self.assertFalse(has_current_task(self.repo))
        views = self.presentations(NEW_KEY)
        self.assert_invalid_pointer_shown(views, OLD_TASK, "missing_directory")
        self.assertEqual(session_path.read_bytes(), snapshot)

    def test_archived_pointer_four_entry_points(self) -> None:
        self.write_task(OLD_TASK, status="completed")
        archive_path = self.archive_task(OLD_TASK)
        session_path = self.write_session(NEW_KEY, OLD_TASK)
        snapshot = session_path.read_bytes()
        with self.as_session(NEW_KEY):
            active = resolve_active_task(self.repo)
            self.assertIsNone(active.task_path)
            self.assertTrue(active.stale)
            self.assertEqual(active.invalid_reason, "archived")
            self.assertEqual(active.archive_path, archive_path)
            self.assertIsNone(get_current_task(self.repo))
        views = self.presentations(NEW_KEY)
        self.assert_invalid_pointer_shown(views, OLD_TASK, "archived", archive_path)
        self.assertEqual(session_path.read_bytes(), snapshot)

    def test_missing_task_json_is_invalid_not_executable(self) -> None:
        task_dir = self.repo / OLD_TASK
        task_dir.mkdir(parents=True)
        session_path = self.write_session(NEW_KEY, OLD_TASK)
        snapshot = session_path.read_bytes()
        with self.as_session(NEW_KEY):
            active = resolve_active_task(self.repo)
            self.assertIsNone(active.task_path)
            self.assertTrue(active.stale)
            self.assertEqual(active.invalid_reason, "missing_task_json")
            self.assertIsNone(get_current_task(self.repo))
        views = self.presentations(NEW_KEY)
        self.assert_invalid_pointer_shown(views, OLD_TASK, "missing_task_json")
        self.assertEqual(session_path.read_bytes(), snapshot)

    def test_normal_pointer_four_entry_points(self) -> None:
        self.write_task(OWN_TASK, extra={"description": "own work"})
        self.write_session(NEW_KEY, OWN_TASK)
        with self.as_session(NEW_KEY):
            active = resolve_active_task(self.repo)
            self.assertEqual(active.task_path, OWN_TASK)
            self.assertFalse(active.stale)
            self.assertIsNone(active.invalid_reason)
        views = self.presentations(NEW_KEY)
        self.assert_normal_current(views, OWN_TASK)

    def test_stale_resolve_does_not_delete_own_session(self) -> None:
        session_path = self.write_session(NEW_KEY, OLD_TASK)
        snapshot = session_path.read_bytes()
        views = self.presentations(NEW_KEY)
        self.assert_no_executable_current(views, OLD_TASK)
        self.assertEqual(session_path.read_bytes(), snapshot)

    def test_explicit_context_id_reads_own_task(self) -> None:
        self.write_task(OWN_TASK)
        self.write_session("injected_key", OWN_TASK)
        env = {**os.environ, "TRELLIS_CONTEXT_ID": "injected_key"}
        with patch.dict(os.environ, env, clear=True):
            active = resolve_active_task(self.repo)
            self.assertEqual(active.task_path, OWN_TASK)
            self.assertEqual(active.context_key, "injected_key")
            self.assertEqual(get_current_task(self.repo), OWN_TASK)

    def test_same_session_set_active_task_roundtrip(self) -> None:
        self.write_task(OWN_TASK)
        with self.as_session(NEW_KEY):
            stored = set_active_task(OWN_TASK, self.repo)
            self.assertIsNotNone(stored)
            assert stored is not None
            self.assertEqual(stored.task_path, OWN_TASK)
            active = resolve_active_task(self.repo)
            self.assertEqual(active.task_path, OWN_TASK)
            self.assertEqual(active.source_type, "session")


if __name__ == "__main__":
    unittest.main()
