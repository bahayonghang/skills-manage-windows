# Design: 统一入口与安全 ZIP 导入

## 1. 前端边界

- `CentralSkillsShell` 只持有一个 Add intent trigger。
- 新 `SkillImportLauncher` 负责选择 `github | local_zip`，并暴露可被 deep-link 复用的 `openImportIntent(intent)`。
- GitHub 分支继续渲染现有 `GitHubRepoImportWizard` 和 store slice。
- ZIP 分支使用独立 `LocalArchiveImportWizard` 与 `localArchiveImportSlice`，状态机为 `choose -> preview -> confirm -> result`。
- 不在一个 DialogContent 内嵌另一个 wizard；launcher 关闭后再打开所选 wizard。

## 2. IPC 与 DTO

建议新增 typed commands：

```text
preview_local_skill_archive(archivePath) -> LocalArchivePreview
import_local_skill_archive(archivePath, expectedFingerprint, resolution, renamedSkillId?) -> LocalArchiveImportResult
```

`LocalArchivePreview` 应包含 archive display name、`archiveFingerprint { sha256, byteLen }`、候选 skill 元数据、`files[{path, byteLen}]`、总计、冲突；不得暴露解析后的任意绝对目标路径。

Preview 与 import 都先按压缩包预算有界读取 archive bytes。Preview 对这份 bytes 计算 SHA-256；import 接收 expected fingerprint，读取一次当前 archive，比较 SHA-256 + byte length 后，使用同一份 bytes 完成 ZIP inventory、candidate 与 staging 准备。这样既不信任前端文件清单，也不在“先 hash、再重新打开/读取”之间留下 TOCTOU 窗口。仓库已有直接 `sha2` 依赖，只有 `zip` crate 仍属于新增生产依赖审批范围。

## 3. ZIP 解析器

新增 `services/local_archive_import/`，分为：

- `inventory.rs`：读取 central directory、规范化条目、预算与结构冲突检查。
- `candidate.rs`：确定 root/wrapper、读取并复用 scanner frontmatter parser、生成 skill id。
- `preview.rs`：查询 Central 冲突并生成 DTO。
- `import.rs`：重验、staging、mutation guard、backup/swap、DB/Operation Log。
- `error.rs` / `tests.rs`：typed errors 与安全矩阵。

新增 `zip` crate 前必须确认依赖。不要复用 shell 解压命令或把前端选中的路径拼进命令行。

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
