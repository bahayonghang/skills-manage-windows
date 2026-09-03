#!/usr/bin/env python3
"""Bounded subprocess helper for Trellis task hooks and Linear sync.

One supervision entry point: deadline, incremental stream caps, timeout flag,
Windows Job Object / process-tree cleanup, and truncated diagnostics that never
dump environment or credentials.
"""

from __future__ import annotations

import json
import os
import signal
import subprocess
import threading
import time
from dataclasses import dataclass
from pathlib import Path
from typing import IO, Mapping

CREATE_NO_WINDOW = 0x08000000
CREATE_NEW_PROCESS_GROUP = 0x00000200
JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE = 0x00002000
JobObjectExtendedLimitInformation = 9
PROCESS_TERMINATE = 0x0001
PROCESS_QUERY_LIMITED_INFORMATION = 0x1000
TH32CS_SNAPPROCESS = 0x00000002

HOOK_TIMEOUT_SECONDS = 30.0
LINEARIS_TIMEOUT_SECONDS = 30.0
DEFAULT_STREAM_CAP_BYTES = 65536
DEFAULT_DIAGNOSTIC_BYTES = 512
CLEANUP_WAIT_SECONDS = 5.0

LAST_WINDOWS_CREATION_FLAGS = 0


@dataclass(frozen=True)
class BoundedProcessResult:
    returncode: int | None
    stdout: bytes
    stderr: bytes
    timed_out: bool
    output_truncated: bool
    cleanup_failed: bool


STILL_ACTIVE = 259


def pid_is_running(pid: int) -> bool:
    if pid <= 0:
        return False
    if os.name == "nt":
        import ctypes
        from ctypes import wintypes

        kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)
        kernel32.OpenProcess.argtypes = [wintypes.DWORD, wintypes.BOOL, wintypes.DWORD]
        kernel32.OpenProcess.restype = wintypes.HANDLE
        kernel32.GetExitCodeProcess.argtypes = [wintypes.HANDLE, ctypes.POINTER(wintypes.DWORD)]
        kernel32.GetExitCodeProcess.restype = wintypes.BOOL
        kernel32.CloseHandle.argtypes = [wintypes.HANDLE]
        kernel32.CloseHandle.restype = wintypes.BOOL
        handle = kernel32.OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, False, pid)
        if not handle:
            return False
        try:
            code = wintypes.DWORD()
            if not kernel32.GetExitCodeProcess(handle, ctypes.byref(code)):
                return False
            return int(code.value) == STILL_ACTIVE
        finally:
            kernel32.CloseHandle(handle)
    try:
        os.kill(pid, 0)
    except ProcessLookupError:
        return False
    except PermissionError:
        return True
    except OSError:
        return False
    return True


def _truncate_utf8(data: bytes, cap: int) -> bytes:
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


def format_process_diagnostic(
    result: BoundedProcessResult,
    *,
    label: str,
    limit: int = DEFAULT_DIAGNOSTIC_BYTES,
) -> str:
    status = "timeout" if result.timed_out else "exit"
    text = (
        f"{label} status={status} timed_out={result.timed_out} "
        f"rc={result.returncode} truncated={result.output_truncated} "
        f"cleanup_failed={result.cleanup_failed} "
        f"stdout_bytes={len(result.stdout)} stderr_bytes={len(result.stderr)}"
    )
    encoded = _truncate_utf8(text.encode("utf-8"), max(1, limit))
    return encoded.decode("utf-8", errors="replace")


def classify_json_command(result: BoundedProcessResult) -> str:
    if result.timed_out:
        return "timeout"
    if result.returncode not in (0,):
        return "nonzero"
    text = result.stdout.decode("utf-8", errors="replace").strip()
    if not text:
        return "ok"
    try:
        json.loads(text)
    except json.JSONDecodeError:
        return "invalid_json"
    return "ok"


def _read_stream(
    stream: IO[bytes] | None,
    cap: int,
    sink: bytearray,
    truncated: list[bool],
    stop: threading.Event,
) -> None:
    if stream is None:
        return
    try:
        while not stop.is_set():
            chunk = stream.read(4096)
            if not chunk:
                break
            room = cap - len(sink)
            if room <= 0:
                truncated[0] = True
                stop.set()
                break
            if len(chunk) > room:
                sink.extend(chunk[:room])
                truncated[0] = True
                stop.set()
                break
            sink.extend(chunk)
    except OSError:
        return
    finally:
        try:
            stream.close()
        except OSError:
            pass


class _WindowsJob:
    def __init__(self) -> None:
        import ctypes
        from ctypes import wintypes

        self._ctypes = ctypes
        self._kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)
        self.handle = None
        self.ok = False

        class JOBOBJECT_BASIC_LIMIT_INFORMATION(ctypes.Structure):
            _fields_ = [
                ("PerProcessUserTimeLimit", ctypes.c_int64),
                ("PerJobUserTimeLimit", ctypes.c_int64),
                ("LimitFlags", wintypes.DWORD),
                ("MinimumWorkingSetSize", ctypes.c_size_t),
                ("MaximumWorkingSetSize", ctypes.c_size_t),
                ("ActiveProcessLimit", wintypes.DWORD),
                ("Affinity", ctypes.c_size_t),
                ("PriorityClass", wintypes.DWORD),
                ("SchedulingClass", wintypes.DWORD),
            ]

        class IO_COUNTERS(ctypes.Structure):
            _fields_ = [
                ("ReadOperationCount", ctypes.c_uint64),
                ("WriteOperationCount", ctypes.c_uint64),
                ("OtherOperationCount", ctypes.c_uint64),
                ("ReadTransferCount", ctypes.c_uint64),
                ("WriteTransferCount", ctypes.c_uint64),
                ("OtherTransferCount", ctypes.c_uint64),
            ]

        class JOBOBJECT_EXTENDED_LIMIT_INFORMATION(ctypes.Structure):
            _fields_ = [
                ("BasicLimitInformation", JOBOBJECT_BASIC_LIMIT_INFORMATION),
                ("IoInfo", IO_COUNTERS),
                ("ProcessMemoryLimit", ctypes.c_size_t),
                ("JobMemoryLimit", ctypes.c_size_t),
                ("PeakProcessMemoryUsed", ctypes.c_size_t),
                ("PeakJobMemoryUsed", ctypes.c_size_t),
            ]

        create = self._kernel32.CreateJobObjectW
        create.argtypes = [ctypes.c_void_p, wintypes.LPCWSTR]
        create.restype = wintypes.HANDLE
        self._kernel32.SetInformationJobObject.argtypes = [
            wintypes.HANDLE,
            ctypes.c_int,
            ctypes.c_void_p,
            wintypes.DWORD,
        ]
        self._kernel32.SetInformationJobObject.restype = wintypes.BOOL
        self._kernel32.AssignProcessToJobObject.argtypes = [wintypes.HANDLE, wintypes.HANDLE]
        self._kernel32.AssignProcessToJobObject.restype = wintypes.BOOL
        self._kernel32.TerminateJobObject.argtypes = [wintypes.HANDLE, wintypes.UINT]
        self._kernel32.TerminateJobObject.restype = wintypes.BOOL
        self._kernel32.CloseHandle.argtypes = [wintypes.HANDLE]
        self._kernel32.CloseHandle.restype = wintypes.BOOL
        handle = create(None, None)
        if not handle:
            return
        info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION()
        info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE
        if self._kernel32.SetInformationJobObject(
            handle,
            JobObjectExtendedLimitInformation,
            ctypes.byref(info),
            ctypes.sizeof(info),
        ) == 0:
            self._kernel32.CloseHandle(handle)
            return
        self.handle = handle
        self.ok = True

    def assign(self, process_handle: int) -> bool:
        if not self.ok or self.handle is None:
            return False
        return bool(self._kernel32.AssignProcessToJobObject(self.handle, process_handle))

    def terminate(self) -> bool:
        if not self.ok or self.handle is None:
            return False
        return bool(self._kernel32.TerminateJobObject(self.handle, 1))

    def close(self) -> None:
        if self.handle is not None:
            self._kernel32.CloseHandle(self.handle)
            self.handle = None
            self.ok = False


def _windows_child_pids(root_pid: int) -> list[int]:
    import ctypes
    from ctypes import wintypes

    class PROCESSENTRY32W(ctypes.Structure):
        _fields_ = [
            ("dwSize", wintypes.DWORD),
            ("cntUsage", wintypes.DWORD),
            ("th32ProcessID", wintypes.DWORD),
            ("th32DefaultHeapID", ctypes.c_void_p),
            ("th32ModuleID", wintypes.DWORD),
            ("cntThreads", wintypes.DWORD),
            ("th32ParentProcessID", wintypes.DWORD),
            ("pcPriClassBase", wintypes.LONG),
            ("dwFlags", wintypes.DWORD),
            ("szExeFile", wintypes.WCHAR * 260),
        ]

    kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)
    kernel32.CreateToolhelp32Snapshot.argtypes = [wintypes.DWORD, wintypes.DWORD]
    kernel32.CreateToolhelp32Snapshot.restype = wintypes.HANDLE
    kernel32.Process32FirstW.argtypes = [wintypes.HANDLE, ctypes.POINTER(PROCESSENTRY32W)]
    kernel32.Process32FirstW.restype = wintypes.BOOL
    kernel32.Process32NextW.argtypes = [wintypes.HANDLE, ctypes.POINTER(PROCESSENTRY32W)]
    kernel32.Process32NextW.restype = wintypes.BOOL
    kernel32.CloseHandle.argtypes = [wintypes.HANDLE]
    kernel32.CloseHandle.restype = wintypes.BOOL

    snapshot = kernel32.CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0)
    if snapshot in (0, wintypes.HANDLE(-1).value):
        return []
    entry = PROCESSENTRY32W()
    entry.dwSize = ctypes.sizeof(PROCESSENTRY32W)
    children: dict[int, list[int]] = {}
    try:
        if not kernel32.Process32FirstW(snapshot, ctypes.byref(entry)):
            return []
        while True:
            children.setdefault(int(entry.th32ParentProcessID), []).append(int(entry.th32ProcessID))
            if not kernel32.Process32NextW(snapshot, ctypes.byref(entry)):
                break
    finally:
        kernel32.CloseHandle(snapshot)

    found: list[int] = []
    stack = [root_pid]
    seen = {root_pid}
    while stack:
        current = stack.pop()
        for child in children.get(current, []):
            if child not in seen:
                seen.add(child)
                found.append(child)
                stack.append(child)
    return found


def _windows_terminate_pid(pid: int) -> None:
    import ctypes
    from ctypes import wintypes

    kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)
    kernel32.OpenProcess.argtypes = [wintypes.DWORD, wintypes.BOOL, wintypes.DWORD]
    kernel32.OpenProcess.restype = wintypes.HANDLE
    kernel32.TerminateProcess.argtypes = [wintypes.HANDLE, wintypes.UINT]
    kernel32.TerminateProcess.restype = wintypes.BOOL
    kernel32.CloseHandle.argtypes = [wintypes.HANDLE]
    kernel32.CloseHandle.restype = wintypes.BOOL
    handle = kernel32.OpenProcess(PROCESS_TERMINATE, False, pid)
    if not handle:
        return
    try:
        kernel32.TerminateProcess(handle, 1)
    finally:
        kernel32.CloseHandle(handle)


def _terminate_tree(proc: subprocess.Popen[bytes], job: _WindowsJob | None) -> bool:
    pid = proc.pid
    descendants = _windows_child_pids(pid) if os.name == "nt" and pid else []
    if os.name == "nt":
        for child_pid in reversed(descendants):
            _windows_terminate_pid(child_pid)
        if job is not None:
            job.terminate()
        if pid:
            _windows_terminate_pid(pid)
        try:
            proc.kill()
        except OSError:
            pass
    else:
        try:
            if pid:
                os.killpg(pid, signal.SIGKILL)
        except OSError:
            try:
                proc.kill()
            except OSError:
                pass

    deadline = time.monotonic() + CLEANUP_WAIT_SECONDS
    cleanup_failed = False
    while time.monotonic() < deadline:
        still_running = bool(pid and pid_is_running(pid))
        if os.name == "nt" and pid:
            live_children = [child for child in _windows_child_pids(pid) if pid_is_running(child)]
            for known in descendants:
                if pid_is_running(known):
                    live_children.append(known)
                    _windows_terminate_pid(known)
            still_running = still_running or bool(live_children)
        if proc.poll() is not None:
            children_alive = False
            if os.name == "nt":
                children_alive = any(pid_is_running(child_pid) for child_pid in descendants)
            if not children_alive:
                break
        if os.name == "nt" and still_running:
            if job is not None:
                job.terminate()
            if pid:
                _windows_terminate_pid(pid)
        time.sleep(0.05)
    else:
        cleanup_failed = True
    if pid and pid_is_running(pid):
        cleanup_failed = True
    if os.name == "nt":
        for child_pid in descendants:
            if pid_is_running(child_pid):
                cleanup_failed = True
    return cleanup_failed


def run_bounded_process(
    argv: list[str] | str,
    *,
    cwd: Path | str | None = None,
    env: Mapping[str, str] | None = None,
    timeout_seconds: float,
    max_stdout_bytes: int = DEFAULT_STREAM_CAP_BYTES,
    max_stderr_bytes: int = DEFAULT_STREAM_CAP_BYTES,
    shell: bool = False,
) -> BoundedProcessResult:
    global LAST_WINDOWS_CREATION_FLAGS

    stdout_buf = bytearray()
    stderr_buf = bytearray()
    truncated = [False]
    stop = threading.Event()
    job: _WindowsJob | None = None
    popen_kwargs: dict[str, object] = {
        "args": argv,
        "cwd": str(cwd) if cwd is not None else None,
        "env": dict(env) if env is not None else None,
        "stdout": subprocess.PIPE,
        "stderr": subprocess.PIPE,
        "stdin": subprocess.DEVNULL,
        "shell": shell,
    }
    if os.name == "nt":
        flags = CREATE_NO_WINDOW | CREATE_NEW_PROCESS_GROUP
        LAST_WINDOWS_CREATION_FLAGS = flags
        popen_kwargs["creationflags"] = flags
        job = _WindowsJob()
    else:
        popen_kwargs["start_new_session"] = True

    try:
        proc = subprocess.Popen(**popen_kwargs)  # type: ignore[arg-type]
    except OSError:
        return BoundedProcessResult(
            returncode=None,
            stdout=b"",
            stderr=b"",
            timed_out=False,
            output_truncated=False,
            cleanup_failed=True,
        )

    if os.name == "nt" and job is not None and job.ok:
        handle = getattr(proc, "_handle", None)
        if handle is not None and not job.assign(int(handle)):
            job.close()
            job = None
            try:
                proc.kill()
            except OSError:
                pass
            try:
                proc.wait(timeout=CLEANUP_WAIT_SECONDS)
            except subprocess.TimeoutExpired:
                pass
            return BoundedProcessResult(
                returncode=proc.returncode,
                stdout=b"",
                stderr=b"",
                timed_out=False,
                output_truncated=False,
                cleanup_failed=True,
            )
    elif os.name == "nt":
        job = None

    stdout_thread = threading.Thread(
        target=_read_stream,
        args=(proc.stdout, max_stdout_bytes, stdout_buf, truncated, stop),
        daemon=True,
    )
    stderr_thread = threading.Thread(
        target=_read_stream,
        args=(proc.stderr, max_stderr_bytes, stderr_buf, truncated, stop),
        daemon=True,
    )
    stdout_thread.start()
    stderr_thread.start()

    deadline = time.monotonic() + max(0.05, timeout_seconds)
    timed_out = False
    output_truncated = False
    cleanup_failed = False
    try:
        while True:
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                timed_out = True
                break
            if stop.is_set() and truncated[0]:
                output_truncated = True
                break
            try:
                proc.wait(timeout=min(0.05, remaining))
                break
            except subprocess.TimeoutExpired:
                continue
        output_truncated = output_truncated or truncated[0]
        if timed_out or output_truncated:
            stop.set()
            cleanup_failed = _terminate_tree(proc, job)
        else:
            try:
                proc.wait(timeout=0.2)
            except subprocess.TimeoutExpired:
                cleanup_failed = True
    finally:
        stop.set()
        stdout_thread.join(timeout=1)
        stderr_thread.join(timeout=1)
        if job is not None:
            job.close()

    return BoundedProcessResult(
        returncode=proc.returncode,
        stdout=bytes(stdout_buf),
        stderr=bytes(stderr_buf),
        timed_out=timed_out,
        output_truncated=output_truncated,
        cleanup_failed=cleanup_failed,
    )
