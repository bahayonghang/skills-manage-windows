#!/usr/bin/env python3
"""Hard UTF-8 budget for Trellis sub-agent context injection.

Hooks import this module. If the import fails they must reject closed and
must not read unbounded context files. Path containment is reused from
``common.paths.resolve_contained_path``; this module does not implement a
second containment system.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path
from typing import Any

from .config import (
    CONTEXT_INJECTION_MAX_FILES,
    CONTEXT_INJECTION_MAX_JSONL_LINES,
    get_context_injection_limits,
)
from .paths import PathContainmentError, resolve_contained_path, truncate_manifest_for_diag

_REJECT_PREFIX = "[inject-subagent-context] REJECT"
_SUMMARY_TEXT = (
    "[Trellis: not inlined (total context limit reached) — remaining files omitted]"
)
_HARD_READ_CAP = 1_048_576
_JSONL_LINE_CAP = 65536


def truncate_utf8(data: bytes, cap: int) -> bytes:
    """Truncate ``data`` to at most ``cap`` bytes without splitting UTF-8."""
    if cap <= 0 or len(data) <= cap:
        return data

    truncated = data[:cap]
    i = len(truncated)
    while i > 0 and (truncated[i - 1] & 0xC0) == 0x80:
        i -= 1
    if i == 0:
        return b""

    lead = truncated[i - 1]
    if lead & 0x80:
        if (lead & 0xE0) == 0xC0:
            seq_len = 2
        elif (lead & 0xF0) == 0xE0:
            seq_len = 3
        elif (lead & 0xF8) == 0xF0:
            seq_len = 4
        else:
            seq_len = 1
        if (i - 1) + seq_len > len(truncated):
            return truncated[: i - 1]
    return truncated


class ContextBudget:
    """Single UTF-8 ledger for the final injected payload."""

    def __init__(self, max_total_bytes: int) -> None:
        self.max_total_bytes = max_total_bytes
        self._buf = bytearray()
        self.files_opened = 0
        self.jsonl_lines_consumed = 0
        self.summary_emitted = False
        self.stopped = False
        self.jsonl_truncated = False

    @property
    def used(self) -> int:
        return len(self._buf)

    def remaining(self) -> int | None:
        if self.max_total_bytes <= 0:
            return None
        return max(0, self.max_total_bytes - len(self._buf))

    def has_room(self, size: int) -> bool:
        if self.max_total_bytes <= 0:
            return True
        return self.used + size <= self.max_total_bytes

    def add(self, size: int) -> None:
        """Legacy no-op kept for hook compatibility; bytes enter via append_block."""
        _ = size

    def payload(self) -> str:
        return bytes(self._buf).decode("utf-8")

    def can_take_jsonl_line(self, max_lines: int) -> bool:
        if self.stopped:
            return False
        if max_lines > 0 and self.jsonl_lines_consumed >= max_lines:
            self.jsonl_truncated = True
            return False
        return True

    def note_jsonl_line(self) -> None:
        self.jsonl_lines_consumed += 1

    def can_open_file(self, max_files: int) -> bool:
        if self.stopped:
            return False
        remain = self.remaining()
        if remain is not None and remain <= 0:
            self.stopped = True
            return False
        if max_files > 0 and self.files_opened >= max_files:
            self.stopped = True
            return False
        return True

    def note_file_opened(self) -> None:
        self.files_opened += 1

    def append_block(self, block: str) -> str:
        if not block:
            return ""
        sep = "\n\n" if self._buf else ""
        remain = self.remaining()
        if remain is not None:
            if remain <= 0 or (self._buf and remain <= len(sep.encode("utf-8"))):
                self.stopped = True
                return ""
        data = (sep + block).encode("utf-8")
        original_len = len(data)
        if remain is not None:
            data = truncate_utf8(data, remain)
            if not data:
                self.stopped = True
                return ""
            if len(data) < original_len:
                self.stopped = True
        self._buf.extend(data)
        text = data.decode("utf-8")
        if sep and text.startswith("\n\n"):
            return text[2:]
        return text

    def emit_summary_once(self) -> str:
        if self.summary_emitted:
            return ""
        self.stopped = True
        self.summary_emitted = True
        return self.append_block(_SUMMARY_TEXT)


def _emit_context_reject(kind: str, manifest: str) -> None:
    shown = truncate_manifest_for_diag(manifest)
    print(f"{_REJECT_PREFIX}: {kind} path={shown}", file=sys.stderr)


def contain_repo_path(base_path: str, file_path: str, *, emit: bool = True) -> Path | None:
    try:
        return resolve_contained_path(file_path, Path(base_path))
    except PathContainmentError as exc:
        if emit:
            _emit_context_reject(exc.kind, file_path)
        return None
    except Exception:
        if emit:
            _emit_context_reject("unresolved", file_path)
        return None


def _truncate_notice(path: str, cap: int) -> str:
    return f"\n[Trellis: truncated at {cap} bytes — read {path} for the full content]"


def _is_binary_content(data: bytes) -> bool:
    if b"\x00" in data:
        return True
    try:
        data.decode("utf-8", errors="strict")
    except UnicodeDecodeError:
        return True
    return False


def _binary_notice(path: str, size: int, reason: str) -> str:
    return (
        f"[Trellis: not inlined (binary file) — {path} ({size} bytes): {reason}]"
    )


def _jsonl_line_cap(limits: dict[str, int]) -> int:
    total = int(limits.get("max_total_bytes") or 0)
    artifact = int(limits.get("max_artifact_bytes") or 0)
    caps = [cap for cap in (total, artifact, _JSONL_LINE_CAP) if cap > 0]
    return min(caps) if caps else _JSONL_LINE_CAP


def _read_capped(path: Path, cap: int) -> bytes:
    with path.open("rb") as handle:
        if cap <= 0:
            return handle.read(_HARD_READ_CAP + 1)
        return handle.read(cap + 1)


def _effective_read_cap(limits: dict[str, int], budget: ContextBudget, per_file_key: str) -> int:
    per_file = int(limits.get(per_file_key) or 0)
    remain = budget.remaining()
    caps = [cap for cap in (per_file, remain if remain is not None else 0, _HARD_READ_CAP) if cap > 0]
    return min(caps) if caps else _HARD_READ_CAP


def budgeted_block(
    budget: ContextBudget,
    header: str,
    plain_path: str,
    content: str,
    reason: str,
    size_for_index: int,
) -> str:
    _ = (plain_path, reason, size_for_index)
    block = f"=== {header} ===\n{content}"
    return budget.append_block(block)


def _max_files(limits: dict[str, int]) -> int:
    return int(limits.get("max_files") or CONTEXT_INJECTION_MAX_FILES)


def _max_jsonl_lines(limits: dict[str, int]) -> int:
    return int(limits.get("max_jsonl_lines") or CONTEXT_INJECTION_MAX_JSONL_LINES)


def read_jsonl_entries(
    base_path: str,
    jsonl_path: str,
    limits: dict[str, int] | None = None,
    budget: ContextBudget | None = None,
) -> list[dict[str, Any]]:
    resolved_limits = dict(limits or {})
    if budget is None:
        budget = ContextBudget(int(resolved_limits.get("max_total_bytes") or 0))
    max_lines = _max_jsonl_lines(resolved_limits)
    contained_jsonl = contain_repo_path(base_path, jsonl_path)
    if contained_jsonl is None:
        return []
    if not contained_jsonl.is_file():
        print(
            f"[inject-subagent-context] WARN: {jsonl_path} not found — "
            f"sub-agent will receive only task artifacts",
            file=sys.stderr,
        )
        return []

    entries: list[dict[str, Any]] = []
    saw_real_entry = False
    line_cap = _jsonl_line_cap(resolved_limits)
    try:
        with contained_jsonl.open("r", encoding="utf-8") as handle:
            for raw_line in handle:
                if budget.stopped:
                    break
                line = raw_line.strip()
                if not line:
                    continue
                if len(line.encode("utf-8")) > line_cap:
                    budget.stopped = True
                    budget.emit_summary_once()
                    break
                try:
                    item = json.loads(line)
                except json.JSONDecodeError:
                    continue
                file_path = item.get("file") or item.get("path")
                if not file_path:
                    continue
                saw_real_entry = True
                if not budget.can_take_jsonl_line(max_lines):
                    break
                budget.note_jsonl_line()
                if contain_repo_path(base_path, str(file_path)) is None:
                    continue
                entries.append(
                    {
                        "file": file_path,
                        "type": item.get("type", "file"),
                        "reason": item.get("reason") or "-",
                    }
                )
    except OSError:
        return entries

    if not saw_real_entry:
        print(
            f"[inject-subagent-context] WARN: {jsonl_path} has no curated "
            f"entries (only seed / empty) — sub-agent will receive only "
            f"task artifacts. See workflow.md planning artifact guidance.",
            file=sys.stderr,
        )
    return entries


def materialize_file(
    base_path: str,
    file_path: str,
    reason: str,
    limits: dict[str, int],
    budget: ContextBudget,
) -> str:
    contained = contain_repo_path(base_path, file_path)
    if contained is None or not contained.is_file():
        return ""
    budget.note_file_opened()
    cap = _effective_read_cap(limits, budget, "max_file_bytes")
    try:
        data = _read_capped(contained, cap)
    except OSError:
        return ""
    size = contained.stat().st_size
    if _is_binary_content(data):
        return budget.append_block(_binary_notice(file_path, size, reason))

    file_cap = int(limits.get("max_file_bytes") or 0)
    truncated_bytes = truncate_utf8(data, file_cap)
    content = truncated_bytes.decode("utf-8", errors="replace")
    if file_cap > 0 and (len(truncated_bytes) < len(data) or len(data) > file_cap):
        content += _truncate_notice(file_path, file_cap)
    return budgeted_block(budget, file_path, file_path, content, reason, size)


def materialize_directory(
    base_path: str,
    dir_path: str,
    reason: str,
    limits: dict[str, int],
    budget: ContextBudget,
    max_files: int = 20,
) -> list[str]:
    contained = contain_repo_path(base_path, dir_path)
    if contained is None or not contained.is_dir():
        return []
    blocks: list[str] = []
    try:
        md_files = sorted(
            child.name
            for child in contained.iterdir()
            if child.name.endswith(".md") and child.is_file()
        )
        rel_dir = dir_path.replace("\\", "/").rstrip("/")
        for filename in md_files[:max_files]:
            if not budget.can_open_file(_max_files(limits)):
                budget.emit_summary_once()
                break
            relative_path = f"{rel_dir}/{filename}" if rel_dir else filename
            block = materialize_file(base_path, relative_path, reason, limits, budget)
            if block:
                blocks.append(block)
    except OSError:
        return blocks
    return blocks


def materialize_jsonl_entries(
    base_path: str,
    jsonl_path: str,
    limits: dict[str, int],
    budget: ContextBudget,
) -> list[str]:
    blocks: list[str] = []
    for entry in read_jsonl_entries(base_path, jsonl_path, limits, budget):
        if budget.stopped:
            budget.emit_summary_once()
            break
        if entry["type"] == "directory":
            blocks.extend(
                materialize_directory(
                    base_path, entry["file"], entry["reason"], limits, budget
                )
            )
            continue
        if not budget.can_open_file(_max_files(limits)):
            budget.emit_summary_once()
            break
        block = materialize_file(
            base_path, entry["file"], entry["reason"], limits, budget
        )
        if block:
            blocks.append(block)
    if budget.jsonl_truncated:
        budget.emit_summary_once()
    return blocks


def materialize_artifact(
    base_path: str,
    file_path: str,
    header_label: str,
    reason: str,
    limits: dict[str, int],
    budget: ContextBudget,
) -> str:
    if budget.stopped:
        budget.emit_summary_once()
        return ""
    contained = contain_repo_path(base_path, file_path)
    if contained is None or not contained.is_file():
        return ""
    cap = _effective_read_cap(limits, budget, "max_artifact_bytes")
    try:
        data = _read_capped(contained, cap)
    except OSError:
        return ""
    size = contained.stat().st_size
    artifact_cap = int(limits.get("max_artifact_bytes") or 0)
    truncated_bytes = truncate_utf8(data, artifact_cap)
    content = truncated_bytes.decode("utf-8", errors="replace")
    if artifact_cap > 0 and (len(truncated_bytes) < len(data) or len(data) > artifact_cap):
        content += _truncate_notice(file_path, artifact_cap)
    return budgeted_block(budget, header_label, file_path, content, reason, size)


def build_task_agent_context(
    repo_root: str | Path,
    task_dir: str,
    agent_type: str,
    limits: dict[str, int] | None = None,
) -> str:
    root = str(repo_root)
    resolved_limits = dict(limits or get_context_injection_limits(Path(root)))
    budget = ContextBudget(int(resolved_limits.get("max_total_bytes") or 0))
    jsonl_path = f"{task_dir}/{agent_type}.jsonl"
    materialize_jsonl_entries(root, jsonl_path, resolved_limits, budget)
    artifacts = (
        (
            f"{task_dir}/prd.md",
            f"{task_dir}/prd.md (Requirements)",
            "Requirements document",
        ),
        (
            f"{task_dir}/design.md",
            f"{task_dir}/design.md (Technical Design)",
            "Technical design document",
        ),
        (
            f"{task_dir}/implement.md",
            f"{task_dir}/implement.md (Execution Plan)",
            "Execution plan document",
        ),
    )
    for path, header, reason in artifacts:
        if budget.stopped:
            budget.emit_summary_once()
            break
        materialize_artifact(root, path, header, reason, resolved_limits, budget)
    return budget.payload()
