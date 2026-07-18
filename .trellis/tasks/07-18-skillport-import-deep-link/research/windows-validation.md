# Windows NSIS / Deep-Link Validation

Date: 2026-07-18 (Asia/Shanghai)

## Final artifact

Command: `pnpm tauri build`

- Result: passed; Tauri produced one NSIS bundle.
- Installer: `D:\Documents\Code\Agents\skills-manage-windows\src-tauri\target\release\bundle\nsis\SkillPort_0.10.14_x64-setup.exe`
- Size: `15205049` bytes
- Modified: `2026-07-18T20:35:04.0644353+08:00`
- SHA256: `C270ED9FB79D959CF9B27A504D836748097B3C2FD28DF1AC1407EAEB22A535DC`
- Install command: exact installer with `/P`
- Installer exit code: `0`
- Installed executable: `C:\Users\lyh\AppData\Local\SkillPort\skillport.exe`

## Scheme registration

Commands:

```powershell
Get-ItemProperty 'HKCU:\Software\Classes\skillport'
Get-ItemProperty 'HKCU:\Software\Classes\skillport\shell\open\command'
```

Observed:

```text
(Default)    = URL:com.bahayonghang.skillport protocol
URL Protocol = <empty registry value, present>
command      = "C:\Users\lyh\AppData\Local\SkillPort\skillport.exe" "%1"
```

No custom NSIS template was used. The installed per-user registry command points to the installed executable and forwards exactly `%1`.

## Cold start

Command:

```powershell
Start-Process 'skillport://import?source=https%3A%2F%2Fgithub.com%2Fowner%2Frepo'
```

Observed on the final installed artifact:

```text
cold_before_pids=
cold_after_pids=17024
cold_after_count=1
path=C:\Users\lyh\AppData\Local\SkillPort\skillport.exe
window_title=SkillPort
```

Screenshot: [windows-cold-start.png](./windows-cold-start.png)

The screenshot was captured six seconds after activation with Win32 `PrintWindow(PW_RENDERFULLCONTENT)`. It shows `/central`, the existing GitHub import wizard at step 1 `Repo URL`, and `https://github.com/owner/repo` prefilled. Preview remains an unclicked button; Preview/Confirm/Import did not run.

## Warm start / single instance / focus

Procedure:

1. Start the installed executable normally without a URI.
2. Record the primary PID and minimize its main window.
3. Run the same `Start-Process` URI command.
4. Sample foreground-window ownership every 10 ms for 2.5 seconds.

Observed:

```text
warm_before_pid=31092
warm_before_count=1
warm_before_minimized=True
warm_after_pids=31092
warm_after_count=1
warm_same_pid=True
warm_after_minimized=False
focus_timeline_samples=158
focus_timeline_hit_count=154
focus_timeline_first_hit_ms=75
focus_timeline_last_hit_ms=2497
```

Screenshot: [windows-warm-start.png](./windows-warm-start.png)

The screenshot shows the same normalized source at wizard step 1. The PID never changed and no second process survived. The primary window restored from minimized and owned the foreground for 154/158 samples, continuously through the final 2497 ms sample. Preview/Confirm/Import remained user actions.

## Runtime log / failed-first evidence

Runtime log: `C:\Users\lyh\.skillsmanage\logs\skillport-2026-07-18.log`

The first installed build exposed a real Windows transport normalization gap:

```text
2026-07-18T12:08:50.768255Z WARN skillport_lib: Rejected cold-start import intent code="unexpected_path"
```

No URI/source/argv was logged. The fix keeps the canonical parser strict and normalizes only the OS-boundary `skillport://import/?...` root slash. Final cold/warm startup entries at `12:36:12` and `12:37:05` contain no deep-link rejection or foreground failure warning.

The Computer Use wrapper was attempted for UI capture but its native pipe was unavailable. Win32 `PrintWindow`, process enumeration, registry reads, minimized-state checks, and foreground-PID sampling were used instead; all commands above ran against the real installed NSIS artifact.
