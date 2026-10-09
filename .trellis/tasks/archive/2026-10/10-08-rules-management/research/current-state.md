# Rules 当前状态与来源证据

采集日期：2026-10-08。仅保留结构、计数和设计依据，不复制用户规则正文或凭据。

## 本机文件

两个目录均为普通目录，文件均为 UTF-8、LF、无 BOM 的普通文件。Claude 无头部；OMP 有 `alwaysApply: true` 和字符串 `description`。

| 文件 | Claude bytes | OMP bytes | 去除头部和首尾空白后的正文 |
| --- | ---: | ---: | --- |
| anti-patterns.md | 9853 | 9927 | 相同 |
| chinese.md | 1519 | 1596 | 相同 |
| clarity.md | 1278 | 1374 | 相同 |
| durable-context.md | 2042 | 2131 | 相同 |
| english.md | 1039 | 1133 | 相同 |
| waza-routing.md | 1525 | 1622 | 相同 |

检测使用 Python Path.read_bytes、UTF-8 decode、开头 YAML 区块提取以及精确/strip 后正文比较。原始完整文件字节不同。

`omp --version` 输出 `omp/18.8.6`。当前进程没有 `CLAUDE_CONFIG_DIR`、`PI_CODING_AGENT_DIR`、`OMP_PROFILE`、`PI_PROFILE`、`PI_CONFIG_DIR`。Windows `AllowDevelopmentWithoutDevLicense` 为 1，实际文件链接创建仍为 NOT_RUN。

## 当前源码

| 文件 | 已确认行为 | 设计影响 |
| --- | --- | --- |
| `src/components/layout/Sidebar.tsx` | 默认208px、收起56px，共用NavItem；Central在223行附近 | Central后追加Rules |
| `src/App.tsx` | lazy routes和AppShell | Rules使用lazy route |
| `src/index.css` | Tailwind 4、主题、语义字号、字体角色、圆角 | 继承现有tokens |
| `src/lib/displayFont.ts:95` | 默认标题geist、正文jetbrains，用户可改 | 尊重已保存字体 |
| `src/components/ui/textarea.tsx` / `button-variants.ts` | 输入、焦点、按压状态 | 复用primitives |
| `src/components/skill/SkillMarkdownRenderer.tsx` | react-markdown/remark-gfm，detail显示SKILL.md | 复用依赖/样式，避开固定标签 |
| `src/stores/collectionStore.ts` | store调用typed IPC，详情请求有归属号 | 请求归属保护 |
| `src-tauri/src/paths.rs:14` | 旧数据在.skillsmanage | Rules新增专用根 |
| `src-tauri/src/services/installation/fs_util.rs:186` | Windows链接函数是symlink_dir | Rules使用symlink_file |
| `src-tauri/src/services/installation/directory_link.rs` | Skills CLI目录放置使用junction | 不复用目录primitive处理文件 |
| `src-tauri/src/fs_util.rs` / `services/resource_budget.rs` | blocking-FS和已有预算 | 单文件1MiB、扫描2048项、copy256MiB |
| `src-tauri/src/ipc_registry.rs` / `ipc_error/boundary.rs` | 命名策略、类型生成和错误边界 | 注册新命令及已审查错误 |

## 官方资料，已在线检查

- [Claude Code memory](https://code.claude.com/docs/en/memory)：用户rules适用于所有项目；支持链接；头部只读取paths、忽略其他字段。项目外部链接和Cowork另有限制。
- [OMP rulebook pipeline](https://github.com/can1357/oh-my-pi/blob/main/docs/rulebook-matching-pipeline.md)：原生用户rules在active agent下，支持md/mdc，按名称去重；alwaysApply控制完整内容注入。
- [OMP context files](https://github.com/can1357/oh-my-pi/blob/main/docs/context-files.md)：默认.omp/agent，目录覆盖/profile改变原生位置；顶层RULES.md有独立sticky语义。
- [OMP helpers](https://github.com/can1357/oh-my-pi/blob/main/packages/coding-agent/src/discovery/helpers.ts) 与 [native provider](https://github.com/can1357/oh-my-pi/blob/main/packages/coding-agent/src/discovery/builtin.ts)：规则用native glob FileType.File和文件读取，默认不递归。阅读源码未建立本机symlink实际发现证据。
- [Microsoft CreateSymbolicLinkW](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-createsymboliclinkw)：区分文件与目录链接，未提权创建需要对应条件。开发者模式配置不能替代运行验收。

## 检查结果

- 创建任务前 `git status --short --branch -uall`：dev，干净。
- Trellis context/phase/packages：原来无当前任务，另有两个in_progress任务，本任务新建为planning。
- `just doctor`：FAIL，pnpm12.10.1与要求10.34.5不匹配；Node26.7.0、Rust/Cargo1.98.0、just、Git、MSVC target、Tauri CLI探测通过。
- 新页面渲染、文件链接与恢复、just ci、NSIS、provider调用、hosted CI、Linux/macOS：NOT_RUN。
- Claude/OMP新会话加载：UNVERIFIED。

字段兼容和正文共享有当前资料依据。工具实际发现/注入及原生界面需实施验收；OMP未发现链接时返回设计评审，不加复制回退。
