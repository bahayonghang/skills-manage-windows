# 执行计划
依赖用户批准；可与 session child 独立实施。父最终 canonical 验证依赖实际匹配 pnpm 可用。

1. 读取原 doctor 日志与官方12设置，记录实际命令解析路径，不打印凭据。
2. 在隔离 cache/cwd 验证已存在 CLI 的无引导 probe；先证明原缺陷，原式 probe 不能再次无界自动下载。
3. 修改 doctor 单一边界与 doctor.test.ts；准确版本要求保持不变。
4. `pnpm exec vitest run src/test/scripts/doctor.test.ts src/test/contracts/developerExperienceContract.test.ts`。若 wrapper仍阻塞，可先 `node node_modules/vitest/vitest.mjs run ...`，结果标 direct。
5. 真实不匹配 probe应明确 mismatch而非 bootstrap timeout，且无新cache；匹配10.34.5 probe应通过。缺匹配工具则按 AC3 BLOCKED，列明精确所需版本，不自行安装。
6. `node scripts/check/run-ci.mjs`、`just ci` 在匹配工具链和已授权写入阶段完成；父集成只需统一跑一次，不重复全套。
7. 强模型检查实际副作用、分类、原症状；rules child更新诊断文档和明确工具适用范围。

不以调整全局PATH、清锁文件、升级pin或兼容表作为默认修复。
