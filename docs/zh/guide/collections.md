# 集合

集合是一组命名的 skills。它用来安装、分享或记录一个完整设置，例如「前端栈」或「代码审查入门包」。

## 它是什么 / 不是什么

| 是 | 不是 |
|----|------|
| 指向中央 skills 的标签。 | skill 文件的存储位置。 |
| 跨平台批量安装的目标。 | 自动同步机制。 |
| 可导出 / 导入的 JSON。 | 内嵌文件的打包格式。 |

skill 文件始终位于 `~/.skillsmanage/skills/`，集合只保存引用。

## 工作流

1. 在 Collections 视图新建集合，填写名称（描述可选）。
2. 通过选择器把中央 skills 加进去。
3. 在集合视图点 **批量安装到**，挑选目标平台。
4. 安装按 skill × 平台执行，使用你选定的方式（symlink 或 copy）。
5. 反复执行批量安装是安全的：已安装的保留，缺失的补齐。

## 导出与导入

集合操作里点 **导出** 保存为 JSON。示例结构（合成值）：

```json
{
  "version": 1,
  "name": "前端栈",
  "description": "前端审查时常用的 skills",
  "skills": [
    "frontend-design",
    "react-best-practices",
    "css-architecture"
  ],
  "createdAt": "2026-04-09T00:00:00.000Z",
  "exportedFrom": "skillport"
}
```

在另一台机器导入同一份 JSON 会重建集合的引用关系。被引用的 skill 必须已经存在于目标机器的中央库；否则导入时会被标记为缺失。

## 何时用集合 vs 单点安装

- **单点安装**：临时、试用新工具、一次性引入某个 skill。
- **集合**：周期性重装、分享给同事、想作为一个整体追踪的场景。

## 下一步

- 在不同机器间迁移 skill 文件：见后续阶段的 Reference 章节。
- 添加新的 skills：[Marketplace](./marketplace)、[GitHub 导入](./github-import)。

---

Last reviewed: 2026-05-04
