# Central Project Install Visibility Contract

> Established 2026-09-10 (task 09-10-central-skill-project-destinations). Central
> Install-to-project used to copy/symlink onto disk without writing
> `projects` / `project_skill_installations`, so Central cards and
> `list_projects_using_skill` stayed empty after a successful dialog.

## 1. Scope / Trigger

Apply when adding or changing:

- Central / Marketplace / Platform batch install with `project_path`
- `SkillWithLinks` list/page enrichment
- Central skill-card project chips
- `record_project_skill_from_central_install`

Do not use this contract for Projects-page `install_skill_to_project_impl`
(already persists psi) or Obsidian `projectBadge` (single vault name).

## 2. Signatures

```rust
pub struct LinkedProject {
    pub project_id: String,
    pub path: String,
}

pub struct SkillWithLinks {
    pub linked_agents: Vec<String>,
    #[serde(default)]
    pub linked_projects: Vec<LinkedProject>,
    // ...
}

pub async fn list_linked_projects_for_skills(
    pool: &DbPool,
    skill_ids: &[String],
) -> Result<HashMap<String, Vec<LinkedProjectRow>>, sqlx::Error>;

pub async fn record_project_skill_from_central_install(
    pool: &DbPool,
    params: RecordProjectSkillFromCentralInstall<'_>,
) -> Result<(), ProjectsError>;
```

```ts
interface LinkedProject { project_id: string; path: string }
interface SkillWithLinks {
  linked_agents: string[];
  linked_projects?: LinkedProject[]; // missing == []
}

buildCentralSkillCardProps(skill, ctx).linkedProjects
```

## 3. Contracts

- `batch_install_central_skills` with `project_path` writes the filesystem
  through `install_central_skill_to_project`, then records metadata. It
  must not call `install_skill_to_project_impl` (that path would install twice).
- Installed **and** skip-already-exists both call the record helper.
  `fs_changed=true` only for a new local (or remote) materialization.
  Skip + DB failure must not delete the existing target.
- Local: idempotent `add_project_impl`. Remote: register the normalized POSIX
  path; **never** `Path::is_dir` on the desktop host.
- SQL for psi/projects lives in `projects_repo`. Installation calls
  `services/projects`; `central_skills` enrichments call `projects_repo`
  from `skills_with_links_from_rows` (shared by unpaged list and page).
- Empty skill-id list: return `{}` and issue no query. `IN` chunks = 500.
- Dedupe key is `project_id`. Card visible name is `getPathBasename(path)`,
  not `projects.name`. Full path is tooltip-only.
- `install_state` pagination still uses global `linked_agents` only.
- Cards are read-only. No component `invoke()`. Central-only prop;
  `unifiedSkillCardVariants` mutex must reject `linkedProjects` on other variants.
- Compact density must keep chips visible (do not idle-hide with platform icons).

## 4. Validation & Error Matrix

| Condition | Required result |
| --- | --- |
| Local `project_path` missing / not a directory | Existing project-path error; no psi row |
| Metadata write fails after **new** FS write | Compensate: remove new target; restore replaced symlink; return the DB error |
| Metadata write fails on **skip** (`fs_changed=false`) | Return the DB error; leave existing target on disk |
| Remote path | Persist without local `is_dir`; compensate on the remote host, not via local `remove_dir` |
| Duplicate (skill, project, agent) | Upsert psi; list enrichment still one chip per `project_id` |
| Two projects share a basename | Two chips, same label, distinct `title` paths |
| No psi rows | Omit the chip row; do not render an empty folder |

## 5. Good / Base / Bad Cases

- Good: Install `impeccable` to `D:\...\PromptHub` for five agents → one chip `PromptHub`; refresh still shows it; Projects page lists the path.
- Base: Skill with only global `linked_agents` → no project chips; platform footer unchanged.
- Good: Skip-already-exists on a historical copy heals the missing psi row.
- Bad: FS-only Central project install; N+1 `list_projects_using_skill` from each card; reuse Obsidian `projectBadge`; remote `add_project_impl` + local `is_dir`.

## 6. Tests Required

- Repo: empty `skill_ids` issues no query; 501 IDs hit both IN chunks; dedupe by `project_id`.
- Installation: Installed writes project+psi; skip still records; skip + DB failure keeps the target; new-copy DB failure compensates.
- `get_central_skills` / page enrichment returns `linked_projects`.
- UnifiedSkillCard: basename, tooltip, same-basename pair, empty, compact, list+grid.
- `unifiedSkillCardVariants`: platform (and other) variants `@ts-expect-error` on `linkedProjects`.
- Store: install-to-project refresh keeps `linked_projects` on the skill.

## 7. Wrong vs Correct

```rust
// Wrong: filesystem-only Central project install
Ok(InstallOutcome::Installed(result)) // caller never writes psi

// Correct
record_project_skill_from_central_install(pool, RecordProjectSkillFromCentralInstall {
    location: ProjectRecordLocation::Local, // or Remote
    fs_changed: true, // false on skip
    ..
}).await?;
```

```tsx
// Wrong: card-level IPC or Obsidian badge
await invoke("list_projects_using_skill", { skillId });
<UnifiedSkillCard variant="central" projectBadge={name} />

// Correct
<UnifiedSkillCard variant="central" linkedProjects={skill.linked_projects} />
```
