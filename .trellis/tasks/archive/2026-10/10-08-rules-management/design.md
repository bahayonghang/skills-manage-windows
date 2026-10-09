# Rules 技术设计

状态：用户已批准实施。目标：本机 Claude Code 与 Oh My Pi。

## 选择与前提

一个 Rules 页面、一个领域 store、一个 Rust Rules service、两组文件链接。当前六条规则正文相同，OMP 元数据可放在共享文件头部，Claude Code 忽略未知字段。因此每条规则保留一份正文，不生成两个工具的内容副本。

整目录链接是代码量更小的备选，但会影响目标独有文件，也不能逐条启停，故选文件级链接。双工具内容副本会新增同步状态，当前内容不需要该机制。

关键前提是两工具可发现文件 symlink。Claude 字段兼容性有官方依据，OMP 本机版本的链接发现需运行验收；失败则返回规划，不引入 copy 降级。

## 边界与数据流

```text
Sidebar + Rules route
        |
RulesView / RulesList / RuleEditor / RuleTargets
        |
rulesStore
        |
existing typed IPC adapter + registry
        |
commands/rules.rs
        |
services/rules
        |
paths.rs + blocking-FS + existing Local mutation guard
        |
~/.skillport/rules/<name>.md
        ^                         ^
        | file symlink            | file symlink
Claude Code rules              OMP rules
```

- 组件只调用 store，不直接 invoke 或读写文件。
- commands 负责参数、命名 IPC 边界和 Operation Log；业务、路径、写入、备份、链接归 service。
- 正文和恢复记录保存在文件系统，不新增 SQLite 表或改变技能 UID。
- 复用现有 blocking-FS、相对链接路径计算、paths_equivalent 和 Local mutation guard。锁在顶层用例获得，内部不重复加锁。
- `ensure_centralized` 继续服务技能流程；Rules 不调用技能专用中央化逻辑，不改原有 Central migration。
- Windows 原 `create_symlink` 用 symlink_dir；Rules 在自己模块中用 symlink_file。原目录链接与junction行为保持原样，不造通用链接框架。

## 文件布局与解析

| 位置 | 内容 |
| --- | --- |
| `~/.skillport/rules/<name>.md` | 可读、可编辑的中央规则 |
| `~/.skillport/rules/.state/import.json` | 初始化来源指纹、逐文件结果，不存正文 |
| `~/.skillport/rules/.state/operations/<operation-id>.json` | 文件接管/删除恢复记录：固定工具ID、受校验名称、指纹、阶段、备份定位信息 |
| `~/.skillport/rules/.backups/<operation-id>/<tool>/<name>.md` | 原始完整字节备份，不进入规则扫描 |
| `~/.claude/rules/<name>.md` | Claude 文件symlink |
| `~/.omp/agent/rules/<name>.md` | OMP 默认agent目录的文件symlink |

Rules 专用常量、根路径和两个工具的路径策略位于 paths.rs。保留旧 APP_DATA_DIR_NAME=.skillsmanage，规则使用独立新根。

首版使用用户指定的默认配置根。进程设置非空 CLAUDE_CONFIG_DIR 时，Claude 显示自定义根不支持并拒绝修改；PI_CODING_AGENT_DIR、PI_CONFIG_DIR、OMP_PROFILE 或 PI_PROFILE 非空时，OMP 同样拒绝修改。当前进程均未设置这些变量，默认目录已验证。预览/应用间复核该条件，页面显示实际目标目录，不增加任意路径设置。

文件名平铺，以.md结尾、Windows大小写不敏感去重；拒绝分隔符、..、保留名称、尾部点/空格和RULES.md。读写同时执行词法与canonical containment，禁止通过父目录或文件symlink越界；目标槽位按lstat/read_link判定。receipt中的路径重新由固定根及受校验名称构建，不能直接信任JSON中的绝对路径。

## 共享文件与格式范围

- 初始化保留Claude正文UTF-8字节与换行，在正文前添加alwaysApply:true及description的有效YAML头部。完整源文件先保存原字节备份，中央文件与原文件的字节差异必须报告。
- 同名OMP正文经去头部及首尾空白比较相同，保留其description；异文则冲突，不自动合并。
- 新建规则默认alwaysApply:true，描述来自输入或首个标题。编辑仅替换正文，不修改元数据；源文件只读预览包含头部，正文预览排除头部。
- 含paths、globs、condition、astCondition、question、scope、agents、enabled:false或非始终应用设置的源文件标记unsupported并保留。首版不跨工具映射条件语义。
- 原生顶层RULES.md有sticky语义，及RULES/RULES@project名字去重风险，首版不创建这些名称。
- 沿用ResourceBudget：单文件1MiB、扫描2048项、批量256MiB。正文内指令仅为数据，不执行命令。

## IPC接口

请求携带捕获的target_id，后端解析该请求目标，非Local在访问文件前拒绝。tool为固定claude-code/omp枚举，renderer不能指定任意路径。

| 命令 | 参数 | 返回与责任 |
| --- | --- | --- |
| list_rules | target_id | 根/目标位置、摘要、实际链接状态、恢复需求 |
| read_rule | target_id,name | 正文、完整文件、元数据、revision、两工具状态 |
| preview_rules_import | target_id | 新增、相同、冲突、不支持、失败及来源指纹 |
| import_existing_rules | target_id,entries[name,source_fingerprints] | 复核后导入逐项结果，不安装链接 |
| create_rule | target_id,name,description,body | 新详情，已有中央文件则拒绝 |
| save_rule | target_id,name,body,expected_revision | 原子保存详情，外部变化报冲突 |
| set_rule_target_enabled | target_id,name,tool,enabled,expected_destination_fingerprint | 单工具实际结果，普通文件接管须来自已确认指纹 |
| delete_rule | target_id,name,expected_revision | 删除中央内容及已验证链接的结果，原内容可恢复 |
| recover_rule_operation | target_id,operation_id | 按receipt阶段恢复/撤销，不接收任意路径 |

摘要字段：name、title、description、bytes、revision、compatibility及两个工具状态。状态为linked/absent/native_equivalent/conflict/broken/unreadable/unsupported/recovery_required；linked只代表已验证文件入口。

使用rules.*稳定错误码区分非法名称、版本冲突、目标冲突、权限不足、锁忙、非Local、格式不支持、恢复受阻、预算及IO错误。接入现有reviewed codes、IPC边界、中英文错误映射及脱敏。Operation Log仅记工具、操作、数量、终态，不写正文、description或原始异常。

## 初始化与文件生命周期

1. 首次进入只扫描。空库显示导入入口，预览逐名称显示两个来源及冲突。
2. 点击导入后取得Local guard、复核来源指纹、备份原始字节，再用同目录临时文件和原子替换提交中央规则。记录逐项结果；重试只补缺项；原目标文件保留。
3. 启用目标时重新检查槽位。缺失可创建，正确链接幂等返回；普通同文文件只有用户确认接管且指纹匹配时替换。异文文件、外部链接保持原样。
4. 在移除原文件前，创建并检查临时文件symlink，验证备份已落盘，并持久化恢复记录。文件原子替换提交链接后，symlink_metadata、read_link、路径等价和内容回读全部通过才记录完成。Windows文件替换以原生故障测试确认，不用shell删除或copy降级。
5. 停用只移除确实指向期望中央文件的文件链接。断链仅在其规范化目标及receipt所有权均匹配时可解除。普通文件和外部链接不删。
6. 保存在锁内检查revision，只替换中央文件。外部变化返回冲突，store留草稿；工具侧链接不变。
7. 删除先备份中央文件并预检查两目标，无冲突才解除本功能链接。部分失败保留中央内容并标记恢复；所有链接处理完成后才删除中央文件。
8. 中断留下receipt。下次进入报告恢复需求；恢复持锁并复核当前指纹。外部变化不覆盖，受阻时保留正文、备份、receipt。

接管确认列出原文件和备份位置，删除确认列出两个工具影响。恢复说明是否恢复原普通文件。备份不自动清理，不把备份目录分发给工具。

## 前端

- /rules为lazy route；Sidebar的Central Skills后插入FileText图标及sidebar.rules，复用NavItem。
- rulesStore拥有列表、详情、草稿、dirty、revision、每工具pending、导入快照和恢复状态；捕获target及请求序号，旧请求不覆盖新目标/选择。
- 明确保存，无输入自动保存，失败保留正文。切换规则前提供保存、放弃、取消确认。草稿留在独立store中，离开页面或切到非Local不清除、不保存；返回Local恢复草稿并复查磁盘revision。当前入口为BrowserRouter，不迁移全局router或新增通用导航阻断层。会话草稿不跨应用退出持久化，编辑区明确显示未保存。
- 列表是平面行，右侧正文/预览和两工具行；分别滚动。复用Textarea、Button和现有react-markdown/remark-gfm与样式，预览无SKILL.md固定标签，不增加编辑器依赖。
- 导入/接管/删除使用现有Dialog；普通编辑在页面内完成。布局详见ui-design.md。

## 兼容与回退

撤回代码不删除新规则及备份。恢复普通文件需要receipt校验对应链接与备份，只解除本功能链接；外部变化时停止，无整目录递归删除。

规则于工具重新加载或新会话生效；OMP provider禁用/名称遮蔽、Claude Cowork外部链接限制不由SkillPort自动改配置。文件系统、工具发现/加载、原生UI、安装包、provider调用与hosted CI分别报告证据状态。
