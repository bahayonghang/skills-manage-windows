import { defineConfig } from 'vitepress'
import { navEn } from './nav.en'
import { navZh } from './nav.zh'
import { sidebarEn } from './sidebar.en'
import { sidebarZh } from './sidebar.zh'

// SkillPort docs site config.
// srcDir defaults to the directory passed on CLI: `vitepress dev docs`.
// outDir is relative to srcDir, so it lands at <repo>/dist-docs and never
// collides with the Tauri/Vite app bundle in <repo>/dist.
export default defineConfig({
  title: 'SkillPort',
  description: 'Manage AI agent skills across platforms.',
  cleanUrls: true,
  lastUpdated: true,
  outDir: '../dist-docs',
  base: '/skills-manage-windows/',
  ignoreDeadLinks: 'localhostLinks',
  // Exclude legacy design notes and the planning doc so they do not become
  // public pages now that the site lives at docs/ (no longer docs/site/).
  srcExclude: [
    'desktop-design.md',
    'research-report.md',
    'vitepress-plan.md',
    'codex-handoffs/**',
  ],
  themeConfig: {
    socialLinks: [
      {
        icon: 'github',
        link: 'https://github.com/bahayonghang/skills-manage-windows',
      },
    ],
    search: {
      provider: 'local',
    },
  },
  locales: {
    root: {
      label: 'English',
      lang: 'en-US',
      themeConfig: {
        nav: navEn,
        sidebar: sidebarEn,
      },
    },
    zh: {
      label: '简体中文',
      lang: 'zh-CN',
      themeConfig: {
        nav: navZh,
        sidebar: sidebarZh,
      },
    },
  },
})
