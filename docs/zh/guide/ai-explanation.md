# AI 解释

AI 解释会用通俗语言总结一个 skill 在做什么。安装陌生 skill 前可以用它快速过一眼，写自己的 skill 时也可以用它起草说明文档。

## 工作机制

在 skill 详情页点 **解释** 时，SkillPort 会：

1. 读取 SKILL.md（frontmatter + 正文）。
2. 发送到 Settings → AI 中配置的 AI 提供商。
3. 流式接收并实时渲染响应。
4. 按 skill 缓存结果，下次打开秒出。

可以随时重跑解释。重跑会失效缓存并覆盖原结果。

## 支持的提供商

| 提供商 | 默认端点 | 备注 |
|--------|---------|------|
| Anthropic Claude | `api.anthropic.com` | 走 Anthropic 原生 message 格式，跳过 `thinking` block。 |
| GLM（智谱） | `open.bigmodel.cn` | OpenAI 兼容的 chat completions。 |
| MiniMax | `api.minimax.chat` | OpenAI 兼容的 chat completions。 |
| Kimi（Moonshot） | `api.moonshot.cn` | OpenAI 兼容的 chat completions。 |
| DeepSeek | `api.deepseek.com` | OpenAI 兼容的 chat completions。 |
| OpenRouter | `openrouter.ai/api/v1` | 一个 key 连接多家模型的代理。 |
| 自定义 | 可配置 | 任意 OpenAI 兼容端点。 |

提供商列表存于 `src/data/aiProviders.ts`，跟随版本更新；国区与国际端点都有预填。

## 配置

Settings → AI 提供以下字段：

- **提供商** — 选上表中任意一个。
- **API key** — 存在本地 SQLite settings 表，应用本身不做静态加密。
- **模型** — 各提供商的模型名（如 `claude-haiku-4-5`、`glm-4.5-flash`）。
- **API base URL** — 走代理或自托管时可覆盖。
- **temperature** 等可选参数（按提供商支持情况而定）。

切换提供商不会作废已缓存的解释；只有重跑解释才会。

## 隐私

- 只有点 **解释** 时才会发起网络请求。
- 发送内容仅为 skill 文本（不含其他元数据）。
- 无遥测；除了你选择的提供商，不会有数据流向 SkillPort 或其他第三方。

## 下一步

- 与其它配置一并看：[设置](./settings)。
- 翻译平台文案与主题 token：[国际化与主题](./i18n-and-themes)。

---

Last reviewed: 2026-05-04
