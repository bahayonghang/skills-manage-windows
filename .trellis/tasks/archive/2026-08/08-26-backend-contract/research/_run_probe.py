"""Isolated skills@1.5.23 capability probe. Does not touch the real user HOME."""

from __future__ import annotations

import json
import os
import shutil
import subprocess
import sys
import tempfile
from datetime import datetime, timezone
from pathlib import Path

PIN = "skills@1.5.23"
NODE = Path(r"D:\GreenSoftware\node\node.exe")
NPX_JS = Path(r"D:\GreenSoftware\node\node_modules\npm\bin\npx-cli.js")
EVIDENCE_DIR = Path(__file__).resolve().parent / "probe-evidence"


def utc_now() -> str:
    return datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def prefix_argv() -> list[str]:
    return [str(NODE), str(NPX_JS), "--yes", f"--package={PIN}", "--", "skills"]


def isolated_env(root: Path) -> dict[str, str]:
    home = root / "home"
    cache = root / "npm-cache"
    prefix = root / "npm-prefix"
    xdg_state = root / "xdg-state"
    xdg_cache = root / "xdg-cache"
    tmp = root / "tmp"
    appdata = home / "AppData" / "Roaming"
    localappdata = home / "AppData" / "Local"
    for path in (home, cache, prefix, xdg_state, xdg_cache, tmp, appdata, localappdata):
        path.mkdir(parents=True, exist_ok=True)
    env = os.environ.copy()
    # Drop inherited user identity so npm/node cannot see the real HOME library.
    for key in list(env):
        upper = key.upper()
        if upper in {
            "HOME",
            "USERPROFILE",
            "HOMEDRIVE",
            "HOMEPATH",
            "APPDATA",
            "LOCALAPPDATA",
            "XDG_STATE_HOME",
            "XDG_CACHE_HOME",
            "TEMP",
            "TMP",
            "TMPDIR",
            "NPM_CONFIG_CACHE",
            "NPM_CONFIG_USERCONFIG",
            "NPM_CONFIG_PREFIX",
            "NPM_CONFIG_GLOBALCONFIG",
        }:
            env.pop(key, None)
    env.update(
        {
            "HOME": str(home),
            "USERPROFILE": str(home),
            "HOMEDRIVE": str(home.drive) or "C:",
            "HOMEPATH": str(home)[2:] if str(home)[1:2] == ":" else str(home),
            "APPDATA": str(appdata),
            "LOCALAPPDATA": str(localappdata),
            "XDG_STATE_HOME": str(xdg_state),
            "XDG_CACHE_HOME": str(xdg_cache),
            "TEMP": str(tmp),
            "TMP": str(tmp),
            "TMPDIR": str(tmp),
            "npm_config_cache": str(cache),
            "npm_config_userconfig": str(home / ".npmrc"),
            "npm_config_prefix": str(prefix),
            "npm_config_update_notifier": "false",
            "npm_config_fund": "false",
            "CI": "1",
        }
    )
    return env


def run_cmd(argv: list[str], env: dict[str, str], cwd: Path, timeout: int) -> dict:
    started = utc_now()
    try:
        completed = subprocess.run(
            argv,
            env=env,
            cwd=str(cwd),
            capture_output=True,
            timeout=timeout,
            check=False,
        )
        return {
            "started_utc": started,
            "finished_utc": utc_now(),
            "argv": argv,
            "returncode": completed.returncode,
            "stdout": completed.stdout.decode("utf-8", "replace"),
            "stderr": completed.stderr.decode("utf-8", "replace"),
            "timed_out": False,
        }
    except subprocess.TimeoutExpired as error:
        stdout = error.stdout.decode("utf-8", "replace") if error.stdout else ""
        stderr = error.stderr.decode("utf-8", "replace") if error.stderr else ""
        return {
            "started_utc": started,
            "finished_utc": utc_now(),
            "argv": argv,
            "returncode": None,
            "stdout": stdout,
            "stderr": stderr,
            "timed_out": True,
        }


def redact(text: str, root: Path) -> str:
    variants = {
        str(root): "<TEMP_ROOT>",
        str(root).replace("\\", "/"): "<TEMP_ROOT>",
        str(root).replace("/", "\\"): "<TEMP_ROOT>",
    }
    redacted = text
    for raw, token in variants.items():
        redacted = redacted.replace(raw, token)
    return redacted


def main() -> int:
    if not NODE.is_file() or not NPX_JS.is_file():
        raise SystemExit("node or npx-cli.js missing; aborting without touching user HOME")
    EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    root = Path(tempfile.mkdtemp(prefix="skills-cli-probe-"))
    env = isolated_env(root)
    records: list[dict] = []
    try:
        records.append(
            {
                "id": "node_version",
                "result": run_cmd([str(NODE), "--version"], env, root, 30),
            }
        )
        records.append(
            {
                "id": "skills_help",
                "result": run_cmd(prefix_argv() + ["--help"], env, root, 180),
            }
        )
        records.append(
            {
                "id": "skills_add_help",
                "result": run_cmd(prefix_argv() + ["add", "--help"], env, root, 180),
            }
        )
        records.append(
            {
                "id": "skills_remove_help",
                "result": run_cmd(prefix_argv() + ["remove", "--help"], env, root, 180),
            }
        )
        # P2/P3 stay fail-closed unless help clearly documents a pinned-SHA
        # and a copy-preserving refresh. No unverified flags are added here.
        payload = {
            "pin": PIN,
            "platform": sys.platform,
            "node_program": str(NODE),
            "npx_js": "<NODE_INSTALL>/node_modules/npm/bin/npx-cli.js",
            "temp_root": "<TEMP_ROOT>",
            "records": records,
        }
        raw_path = EVIDENCE_DIR / "probe-raw.json"
        published = json.loads(redact(json.dumps(payload, ensure_ascii=False), root))
        raw_path.write_text(json.dumps(published, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
        print(raw_path)
        print("TEMP_ROOT", root)
        for item in records:
            result = item["result"]
            print(
                item["id"],
                "rc=",
                result["returncode"],
                "timeout=",
                result["timed_out"],
                "stdout_len=",
                len(result["stdout"]),
                "stderr_len=",
                len(result["stderr"]),
            )
        return 0
    finally:
        shutil.rmtree(root, ignore_errors=True)


if __name__ == "__main__":
    raise SystemExit(main())
