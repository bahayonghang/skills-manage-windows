# 让 doctor 的 pnpm 探测保持只读并恢复标准门禁证据

## Goal
P1。使工具链诊断不触发包管理器自动获取，真实区分版本错配、超时和代码失败；对应父任务第1、2、4项关于证据、根因和工具分工的要求。

## Evidence
`scripts/check/doctor.mjs:44` 在项目 cwd 运行 pnpm --version，5 秒超时；本机全局 pnpm 12.3.4 与项目 10.34.5 不符，自动引导产生用户缓存，原命令 exit 1。详见父 research/test-logs/doctor.md。直接本地 CLI 测试通过不等于 canonical gate 通过。

## Requirements
- R1：doctor 的 pnpm probe 不得下载、自更新、切换全局版本或写全局配置；保留项目既定 10.34.5 pin。
- R2：保留明确版本错配/不可执行/超时错误和有界等待，输出不得声称未发生过的无副作用。
- R3：对已安装匹配/不匹配工具链分别验证；标准 run-ci 和 just ci 的结果与 direct CLI 分开记录。

## Acceptance Criteria
- [x] AC1（R1）：临时 cwd/cache 夹具使用本机实际 pnpm 12.3.4，在不匹配 packageManager 下诊断及时返回，隔离缓存无新增文件、无包获取尝试，父进程环境和仓库字节不变。
- [x] AC2（R1, R2）：doctor.test.ts 覆盖匹配/错配、缺失、超时、脱敏和无全局写副作用；不通过调大超时或去掉版本检查掩盖失败。
- [ ] AC3（R3）：已有 pnpm 10.34.5 可用时匹配 probe 通过并跑标准全门禁；若本机缺该版本，将安装需求独立列为 BLOCKED，不用 pnpm 12 或 direct CLI 结果顶替。

## Implementation evidence (2026-09-06)

Command parse path: `C:\Users\lyh\scoop\shims\pnpm.exe` → `scoop\apps\pnpm\current` junction → **12.3.4**. Official pnpm 12 `pmOnFail` default is `download`; child env `pnpm_config_pm_on_fail=ignore` disables auto version management. Probe args remain `pnpm --version`; timeout stays 5000ms; pin stays `10.34.5`.

Original defect (isolated cache/cwd, mismatching `packageManager: pnpm@10.34.5`, no ignore): empty cwd returned `12.3.4` in 78ms; mismatch `ETIMEDOUT` at 5006ms and wrote `registry.npmjs.org/pnpm.jsonl` plus `package-manager-store` engine locks. Same cwd with ignore returned `12.3.4` in 59ms. User scoop store file count unchanged during that isolated proof.

Product change: `withPnpmReadonlyProbeEnv` injects `pnpm_config_pm_on_fail=ignore` only into the pnpm probe child env. Parent `process.env` is not mutated. Other probes keep the caller env.

AC1/AC2: `doctor.test.ts` real 12.3.4 mismatch probe returned promptly with actual `12.3.4`, isolated cache/home had no new files, repo `package.json` bytes and parent env unchanged. Owned hanging Node subprocess still classifies as `ETIMEDOUT` mismatch. Version check still requires `10.34.5`.

AC3 matching probe: leftover engine `...\package-manager-store\v11\links\@pnpm\exe\10.34.5\...\pnpm.exe` returned `ok` / `10.34.5` with no isolated cache writes. Scoop current was **not** switched. `just ci` / `node scripts/check/run-ci.mjs` were **not** run (parent owns). PATH `pnpm exec` (12.3.4) is **not** canonical: it raised `ERR_PNPM_PACKAGE_MANAGER_REMOVE_MODULES_DIR` and started removing `node_modules/.pnpm` even with ignore. Recovery used pinned 10.34.5 `install --frozen-lockfile` (reused store, downloaded 0, lockfile unchanged).

Vitest: PATH `pnpm exec` blocked. Matching 10.34.5 `pnpm exec vitest run src/test/scripts/doctor.test.ts src/test/contracts/developerExperienceContract.test.ts` exit 0, 20 passed. Same files via `node node_modules/vitest/vitest.mjs run ...` exit 0, 20 passed — labeled **direct**, not a substitute for `just ci`.

## Out of Scope / Approval
planning。当前不安装/删除包、不清理已出现的用户缓存、不升级 Node/pnpm/Rust pin、不修改 package.json/lockfile。本计划批准只授权仓库脚本与测试；用户级工具安装仍需明确授权，不能以 AC3 自动获取。
