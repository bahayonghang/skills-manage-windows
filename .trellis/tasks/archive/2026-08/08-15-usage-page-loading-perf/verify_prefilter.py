#!/usr/bin/env python3
"""Verify substring pre-filter needles against real session logs.

For every sampled line, compute whether the CURRENT (unfiltered) parser would
produce an effect: capture session_meta, or emit a skill call. If the line has
an effect, the raw line MUST contain the pre-filter needle, otherwise the
filter would drop a parsing-relevant line.

Codex needles: "<skill>" or "session_meta"
Grok updates needle: "<command-name>"
"""
import json
import os
import random
import re
import sys
from pathlib import Path

CODEX_NAME_RE = re.compile(r"<name>([^<]+)</name>")
GROK_CMD_RE = re.compile(r"<command-name>([^<]+)</command-name>")

CODEX_BUILTINS = {
    "exit", "help", "model", "clear", "compact", "undo", "diff", "history",
    "settings", "version", "approve", "status", "imagegen", "openai-docs",
    "plugin-creator", "skill-creator", "skill-installer",
}


def codex_line_effect(line: str) -> str | None:
    """Return 'session_meta' or 'skill' if the unfiltered parser would act."""
    try:
        entry = json.loads(line)
    except Exception:
        return None
    if not isinstance(entry, dict):
        return None
    t = entry.get("type")
    if t == "session_meta":
        return "session_meta"
    if t == "response_item":
        content = (entry.get("payload") or {}).get("content")
        if not isinstance(content, list):
            return None
        for part in content:
            if not isinstance(part, dict) or part.get("type") != "input_text":
                continue
            text = part.get("text") or ""
            if "<skill>" not in text:
                continue
            for m in CODEX_NAME_RE.finditer(text):
                if m.group(1) not in CODEX_BUILTINS:
                    return "skill"
    return None


def grok_updates_line_effect(line: str) -> str | None:
    try:
        record = json.loads(line)
    except Exception:
        return None
    if not isinstance(record, dict):
        return None
    update = ((record.get("params") or {}).get("update") or {})
    if update.get("sessionUpdate") != "user_message_chunk":
        return None
    mc = update.get("content") or {}
    if mc.get("type") != "text":
        return None
    text = mc.get("text") or ""
    if GROK_CMD_RE.search(text):
        return "skill"
    return None


def sample_files(root: Path, per_kind: int) -> list[Path]:
    files = [p for p in root.rglob("*.jsonl") if p.is_file()]
    files.sort(key=lambda p: p.stat().st_size, reverse=True)
    largest = files[:per_kind]
    rest = files[per_kind:]
    random.seed(42)
    random_files = random.sample(rest, min(per_kind, len(rest)))
    return largest + random_files


def verify(root: Path, needle: str, alt_needle: str | None, effect_fn, label: str, sample: int):
    files = sample_files(root, sample)
    total_lines = 0
    relevant = 0
    violations = []
    for path in files:
        try:
            with open(path, "r", encoding="utf-8", errors="replace") as fh:
                for line in fh:
                    line = line.rstrip("\n")
                    if not line.strip():
                        continue
                    total_lines += 1
                    effect = effect_fn(line)
                    if effect is None:
                        continue
                    relevant += 1
                    has_needle = needle in line or (alt_needle is not None and alt_needle in line)
                    if not has_needle:
                        violations.append((str(path), effect, line[:200]))
        except OSError as exc:
            print(f"  unreadable {path}: {exc}")
    print(f"[{label}] files={len(files)} lines={total_lines} relevant={relevant} violations={len(violations)}")
    for path, effect, preview in violations[:5]:
        print(f"  VIOLATION {path} effect={effect}: {preview}")
    return len(violations)


def main():
    home = Path.home()
    bad = 0
    codex_dir = home / ".codex" / "sessions"
    codex_sample = int(os.environ.get("CODEX_SAMPLE", "6"))
    if codex_dir.is_dir():
        bad += verify(codex_dir, "<skill>", "session_meta", codex_line_effect, "codex", sample=codex_sample)
    grok_root = home / ".grok" / "sessions"
    if grok_root.is_dir():
        updates = [p for p in grok_root.rglob("updates.jsonl") if p.is_file()]
        total_lines = relevant = violations = 0
        for path in updates:
            try:
                with open(path, "r", encoding="utf-8", errors="replace") as fh:
                    for line in fh:
                        line = line.rstrip("\n")
                        if not line.strip():
                            continue
                        total_lines += 1
                        if grok_updates_line_effect(line) is None:
                            continue
                        relevant += 1
                        if "<command-name>" not in line:
                            violations += 1
                            if violations <= 5:
                                print(f"  VIOLATION {path}: {line[:200]}")
            except OSError as exc:
                print(f"  unreadable {path}: {exc}")
        print(f"[grok updates] files={len(updates)} lines={total_lines} relevant={relevant} violations={violations}")
        bad += violations
    print("RESULT:", "PASS" if bad == 0 else f"FAIL ({bad} violations)")
    sys.exit(0 if bad == 0 else 1)


if __name__ == "__main__":
    main()
