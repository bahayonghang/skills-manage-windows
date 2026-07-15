# 共享核心 CLI 设计

## 1. Architecture

```text
src-tauri/src/bin/skillport-cli.rs
  - Clap parsing
  - output / exit-code adapter
              |
              v
src-tauri/src/cli_api/
  - CliContext
  - stable request/response DTO
  - source/ref resolution
              |
              v
existing db + services
  marketplace / github_import / installation / central_skills

Tauri commands --------------------------^ same services
```

Binary 不依赖 `commands::*`。`cli_api` 是唯一新增公共 façade，只暴露 CLI 所需 use case；现有 service 的内部 helper 不因 CLI 被批量改为 `pub`。

## 2. Runtime Context

```rust
pub struct CliContext {
    db: DbPool,
    secrets: Arc<dyn SecretStore>,
    target: ActiveTarget, // Local in MVP
}
```

初始化使用 `paths::app_data_dir()`、`db::create_pool/init_database`、`SystemSecretStore` 与 `ActiveTarget::Local`。CLI 使用 Tokio runtime；如需增加 `macros` / `rt-multi-thread` feature，应验证 Tauri binary size/build 不回归。

CLI 与桌面端共享 SQLite；mutation 通过前置子任务的 `CentralMutationGuard`。首发不做跨进程 UI push notification：运行中的 GUI 可在现有刷新入口重新查询，限制需写入文档。

## 3. Command Model

```text
skillport-cli [--json] [--lang en|zh] skills list
skillport-cli [--json] skills show <uid|slug|name>
skillport-cli [--json] skills search <query> [--limit N]
skillport-cli [--json] skills install <source> [--sync] [--agent ID...] [--method ...]
skillport-cli [--json] skills sync <ref>... | --all [--agent ID...] [--dry-run]
```

`install` source resolver：

1. GitHub `https://...` repo/tree URL → existing GitHub preview/import。
2. `owner/repo@skill` → existing skills.sh resolution/install。
3. 其他输入返回 invalid-source；MVP 不通过 filesystem existence 猜 local source。

安装先 preview，重复项默认返回 structured conflict；只有 `--replace` 才构造 overwrite selection。`--sync` 在 import 成功后调用同一 installation batch use case；部分 Agent 失败返回 partial result 和非零 exit code，不回滚已成功 Agent。

`skills sync` 要求 refs 或 `--all`，避免空参数隐式同步整个中央库。`--dry-run` 返回计划目标、method、skip reason，不写 DB/FS。

## 4. Output Contract

JSON envelope 版本化：

```json
{
  "schemaVersion": 1,
  "ok": true,
  "data": {},
  "warnings": []
}
```

失败：

```json
{
  "schemaVersion": 1,
  "ok": false,
  "error": { "code": "skill.ambiguous", "message": "...", "details": {} }
}
```

JSON code/fields 不本地化；`message` 可本地化但脚本不得依赖。human renderer 使用 CLI 自有小型 message catalog，避免把 CLI 文案放入 service 或前端 React i18n。

建议 exit codes：

| Code | Meaning |
| --- | --- |
| 0 | success |
| 2 | invalid input/source |
| 3 | not found/ambiguous/duplicate decision required |
| 4 | central mutation busy/timeout |
| 5 | partial failure |
| 1 | unexpected/internal failure |

## 5. Shared Service Adaptation

- 查询：新增/复用 DB repository + stable `SkillRef` resolver。
- search：直接调用 `marketplace::search_skills_sh_impl`。
- install：复用 skills.sh/GitHub snapshot、preview 和 import；进度 sink 在 CLI 下为 noop/structured reporter，不依赖 `AppHandle`。
- sync：复用 `services::installation` 的 Local transport 和批量逻辑；不能从 `commands::linker` 复制循环。
- operation logs：CLI mutation 继续写现有 operation log，source/detail 标识 CLI 且遵守 redaction。

若现有 service 签名过度依赖 Tauri，只提取最小 use case 参数或 progress trait；Tauri command 与 CLI 随后都调用该 use case。

## 6. Build And Distribution

- Cargo 增加 Clap 与 CLI binary target。
- `scripts/run-rust-cli.mjs` 或等价脚本提供 `npm run cli --`、build/install；按 Windows PowerShell/Cargo 路径处理，不假设 bash。
- `cargo install` 是 PATH 安装契约。Tauri NSIS 是否将 CLI 写入 PATH 不在 MVP；Windows release gate只要求同一构建可生成并验证 binary。
- README/README_CN 同步命令、Local-only 限制、凭据、duplicate safety、GUI refresh 说明。

## 7. Security And Rollback

- token 只从现有 `SecretStore` 获取；JSON/error/log 不输出 token、private URL credential 或完整响应。
- CLI 失败不得 fallback 到直接 fs copy 或无锁 mutation。
- CLI binary/scripts 可独立回滚，shared service 和 identity/lock 保持桌面端可用。
