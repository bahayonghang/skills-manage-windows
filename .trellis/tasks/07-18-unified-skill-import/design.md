# Design: 统一入口与安全 ZIP 导入

## 1. 前端边界

- `CentralSkillsShell` 只持有一个 Add intent trigger。
- 新 `SkillImportLauncher` 负责选择 `github | local_zip`，并暴露可被 deep-link 复用的 `openImportIntent(intent)`。
- GitHub 分支继续渲染现有 `GitHubRepoImportWizard` 和 store slice。
- ZIP 分支使用独立 `LocalArchiveImportWizard` 与 Zustand store/controller，状态机为 `choose -> preview -> importing -> result`；preview 页的显式 Import 按钮承担用户确认。
- 不在一个 DialogContent 内嵌另一个 wizard；launcher 关闭后再打开所选 wizard。

## 2. IPC 与 DTO

现有 typed commands：

```text
preview_local_skill_archive(archivePath) -> LocalArchivePreview
import_local_skill_archive(archivePath, expectedFingerprint, resolution, renamedSkillId?) -> LocalArchiveImportResult
```

`LocalArchivePreview` 包含 archive display name、`fingerprint { sha256, byteLen }`、候选 skill 元数据、`files[{path, byteLen}]`、总计、冲突；不得暴露解析后的任意绝对目标路径。

Preview 与 import 都先按压缩包预算有界读取 archive bytes。Preview 对这份 bytes 计算 SHA-256；import 接收 expected fingerprint，读取一次当前 archive，比较 SHA-256 + byte length 后，使用同一份 bytes 完成 ZIP inventory、candidate 与 staging 准备。这样既不信任前端文件清单，也不在“先 hash、再重新打开/读取”之间留下 TOCTOU 窗口。仓库已有直接 `sha2` 依赖，只有 `zip` crate 仍属于新增生产依赖审批范围。

## 3. ZIP 解析器

新增 `services/local_archive_import/`，分为：

- `inventory.rs`：读取 central directory、规范化条目、预算与结构冲突检查。
- `candidate.rs`：确定 root/wrapper、读取并复用 scanner frontmatter parser、生成 skill id。
- `preview.rs`：查询 Central 冲突并生成 DTO。
- `import.rs`：重验、staging、mutation guard、backup/swap、DB/Operation Log。
- `error.rs` / `tests.rs`：typed errors 与安全矩阵。

直接 `zip` 依赖已审批并锁定为 `2.4.2`（仅 `deflate`）；本轮不调整依赖。不要复用 shell 解压命令或把前端选中的路径拼进命令行。

## 4. 结构判定

1. 规范化全部 regular-file paths。
2. 若根有 `SKILL.md`，effective root 为 archive root，其他所有安全文件都属于技能包。
3. 否则收集 `*/SKILL.md`；仅当它们全部指向同一个最浅包装目录且没有同级第二候选时，剥离该目录。
4. 其余情况返回 `ambiguous_archive_layout` 或 `no_skill_manifest`。
5. 剥离后再次检查路径重复、前缀冲突、根 `SKILL.md` 和预算。

## 5. 安全矩阵

| 条件 | 行为 |
| --- | --- |
| absolute / traversal / Windows drive / UNC | preview 失败 |
| `a` 与 `A`、`a/b` 与文件 `a` | preview 失败，保证 Windows 一致 |
| symlink / encrypted / unsupported method | preview 失败 |
| 压缩包、条目、展开总量或文件数超预算 | preview 失败并显示可理解错误 |
| preview 后 ZIP 被替换或原地修改 | import 对单次有界读取的 bytes 校验 SHA-256 + byte length；不匹配返回 `archive_changed_since_preview` |
| overwrite 中途失败 | 恢复 backup，数据库保持旧状态 |

## 6. 持久化

使用与 GitHub import 相同的中央 mutation guard/atomic replacement 思路，但不把 ZIP 伪装成 GitHub source。来源保持无 repository assignment / unknown source，默认不新增 schema；现有 `central_updates` 应将其解析为 unsupported，并用回归测试锁定不会产生 GitHub remote-missing/error。Operation Log 仍记录脱敏的 local archive 来源摘要。

## 7. 回滚

- UI 可回退为原 GitHub CTA，GitHub command/store 不受影响。
- ZIP command/service 独立删除即可；不得把共享 GitHub 逻辑改造成 ZIP 专用抽象。
- 若 schema 无法无损表达 archive 来源，先不持久化来源扩展，只保留 Operation Log，不为此扩大迁移范围。

## 8. 2026-07-19 验收修复设计

### 8.1 原子恢复

- 将 target rollback 改为返回 typed `Result`，不再吞掉 remove/rename 失败。
- overwrite 已把新 staging rename 到 target 后若 DB upsert 失败，先删除新 target，再把 backup rename 回原 target；只有没有 backup 时才执行纯 cleanup。
- 删除新 target 失败时保留 backup 并返回稳定 rollback code；恢复 backup 失败时同样保留 backup，不再继续清理可恢复证据。
- 集成测试使用真实临时 Central 目录和可控 DB failure，断言旧文件恢复、新文件消失、staging/backup 清理；rename、skip 和成功 overwrite 也走同一端到端 harness。

### 8.2 Operation Log 与安全错误

- command 层围绕 import service 计时并调用现有 `record_operation_log_best_effort`；成功和失败各记录一条 `central / local_archive.import` 事件。
- 成功 subject 使用最终 skill id/name；details 只含 `sourceType=local_archive`、resolution、file count 和 byte count，不记录 archive path、fingerprint 或文件名 payload。
- 失败日志只写稳定 `error.code()`，不把 `Display` 文本交给 Operation Log。
- IPC error 统一为 `code:safe summary`。所有 enum 变体的 safe summary 均不包含绝对路径、ZIP entry、数据库/IO 或攻击者控制文本。
- 前端复用 `formatBackendError`，为所有 local archive code 提供 `backendErrors.*` 中英文文案；未知 code 使用本地化通用错误，不回退显示 raw backend message。

### 8.3 单一前端状态机

- `localArchiveImportSlice.ts` 提供独立 Zustand store/controller，拥有 step、archive path、preview、resolution、rename、pending、result 和 error code。
- store action 负责 preview/import IPC 与 reset；wizard 只负责 Tauri file picker、调用 store action 和渲染。关闭后 reset，成功后由 store result 驱动 Central refresh 回调。
- launcher 继续只发 `github | local_zip` intent；GitHub wizard 和 deep-link controller 不迁入 ZIP store。

### 8.4 测试边界

- 前端测试覆盖 launcher GitHub/ZIP 分流、remote disabled、file picker cancel、preview、conflict resolutions、import failure/success、reset 和安全 i18n error。
- Rust inventory fixtures 补齐 symlink、encrypted、unsupported method、文件数/压缩字节/展开字节/单文件与压缩比预算。
- Rust integration 覆盖 preview 零写入、fingerprint mismatch、overwrite/rename/skip、DB failure restore、cleanup、无 repository assignment 和成功/失败 Operation Log 脱敏。
