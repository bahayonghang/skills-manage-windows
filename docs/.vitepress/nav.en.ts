import type { DefaultTheme } from 'vitepress'

export const navEn: DefaultTheme.NavItem[] = [
  {
    text: 'Guide',
    link: '/guide/introduction',
    activeMatch: '/guide/',
  },
  {
    text: 'Architecture',
    link: '/architecture/overview',
    activeMatch: '/architecture/',
  },
  {
    text: 'Reference',
    link: '/reference/platform-paths',
    activeMatch: '/reference/',
  },
  {
    text: 'Blog',
    link: '/blog/',
    activeMatch: '/blog/',
  },
  {
    text: 'Releases',
    link: '/release-notes/',
    activeMatch: '/release-notes/',
  },
  {
    text: 'GitHub',
    link: 'https://github.com/bahayonghang/skills-manage-windows',
  },
]
