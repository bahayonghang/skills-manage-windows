#!/usr/bin/env python3
"""Focused tests for Trellis task-slug and context-path containment (SEC-001/002)."""

from __future__ import annotations

import argparse
import builtins
import contextlib
import hashlib
import importlib.util
import io
import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

_SCRIPTS_DIR = Path(__file__).resolve().parents[1]
if str(_SCRIPTS_DIR) not in sys.path:
    sys.path.insert(0, str(_SCRIPTS_DIR))

from common.paths import (  # noqa: E402
    PathContainmentError,
    resolve_contained_path,
)
from common.task_context import cmd_add_context  # noqa: E402
from common.task_store import cmd_create  # noqa: E402

SENTINEL_BODY = "SENTINEL_SECRET_UNIQ_9f3a2b_trellis_path_security"
REAL_REPO = Path(__file__).resolve().parents[3]
REAL_TASKS = REAL_REPO / ".trellis" / "tasks"
IS_WINDOWS = os.name == "nt"
POSIX_SKIP = "AC10 missing evidence / UNVERIFIED: POSIX symlink vectors not run on Windows host"


def _load_hook(kind: str):
    path = REAL_REPO / f".{kind}" / "hooks" / "inject-subagent-context.py"
    spec = importlib.util.spec_from_file_location(f"inject_{kind}_path_security", path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load hook {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def _create_args(**overrides: object) -> argparse.Namespace:
    values: dict[str, object] = {
        "title": "Path Security Fixture",
        "slug": None,
        "meta": None,
        "package": None,
        "workflow": None,
        "assignee": "testdev",
        "description": "fixture",
        "parent": None,
        "base_branch": "main",
        "no_start": True,
        "priority": "P2",
    }
    values.update(overrides)
    return argparse.Namespace(**values)


def _add_args(task_dir: str, path: str, jsonl: str = "implement") -> argparse.Namespace:
    return argparse.Namespace(dir=task_dir, file=jsonl, path=path, reason="test")


def _make_junction(link: Path, target: Path) -> None:
    target.mkdir(parents=True, exist_ok=True)
    completed = subprocess.run(
        ["cmd", "/c", "mklink", "/J", str(link), str(target)],
        capture_output=True,
        text=True,
    )
    if completed.returncode != 0:
        raise OSError(completed.stderr or completed.stdout or "mklink /J failed")


class _RepoFixture(unittest.TestCase):
    def setUp(self) -> None:
        self._old_cwd = Path.cwd()
        self._real_task_names = (
            {p.name for p in REAL_TASKS.iterdir()} if REAL_TASKS.is_dir() else set()
        )
        self._temp = tempfile.TemporaryDirectory()
        workspace = Path(self._temp.name)
        self.repo = workspace / "repo"
        self.outside = workspace / "outside"
        self.prefix_trap = workspace / "repo-evil"
        self.repo.mkdir()
        self.outside.mkdir()
        self.prefix_trap.mkdir()
        self.sentinel = self.outside / "sentinel.txt"
        self.sentinel.write_text(SENTINEL_BODY, encoding="utf-8")
        (self.prefix_trap / "leaked.md").write_text(SENTINEL_BODY, encoding="utf-8")
        (self.repo / ".trellis").mkdir()
        (self.repo / ".trellis" / ".developer").write_text("name=testdev\n", encoding="utf-8")
        (self.repo / ".trellis" / "tasks").mkdir()
        spec = self.repo / "spec"
        spec.mkdir()
        self.notes = spec / "notes.md"
        self.notes.write_text("# in-repo notes\n", encoding="utf-8")
        nested = spec / "nested"
        nested.mkdir()
        (nested / "index.md").write_text("# nested\n", encoding="utf-8")
        self.task_dir = self.repo / ".trellis" / "tasks" / "09-02-fixture"
        self.task_dir.mkdir()
        (self.task_dir / "task.json").write_text("{}\n", encoding="utf-8")
        self.jsonl = self.task_dir / "implement.jsonl"
        self.jsonl.write_text("", encoding="utf-8")
        self._junctions: list[Path] = []
        os.chdir(self.repo)

    def tearDown(self) -> None:
        for link in self._junctions:
            try:
                os.rmdir(link)
            except OSError:
                try:
                    link.unlink()
                except OSError:
                    pass
        os.chdir(self._old_cwd)
        self._temp.cleanup()
        after = {p.name for p in REAL_TASKS.iterdir()} if REAL_TASKS.is_dir() else set()
        self.assertEqual(
            after,
            self._real_task_names,
            "tests must not create tasks in the real repo .trellis/tasks/",
        )

    def add_junction(self, link: Path, target: Path) -> None:
        _make_junction(link, target)
        self._junctions.append(link)

    def sentinel_bytes(self) -> bytes:
        return self.sentinel.read_bytes()

    def call_create(self, **overrides: object) -> tuple[int, str, str]:
        out = io.StringIO()
        err = io.StringIO()
        with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
            rc = cmd_create(_create_args(**overrides))
        return rc, out.getvalue(), err.getvalue()

    def call_add_context(self, path: str) -> tuple[int, str, str]:
        out = io.StringIO()
        err = io.StringIO()
        with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
            rc = cmd_add_context(_add_args("09-02-fixture", path))
        return rc, out.getvalue(), err.getvalue()


class TestPathContainmentHelper(_RepoFixture):
    def test_helper_allows_in_repo_file(self) -> None:
        resolved = resolve_contained_path("spec/notes.md", self.repo)
        self.assertEqual(resolved, self.notes.resolve())

    def test_helper_rejects_dotdot_and_prefix_trap(self) -> None:
        with self.assertRaises(PathContainmentError) as escaped:
            resolve_contained_path("../outside/sentinel.txt", self.repo)
        self.assertEqual(escaped.exception.kind, "dotdot")
        self.assertNotIn(SENTINEL_BODY, str(escaped.exception))
        self.assertNotIn(str(self.sentinel.resolve()), str(escaped.exception))
        with self.assertRaises(PathContainmentError):
            resolve_contained_path("../repo-evil/leaked.md", self.repo)


class TestTaskSlug(_RepoFixture):
    SLUG_DENY = (
        "",
        ".",
        "..",
        "foo/bar",
        "foo\\bar",
        "/tmp/evil",
        "C:\\Windows",
        "C:/Windows",
        "C:foo",
        "\\\\server\\share",
        "//server/share",
        "Foo",
        "foo_bar",
        "foo--bar",
        "-foo",
        "foo-",
        "foo bar",
        "foo.bar",
        "..\\..\\Windows",
        "foo/../../etc",
    )

    def test_task_slug_rejects_closed_form_matrix_before_write(self) -> None:
        tasks = self.repo / ".trellis" / "tasks"
        before_names = {p.name for p in tasks.iterdir()}
        before_sentinel = self.sentinel_bytes()
        for slug in self.SLUG_DENY:
            with self.subTest(slug=slug):
                rc, _out, err = self.call_create(slug=slug)
                self.assertNotEqual(rc, 0, err)
                self.assertEqual({p.name for p in tasks.iterdir()}, before_names)
                self.assertEqual(self.sentinel_bytes(), before_sentinel)

    def test_task_slug_rejects_empty_explicit_without_title_fallback(self) -> None:
        rc, _out, err = self.call_create(title="Hello World", slug="")
        self.assertNotEqual(rc, 0, err)
        created = list((self.repo / ".trellis" / "tasks").glob("*-hello-world"))
        self.assertEqual(created, [])

    def test_task_slug_title_derived_create(self) -> None:
        rc, out, err = self.call_create(title="Hello World", slug=None)
        self.assertEqual(rc, 0, err)
        self.assertIn("hello-world", out)
        created = list((self.repo / ".trellis" / "tasks").glob("*-hello-world"))
        self.assertEqual(len(created), 1)

    def test_task_slug_does_not_mkdir_tasks_on_reject(self) -> None:
        missing_tasks_repo = self.repo
        (missing_tasks_repo / ".trellis" / "tasks").rename(
            missing_tasks_repo / ".trellis" / "tasks.bak"
        )
        try:
            rc, _out, err = self.call_create(slug="..")
            self.assertNotEqual(rc, 0, err)
            self.assertFalse((missing_tasks_repo / ".trellis" / "tasks").exists())
        finally:
            (missing_tasks_repo / ".trellis" / "tasks.bak").rename(
                missing_tasks_repo / ".trellis" / "tasks"
            )

    @unittest.skipUnless(IS_WINDOWS, "AC9 Windows drive/UNC slug vectors")
    def test_task_slug_rejects_windows_drive_and_unc(self) -> None:
        before = self.sentinel_bytes()
        for slug in ("D:\\nope", "\\\\localhost\\c$\\Windows", "\\\\?\\C:\\Windows"):
            with self.subTest(slug=slug):
                rc, _out, err = self.call_create(slug=slug)
                self.assertNotEqual(rc, 0, err)
                self.assertEqual(self.sentinel_bytes(), before)

    @unittest.skipUnless(IS_WINDOWS, "AC9 Windows junction task destination")
    def test_task_slug_existing_junction_does_not_change_outside_sentinel(self) -> None:
        with mock.patch("common.task_store.generate_task_date_prefix", return_value="09-02"):
            link = self.repo / ".trellis" / "tasks" / "09-02-evil-slug"
            self.add_junction(link, self.outside)
            before = self.sentinel_bytes()
            rc, _out, err = self.call_create(slug="evil-slug")
            self.assertNotEqual(rc, 0, err)
            self.assertEqual(self.sentinel_bytes(), before)
            self.assertNotIn(SENTINEL_BODY, err)

    @unittest.skipUnless(sys.platform != "win32", POSIX_SKIP)
    def test_task_slug_posix_symlink_escape(self) -> None:
        with mock.patch("common.task_store.generate_task_date_prefix", return_value="09-02"):
            link = self.repo / ".trellis" / "tasks" / "09-02-evil-slug"
            link.symlink_to(self.outside)
            self._junctions.append(link)
            before = self.sentinel_bytes()
            rc, _out, err = self.call_create(slug="evil-slug")
            self.assertNotEqual(rc, 0, err)
            self.assertEqual(self.sentinel_bytes(), before)


class TestAddContext(_RepoFixture):
    def test_add_context_allows_in_repo_file_and_directory(self) -> None:
        rc, out, err = self.call_add_context("spec/notes.md")
        self.assertEqual(rc, 0, err)
        self.assertIn("spec/notes.md", self.jsonl.read_text(encoding="utf-8"))
        rc, out, err = self.call_add_context("spec/nested")
        self.assertEqual(rc, 0, err)
        stored = self.jsonl.read_text(encoding="utf-8")
        self.assertIn("spec/nested/", stored)

    def test_add_context_rejects_escape_matrix_without_rewriting_jsonl(self) -> None:
        vectors = [
            "../outside/sentinel.txt",
            "..\\outside\\sentinel.txt",
            "../repo-evil/leaked.md",
            "/tmp/sentinel.txt",
            str(self.sentinel),
        ]
        if IS_WINDOWS:
            vectors.extend(
                [
                    "C:\\Windows\\win.ini",
                    "\\\\server\\share\\file.txt",
                    "\\\\?\\C:\\Windows\\win.ini",
                ]
            )
        original = self.jsonl.read_bytes()
        before = self.sentinel_bytes()
        for path in vectors:
            with self.subTest(path=path):
                rc, _out, err = self.call_add_context(path)
                self.assertNotEqual(rc, 0, err)
                self.assertEqual(self.jsonl.read_bytes(), original)
                self.assertEqual(self.sentinel_bytes(), before)
                self.assertNotIn(SENTINEL_BODY, err)
                self.assertNotIn(SENTINEL_BODY, _out)

    @unittest.skipUnless(IS_WINDOWS, "AC9 Windows case-canonical in-repo path")
    def test_add_context_windows_case_persists_canonical_relative(self) -> None:
        rc, _out, err = self.call_add_context("SPEC/notes.md")
        self.assertEqual(rc, 0, err)
        stored = self.jsonl.read_text(encoding="utf-8")
        self.assertIn("spec/notes.md", stored.lower())
        entry = json.loads(stored.strip().splitlines()[-1])
        self.assertFalse(Path(entry["file"]).is_absolute())

    @unittest.skipUnless(IS_WINDOWS, "AC9 Windows junction context escape")
    def test_add_context_rejects_windows_junction_escape(self) -> None:
        docs = self.repo / "docs"
        self.add_junction(docs, self.outside)
        original = self.jsonl.read_bytes()
        before = self.sentinel_bytes()
        rc, _out, err = self.call_add_context("docs/sentinel.txt")
        self.assertNotEqual(rc, 0, err)
        self.assertEqual(self.jsonl.read_bytes(), original)
        self.assertEqual(self.sentinel_bytes(), before)
        self.assertNotIn(str(self.sentinel.resolve()), err)
        self.assertNotIn(SENTINEL_BODY, err)

    @unittest.skipUnless(sys.platform != "win32", POSIX_SKIP)
    def test_add_context_posix_symlink_escape(self) -> None:
        link = self.repo / "docs-link.md"
        link.symlink_to(self.sentinel)
        self._junctions.append(link)
        original = self.jsonl.read_bytes()
        rc, _out, err = self.call_add_context("docs-link.md")
        self.assertNotEqual(rc, 0, err)
        self.assertEqual(self.jsonl.read_bytes(), original)
        self.assertNotIn(SENTINEL_BODY, err)


class TestHook(_RepoFixture):
    @classmethod
    def setUpClass(cls) -> None:
        cls.codex = _load_hook("codex")
        cls.claude = _load_hook("claude")

    def _write_jsonl(self, file_path: str, entry_type: str = "file") -> str:
        entry = {"file": file_path, "reason": "planted"}
        if entry_type == "directory":
            entry["type"] = "directory"
        self.jsonl.write_text(json.dumps(entry) + "\n", encoding="utf-8")
        return ".trellis/tasks/09-02-fixture/implement.jsonl"

    def _materialize(self, module, jsonl_rel: str) -> tuple[list[str], str, list[str]]:
        limits = dict(module.DEFAULT_LIMITS)
        budget = module._Budget(limits["max_total_bytes"])
        hits: list[str] = []
        real_open = builtins.open
        forbidden = self.sentinel.resolve()

        def tracking_open(file, *args, **kwargs):
            try:
                candidate = Path(file)
                if candidate.resolve() == forbidden:
                    hits.append(str(file))
            except Exception:
                pass
            return real_open(file, *args, **kwargs)

        err = io.StringIO()
        with mock.patch("builtins.open", tracking_open), contextlib.redirect_stderr(err):
            blocks = module._materialize_jsonl_entries(
                str(self.repo), jsonl_rel, limits, budget
            )
        return blocks, err.getvalue(), hits

    def test_hook_allows_in_repo_file_and_directory(self) -> None:
        jsonl_rel = self._write_jsonl("spec/notes.md")
        for module in (self.codex, self.claude):
            blocks, err, hits = self._materialize(module, jsonl_rel)
            self.assertTrue(any("in-repo notes" in block for block in blocks), err)
            self.assertEqual(hits, [])
        self.jsonl.write_text(
            json.dumps({"file": "spec/nested/", "type": "directory", "reason": "dir"})
            + "\n",
            encoding="utf-8",
        )
        for module in (self.codex, self.claude):
            blocks, err, _hits = self._materialize(
                module, ".trellis/tasks/09-02-fixture/implement.jsonl"
            )
            self.assertTrue(any("nested" in block for block in blocks), err)

    def test_hook_rejects_escape_matrix_before_read(self) -> None:
        vectors = [
            "../outside/sentinel.txt",
            str(self.sentinel),
            "../repo-evil/leaked.md",
        ]
        if IS_WINDOWS:
            vectors.extend(["C:\\Windows\\win.ini", "\\\\server\\share\\file.txt"])
        for path in vectors:
            with self.subTest(path=path):
                jsonl_rel = self._write_jsonl(path)
                for label, module in (("codex", self.codex), ("claude", self.claude)):
                    blocks, err, hits = self._materialize(module, jsonl_rel)
                    joined = "\n".join(blocks)
                    self.assertTrue(err or hits == [], f"{label} {path}: {err!r}")
                    self.assertIn("REJECT", err)
                    self.assertNotIn(SENTINEL_BODY, err)
                    self.assertNotIn(SENTINEL_BODY, joined)
                    self.assertEqual(hits, [])
                    resolved_outside = str(self.sentinel.resolve())
                    if path != str(self.sentinel):
                        self.assertNotIn(resolved_outside, err)
                        self.assertNotIn(resolved_outside, joined)

    def test_hook_codex_and_claude_equivalent_on_vectors(self) -> None:
        jsonl_rel = self._write_jsonl("../outside/sentinel.txt")
        left = self._materialize(self.codex, jsonl_rel)
        right = self._materialize(self.claude, jsonl_rel)
        self.assertEqual(left[0], right[0])
        self.assertIn("REJECT", left[1])
        self.assertIn("REJECT", right[1])
        self.assertEqual(left[2], right[2])
        self.assertEqual(
            hashlib.sha256(
                (REAL_REPO / ".codex" / "hooks" / "inject-subagent-context.py").read_bytes()
            ).hexdigest(),
            hashlib.sha256(
                (REAL_REPO / ".claude" / "hooks" / "inject-subagent-context.py").read_bytes()
            ).hexdigest(),
        )

    @unittest.skipUnless(IS_WINDOWS, "AC9 Windows junction hook reject")
    def test_hook_rejects_windows_junction_escape(self) -> None:
        docs = self.repo / "docs"
        self.add_junction(docs, self.outside)
        jsonl_rel = self._write_jsonl("docs/sentinel.txt")
        for module in (self.codex, self.claude):
            blocks, err, hits = self._materialize(module, jsonl_rel)
            self.assertIn("REJECT", err)
            self.assertNotIn(SENTINEL_BODY, err)
            self.assertNotIn(SENTINEL_BODY, "\n".join(blocks))
            self.assertNotIn(str(self.sentinel.resolve()), err)
            self.assertEqual(hits, [])

    @unittest.skipUnless(sys.platform != "win32", POSIX_SKIP)
    def test_hook_posix_symlink_escape(self) -> None:
        link = self.repo / "leak.md"
        link.symlink_to(self.sentinel)
        self._junctions.append(link)
        jsonl_rel = self._write_jsonl("leak.md")
        for module in (self.codex, self.claude):
            blocks, err, hits = self._materialize(module, jsonl_rel)
            self.assertIn("REJECT", err)
            self.assertNotIn(SENTINEL_BODY, err)
            self.assertNotIn(SENTINEL_BODY, "\n".join(blocks))
            self.assertEqual(hits, [])


if __name__ == "__main__":
    unittest.main()
