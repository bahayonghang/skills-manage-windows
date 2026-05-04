# SkillPort VitePress 中英文文档站点 · 实施计划

> 版本：v0.1 ｜ 日期：2026-05-04 ｜ 适用仓库：`skills-manage-windows`（包名 `skillport`）

## 一、目标

把现有 `README.md`、`README_CN.md`、`CHANGELOG*.md`、`docs/desktop-design.md`、`docs/research-report.md` 以及散落在源码里的领域知识，沉淀为一个可发布、可搜索、中英对称的 VitePress 文档站。
覆盖三类读者：终端用户（安装/使用/SSH/Marketplace）、贡献者（架构/IPC/数据模型/测试）、二次集成方（数据导入导出/Skill 协议）。

```text
┌──────────┬────────────────────────────────────────────┐
│ 项目     │ 内容                                       │
├──────────┼────────────────────────────────────────────┤
│ 工具链   │ VitePress 1.x + pnpm + Node LTS            │
│ 部署目标 │ GitHub Pages（默认）/ Cloudflare Pages 备选│
│ 双语策略 │ i18n routes：/ → en，/zh/ → zh-CN          │
│ 搜索     │ 内置 minisearch，先本地后 Algolia DocSearch│
│ 主题     │ 默认主题 + 自定义 nav/sidebar，无重型定制  │
└──────────┴────────────────────────────────────────────┘
```

## 二、现状盘点

```text
┌──────────────────────────┬─────────────────────────────┐
│ 已有资产                 │ 处置                        │
├──────────────────────────┼─────────────────────────────┤
│ README.md                │ 拆分为 intro/install/usage  │
│ README_CN.md             │ 同上，作为 zh 镜像          │
│ CHANGELOG.md / .zh.md    │ 直接渲染到 release 章节     │
│ docs/desktop-design.md   │ 拆入 architecture / design  │
│ docs/research-report.md  │ 收入 reference 章节         │
│ docs/codex-handoffs/*    │ 不进站点，保留在仓库内      │
│ src/i18n/locales/*.json  │ 复用为术语对照来源          │
│ CLAUDE.md（项目）        │ 摘录为 contributing 章节    │
└──────────────────────────┴─────────────────────────────┘
```

缺口：缺少 IPC 命令字典、数据库 schema 字段说明、Skill 协议条目、SSH Remote 故障排查、平台路径完整表的英文版。计划阶段二补齐。

## 三、信息架构

中英对称，路径前缀决定语言。Sidebar 按读者动线分四组：上手 → 使用 → 架构 → 参考。

```text
┌──────────────┬──────────────────────────────────────────┐
│ 顶部导航     │ Guide / Architecture / Reference / Blog  │
├──────────────┼──────────────────────────────────────────┤
│ Guide        │ 入门、安装、SSH、Marketplace、设置       │
│ Architecture │ 架构总览、IPC、数据模型、扫描/安装机制   │
│ Reference    │ 平台路径、Skill 协议、CLI/快捷键、术语   │
│ Blog         │ 发布说明、设计决策、调研笔记             │
└──────────────┴──────────────────────────────────────────┘
```

**主线链路**

```text
[读者入口] ──┬── 终端用户 ──┬── 安装 ── 首启动扫描 ── 使用页面
             │              └── Marketplace / SSH / 设置
             ├── 贡献者 ────┬── 架构总览 ── IPC 字典
             │              └── 数据模型 ── 测试与构建
             └── 集成方 ────┬── Skill 协议
                            └── 导入导出 schema
```

## 四、目录结构

文档源码迁移为 `docs/`（与既有 `docs/*.md` 共目录）。VitePress 视 `docs/` 为 srcDir，并通过 `srcExclude` 排除 `desktop-design.md` / `research-report.md` / `vitepress-plan.md` / `codex-handoffs/**`，避免污染既有调研档案。

```text
docs/
├── site/
│   ├── .vitepress/
│   │   ├── config.ts            # 顶层站点配置（含 i18n locales）
│   │   ├── theme/
│   │   │   ├── index.ts
│   │   │   └── custom.css       # 仅覆盖 brand color / 字号
│   │   ├── nav.en.ts            # 英文导航
│   │   ├── nav.zh.ts            # 中文导航
│   │   ├── sidebar.en.ts
│   │   └── sidebar.zh.ts
│   ├── public/
│   │   ├── logo.svg
│   │   └── images/              # 复用根目录 images/*.png
│   ├── index.md                 # 英文首页（hero + features）
│   ├── guide/
│   │   ├── introduction.md
│   │   ├── installation.md
│   │   ├── first-run.md
│   │   ├── central-skills.md
│   │   ├── platforms.md
│   │   ├── collections.md
│   │   ├── discover.md
│   │   ├── marketplace.md
│   │   ├── github-import.md
│   │   ├── ai-explanation.md
│   │   ├── ssh-remote.md
│   │   ├── settings.md
│   │   ├── i18n-and-themes.md
│   │   └── troubleshooting.md
│   ├── architecture/
│   │   ├── overview.md
│   │   ├── frontend.md          # 路由/Stores/组件分层
│   │   ├── backend.md           # commands/services/db 分层
│   │   ├── ipc-commands.md      # 由源码扫描自动汇总
│   │   ├── data-model.md        # schema/* 字段表
│   │   ├── scanning.md
│   │   ├── installation-engine.md
│   │   ├── marketplace-pipeline.md
│   │   └── ssh-mode.md
│   ├── reference/
│   │   ├── platform-paths.md
│   │   ├── skill-protocol.md
│   │   ├── state-import-export.md
│   │   ├── shortcuts.md
│   │   ├── cli-just.md
│   │   ├── glossary.md
│   │   └── faq.md
│   ├── blog/
│   │   ├── index.md
│   │   ├── 2026-04-09-design.md         # 来自 desktop-design.md
│   │   └── 2026-04-09-research.md       # 来自 research-report.md
│   ├── release-notes/
│   │   └── index.md             # 镜像 CHANGELOG.md
│   └── zh/                      # 中文镜像，结构同上
│       ├── index.md
│       ├── guide/...
│       ├── architecture/...
│       ├── reference/...
│       ├── blog/...
│       └── release-notes/index.md
└── （保留：desktop-design.md / research-report.md / codex-handoffs/）
```

## 五、内容映射

每个新章节都给出来源，避免凭空写作。

```text
┌─────────────────────────────┬─────────────────────────────────────────┐
│ 站点章节                    │ 来源                                    │
├─────────────────────────────┼─────────────────────────────────────────┤
│ guide/introduction          │ README.md Overview                      │
│ guide/installation          │ README.md Download + Development        │
│ guide/first-run             │ desktop-design.md 启动扫描流程          │
│ guide/central-skills        │ desktop-design.md 页面 B + Central 概念 │
│ guide/platforms             │ README.md Supported Platforms 表格      │
│ guide/collections           │ desktop-design.md 页面 D                │
│ guide/discover              │ CLAUDE.md Discover 段                   │
│ guide/marketplace           │ CLAUDE.md Marketplace + officialSources │
│ guide/github-import         │ services/github_import 模块逐文件提炼   │
│ guide/ai-explanation        │ services/ai_provider/* + aiProviders.ts │
│ guide/ssh-remote            │ README.md SSH Remote Mode               │
│ guide/settings              │ pages/SettingsView + commands/settings  │
│ architecture/overview       │ desktop-design.md 三、四节              │
│ architecture/frontend       │ src/pages、src/stores、组件分层         │
│ architecture/backend        │ src-tauri/src/commands、services 分层   │
│ architecture/ipc-commands   │ 扫描 #[tauri::command] 自动生成清单     │
│ architecture/data-model     │ src-tauri/src/db/schema/* 各表字段      │
│ architecture/scanning       │ services/scanner/* + commands/scanner   │
│ architecture/installation-* │ services/installation/* 含 ensure_centralized │
│ architecture/marketplace-*  │ services/marketplace + commands/marketplace │
│ architecture/ssh-mode       │ targets/* + commands/targets            │
│ reference/platform-paths    │ README.md 平台表 + research-report 表   │
│ reference/skill-protocol    │ research-report.md 一、二节             │
│ reference/state-import-*    │ desktop-design.md JSON schema 段        │
│ reference/cli-just          │ README.md Just Commands 段              │
│ reference/glossary          │ i18n locales/*.json + CLAUDE.md 命名约定│
│ blog/*                      │ desktop-design.md / research-report.md  │
│ release-notes               │ CHANGELOG.md / CHANGELOG.zh.md          │
└─────────────────────────────┴─────────────────────────────────────────┘
```

## 六、VitePress 配置要点

`docs/.vitepress/config.ts` 关键字段：

```ts
import { defineConfig } from 'vitepress'
import { en } from './nav.en'
import { zh } from './nav.zh'

export default defineConfig({
  srcDir: '.',
  outDir: '../../dist-docs',
  cleanUrls: true,
  lastUpdated: true,
  title: 'SkillPort',
  description: 'Manage AI agent skills across platforms.',
  head: [['link', { rel: 'icon', href: '/logo.svg' }]],
  themeConfig: {
    logo: '/logo.svg',
    search: { provider: 'local' },
    socialLinks: [
      { icon: 'github', link: 'https://github.com/bahayonghang/skills-manage-windows' },
    ],
  },
  locales: {
    root: { label: 'English', lang: 'en-US', themeConfig: en },
    zh:   { label: '简体中文', lang: 'zh-CN', themeConfig: zh },
  },
  ignoreDeadLinks: 'localhostLinks',
})
```

约束：

- 不引入复杂主题包，只覆盖品牌色与代码块字体。
- 站点产物 `dist-docs/` 独立于应用 `dist/`，避免与 Vite/Tauri 构建混淆。
- `pnpm` 脚本新增：`docs:dev` / `docs:build` / `docs:preview`，不污染默认 `dev` / `build`。
- VitePress 自带 markdown-it、shiki、minisearch；mermaid 图通过 `vitepress-plugin-mermaid` 按需引入，不强制。

## 七、双语策略

```text
┌──────────────┬────────────────────────────────────────────┐
│ 维度         │ 规则                                       │
├──────────────┼────────────────────────────────────────────┤
│ 路径         │ 英文为根；中文挂 /zh/                       │
│ 文件结构     │ 中英两套 markdown 目录平级镜像              │
│ 链接互跳     │ themeConfig.localeLinks 自动切换            │
│ 翻译范围     │ guide / reference 全译；architecture 选译   │
│ 不译内容     │ IPC 命令名、字段名、文件路径、命令行示例    │
│ 术语对齐     │ Central Skills/中央技能库、Lobster/龙虾平台 │
│ 维护节奏     │ 英文先行；中文 PR 跟随同 commit 完成        │
└──────────────┴────────────────────────────────────────────┘
```

写作语言要求与 README 现状保持一致：英文用主动语态、短句；中文遵循项目 INTJ 风格，去欧化句式。

## 八、写作与质量约定

- 单文件不超过 800 行；一个主题超长就拆子页。
- 路径在表格里走相对路径，如 `src-tauri/src/commands/scanner.rs`，避免绝对路径。
- 命令行块明确平台（PowerShell / Bash），Windows 优先。
- 截图来自仓库 `images/` 子集，不重新拍摄；新增图必须放 `docs/public/images/`。
- 涉及代码片段时使用真实文件路径，不再造伪代码示例（除已有调研报告里的设计稿）。
- 每篇文末保留一行 "Last reviewed: YYYY-MM-DD"，配合 VitePress lastUpdated。

## 九、实施阶段

```text
┌────┬──────────────────────────────────────┬──────┐
│ 阶 │ 工作                                  │ 周   │
├────┼──────────────────────────────────────┼──────┤
│ P1 │ 站点骨架：config + nav + sidebar      │ 0.5  │
│    │ 首页 + introduction + installation     │      │
│ P2 │ guide 全部章节（含 SSH/Marketplace）  │ 1.0  │
│ P3 │ architecture + ipc-commands 自动化    │ 1.0  │
│ P4 │ reference + blog + release-notes      │ 0.5  │
│ P5 │ 中文镜像：guide/reference 全译        │ 1.0  │
│ P6 │ CI / GitHub Pages / 链接检查 / 截图   │ 0.5  │
└────┴──────────────────────────────────────┴──────┘
```

每阶段产出一次 PR，便于审查。P3 的 IPC 字典和 P5 的中文翻译是潜在风险点，留 buffer。

## P3 落地记录（2026-05-04）

- `docs/site/architecture/` 9 篇英文：`overview` / `frontend` / `backend` / `ipc-commands` / `data-model` / `scanning` / `installation-engine` / `marketplace-pipeline` / `ssh-mode`
- `docs/site/zh/architecture/` 9 篇中文镜像（结构对称，复用同一 `_generated/*.md`）
- `scripts/build-ipc-dict.mjs` 扫 `src-tauri/src/**/*.rs` 中 `#[tauri::command]`，输出 `_generated/ipc-commands.md`：当前 107 条命令
- `scripts/build-schema-table.mjs` 扫 `src-tauri/src/db/schema/*.rs` 中 `CREATE TABLE` / `CREATE INDEX`，输出 `_generated/data-model.md`：当前 19 张表
- `package.json scripts.docs:gen` 串联两个生成器；`docs:dev` / `docs:build` 自动先跑 `docs:gen`，CI 跟随
- `sidebar.en.ts` / `sidebar.zh.ts` 新增 Architecture 三组（基础 / 参考 / 子系统）
- `nav.en.ts` / `nav.zh.ts` 新增 Architecture / 架构 顶导入口

## P4 落地记录（2026-05-04）

- `docs/site/reference/` 7 篇 + `docs/site/zh/reference/` 7 篇：`platform-paths` / `skill-protocol` / `state-import-export` / `shortcuts` / `cli-just` / `glossary` / `faq`
- `docs/site/blog/` 3 篇 + `docs/site/zh/blog/` 3 篇：`index` + 2026-04-09 design + 2026-04-09 research
- `docs/site/release-notes/index.md` + `docs/site/zh/release-notes/index.md`：通过 markdown include 复用根目录 `CHANGELOG.md` / `CHANGELOG.zh.md`
- `sidebar.en.ts` / `sidebar.zh.ts` 新增 reference / blog / release-notes 三组
- `nav.en.ts` / `nav.zh.ts` 顶导新增 Reference / Blog / Releases / 参考 / 博客 / 发布 入口

## P5 落地记录（2026-05-04）

中文翻译并未集中到 P5 阶段——按本仓库实际节奏，P2 / P3 / P4 三个阶段每篇英文落地时同步交付了中文镜像。P5 只剩对齐校验与术语补全：

- 中英文章节数量对齐：guide ×14、architecture ×9、reference ×7、blog ×3、release-notes ×1，共 34 对，全部双语对称。
- 中文路由前缀统一为 `/zh/`，sidebar 与 nav 在 P3 / P4 阶段已同步。
- 术语对照已在 `reference/glossary.md` 与 `zh/reference/glossary.md` 双向声明（Skill ↔ 技能、Lobster ↔ 龙虾、Centralize ↔ 集中化 等）。
- IPC 字典与 schema 字段表是英文源单一生成产物，中英文章节通过相对路径 include 同一份 `_generated/*.md`。

## 站点目录迁移记录（2026-05-04）

为简化路径，把站点 srcDir 从 `docs/site/` 上提到 `docs/`。同步动作：

- `mv docs/site/{.vitepress,architecture,blog,guide,index.md,public,reference,release-notes,zh} docs/`
- `docs/.vitepress/config.ts`：`outDir` 改 `../dist-docs`，新增 `srcExclude` 排除 `desktop-design.md` / `research-report.md` / `vitepress-plan.md` / `codex-handoffs/**`，让现有调研稿不变成站点页面。
- `package.json` scripts：`vitepress dev|build|preview docs/site` → `vitepress dev|build|preview docs`。
- `scripts/build-ipc-dict.mjs` / `scripts/build-schema-table.mjs`：`outDir` 路径里的 `'site'` 节去掉。
- `docs/release-notes/index.md` / `docs/zh/release-notes/index.md`：markdown include 各少一层 `../`。
- `docs/architecture/ipc-commands.md` / 中文镜像：文案里 `docs/site/...` → `docs/...`。
- `README.md` / `README_CN.md` / `docs/guide/installation.md` / 中文镜像：文档路径同步收敛。
- `justfile` 新增 `docs` recipe：`pnpm docs:dev`。

`<!--@include: ./_generated/...-->`（en）和 `<!--@include: ../../architecture/_generated/...-->`（zh）保持原样——它们是 srcDir 内相对路径，不受迁移影响。

- `images/{01-06}.png` 与 `images/app-damaged.png` 复制到 `docs/site/public/images/`，让 markdown 通过 `/images/...` 直链访问。
- `.github/workflows/docs.yml` 在原 `build` job 之后新增 `deploy` job：仅在 `push` 到 `main` 且仓库为 `bahayonghang/skills-manage-windows` 时触发，用 `peaceiris/actions-gh-pages@v4` 把 `dist-docs/` 推到 `gh-pages` 分支，commit message `docs: deploy <sha>`。
- 死链检查复用 VitePress 自带 `ignoreDeadLinks: 'localhostLinks'`：构建时未匹配的本地链接会失败，`pnpm docs:build` 即作为最低门禁。
- 截图按 plan 第八节约束："仓库 `images/` 子集，不重新拍摄"——本轮没有新增图，只复制原图。

## 十、自动化与 CI

```text
┌─────────────────────┬─────────────────────────────────────────┐
│ 任务                │ 实现                                    │
├─────────────────────┼─────────────────────────────────────────┤
│ IPC 命令字典生成    │ scripts/build-ipc-dict.mjs，扫描 rs 源码 │
│                     │ 抽取 #[tauri::command] 与函数签名        │
│ Schema 字段表生成   │ scripts/build-schema-table.mjs，         │
│                     │ 解析 db/schema/*.rs                     │
│ 死链检查            │ vitepress build + lychee 离线扫描        │
│ 拼写/术语校对       │ cspell + project-words 词典             │
│ GitHub Actions      │ docs.yml：PR 跑 build；main 跑 deploy   │
│ 部署                │ peaceiris/actions-gh-pages 推 gh-pages  │
└─────────────────────┴─────────────────────────────────────────┘
```

`pnpm docs:gen` 串联两个生成脚本，写入 `docs/architecture/_generated/` 后被 markdown 引用，避免手工同步。

## 十一、风险与对策

```text
┌──────────────────────────┬──────────────────────────────────┐
│ 风险                     │ 对策                             │
├──────────────────────────┼──────────────────────────────────┤
│ 中英内容漂移             │ PR 模板要求双语同更，CI 检查文件 │
│                          │ 数量与标题对齐                   │
│ IPC/数据模型快速变动     │ 用脚本自动生成；只人工写说明段   │
│ 路径表过期               │ 引用 README.md 表为单一来源      │
│ 部署体积膨胀             │ 不打包应用截图原图，使用压缩版   │
│ 调研报告与现状脱节       │ 仅放入 blog，明确标注日期与状态  │
└──────────────────────────┴──────────────────────────────────┘
```

## 十二、Phase 1 落地清单

P1 必须产出，可直接执行：

入口  `docs/site/.vitepress/config.ts`
      站点配置 + i18n locales

入口  `docs/site/.vitepress/sidebar.en.ts` / `sidebar.zh.ts`
      四组导航（Guide/Architecture/Reference/Blog）

入口  `docs/site/index.md` / `docs/site/zh/index.md`
      Hero + 5 个 feature 卡片

实现  `docs/site/guide/introduction.md` / `installation.md`
      迁移 README 对应段落

配置  `package.json` 新增脚本
      `docs:dev` / `docs:build` / `docs:preview`

配置  `.github/workflows/docs.yml`
      PR 构建 + main 部署到 gh-pages

完成 P1 后即可发布最小可用站点，后续阶段持续注入内容。

---

Last reviewed: 2026-05-04
