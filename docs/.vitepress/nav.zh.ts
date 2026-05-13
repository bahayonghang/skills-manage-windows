import type { DefaultTheme } from 'vitepress'

export const navZh: DefaultTheme.NavItem[] = [
  {
    text: '指南',
    link: '/zh/guide/introduction',
    activeMatch: '/zh/guide/',
  },
  {
    text: '架构',
    link: '/zh/architecture/overview',
    activeMatch: '/zh/architecture/',
  },
  {
    text: '参考',
    link: '/zh/reference/platform-paths',
    activeMatch: '/zh/reference/',
  },
  {
    text: '博客',
    link: '/zh/blog/',
    activeMatch: '/zh/blog/',
  },
  {
    text: '发布',
    link: '/zh/release-notes/',
    activeMatch: '/zh/release-notes/',
  },
  {
    text: 'GitHub',
    link: 'https://github.com/bahayonghang/skills-manage-windows',
  },
]
