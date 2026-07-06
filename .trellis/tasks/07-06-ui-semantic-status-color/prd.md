# 语义状态色与 token 纪律收敛

## Goal

消灭绕过 `statusTone.ts` token 体系的原生调色板残留：SourceMeta 的装饰性彩虹（唯一系统性违例）、`statusAccent` API 的原生色名判别值、以及 `destructive/5` 透明度漂移。修复后语义色随 6 主题 × 14 accent 正确换肤。

## Confirmed Facts

- `src/lib/statusTone.ts:1-14` 是项目语义色契约：success/warning/info/error 走 `--success/--warning/--info/--destructive` token（随主题换肤），**明文禁止 `dark:text-amber-300` 式二元适配**；标准 tint 为 `bg-{tone}/10`（必要时 /15）。
- `src/components/central/updateCenter/SourceMeta.tsx:25-51`：`ROW_STYLES` 给 repository/path/url/hash 四类字段分别涂 `bg-sky-500/10 + text-sky-700 dark:text-sky-300`、emerald、cyan、violet 原生调色板 + `dark:` 二元适配。字段类型 ≠ 状态，这是无语义装饰彩虹；固定色相不随主题换肤（Latte/Claude Light 下违和）。同文件 cache 行（`:41-45`）已是正确中性样板：`bg-muted/35 ring-border/60 + text-muted-foreground + text-foreground/90`。
- `src/components/skill/UnifiedSkillCard.tsx:99,232`：`statusAccent?: "amber" | "red"` 用原生色名做判别值；产源 `src/lib/centralSkillCardStatus.ts:4-10`（`statusAccentOf`：update_available→"amber"，remote_missing/error→"red"）；传播链 `src/components/central/centralSkillCardProps.ts:50-62`、`UnifiedSkillCard.tsx:252,366,479`、`src/test/UnifiedSkillCard.test.tsx:169`。
- `src/components/skill/SkillDetailPreview.tsx:108`：AI 解释错误盒手写 `border-destructive/30 bg-destructive/5`，偏离 statusTone 标准（`statusChipClass.error` = `border-destructive/30 bg-destructive/10 text-destructive`）。

## Requirements

- SourceMeta 五类行统一样式：全部采用 cache 行的中性样板（或 `statusTone` 类，若某行确有状态语义）；字段区分靠 `<dt>` 标签文字，不靠色相。删除 `ROW_STYLES` 中全部原生调色板与 `dark:` 类。
- `statusAccent` 判别值从 `"amber" | "red"` 改为 `"warning" | "error"`（对齐 `StatusTone` 命名），同步改 `statusAccentOf`、`centralSkillCardProps.ts`、`UnifiedSkillCard.tsx` 内部映射与测试；映射到的实际类继续走 warning/destructive token（视觉不变）。
- `SkillDetailPreview.tsx:108` 错误盒改用 `statusChipClass.error`（或至少把 `/5` 提到 `/10`）。

## Acceptance Criteria

- [ ] `Grep 'dark:text-(sky|emerald|cyan|violet)-'` 与 `Grep 'bg-(sky|emerald|cyan|violet)-500/'` 在 `src/`（排除 `src/test/` 与 `src/lib/tagColor.ts`）0 命中。（2026-07-06 裁决：`tagColor.ts` 的 `TAG_SCHEMES` 是刻意设计的 10 色标签**身份**色板——按名称 hash 区分用户 tag，注释明确不依赖 data-accent 以避免全部同色——属内容区分色而非状态色，statusTone 四 tone 无法承载，豁免不改。）
- [ ] `statusAccent` 类型与 `statusAccentOf` 返回值不再含 `"amber"`/`"red"` 字面量；`pnpm test -- src/test/UnifiedSkillCard.test.tsx` 通过。
- [ ] Latte 与 Claude Light 主题下目视检查更新中心 SourceMeta：无固定冷色块违和，随主题换肤。
- [ ] `pnpm typecheck && pnpm lint` 通过；收尾跑 `just ci`。

## Out Of Scope

- SourceMeta 信息结构重排（行序、字段增减）。
- 其他页面新增状态色场景。
- `input.tsx` / `button-variants.ts` 里 shadcn 自带的 `dark:bg-input/30` 等结构性表面微调（非状态色，可辩护，不动）。
