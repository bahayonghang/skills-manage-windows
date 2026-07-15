# 构建共享核心 SkillPort CLI

## Goal

交付一个面向用户、脚本和 AI Agent 的 `skillport-cli`。CLI 与桌面端共享现有 Rust service/use case，可从 skills.sh 或 GitHub 安装技能，并安全地同步到本机已启用 Agent，不维护第二套业务逻辑。

目标调用：

```powershell
npm run cli -- skills install vercel-labs/agent-skills@react-best-practices --sync
```

## Background

- `src-tauri/Cargo.toml:9-11` 已将 `skillport_lib` 构建为 `rlib`，具备添加同 package binary 的基础。
- `src-tauri/src/services/marketplace/skills_sh.rs` 已实现 skills.sh 搜索、解析和安装；`services/github_import/` 已实现 GitHub snapshot/preview/import；`services/installation/` 已实现中央技能向 Agent 的安装编排。
- 现有 Tauri command 依赖 `State<AppState>` / `AppHandle`，CLI 不应直接调用 command；需要窄、Tauri-free 的 façade。
- 本任务依赖 `07-15-stable-skill-identity-mutation-lock` 提供稳定 `uid` resolver 与共享 mutation lock。

## Requirements

- **C1 Shared Use Cases**：CLI 和 Tauri IPC 调用同一 service/use case；CLI 不复制 marketplace、GitHub import、installation、database 或 filesystem 编排。
- **C2 Runtime**：新增 `skillport-cli` binary 与本机 `CliContext`，初始化同一 Local SQLite DB、`SystemSecretStore` 和 Local target，不构造 Tauri `AppState`。
- **C3 Commands**：MVP 支持 `skills list`、`skills show`、`skills search`、`skills install` 和 `skills sync`。
- **C4 Sources**：`skills install` 至少支持 `owner/repo@skill` skills.sh shorthand 与 GitHub repo/tree URL；来源判定确定性，不按偶然路径存在性猜测。
- **C5 Sync Semantics**：`install --sync` 将新技能同步到本机已启用 Agent；`skills sync <ref>...` 要求显式 refs 或 `--all`，支持 `--agent`、`--method` 和 `--dry-run`，不使用参考项目的隐式 active preset。
- **C6 Duplicate Safety**：重复技能默认 preview/fail，不静默 overwrite；覆盖必须显式 `--replace`，批量 destructive 操作还需 `--yes`。
- **C7 Identity**：输入支持 uid/slug/唯一 name，JSON 同时返回稳定 `uid` 与兼容 `id`；不得把目录 slug 宣称为稳定 ID。
- **C8 Output**：全局 `--json` 返回版本化 envelope、稳定字段和 error code；人类输出与 JSON 数据分离，stdout 只放成功 payload，stderr 放诊断。
- **C9 Exit Codes**：至少区分 success、invalid input、not found/ambiguous、busy/timeout、partial failure 和 internal error，并由测试锁定。
- **C10 i18n**：JSON 使用 locale-neutral code；human output 使用最小 Rust-side message catalog，支持英文与中文，不在业务 service 硬编码 CLI 文案。
- **C11 Packaging**：提供 `npm run cli -- ...` 开发入口、`cargo install --path src-tauri --bin skillport-cli --locked --force` 安装路径，并在 Windows build 中验证 CLI binary 生成。
- **C12 Local MVP**：首发仅管理 Local target；SSH/WSL CLI、Git backup、snapshot/restore 和 Git remote commands 不在范围内。

## Acceptance Criteria

- [x] `skillport-cli --help`、各命令 help 与中英文 human output 可用；`--json` schema/exit code 通过 tests。
- [x] list/show 可通过 uid、slug、唯一 name 查询同一 Central 技能；多 name 匹配明确失败，平台同名行不干扰。
- [x] search 使用现有 skills.sh service，并支持 `--limit`。
- [x] exact shorthand 与 GitHub URL 安装复用现有 preview/import pipeline，duplicate 默认不覆盖。
- [x] `install --sync` 和显式 `skills sync` 复用 installation service，支持 dry-run、agent 过滤与 Windows symlink→copy fallback。
- [x] CLI mutation 复用 shared lock，busy/timeout 映射为稳定 code/exit 4，不发生无锁写入。
- [x] CLI E2E 使用临时 HOME 空 DB smoke 与离线 snapshot exact-shorthand fixture，通过 import → list/show → duplicate preview → dry-run sync。
- [x] Tauri 现有调用路径保持调用相同 service，完整前端/Rust 回归测试通过。
- [x] `just ci` 与 Windows `pnpm tauri build` 通过，desktop exe、CLI exe 与 NSIS 同轮生成。

## Out Of Scope

- Git backup、Git status/pull/push、snapshot/restore、多设备合并。
- SSH/WSL target CLI、远端凭据交互或跨机器锁。
- preset/scenario 迁移；SkillPort 继续使用 collections 与显式 Agent 选择。
- 首发提供 local-folder/zip 安装、update/remove/adopt/tag/collection 管理；这些可在共享 façade 稳定后追加。

## Dependency

依赖 `07-15-stable-skill-identity-mutation-lock`。该子任务未验收前，本任务不得启动 mutation 命令的最终验收，也不得实现临时 lock/resolver。
