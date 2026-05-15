import type { DefaultTheme } from 'vitepress'

export const sidebarEn: DefaultTheme.SidebarMulti = {
  '/guide/': [
    {
      text: 'Getting Started',
      items: [
        { text: 'Introduction', link: '/guide/introduction' },
        { text: 'Installation', link: '/guide/installation' },
        { text: 'First Run', link: '/guide/first-run' },
      ],
    },
    {
      text: 'Core Workflows',
      items: [
        { text: 'Central Skills', link: '/guide/central-skills' },
        { text: 'Platforms', link: '/guide/platforms' },
        { text: 'Collections', link: '/guide/collections' },
        { text: 'Projects', link: '/guide/projects' },
      ],
    },
    {
      text: 'External Sources',
      items: [
        { text: 'Marketplace', link: '/guide/marketplace' },
        { text: 'GitHub Import', link: '/guide/github-import' },
        { text: 'AI Explanation', link: '/guide/ai-explanation' },
      ],
    },
    {
      text: 'Advanced',
      items: [
        { text: 'SSH Remote', link: '/guide/ssh-remote' },
        { text: 'Settings', link: '/guide/settings' },
        { text: 'i18n and Themes', link: '/guide/i18n-and-themes' },
        { text: 'Troubleshooting', link: '/guide/troubleshooting' },
      ],
    },
  ],
  '/architecture/': [
    {
      text: 'Foundations',
      items: [
        { text: 'Overview', link: '/architecture/overview' },
        { text: 'Frontend', link: '/architecture/frontend' },
        { text: 'Backend', link: '/architecture/backend' },
      ],
    },
    {
      text: 'Reference',
      items: [
        { text: 'IPC Commands', link: '/architecture/ipc-commands' },
        { text: 'Data Model', link: '/architecture/data-model' },
      ],
    },
    {
      text: 'Subsystems',
      items: [
        { text: 'Scanning', link: '/architecture/scanning' },
        { text: 'Installation Engine', link: '/architecture/installation-engine' },
        { text: 'Marketplace Pipeline', link: '/architecture/marketplace-pipeline' },
        { text: 'SSH Mode', link: '/architecture/ssh-mode' },
      ],
    },
  ],
  '/reference/': [
    {
      text: 'Surface',
      items: [
        { text: 'Platform Paths', link: '/reference/platform-paths' },
        { text: 'Skill Protocol', link: '/reference/skill-protocol' },
        { text: 'State Import / Export', link: '/reference/state-import-export' },
      ],
    },
    {
      text: 'Tooling',
      items: [
        { text: 'Shortcuts', link: '/reference/shortcuts' },
        { text: 'CLI: just', link: '/reference/cli-just' },
      ],
    },
    {
      text: 'Vocabulary',
      items: [
        { text: 'Glossary', link: '/reference/glossary' },
        { text: 'FAQ', link: '/reference/faq' },
      ],
    },
  ],
  '/blog/': [
    {
      text: 'Posts',
      items: [
        { text: 'Index', link: '/blog/' },
        { text: 'Desktop Design (2026-04-09)', link: '/blog/2026-04-09-design' },
        { text: 'Skill Protocol Research (2026-04-09)', link: '/blog/2026-04-09-research' },
      ],
    },
  ],
  '/release-notes/': [
    {
      text: 'Releases',
      items: [
        { text: 'Changelog', link: '/release-notes/' },
      ],
    },
  ],
}
