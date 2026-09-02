# Step 0 — Remeasured untyped IPC baseline

Date: 2026-09-03
HEAD vs audit: **no name drift**. Audit count was 47; remasured count is **47**. The six design batches are disjoint and their union equals this list.

## Scan commands

```powershell
pnpm exec vitest run src/test/contracts/ipcCommandCoverage.test.ts
rg -n "UNTYPED_IPC_COMMANDS" src/lib/ipc/commandMap.ts
rg --glob "*.{ts,tsx}" -n "invoke(?:Raw)?(?:<(?:[^<>]|<[^<>]*>)*>)?\(\s*\"([a-z0-9_]+)\"" src --glob "!src/test/**" --glob "!src/lib/ipc/**"
```

Authoritative names: `src/lib/ipc/commandMap.ts::UNTYPED_IPC_COMMANDS` (already sorted).
Production invoke scanner: `src/test/contracts/ipcCommandCoverage.test.ts::scanInvokedCommands` (excludes `src/test/**` and `src/lib/ipc/**`).

## Count

47

## Sorted names

1. accept_ai_tag_review
2. add_project
3. add_registry
4. add_scan_directory
5. add_skill_to_collection
6. assign_skill_tags
7. assign_skills_to_repository
8. browse_skills_sh_directory
9. bulk_suggest_skill_tags
10. cancel_ai_tag_job
11. create_collection
12. create_or_update_skill_repository
13. create_skill_tag
14. explain_skill
15. explain_skill_stream
16. export_collection
17. get_agents
18. get_app_runtime_info
19. get_collection_detail
20. get_collections
21. get_pending_ai_tag_reviews
22. get_project_skills
23. get_scan_directories
24. get_settings
25. get_skill_explanation
26. get_skill_repositories
27. get_skill_tags
28. list_projects
29. list_projects_using_skill
30. list_registries
31. pick_project_folder
32. preview_delete_central_skills
33. preview_delete_skill_repository
34. read_skills_sh_file
35. record_frontend_runtime_log
36. refresh_skill_explanation
37. rename_project
38. rescan_project
39. resolve_skills_sh_url
40. search_marketplace_skills
41. search_skills_sh
42. set_project_pinned
43. set_scan_directory_active
44. set_settings
45. set_skill_repository_pinned
46. skip_ai_tag_review
47. update_collection

## Six-batch partition (6 + 8 + 7 + 8 + 12 + 6 = 47)

### Batch 1 — Collections (6)

- create_collection — `src/stores/collectionStore.ts`
- get_collections — `src/stores/collectionStore.ts`
- get_collection_detail — `src/stores/collectionStore.ts`
- add_skill_to_collection — `src/stores/collectionStore.ts`
- update_collection — `src/stores/collectionStore.ts`
- export_collection — `src/stores/collectionStore.ts`

### Batch 2 — Projects (8)

- pick_project_folder — `src/stores/projectsStore.ts`
- add_project — `src/stores/projectsStore.ts`
- list_projects — `src/stores/projectsStore.ts`
- rename_project — `src/stores/projectsStore.ts`
- set_project_pinned — `src/stores/projectsStore.ts`
- rescan_project — `src/stores/projectsStore.ts`
- get_project_skills — `src/stores/projectsStore.ts`
- list_projects_using_skill — `src/stores/skillDetailStore.ts`

### Batch 3 — Settings/runtime/scanner (7)

- get_app_runtime_info — `src/stores/appUpdateStore.ts`
- record_frontend_runtime_log — `src/lib/runtimeLogger.ts` (`invokeRaw`)
- get_scan_directories — `src/stores/settingsStore.ts`
- add_scan_directory — `src/stores/settingsStore.ts`
- set_scan_directory_active — `src/stores/settingsStore.ts`
- get_settings — `src/stores/settingsStore.aiSlice.ts`
- set_settings — `src/stores/settingsStore.aiSlice.ts`

Note: `get_app_runtime_info` and `record_frontend_runtime_log` are the two remaining infallible commands (`IpcResult` not used). Codegen currently requires `IpcResult<T>`; batch 3 must teach `ipc_codegen.rs` to export a bare return type without changing runtime policy.

### Batch 4 — Marketplace/skills.sh/agents (8)

- get_agents — `src/stores/centralSkillsStore.listSlice.ts`
- list_registries — `src/stores/marketplaceStore.registrySlice.ts`
- add_registry — `src/stores/marketplaceStore.registrySlice.ts`
- search_marketplace_skills — `src/stores/marketplaceStore.registrySlice.ts`
- search_skills_sh — `src/stores/marketplaceStore.skillsShSlice.ts`
- resolve_skills_sh_url — `src/stores/marketplaceStore.skillsShSlice.ts`
- browse_skills_sh_directory — `src/stores/marketplaceStore.skillsShSlice.ts`
- read_skills_sh_file — `src/stores/marketplaceStore.skillsShSlice.ts`

### Batch 5 — Central repositories/tags/reviews (12)

- preview_delete_central_skills — `src/stores/centralSkillsStore.installSlice.ts`
- preview_delete_skill_repository — `src/stores/centralSkillsStore.installSlice.ts`
- get_skill_repositories — list/install/metadata/update slices
- create_or_update_skill_repository — `src/stores/centralSkillsStore.metadataSlice.ts`
- assign_skills_to_repository — `src/stores/centralSkillsStore.metadataSlice.ts`
- set_skill_repository_pinned — `src/stores/centralSkillsStore.metadataSlice.ts`
- get_skill_tags — list/install/metadata/update slices
- create_skill_tag — `src/stores/centralSkillsStore.metadataSlice.ts`
- assign_skill_tags — `src/stores/centralSkillsStore.metadataSlice.ts`
- get_pending_ai_tag_reviews — list/install/metadata slices
- accept_ai_tag_review — `src/stores/centralSkillsStore.metadataSlice.ts`
- skip_ai_tag_review — `src/stores/centralSkillsStore.metadataSlice.ts`

### Batch 6 — AI explanation/jobs (6)

- bulk_suggest_skill_tags — `src/stores/centralSkillsStore.metadataSlice.ts`
- cancel_ai_tag_job — `src/stores/centralSkillsStore.updateSlice.ts`
- get_skill_explanation — `src/stores/skillDetailStore.ts`, `src/stores/marketplaceStore.githubImportSlice.ts`
- explain_skill — `src/stores/skillDetailStore.ts`
- explain_skill_stream — `src/stores/skillDetailStore.ts`
- refresh_skill_explanation — `src/stores/skillDetailStore.ts`, `src/stores/marketplaceStore.registrySlice.ts`

## Disjointness check

Intersection of any two batches: empty.
Union size: 47.
Unassigned remasured names: none.
Names in a batch but not in remasured baseline: none.
