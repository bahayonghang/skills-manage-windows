# Windows updater staging 演练手册

本手册独立于公开 release。它验证从上一稳定 Windows NSIS 安装包升级到 `Release Desktop` 构建候选版本的过程；未公开候选版本绝不能借用公开 `latest` channel。

## 审批门禁

启用 `run_updater_staging_smoke` 前，必须单独批准 staging feed URL、隔离 Windows runner/environment、凭据、候选 SHA、上一稳定版本和回滚目标。workflow 默认关闭，并拒绝 `github.com` feed。本手册不授权创建 tag、GitHub Release、Azure 资源、secret 或 environment。

## 输入与证据

- 在隔离目录中安装并验证上一稳定 NSIS。
- 在获批 HTTPS staging feed 提供候选 `latest.json`、最终签名 NSIS、updater `.sig` 与 checksum。
- 记录上一版本、候选冻结 SHA、SHA256SUMS、Authenticode 结果、updater 签名结果和 staging URL。
- 手动使用带精确 40 位 `rehearsal_ref` 的 `rehearsal` 模式；不得选择 `publish`。

## 执行

1. 确认旧安装包 Authenticode 有效且可启动。
2. 确认候选 NSIS 的 Authenticode、updater 签名、`latest.json` 与 checksum 都对应 staging 字节。
3. 启动旧应用，从 staging feed 请求更新，并等待 passive 安装完成。
4. 重启后确认候选版本与可执行文件路径，保留安装和 updater 日志作为证据。
5. 运行候选版本卸载器，并确认安装目录已删除。

## 回滚

任一验签、启动或版本检查失败即停止。恢复保存的旧安装包和已知正常的 staging metadata；不要移动候选 tag、修改公开 release 或轮换凭据。记录失败步骤并保留候选产物以便诊断。
