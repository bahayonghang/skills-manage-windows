# 重 IO 的 spawn_blocking 约定

## 规则

async 上下文（`#[tauri::command]` 及 services 的 async fn）中：

- **递归遍历、递归拷贝/删除、批量落盘、目录搬迁**必须经 `crate::fs_util::run_blocking_fs` 包装，禁止直接调用同步 `std::fs`。
- 单文件小读写（无循环、≤1 层目录）可豁免，但新增豁免需在 PR 中给出评估理由。
- 全仓唯一包装入口：`src-tauri/src/fs_util.rs`（`services/installation/fs_util.rs` 为兼容 re-export）。禁止自创第二种包装。

## 行为保持要点

包装是行为保持型改造：错误传播路径、提前返回点、循环 continue/break、错误消息文本必须与同步版本逐行等价。闭包捕获需要克隆时，注意不要把整批数据（如全部文件字节）预克隆成双缓冲——逐项「瞬时克隆 → 写入」即可。

## Windows 坑：AppHandle 不得进 blocking 闭包

blocking 闭包**按值持有 `AppHandle`（含 `Option<AppHandle>`）会破坏 Windows 测试二进制**：AppHandle 的 drop-glue 把 tauri/muda 菜单与对话框代码链入测试二进制，引入 `comctl32.dll!TaskDialogIndirect` 导入，而测试二进制无 comctl32 v6 manifest，进程加载即 `STATUS_ENTRYPOINT_NOT_FOUND`（0xc0000139），全部测试无法启动。

正确姿势：进度/事件发射保留在 async 侧（按引用持有 AppHandle），闭包只接管纯 fs 工作。参见 `services/github_import/progress.rs` 内注释。

> 来源任务：06-11-spawn-blocking-io（2026-06-11）
