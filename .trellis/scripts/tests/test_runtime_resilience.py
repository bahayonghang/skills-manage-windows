#!/usr/bin/env python3
"""Focused tests for sub-agent context budget, process bounds, and offline create."""

from __future__ import annotations

import argparse
import contextlib
import hashlib
import importlib.util
import io
import json
import os
import subprocess
import sys
import tempfile
import time
import unittest
from pathlib import Path
from unittest import mock

_SCRIPTS_DIR = Path(__file__).resolve().parents[1]
if str(_SCRIPTS_DIR) not in sys.path:
    sys.path.insert(0, str(_SCRIPTS_DIR))

from common.config import (  # noqa: E402
    CONTEXT_INJECTION_MAX_FILES,
    CONTEXT_INJECTION_MAX_JSONL_LINES,
    get_context_injection_limits,
)
from common.context_injection_budget import (  # noqa: E402
    ContextBudget,
    build_task_agent_context,
    materialize_jsonl_entries,
)
from common.git import resolve_default_branch  # noqa: E402
from common.subprocess_supervision import (  # noqa: E402
    CREATE_NO_WINDOW,
    classify_json_command,
    format_process_diagnostic,
    pid_is_running,
    run_bounded_process,
)
from common import subprocess_supervision as procsup  # noqa: E402
from common.task_store import cmd_create  # noqa: E402
from common.task_utils import run_task_hooks  # noqa: E402

REAL_REPO = Path(__file__).resolve().parents[3]
REAL_TASKS = REAL_REPO / ".trellis" / "tasks"
IS_WINDOWS = os.name == "nt"
POSIX_PROCESS_SKIP = (
    "AC17 missing evidence / UNVERIFIED: POSIX process-group cleanup not run on Windows host"
)
SECRET = "fixture-secret-LINEAR_KEY_9f3a2b_trellis_runtime"
CODEX_HOOK = REAL_REPO / ".codex" / "hooks" / "inject-subagent-context.py"
CLAUDE_HOOK = REAL_REPO / ".claude" / "hooks" / "inject-subagent-context.py"


def _load_hook(kind: str):
    path = REAL_REPO / f".{kind}" / "hooks" / "inject-subagent-context.py"
    spec = importlib.util.spec_from_file_location(f"inject_{kind}_runtime", path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load hook {path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[f"inject_{kind}_runtime"] = module
    spec.loader.exec_module(module)
    return module


def _load_linear_sync():
    path = _SCRIPTS_DIR / "hooks" / "linear_sync.py"
    spec = importlib.util.spec_from_file_location("linear_sync_runtime", path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load {path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules["linear_sync_runtime"] = module
    spec.loader.exec_module(module)
    return module


def _create_args(**overrides: object) -> argparse.Namespace:
    values: dict[str, object] = {
        "title": "Runtime Resilience Fixture",
        "slug": "runtime-resilience-fixture",
        "meta": None,
        "package": None,
        "workflow": None,
        "assignee": "testdev",
        "description": "fixture",
        "parent": None,
        "base_branch": None,
        "no_start": True,
        "priority": "P2",
    }
    values.update(overrides)
    return argparse.Namespace(**values)


def _git(repo: Path, *args: str, check: bool = True) -> subprocess.CompletedProcess[str]:
    completed = subprocess.run(
        ["git", "-c", "i18n.logOutputEncoding=UTF-8", *args],
        cwd=repo,
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
        check=False,
    )
    if check and completed.returncode != 0:
        raise RuntimeError(completed.stderr or completed.stdout or f"git {args} failed")
    return completed


class _RepoFixture(unittest.TestCase):
    def setUp(self) -> None:
        self._old_cwd = Path.cwd()
        self._real_task_names = (
            {p.name for p in REAL_TASKS.iterdir()} if REAL_TASKS.is_dir() else set()
        )
        self._temp = tempfile.TemporaryDirectory()
        self.repo = Path(self._temp.name) / "repo"
        self.repo.mkdir()
        (self.repo / ".trellis").mkdir()
        (self.repo / ".trellis" / ".developer").write_text("name=testdev\n", encoding="utf-8")
        (self.repo / ".trellis" / "tasks").mkdir()
        spec = self.repo / "spec"
        spec.mkdir()
        self.task_dir = self.repo / ".trellis" / "tasks" / "09-02-fixture"
        self.task_dir.mkdir()
        (self.task_dir / "task.json").write_text("{}\n", encoding="utf-8")
        self.jsonl = self.task_dir / "implement.jsonl"
        self.jsonl.write_text("", encoding="utf-8")
        os.chdir(self.repo)

    def tearDown(self) -> None:
        os.chdir(self._old_cwd)
        self._temp.cleanup()
        after = {p.name for p in REAL_TASKS.iterdir()} if REAL_TASKS.is_dir() else set()
        self.assertEqual(
            after,
            self._real_task_names,
            "tests must not create tasks in the real repo .trellis/tasks/",
        )

    def write_jsonl(self, entries: list[dict]) -> str:
        self.jsonl.write_text(
            "".join(json.dumps(entry) + "\n" for entry in entries),
            encoding="utf-8",
        )
        return ".trellis/tasks/09-02-fixture/implement.jsonl"

    def limits(self, **overrides: int) -> dict[str, int]:
        values = {
            "max_file_bytes": 1024,
            "max_artifact_bytes": 2048,
            "max_total_bytes": 512,
            "max_files": 2,
            "max_jsonl_lines": 2,
        }
        values.update(overrides)
        return values

    def tracking_open(self, hits: list[str]):
        real_open = Path.open

        def wrapped(path_self: Path, *args, **kwargs):
            hits.append(str(path_self))
            return real_open(path_self, *args, **kwargs)

        return wrapped

    def call_create(self, **overrides: object) -> tuple[int, str, str]:
        out = io.StringIO()
        err = io.StringIO()
        with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
            rc = cmd_create(_create_args(**overrides))
        return rc, out.getvalue(), err.getvalue()


class TestContextBudget(_RepoFixture):
    def test_context_budget_caps_reason_notice_artifact_files_and_jsonl_lines(self) -> None:
        long_reason = "R" * 4000
        long_notice_file = self.repo / "spec" / "huge.bin"
        long_notice_file.write_bytes(b"\x00" + b"N" * 4000)
        huge_artifact = self.task_dir / "prd.md"
        huge_artifact.write_text("A" * 4000, encoding="utf-8")
        extra_a = self.repo / "spec" / "a.md"
        extra_b = self.repo / "spec" / "b.md"
        extra_c = self.repo / "spec" / "c.md"
        extra_a.write_bytes(b"alpha-one\n")
        extra_b.write_bytes(b"beta-two\n")
        extra_c.write_bytes(b"gamma-three\n")
        jsonl_rel = self.write_jsonl(
            [
                {"file": "spec/huge.bin", "reason": long_reason},
                {"file": "spec/a.md", "reason": long_reason},
                {"file": "spec/b.md", "reason": long_reason},
                {"file": "spec/c.md", "reason": long_reason},
            ]
        )
        limits = self.limits(max_total_bytes=256, max_files=8, max_jsonl_lines=8)
        payload = build_task_agent_context(
            self.repo, ".trellis/tasks/09-02-fixture", "implement", limits
        )
        encoded = payload.encode("utf-8")
        self.assertLessEqual(len(encoded), 256)
        self.assertNotIn(long_reason, payload)
        self.assertLessEqual(payload.count("remaining files omitted"), 1)

        line_limits = self.limits(max_total_bytes=4096, max_files=8, max_jsonl_lines=1)
        hits: list[str] = []
        budget = ContextBudget(line_limits["max_total_bytes"])
        with mock.patch.object(Path, "open", self.tracking_open(hits)):
            materialize_jsonl_entries(str(self.repo), jsonl_rel, line_limits, budget)
        opened = "\n".join(hits)
        self.assertNotIn(str(extra_a.resolve()), opened)
        self.assertNotIn(str(extra_b.resolve()), opened)
        self.assertNotIn(str(extra_c.resolve()), opened)

    def test_context_budget_stops_opening_files_after_file_cap(self) -> None:
        first = self.repo / "spec" / "one.md"
        second = self.repo / "spec" / "two.md"
        third = self.repo / "spec" / "three.md"
        first.write_bytes(b"one\n")
        second.write_bytes(b"two\n")
        third.write_bytes(b"three\n")
        jsonl_rel = self.write_jsonl(
            [
                {"file": "spec/one.md", "reason": "keep"},
                {"file": "spec/two.md", "reason": "stop-before"},
                {"file": "spec/three.md", "reason": "must-not-open"},
            ]
        )
        limits = self.limits(max_total_bytes=4096, max_files=1, max_jsonl_lines=8)
        hits: list[str] = []
        budget = ContextBudget(limits["max_total_bytes"])
        with mock.patch.object(Path, "open", self.tracking_open(hits)):
            materialize_jsonl_entries(str(self.repo), jsonl_rel, limits, budget)
        opened = "\n".join(hits)
        self.assertIn("one.md", opened)
        self.assertNotIn(str(second.resolve()), opened)
        self.assertNotIn(str(third.resolve()), opened)
        self.assertNotIn("three.md", opened)

    def test_context_budget_stops_opening_targets_after_jsonl_line_cap(self) -> None:
        first = self.repo / "spec" / "line1.md"
        second = self.repo / "spec" / "line2.md"
        first.write_bytes(b"KEEP_ME\n")
        second.write_bytes(b"DROP_ME\n")
        jsonl_rel = self.write_jsonl(
            [
                {"file": "spec/line1.md", "reason": "one"},
                {"file": "spec/line2.md", "reason": "two"},
            ]
        )
        limits = self.limits(max_total_bytes=4096, max_files=8, max_jsonl_lines=1)
        hits: list[str] = []
        budget = ContextBudget(limits["max_total_bytes"])
        with mock.patch.object(Path, "open", self.tracking_open(hits)):
            materialize_jsonl_entries(str(self.repo), jsonl_rel, limits, budget)
        opened = "\n".join(hits)
        payload = budget.payload()
        self.assertIn("line1.md", opened)
        self.assertIn("KEEP_ME", payload)
        self.assertNotIn(str(second.resolve()), opened)
        self.assertNotIn("line2.md", opened)
        self.assertNotIn("DROP_ME", payload)
        self.assertEqual(budget.jsonl_lines_consumed, 1)
        self.assertEqual(payload.count("remaining files omitted"), 1)
        self.assertLessEqual(len(payload.encode("utf-8")), limits["max_total_bytes"])

    def test_context_budget_small_input_bytes_and_order_unchanged(self) -> None:
        (self.repo / "spec" / "a.md").write_bytes(b"alpha\n")
        (self.repo / "spec" / "b.md").write_bytes(b"beta\n")
        jsonl_rel = self.write_jsonl(
            [
                {"file": "spec/a.md", "reason": "first"},
                {"file": "spec/b.md", "reason": "second"},
            ]
        )
        limits = self.limits(max_total_bytes=4096, max_files=8, max_jsonl_lines=8)
        budget = ContextBudget(limits["max_total_bytes"])
        blocks = materialize_jsonl_entries(str(self.repo), jsonl_rel, limits, budget)
        payload = budget.payload()
        expected = "=== spec/a.md ===\nalpha\n\n\n=== spec/b.md ===\nbeta\n"
        self.assertEqual(payload, expected)
        self.assertEqual(blocks[0], "=== spec/a.md ===\nalpha\n")
        self.assertEqual(blocks[1], "=== spec/b.md ===\nbeta\n")
        self.assertNotIn("remaining files omitted", payload)

    def test_context_injection_count_limits_are_internal_not_config_keys(self) -> None:
        config = self.repo / ".trellis" / "config.yaml"
        config.write_text(
            "context_injection:\n  max_file_bytes: 12\n  max_files: 1\n  max_jsonl_lines: 1\n",
            encoding="utf-8",
        )
        limits = get_context_injection_limits(self.repo)
        self.assertEqual(limits["max_file_bytes"], 12)
        self.assertEqual(limits["max_files"], CONTEXT_INJECTION_MAX_FILES)
        self.assertEqual(limits["max_jsonl_lines"], CONTEXT_INJECTION_MAX_JSONL_LINES)
        self.assertGreater(CONTEXT_INJECTION_MAX_FILES, 0)
        self.assertGreater(CONTEXT_INJECTION_MAX_JSONL_LINES, 0)


class TestHookBudgetConsumption(_RepoFixture):
    @classmethod
    def setUpClass(cls) -> None:
        cls.hooks_present = CODEX_HOOK.is_file() and CLAUDE_HOOK.is_file()
        if cls.hooks_present:
            cls.codex = _load_hook("codex")
            cls.claude = _load_hook("claude")

    def test_hooks_stay_byte_identical_and_fail_closed_without_budget(self) -> None:
        if not self.hooks_present:
            self.skipTest("local gitignored inject-subagent-context.py hooks are absent")
        self.assertEqual(
            hashlib.sha256(CODEX_HOOK.read_bytes()).hexdigest(),
            hashlib.sha256(CLAUDE_HOOK.read_bytes()).hexdigest(),
        )
        (self.repo / "spec" / "notes.md").write_text("in-repo notes\n", encoding="utf-8")
        jsonl_rel = self.write_jsonl([{"file": "spec/notes.md", "reason": "planted"}])
        for module in (self.codex, self.claude):
            original = module._Budget
            hits: list[str] = []
            try:
                module._Budget = None
                with mock.patch.object(Path, "open", self.tracking_open(hits)), contextlib.redirect_stderr(
                    io.StringIO()
                ):
                    blocks = module._materialize_jsonl_entries(
                        str(self.repo),
                        jsonl_rel,
                        dict(module.DEFAULT_LIMITS),
                        original(4096),
                    )
            finally:
                module._Budget = original
            self.assertEqual(blocks, [])
            opened = "\n".join(hits)
            self.assertNotIn("notes.md", opened)


class TestBoundedProcess(unittest.TestCase):
    def setUp(self) -> None:
        self._temp = tempfile.TemporaryDirectory()
        self.workdir = Path(self._temp.name)

    def tearDown(self) -> None:
        self._temp.cleanup()

    def test_process_timeout_is_identifiable(self) -> None:
        result = run_bounded_process(
            [sys.executable, "-c", "import time; time.sleep(30)"],
            cwd=self.workdir,
            timeout_seconds=0.4,
            max_stdout_bytes=1024,
            max_stderr_bytes=1024,
        )
        self.assertTrue(result.timed_out)
        self.assertNotEqual(result.timed_out, False)
        diagnostic = format_process_diagnostic(result, label="hook")
        self.assertIn("timeout", diagnostic.lower())
        self.assertNotIn(SECRET, diagnostic)

    def test_process_tree_timeout_cleans_descendants(self) -> None:
        child_pid_path = self.workdir / "child.pid"
        parent_pid_path = self.workdir / "parent.pid"
        parent = self.workdir / "parent.py"
        parent.write_text(
            "import os, subprocess, sys, time\n"
            "from pathlib import Path\n"
            "child_pid_path = Path(sys.argv[1])\n"
            "parent_pid_path = Path(sys.argv[2])\n"
            "parent_pid_path.write_text(str(os.getpid()), encoding='utf-8')\n"
            "time.sleep(0.4)\n"
            "child = subprocess.Popen(\n"
            "    [sys.executable, '-c', 'import time; time.sleep(60)'],\n"
            "    stdout=subprocess.DEVNULL,\n"
            "    stderr=subprocess.DEVNULL,\n"
            ")\n"
            "child_pid_path.write_text(str(child.pid), encoding='utf-8')\n"
            "time.sleep(60)\n",
            encoding="utf-8",
        )
        result = run_bounded_process(
            [sys.executable, str(parent), str(child_pid_path), str(parent_pid_path)],
            cwd=self.workdir,
            timeout_seconds=1.4,
            max_stdout_bytes=1024,
            max_stderr_bytes=1024,
        )
        self.assertTrue(result.timed_out)
        self.assertFalse(result.cleanup_failed)
        parent_pid = int(parent_pid_path.read_text(encoding="utf-8")) if parent_pid_path.is_file() else 0
        deadline = time.monotonic() + 5
        child_pid = 0
        if child_pid_path.is_file():
            child_pid = int(child_pid_path.read_text(encoding="utf-8"))
        while time.monotonic() < deadline:
            parent_alive = pid_is_running(parent_pid) if parent_pid else False
            child_alive = pid_is_running(child_pid) if child_pid else False
            if not parent_alive and not child_alive:
                break
            time.sleep(0.05)
        else:
            self.fail(
                f"process tree still alive parent={parent_pid} child={child_pid} "
                f"cleanup_failed={result.cleanup_failed}"
            )
        if IS_WINDOWS:
            self.assertTrue(procsup.LAST_WINDOWS_CREATION_FLAGS & CREATE_NO_WINDOW)

    @unittest.skipUnless(not IS_WINDOWS, POSIX_PROCESS_SKIP)
    def test_process_posix_process_group_cleanup(self) -> None:
        result = run_bounded_process(
            [sys.executable, "-c", "import time, os, subprocess, sys; "
             "subprocess.Popen([sys.executable, '-c', 'import time; time.sleep(60)']); time.sleep(60)"],
            cwd=self.workdir,
            timeout_seconds=0.5,
            max_stdout_bytes=1024,
            max_stderr_bytes=1024,
        )
        self.assertTrue(result.timed_out)
        self.assertFalse(result.cleanup_failed)

    def test_process_output_and_diagnostics_are_bounded_without_secrets(self) -> None:
        env = {**os.environ, "LINEAR_API_KEY": SECRET, "TASK_JSON_PATH": str(self.workdir / "task.json")}
        script = (
            "import os, sys\n"
            "sys.stdout.write('A' * 8000 + os.environ['LINEAR_API_KEY'])\n"
            "sys.stdout.flush()\n"
            "sys.stderr.write('E' * 8000)\n"
            "sys.stderr.flush()\n"
        )
        result = run_bounded_process(
            [sys.executable, "-c", script],
            cwd=self.workdir,
            env=env,
            timeout_seconds=5,
            max_stdout_bytes=1024,
            max_stderr_bytes=1024,
        )
        self.assertLessEqual(len(result.stdout), 1024)
        self.assertLessEqual(len(result.stderr), 1024)
        self.assertTrue(result.output_truncated)
        diagnostic = format_process_diagnostic(result, label="hook")
        self.assertLessEqual(len(diagnostic.encode("utf-8")), 1024)
        self.assertNotIn(SECRET, diagnostic)
        self.assertNotIn(SECRET, result.stdout.decode("utf-8", errors="replace"))
        self.assertNotIn("LINEAR_API_KEY", diagnostic)
        joined_env = " ".join(f"{k}={v}" for k, v in env.items())
        self.assertNotIn(joined_env, diagnostic)

    def test_process_linearis_outcomes_are_distinguishable(self) -> None:
        ok = run_bounded_process(
            [sys.executable, "-c", "print('{\\\"identifier\\\":\\\"ABC-1\\\"}')"],
            cwd=self.workdir,
            timeout_seconds=5,
            max_stdout_bytes=1024,
            max_stderr_bytes=1024,
        )
        nonzero = run_bounded_process(
            [sys.executable, "-c", "raise SystemExit(3)"],
            cwd=self.workdir,
            timeout_seconds=5,
            max_stdout_bytes=1024,
            max_stderr_bytes=1024,
        )
        timed = run_bounded_process(
            [sys.executable, "-c", "import time; time.sleep(30)"],
            cwd=self.workdir,
            timeout_seconds=0.3,
            max_stdout_bytes=1024,
            max_stderr_bytes=1024,
        )
        invalid = run_bounded_process(
            [sys.executable, "-c", "print('not-json')"],
            cwd=self.workdir,
            timeout_seconds=5,
            max_stdout_bytes=1024,
            max_stderr_bytes=1024,
        )
        kinds = {
            classify_json_command(ok),
            classify_json_command(nonzero),
            classify_json_command(timed),
            classify_json_command(invalid),
        }
        self.assertEqual(kinds, {"ok", "nonzero", "timeout", "invalid_json"})
        self.assertEqual(classify_json_command(ok), "ok")
        self.assertEqual(classify_json_command(nonzero), "nonzero")
        self.assertEqual(classify_json_command(timed), "timeout")
        self.assertEqual(classify_json_command(invalid), "invalid_json")

        linear = _load_linear_sync()
        linear.LINEARIS_ARGV = [sys.executable, "-c", "print('{\\\"identifier\\\":\\\"ABC-1\\\"}')"]
        parsed = linear.invoke_linearis("issues", "create", "title")
        self.assertEqual(parsed.kind, "ok")
        self.assertEqual(parsed.payload, {"identifier": "ABC-1"})

    def test_process_linearis_caps_streams_without_secrets(self) -> None:
        env = {**os.environ, "LINEAR_API_KEY": SECRET}
        script = (
            "import os, sys\n"
            "sys.stdout.write('S' * 8000 + os.environ['LINEAR_API_KEY'])\n"
            "sys.stderr.write('err-head' + 'T' * 8000)\n"
        )
        result = run_bounded_process(
            [sys.executable, "-c", script],
            cwd=self.workdir,
            env=env,
            timeout_seconds=5,
            max_stdout_bytes=512,
            max_stderr_bytes=512,
        )
        kind = classify_json_command(result)
        self.assertEqual(kind, "invalid_json")
        diagnostic = format_process_diagnostic(result, label="linearis")
        self.assertNotIn(SECRET, diagnostic)
        self.assertLessEqual(len(result.stdout), 512)
        self.assertLessEqual(len(result.stderr), 512)

    def test_process_run_task_hooks_uses_supervisor(self) -> None:
        repo = self.workdir / "hook-repo"
        repo.mkdir()
        (repo / ".trellis").mkdir()
        task_json = repo / "task.json"
        task_json.write_text("{}\n", encoding="utf-8")
        sleeper = subprocess.list2cmdline(
            [sys.executable, "-c", "import time; time.sleep(30)"]
        )
        with mock.patch("common.task_utils.get_hooks", return_value=[sleeper]), mock.patch.object(
            procsup,
            "HOOK_TIMEOUT_SECONDS",
            0.4,
        ):
            err = io.StringIO()
            with contextlib.redirect_stderr(err):
                run_task_hooks("after_create", task_json, repo)
        text = err.getvalue()
        self.assertIn("timeout", text.lower())
        self.assertNotIn(SECRET, text)


class TestOfflineTaskCreate(_RepoFixture):
    def _init_git(self, branch: str = "feature-local") -> None:
        _git(self.repo, "init", "-b", branch)
        _git(self.repo, "config", "user.email", "test@example.com")
        _git(self.repo, "config", "user.name", "Test")
        _git(self.repo, "commit", "--allow-empty", "-m", "init")

    def _network_verbs(self, spy: mock.Mock) -> list[tuple[str, ...]]:
        verbs: list[tuple[str, ...]] = []
        for call in spy.call_args_list:
            cmd = call.args[0]
            if not cmd or cmd[0] != "git":
                continue
            args = list(cmd)
            if len(args) >= 3 and args[1] == "-c":
                args = args[3:]
            else:
                args = args[1:]
            verbs.append(tuple(args))
        return verbs

    def test_offline_task_create_does_not_call_network_git(self) -> None:
        self._init_git("feature-local")
        with mock.patch("common.git.subprocess.run", wraps=subprocess.run) as spy:
            rc, _out, err = self.call_create(slug="offline-no-origin")
        self.assertEqual(rc, 0, err)
        verbs = self._network_verbs(spy)
        for verb in verbs:
            self.assertNotEqual(verb[:2], ("remote", "show"))
            self.assertNotIn(verb[:1], {("fetch",), ("ls-remote",), ("pull",), ("push",), ("clone",)})
        created = list((self.repo / ".trellis" / "tasks").glob("*-offline-no-origin"))
        self.assertEqual(len(created), 1)
        data = json.loads((created[0] / "task.json").read_text(encoding="utf-8"))
        self.assertEqual(data["base_branch"], "feature-local")

    def test_offline_task_create_leaves_no_partial_dir_on_resolve_or_validation_failure(self) -> None:
        self._init_git("feature-local")
        before = {p.name for p in (self.repo / ".trellis" / "tasks").iterdir()}
        rc, _out, err = self.call_create(slug="..")
        self.assertNotEqual(rc, 0, err)
        self.assertEqual({p.name for p in (self.repo / ".trellis" / "tasks").iterdir()}, before)
        self.assertFalse(any("runtime-resilience" in p.name or p.name.endswith("-..") for p in (self.repo / ".trellis" / "tasks").iterdir()))

        with mock.patch(
            "common.task_store.resolve_default_branch",
            side_effect=RuntimeError("resolver boom"),
        ):
            with self.assertRaises(RuntimeError):
                self.call_create(slug="should-not-exist")
        self.assertFalse(any("should-not-exist" in p.name for p in (self.repo / ".trellis" / "tasks").iterdir()))
        self.assertEqual(
            list((self.repo / ".trellis" / "tasks").glob("*should-not-exist*")),
            [],
        )

        missing_tasks = self.repo / ".trellis" / "tasks"
        missing_tasks.rename(self.repo / ".trellis" / "tasks.bak")
        try:
            with mock.patch(
                "common.task_store.resolve_default_branch",
                side_effect=RuntimeError("resolver boom"),
            ):
                with self.assertRaises(RuntimeError):
                    self.call_create(slug="no-tasks-dir")
            self.assertFalse((self.repo / ".trellis" / "tasks").exists())
        finally:
            if not missing_tasks.exists():
                (self.repo / ".trellis" / "tasks.bak").rename(missing_tasks)

    def test_offline_task_create_base_branch_fixtures_keep_task_json_shape(self) -> None:
        self._init_git("feature-local")
        _git(self.repo, "symbolic-ref", "refs/remotes/origin/HEAD", "refs/remotes/origin/main")
        rc, _out, err = self.call_create(slug="from-origin-head", base_branch=None)
        self.assertEqual(rc, 0, err)
        created = list((self.repo / ".trellis" / "tasks").glob("*-from-origin-head"))
        self.assertEqual(len(created), 1)
        origin_data = json.loads((created[0] / "task.json").read_text(encoding="utf-8"))
        self.assertEqual(origin_data["base_branch"], "main")
        self.assertEqual(resolve_default_branch(self.repo), "main")

        rc, _out, err = self.call_create(slug="explicit-base", base_branch="release-1")
        self.assertEqual(rc, 0, err)
        created = list((self.repo / ".trellis" / "tasks").glob("*-explicit-base"))
        explicit_data = json.loads((created[0] / "task.json").read_text(encoding="utf-8"))
        self.assertEqual(explicit_data["base_branch"], "release-1")

        other = Path(self._temp.name) / "no-origin"
        other.mkdir()
        (other / ".trellis").mkdir()
        (other / ".trellis" / ".developer").write_text("name=testdev\n", encoding="utf-8")
        (other / ".trellis" / "tasks").mkdir()
        os.chdir(other)
        try:
            _git(other, "init", "-b", "only-current")
            _git(other, "config", "user.email", "test@example.com")
            _git(other, "config", "user.name", "Test")
            _git(other, "commit", "--allow-empty", "-m", "init")
            rc, _out, err = self.call_create(slug="current-only", base_branch=None)
            self.assertEqual(rc, 0, err)
            created = list((other / ".trellis" / "tasks").glob("*-current-only"))
            current_data = json.loads((created[0] / "task.json").read_text(encoding="utf-8"))
            self.assertEqual(current_data["base_branch"], "only-current")
        finally:
            os.chdir(self.repo)

        expected_keys = {
            "id",
            "name",
            "title",
            "description",
            "status",
            "dev_type",
            "scope",
            "package",
            "priority",
            "creator",
            "assignee",
            "createdAt",
            "completedAt",
            "branch",
            "base_branch",
            "worktree_path",
            "commit",
            "pr_url",
            "subtasks",
            "children",
            "parent",
            "relatedFiles",
            "notes",
            "meta",
        }
        self.assertEqual(set(origin_data), expected_keys)
        self.assertEqual(set(explicit_data), expected_keys)
        self.assertEqual(set(current_data), expected_keys)


if __name__ == "__main__":
    unittest.main()
