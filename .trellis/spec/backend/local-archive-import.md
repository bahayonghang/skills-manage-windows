# Local Archive Import Contract

## Scope

适用于本机 Central 的 `.zip` 技能 preview/import。SSH/WSL 不上传本地 archive；GitHub import 继续使用独立 acquisition 与 wizard 契约。

## Pipeline

```text
bounded read -> inventory/safety validation -> candidate -> preview fingerprint
user confirm -> bounded read once -> fingerprint recheck -> inventory/candidate recheck
-> staging -> Central mutation guard -> backup/swap -> DB upsert -> cleanup
command boundary -> redacted best-effort Operation Log -> safe IPC error
```

## Contracts

- Preview 只读取 archive 和 Central 冲突，不创建 staging、不写 Central/DB/Operation Log。
- Import 必须在任何 staging/写入前比较 preview 的 SHA-256 + byte length，并对同一份 bytes 重跑完整校验。
- Inventory 拒绝 absolute/drive/UNC/traversal/backslash、case/duplicate/prefix collision、symlink、encrypted、unsupported compression，以及 archive/file/entry/expanded/compression-ratio 预算超限。
- Overwrite 在 swap 后失败时先删除 replacement target，再恢复 backup。Rollback helper 返回 typed error；不得吞掉 remove/rename 失败。
- 成功后 archive skill 保持 unknown/local repository assignment，不伪装为 GitHub source。
- Import command 成功/失败各记录一条 `central / local_archive.import`。details 只含 source type、resolution 和计数；失败 error summary 只写 stable domain code。
- Domain error 可保留内部诊断；IPC 必须调用 `to_ipc_error()`，只返回 `local_archive.<code>:<safe summary>`，不得包含绝对路径、entry payload、fingerprint 或 DB/IO 文本。

## Tests Required

- Preview 零写入与 fingerprint mismatch 早退。
- Root/wrapper candidate 与完整安全/预算矩阵。
- Overwrite/rename/skip；强制 DB failure 后旧目录恢复且 staging/backup 清理。
- Unknown repository assignment。
- Operation Log 成功/失败字段与恶意 payload 脱敏。
- IPC path/entry/internal error 不回显 payload。

## Quality Check

- `cargo test local_archive_import`
- `cargo clippy -- -D warnings`
- 搜索 command 边界，确认没有 `.map_err(|e| e.to_string())` 绕过 safe envelope。
- 搜索 Operation Log details，确认没有 archive path、fingerprint 或 raw error。
