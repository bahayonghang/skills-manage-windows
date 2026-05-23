import type { DefaultTheme } from 'vitepress'

export const sidebarZh: DefaultTheme.SidebarMulti = {
  '/zh/guide/': [
    {
      text: '上手',
      items: [
        { text: '简介', link: '/zh/guide/introduction' },
        { text: '安装', link: '/zh/guide/installation' },
        { text: '首次启动', link: '/zh/guide/first-run' },
      ],
    },
    {
      text: '核心工作流',
      items: [
        { text: '中央技能库', link: '/zh/guide/central-skills' },
        { text: '更新中心', link: '/zh/guide/update-center' },
        { text: '平台', link: '/zh/guide/platforms' },
        { text: '集合', link: '/zh/guide/collections' },
        { text: '项目', link: '/zh/guide/projects' },
      ],
    },
    {
      text: '外部来源',
      items: [
        { text: 'Marketplace', link: '/zh/guide/marketplace' },
        { text: 'GitHub 导入', link: '/zh/guide/github-import' },
        { text: 'AI 解释', link: '/zh/guide/ai-explanation' },
      ],
    },
    {
      text: '进阶',
      items: [
        { text: 'SSH 远程', link: '/zh/guide/ssh-remote' },
        { text: '设置', link: '/zh/guide/settings' },
        { text: '国际化与主题', link: '/zh/guide/i18n-and-themes' },
        { text: '故障排查', link: '/zh/guide/troubleshooting' },
      ],
    },
  ],
  '/zh/architecture/': [
    {
      text: '基础',
      items: [
        { text: '总览', link: '/zh/architecture/overview' },
        { text: '前端', link: '/zh/architecture/frontend' },
        { text: '后端', link: '/zh/architecture/backend' },
      ],
    },
    {
      text: '参考',
      items: [
        { text: 'IPC 命令字典', link: '/zh/architecture/ipc-commands' },
        { text: '数据模型', link: '/zh/architecture/data-model' },
      ],
    },
    {
      text: '子系统',
      items: [
        { text: '扫描机制', link: '/zh/architecture/scanning' },
        { text: '安装引擎', link: '/zh/architecture/installation-engine' },
        { text: 'Marketplace 流水线', link: '/zh/architecture/marketplace-pipeline' },
        { text: 'SSH 模式', link: '/zh/architecture/ssh-mode' },
      ],
    },
  ],
  '/zh/reference/': [
    {
      text: '表面',
      items: [
        { text: '平台路径', link: '/zh/reference/platform-paths' },
        { text: '技能协议', link: '/zh/reference/skill-protocol' },
        { text: '状态导入 / 导出', link: '/zh/reference/state-import-export' },
      ],
    },
    {
      text: '工具',
      items: [
        { text: '快捷键', link: '/zh/reference/shortcuts' },
        { text: 'CLI：just', link: '/zh/reference/cli-just' },
      ],
    },
    {
      text: '词汇',
      items: [
        { text: '术语表', link: '/zh/reference/glossary' },
        { text: '常见问题', link: '/zh/reference/faq' },
      ],
    },
  ],
  '/zh/blog/': [
    {
      text: '文章',
      items: [
        { text: '索引', link: '/zh/blog/' },
        { text: '桌面设计（2026-04-09）', link: '/zh/blog/2026-04-09-design' },
        { text: '技能协议调研（2026-04-09）', link: '/zh/blog/2026-04-09-research' },
      ],
    },
  ],
  '/zh/release-notes/': [
    {
      text: '发布',
      items: [
        { text: '更新日志', link: '/zh/release-notes/' },
      ],
    },
  ],
}
