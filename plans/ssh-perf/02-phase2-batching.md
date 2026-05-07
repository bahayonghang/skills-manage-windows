# 阶段 2 — 批量化扫描

## 目标

把 `scan_ssh_skills_impl` 从 100+ 次 SSH 命令压到 1-3 次。这是最大单点提速。

## 当前路径与问题

```text
现状（伪代码）：
for agent in agents (~30):
  exists(root)                           ← SSH#1..30
  if not exists:
    remote_agent_parent_detected(root)   ← SSH#31..60 (有时)
  scan_ssh_directory(root):
    exists(root)                         ← SSH#61..90
    list_dir(root)                       ← SSH#91..120
    for entry in entries:
      exists(skill_md)                   ← SSH#121..N
      read_file(skill_md)                ← SSH#N+1..2N
```

理论合并后：

```text
单次 ssh 远端跑一个脚本：
  for root in roots: find $root -maxdepth 3 -name SKILL.md -print0
  → 1 次 SSH 拿到全部 (root, skill_md_path) 列表

单次 ssh 远端再跑一个脚本：
  for f in candidates: print "===PATH:$f===" && cat "$f"
  → 1 次 SSH 拿到全部内容

总计 2 次 SSH，独立于 agent 数量与技能数量。
```

## 任务清单

### 2.1 加 `run_remote_script` 通用机制

文件：`src-tauri/src/targets/exec.rs`

现状已经有 `run_command_with_stdin`、`run_script`，但 `run_script` 实现细节未知。要确认其能：
- 通过 stdin 传任意大小脚本
- 返回 stdout 完整内容（不只是 lines）
- 错误时拿到 stderr

如不满足要求，新加：

```text
pub async fn run_bash_script(&self, body: &str) -> Result<String, String>
  → 等价于：ssh host bash --noprofile --norc -s <<< body
```

### 2.2 设计远端探测脚本（POSIX bash）

```bash
#!/usr/bin/env bash
set -uo pipefail
ROOTS=(
  "$HOME/.claude/skills"
  "$HOME/.agents/skills"
  "$HOME/.skillsmanage/skills"
  ...
)
for root in "${ROOTS[@]}"; do
  if [ -d "$root" ]; then
    printf 'ROOT_OK\t%s\n' "$root"
    find "$root" -maxdepth 3 -name SKILL.md -type f -print 2>/dev/null \
      | while IFS= read -r f; do
          printf 'SKILL\t%s\t%s\n' "$root" "$f"
        done
  else
    printf 'ROOT_MISS\t%s\n' "$root"
  fi
done
```

输出协议：

```text
ROOT_OK<TAB>/home/lyh/.claude/skills
SKILL<TAB>/home/lyh/.claude/skills<TAB>/home/lyh/.claude/skills/foo/SKILL.md
SKILL<TAB>/home/lyh/.claude/skills<TAB>/home/lyh/.claude/skills/bar/SKILL.md
ROOT_MISS<TAB>/home/lyh/.kiro/skills
```

Rust 端解析 line by line，构造 `(agent_id 列表, skill_md path)` 映射（agent_id 通过 root 反查）。

### 2.3 设计远端批量读取脚本

收到候选 SKILL.md 列表后，第二次 SSH：

```bash
#!/usr/bin/env bash
set -uo pipefail
PATHS=(
  "/home/lyh/.claude/skills/foo/SKILL.md"
  "/home/lyh/.claude/skills/bar/SKILL.md"
  ...
)
for f in "${PATHS[@]}"; do
  printf '\x01PATH\x02%s\x03\n' "$f"
  if [ -r "$f" ]; then
    cat -- "$f" || true
  fi
  printf '\x04EOF\x05\n'
done
```

用 ASCII 控制字符做分隔避免与 SKILL.md 内容冲突。Rust 端按 `\x04EOF\x05\n` 切片。

如果路径数太多导致 argv 超限：用 `xargs -0` 或脚本分批。设阈值如 1000 路径一批。

### 2.4 改写 `scan_ssh_skills_impl`

文件：`src-tauri/src/services/scanner/mod.rs`

```text
新流程：
1. probe_ssh_target  → 拿 remote_home（已有）
2. 收集所有 agent 的 root 路径，去重，替换 ~ 为 $HOME 后嵌脚本
3. run_bash_script 探测脚本 → parse 输出
4. 按 root 反查 agent，构造 candidates: HashMap<agent_id, Vec<SKILL.md>>
5. 收集所有不重复 SKILL.md 路径
6. run_bash_script 批量读 → parse 内容
7. 走原 parse_skill_md_content + persist 路径
```

要保留的原行为：
- `db::update_agent_detected` 标记 root 是否存在
- `db::delete_stale_skill_installations` 清理消失的安装
- `scanned_root_cache` 等价语义（同 root 多 agent）

### 2.5 probe_ssh_target 5 合 1

文件：`src-tauri/src/targets/exec.rs`

```bash
#!/usr/bin/env bash
printf 'HOME\t%s\n' "$HOME"
printf 'OS\t%s\n' "$(uname -s 2>/dev/null || echo unknown)"
mkdir -p -- "$HOME/.skillsmanage/skills" 2>&1 \
  && printf 'MKDIR_OK\n' \
  || printf 'MKDIR_FAIL\t%s\n' "$?"
```

一次 SSH 替代原 5 次。

### 2.6 本地扫描路径不动

`scan_all_skills_impl`（本地）保持原 fs 路径，本阶段只动 SSH 路径。

## 性能预期

```text
┌────────────────────────────────┬────────────────────────┐
│ 现状                           │ 100-200 次 SSH spawn    │
│ 阶段 2 完成（无连接复用）       │ 2-3 次 SSH spawn        │
│ 阶段 3 完成（russh 复用 session）│ 1 次 TCP 握手 + N 次 channel │
└────────────────────────────────┴────────────────────────┘
```

阶段 2 单独完成时已经接近最优；阶段 3 进一步去掉每次 spawn 的 ssh.exe 启动开销（Windows 上每次 50-100ms）。

## 风险

| 风险                            | 缓解                                       |
|---------------------------------|--------------------------------------------|
| 远端没装 bash（如 BSD/Alpine）   | 探测 OS 后选 sh 子集；阶段 2 先要求 bash    |
| SKILL.md 内容含 `\x04EOF\x05`    | 选更冷僻的分隔（如 base64 of "===EOF==="）  |
| 路径含 tab/换行                  | 用 `\0` 分隔 + `read -d ''`                |
| 远端 find 无 -maxdepth          | 几乎不可能；GNU find / BSD find 都支持      |
| 单脚本输出过大（数 MB）          | 阶段 4 加进度事件，按 root 分批读           |
| 部分 SKILL.md 解析失败           | per-skill try-catch，单条失败不破坏整体     |

## 测试

| 用例                                  | 验证                                  |
|---------------------------------------|---------------------------------------|
| mock SSH 单测：调用次数            | scan_ssh_skills_impl 调 run_command 次数 ≤ 3 |
| 30 agent × 5 技能 集成测试            | 端到端时间 ≤ 10s（家宽）              |
| SKILL.md 内容含特殊字符（控制字符、二进制） | 解析仍正确                            |
| 一个 root 不存在                       | agent.is_detected 仍标 false         |
| 远端 home 含空格                       | 路径仍正确                           |

## 文件改动清单

```text
src-tauri/src/targets/exec.rs              +1 函数 run_bash_script 或确认 run_script 可用
src-tauri/src/services/scanner/mod.rs      ~150 行 重写 scan_ssh_skills_impl
src-tauri/src/services/scanner/ssh_batch.rs +200 行 探测/批读脚本生成 + parser（新文件）
src-tauri/src/services/scanner/tests.rs    +5 用例
```

## 估时

2-3 天工作日。
