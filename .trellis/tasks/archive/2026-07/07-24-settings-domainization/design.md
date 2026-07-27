# Technical Design

## 1. Boundaries

本任务只在两个入口建立域边界：

1. `commands::settings` 的 generic IPC setter：key 分类、值校验、日志分类。
2. `targets` 的持久配置加载：SSH / WSL schema validation、隔离、active-target fallback。

`db::set_setting` / `db::set_settings` 继续保持无策略的 repository primitive，供 target、secret、migration 与 scanner 等内部域使用。generic allowlist 不下沉到 repository。

## 2. Generic Settings Policy

新增一个后端 settings policy 模块，返回内部 `SettingCategory` 与已验证值。初始清单仅覆盖 live renderer 写入：

- `platform_category_visibility`
- `central_update_check_mode_v1`
- `display_*font*` / `body_*font*` 的现有精确 key 与 `font_scale_v1`
- `ai_provider`、`ai_tag_concurrency`、`ai_tag_interval_ms`、`ai_tag_stop_on_rate_limit`
- 现有 AI provider-scoped preference key 族：`ai_region__*`、`ai_model__*`、`ai_api_url__*`、`ai_custom_base_url__*`、`ai_protocol__*`

实现使用精确 key / 受限 prefix+suffix 匹配，不接受任意 `ai_*`。校验规则复用已有前端枚举与范围：更新模式和布尔值严格枚举；数字 parse 后检查现有 UI 范围；JSON preference 反序列化为 typed shape；字体、provider、model、URL 等字符串设合理长度上限并拒绝控制字符。空字符串仅在现有 UI 明确用作“默认/未设置”的字段允许。

错误字符串采用现有 parser 可识别的小写 code：

- `setting_key_forbidden: This setting cannot be changed through the generic settings API.`
- `setting_value_invalid: The setting value is invalid.`

错误不回显自由 key/value。batch 先遍历并收集 category；全部通过后一次调用 `db::set_settings`。operation log details 仅写 `{ categories, keyCount, valueStored }`，单写 subject 也使用 category 而非原始 key。

## 3. Target Configuration Snapshot

在 `targets` 下建立统一配置加载器，读取：

```text
ssh_targets_v1
wsl_targets_v1
active_target_id_v1
target_config_quarantine_v1   # internal metadata, generic IPC 禁写
```

输出一个内部 snapshot：

```text
TargetConfigSnapshot {
  ssh_targets,
  wsl_targets,
  active_target_id,
  quarantine_status,
}
```

每个 domain 先做 JSON shape 解析，再做最小语义 schema 校验：顶层数组、必需字段可反序列化、ID 非空且不等于 `local`、域内 ID 唯一、SSH/WSL 的必需连接字段非空。兼容字段 `credentialKey` / `protectedPassword` 保持可选；不因旧 credential fallback 本身隔离。

### Corruption flow

```text
read three settings
  -> validate SSH and WSL independently
  -> bad domain becomes []
  -> compute SHA-256(raw bytes), byte length, stable reason code
  -> validate active id against Local + surviving IDs
  -> build one HashMap of changed settings
  -> db::set_settings (one transaction)
  -> return recovered snapshot
```

`target_config_quarantine_v1` 为 versioned JSON，只保存每个域最近一次 incident：

```text
TargetConfigQuarantineStatus {
  version: 1,
  incidents: [
    { domain, detectedAt, reasonCode, sourceBytes, sourceSha256 }
  ],
  activeTargetReset: bool
}
```

不得包含 raw JSON 或 serde error text。相同 digest 重复读取不新增无限历史；不同 incident 替换该域的 latest 记录。状态没有本任务内的 clear/export 命令，因此重启后仍可见。

## 4. Startup and Runtime Integration

- DB schema 初始化和 pending operation recovery 后、`AppState` manage 前，同步调用 target snapshot recovery。错误只记录稳定 code，不格式化 raw/source；不得 panic。
- `TargetRegistry::list_targets`、`active_target`、`target_by_id` 和 dedicated CRUD 的读取路径复用 snapshot loader 或其 validated domain helpers，避免启动检查与运行期行为漂移。
- dedicated save functions 继续直接使用 repository helper；generic policy 不拦截。
- 如 active id 指向被隔离域或不存在的 target，snapshot transaction 同时写 `local`。`list_targets` 因而至少返回 Local，健康域仍可列出。

## 5. IPC and Frontend Flow

新增只读 typed command `get_target_config_quarantine_status` 并注册到 Rust handler 与 `src/lib/ipc/commandMap.ts`。返回结构在 Rust/TS 两端使用 camelCase 对齐。

`targetStore.loadTargets()` 并行读取 `list_targets` 与 quarantine status；成功后同时提交 `targets`、`activeTarget`、`quarantineStatus`。后端 recovery 已将损坏配置转成安全 snapshot，因此启动不再依赖被吞掉的 rejection 来表达这一状态。

Settings bindings 把状态传给 `SettingsConnectionsPage` / `RemoteTargetsSettingsSection`。连接 section 顶部使用现有扁平 Settings 视觉语言和 lucide warning icon 渲染 `role="status"` 告警；中英文文案只呈现域、时间、bytes、digest 的短前缀和 Local fallback，不包含 raw 配置。

## 6. Compatibility

- generic getters 及现有 settings DB schema 不变。
- AI secret dedicated commands 与历史 secret migration key 保持原路径；allowlist 不允许 generic 写这些 key。
- 旧 SSH `protectedPassword` 仍可正常加载；quarantine metadata 不复制该字段。
- `list_targets` 返回类型不变，降低对现有 UI / CLI consumer 的影响；状态走独立 command。
- 不引入新 production dependency；SHA-256 复用已有 `sha2`。

## 7. Failure and Rollback

- policy 误漏合法 key：focused UI tests / generic setter inventory test 会失败；补清单与 validator，不放宽为任意 key。
- quarantine transaction 失败：返回 typed DB error并保留原数据，禁止只清空一半；启动记录稳定 code，后续 loader 可重试。
- frontend status command 失败：target 列表仍可加载，store 记录可见错误；不得伪造“无隔离”。
- 回滚代码不会自动恢复已隔离的原始配置，因为原始 blob 按凭据边界不再保存；SHA-256 证据用于诊断而非恢复。

## 8. Security Notes

- raw target JSON 仅在函数局部内存参与 parse/hash，绝不进入 tracing、operation log、IPC、export 或新 settings value。
- serde parse error 只映射到 stable reason code，例如 `invalid_json` / `invalid_schema` / `duplicate_id` / `reserved_id`。
- generic setting logs 不记录 arbitrary key，防止调用方把敏感内容伪装进 key 后持久化。
