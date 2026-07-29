# 实施计划：GitHub 网络边界收敛

## 1. 开始前检查

- [ ] 运行 `python ./.trellis/scripts/task.py start 07-24-net-boundary-ssrf`，确认唯一当前任务为本子任务。
- [ ] 加载 `trellis-before-dev`，阅读 backend error、GitHub preview、frontend store/test 规范。
- [ ] 记录工作区已有 Trellis/tooling 改动并保持不动。

## 2. 结构化 Markdown 预览 IPC

- [ ] 将 store 方法从 `(sourcePath, downloadUrl)` 改为接收当前 `GitHubRepoRef + sourcePath`，更新 wizard 调用和重试回调。
- [ ] 将 command 的 `download_url` 参数替换为 `repo: GitHubRepoRef`；远端 workspace 分支保持原行为。
- [ ] 新增结构化 raw Markdown service helper，验证 repo 字段与 source path，只从固定 endpoint 构造 URL。
- [ ] 更新 IPC/store/component 定向测试，证明伪造 URL 不再进入 payload。

## 3. HTTP 策略与流式预算

- [ ] 为共享 client 配置 5 秒连接超时、30 秒总超时和 `Policy::none()`。
- [ ] 在发送前防御性校验固定 endpoint URL，并保持 PAT 仅发往 direct GitHub。
- [ ] 使用 `bytes_stream()` 实现 metadata/repository-file budgeted reader，保留 content-length 预检与 typed budget error。
- [ ] 保持 mirror rate-limit/transport fallback 分类和错误文案不变。

## 4. 回归测试

- [ ] 纯函数测试：危险 scheme/host/IP/端口/userinfo/fragment/路径字段均拒绝或不可构造。
- [ ] HTTP fixture：3xx 不跟随到第二个 listener，direct 失败后 mirror fallback 保持既有行为。
- [ ] 流式 fixture：无 content-length 的 body 在 cap+1 chunk 返回 `Budget`；不需要完整读取 server body。
- [ ] 认证测试：direct endpoint 带 PAT，三个 mirror endpoint 均不带 PAT。
- [ ] 前端测试：IPC payload 只含 `repo/sourcePath/previewWorkspaceId`。

## 5. 验证梯度

- [ ] `cd src-tauri; cargo test github_import --locked`
- [ ] 运行对应 frontend Vitest 文件。
- [ ] `pnpm typecheck`
- [ ] `pnpm lint`
- [ ] `cd src-tauri; cargo fmt --all -- --check`
- [ ] `cd src-tauri; cargo clippy --all-targets --locked -- -D warnings`
- [ ] `just ci`

## 6. 完成与回滚点

- [ ] 检查 diff 只包含本子任务、必要 spec 与测试；不纳入已有 Trellis runtime/tooling 改动。
- [ ] 若 redirect 禁用导致受控 mirror 回归，先确认该 endpoint 的固定跳转目标并将目标显式加入 endpoint 配置；禁止恢复开放式自动 redirect。
- [ ] 更新 `github-import-preview-contract.md`，运行 `trellis-check`，提交工作改动后归档本子任务并在父任务中登记完成。
