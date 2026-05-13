# FAQ

The questions that come up the most.

## General

**Is SkillPort affiliated with Anthropic / OpenAI / GitHub?**
No. SkillPort is an independent unofficial fork of [`iamzhihuix/skills-manage`](https://github.com/iamzhihuix/skills-manage), with no endorsement or sponsorship from any platform vendor.

**What does the data live as?**
A SQLite file at `~/.skillsmanage/db.sqlite` plus the canonical Central skills under `~/.skillsmanage/skills/`. Both directory names are kept for compatibility with existing installations.

**Are AI explanations sent to a vendor?**
Only when you click "Explain" or "Bulk explain". The request goes to the AI provider you configured in Settings. No telemetry runs in the background.

## Installation

**Why does Windows say it cannot create a symlink?**
Symlinks require either Developer Mode or running as admin on Windows. SkillPort detects the failure and falls back to copy mode automatically.

**Why is the macOS app "damaged"?**
Builds are unsigned and Gatekeeper quarantines them. Run `xattr -dr com.apple.quarantine "/Applications/SkillPort.app"` after copying the app to `/Applications`, then launch from Finder.

**Do I need to install Tauri runtime separately?**
No. The bundle is self-contained. Tauri prerequisites are only needed for development builds.

## Skills

**Why does my new skill not show up?**
SkillPort only scans configured directories. Either drop the skill into `~/.skillsmanage/skills/` (Central) or add the parent directory in Settings → Scan Directories.

**Why does the same skill appear twice in Discover?**
Pre-0.10.0 the scanner did not deduplicate shared roots like `.agents/skills`. 0.10.0 collapses them; if you still see duplicates, run a full rescan from Settings.

**How do I remove a skill from every platform at once?**
Open the skill detail page and click "Uninstall from all". The action walks `skill_installations`, removes each link / copy, then deletes Central if you also tick "Delete from Central".

## Marketplace and GitHub Import

**Sync says rate-limited.**
GitHub anonymous requests are limited to 60/hour. Add a Personal Access Token (no scopes required for public repos) in Settings → GitHub.

**Why does my private repo fail to import?**
Use a fine-grained PAT with read access to that repo. Classic PATs work too with the `repo` scope.

## SSH Mode

**Why are remote installs always copy?**
Symlink behavior across SSH varies by filesystem and shell. SkillPort defaults to copy on remote targets to keep behavior predictable. Symlink and remote Discover are not enabled in this version.

**Is the password ever stored in plaintext?**
No. Password-based SSH credentials go to the OS credential store. Private key files are read at connect time and never copied.

## Data Hygiene

**Where is my GitHub PAT stored?**
In `settings` table, key `github.pat`. It is not encrypted at rest by SkillPort. Keep your `~/.skillsmanage` directory protected with the OS user's permissions.

**Can I move SkillPort between machines?**
Use Settings → Data → Export, copy the resulting JSON to the target machine, and run Import. Secrets are stripped on export — re-enter them on the new machine.

Last reviewed: 2026-05-04
