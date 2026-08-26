# Marketplace、导入与 Skills CLI 操作日志覆盖

状态：**planning**。依赖：`08-26-observability-core-contracts` 完成并冻结 interface。

## Goal

补齐 registry、Marketplace/skills.sh install、GitHub/local imports、portable state 与 Skills CLI 的写操作、
job 和外部副作用审计，同时维持严格的 URL/token/path/content/output 禁止边界。

## Requirements

- M1：registry add/remove/sync、Marketplace/skills.sh install、explanation create/refresh 与显式 AI connection test
  记录；search/browse/read/resolve 和成功 preview 为 runtime-only。
- M2：GitHub PAT set/clear/test、GitHub import 记录 stable action/result；preview/fetch/discard snapshot按 policy明确
  runtime-only 或受控 excluded；绝不记录 token、URL、owner/repo/ref/SHA、source path或响应正文。
- M3：local archive import、portable state export/save/import/cancel 可审计；preview runtime-only；不记录文件路径、
  manifest、state payload、fingerprint或导出内容。
- M4：Skills CLI add/remove/link/unlink/install/export/reveal/cancel 可审计；doctor/list/preview/read为 runtime-only；
  不记录 package command、argv/env、stdout/stderr、source string或 filesystem path。
- M5：long jobs 使用 started/final/cancel/partial；外层用户 command owning row，nested install/import不重复记。
- M6：现有 stable GitHub/import/Skills CLI codes保持 IPC/Operation/Runtime一致，unknown fail closed。

## Acceptance Criteria

- [ ] 本 child 的所有 operation policy entries 有唯一 owning row；成功读取/preview 不产生 Operation row。
- [ ] GitHub/AI credentials 的 set/clear/test只记录是否配置/测试结果，不含 secret或远端 identity。
- [ ] imports/portable state/Skills CLI 的 success/failure/partial/cancel与started/interrupted有 focused tests。
- [ ] URL/ref/SHA/path/manifest/content/command/output/AI prompt-response对抗种子不出现在任何日志/导出。
- [ ] nested Marketplace/import/install helper不产生重复用户 operation，既有 progress/job semantics不变。

## Out of Scope

- 修改 GitHub/Marketplace协议、archive算法、Skills CLI业务流程或 portable-state格式。
- Central core install/delete/update、catalog/projects/Obsidian；由其它 coverage child负责。
- 真实 GitHub/AI/Skills CLI/remote provider调用作为自动验收证据。
