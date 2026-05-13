# Troubleshooting

This page collects the recurring issues users hit. Each entry lists the symptom, the most likely cause, and a fix.

## Scan returns nothing

- **Cause**: no platform skills directory exists yet, or the central path is empty.
- **Fix**: create at least one platform skills directory (for example `~/.claude/skills/`) or import a skill from Marketplace. SkillPort never creates platform directories on your behalf.

## A skill installed via symlink "disappears"

- **Cause**: the symlink target was deleted outside SkillPort.
- **Fix**: re-run a scan; the broken installation is dropped from the database. Re-install the skill from Central — the canonical files are still under `~/.skillsmanage/skills/`.

## Windows says symlink mode is unavailable

- **Cause**: developer mode is off, or the filesystem does not allow non-admin symlinks.
- **Fix**: switch the install method to **copy** for that platform, or enable Developer Mode in Windows Settings (Privacy & Security → For developers).

## GitHub Marketplace sync hits a 403

- **Cause**: anonymous rate limit, or PAT lacks the right scope.
- **Fix**: add a PAT in Settings → GitHub PAT, or wait for the rate limit window to reset. For private repos, ensure the PAT has the `repo` scope.

## AI Explanation never returns

- **Cause**: misconfigured base URL, expired API key, or provider outage.
- **Fix**: open Settings → AI and re-test the credentials. If the request hangs, switch provider or model and retry.

## SSH target connects but scan is empty

- **Cause**: the remote user has no platform directories (typical on a fresh server) or `$HOME` resolved to an unexpected path.
- **Fix**: verify the remote login lands in the expected home; check that `~/.<platform>/skills/` directories exist on the host.

## After upgrading, the database looks empty

- **Cause**: the database file was moved or the active SSH target switched.
- **Fix**: confirm `~/.skillsmanage/db.sqlite` exists and is readable. If you switched targets, switch back to Local. SkillPort does not destructively migrate data on upgrade.

## Where to go next

- Permanent platform-specific notes: [Platforms](./platforms).
- Configure scan paths and visibility: [Settings](./settings).

---

Last reviewed: 2026-05-04
