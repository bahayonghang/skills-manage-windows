use super::*;
#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use crate::secrets::{
        MockSecretStore, SecretError, SecretStorageState, SecretStore, GITHUB_PAT_SECRET_KEY,
    };
    use crate::services::resource_budget::ResourceBudget;
    use flate2::{write::GzEncoder, Compression};
    use serde_json::Value;
    use std::collections::HashMap;
    use tempfile::tempdir;

    async fn setup_test_db() -> DbPool {
        let (pool, dir) = crate::test_support::file_pool().await;
        // 历史行为：泄漏 TempDir 让 db 文件活过测试生命周期。
        std::mem::forget(dir);
        pool
    }

    fn sample_frontmatter(name: &str, description: &str) -> String {
        format!("---\nname: {name}\ndescription: {description}\n---\n\n# {name}\n")
    }

    fn planning_with_files_like_skill() -> String {
        r#"---
name: planning-with-files-zh
description: Plan with task_plan.md, findings.md, and progress.md files.
hooks:
  UserPromptSubmit:
    - hooks:
        - type: command
          command: "echo '---BEGIN PLAN DATA---'"
metadata:
  version: "1.0.0"
---

# planning-with-files-zh
"#
        .to_string()
    }

    fn repo_snapshot(files: &[(&str, String)]) -> GitHubRepoSnapshot {
        GitHubRepoSnapshot {
            files: files
                .iter()
                .map(|(path, content)| (path.to_string(), content.as_bytes().to_vec()))
                .collect::<HashMap<_, _>>(),
        }
    }

    fn root_repo_snapshot() -> GitHubRepoSnapshot {
        repo_snapshot(&[
            (
                "SKILL.md",
                sample_frontmatter("twitterapi-io", "root skill"),
            ),
            ("README.md", "# repo\n".to_string()),
        ])
    }

    fn multi_skill_snapshot() -> GitHubRepoSnapshot {
        repo_snapshot(&[
            (
                "skills/agent-planner/SKILL.md",
                sample_frontmatter("Agent Planner", "Agent Planner description"),
            ),
            (
                "skills/commit/SKILL.md",
                sample_frontmatter("Commit", "Commit description"),
            ),
            (
                "skills/code-review/SKILL.md",
                sample_frontmatter("Code Review", "Code Review description"),
            ),
            ("skills/commit/README.md", "# commit\n".to_string()),
        ])
    }

    fn namespaced_skill_snapshot() -> GitHubRepoSnapshot {
        repo_snapshot(&[
            (
                "skills/.curated/openai-docs/SKILL.md",
                sample_frontmatter("openai-docs", "OpenAI docs skill"),
            ),
            (
                "skills/.curated/openai-docs/references/api.md",
                "# api\n".to_string(),
            ),
            (
                "skills/.system/skill-creator/SKILL.md",
                sample_frontmatter("skill-creator", "Create skills"),
            ),
            (
                "skills/.system/skill-creator/scripts/init_skill.py",
                "print('hi')\n".to_string(),
            ),
        ])
    }

    fn content_skills_snapshot() -> GitHubRepoSnapshot {
        repo_snapshot(&[
            (
                "content/skills/development-workflows/code-auditor/SKILL.md",
                sample_frontmatter("code-auditor", "Audit code"),
            ),
            (
                "content/skills/development-workflows/code-auditor/references/checklist.md",
                "# checklist\n".to_string(),
            ),
            (
                "content/skills/git-github-collaboration/git-commit/SKILL.md",
                sample_frontmatter("git-commit", "Commit changes"),
            ),
            ("README.md", "# repo\n".to_string()),
        ])
    }

    fn agent_path_snapshot() -> GitHubRepoSnapshot {
        repo_snapshot(&[
            (
                ".agents/skills/universal-skill/SKILL.md",
                sample_frontmatter("universal-skill", "Universal skill"),
            ),
            (
                ".claude/skills/claude-skill/SKILL.md",
                sample_frontmatter("claude-skill", "Claude skill"),
            ),
            (
                ".codex/skills/codex-skill/SKILL.md",
                sample_frontmatter("codex-skill", "Codex skill"),
            ),
        ])
    }

    fn recursive_fallback_snapshot() -> GitHubRepoSnapshot {
        repo_snapshot(&[
            (
                "packages/example/skill/SKILL.md",
                sample_frontmatter("fallback-skill", "Fallback skill"),
            ),
            (
                "node_modules/example/ignored/SKILL.md",
                sample_frontmatter("ignored-node-module", "Ignored"),
            ),
            (
                "target/example/ignored/SKILL.md",
                sample_frontmatter("ignored-target", "Ignored"),
            ),
        ])
    }

    fn compound_plugin_like_snapshot() -> GitHubRepoSnapshot {
        repo_snapshot(&[
            (
                "plugins/compound-engineering/skills/ce-work/SKILL.md",
                sample_frontmatter("ce-work", "Real plugin skill"),
            ),
            (
                "tests/fixtures/custom-paths/custom-skills/custom-skill/SKILL.md",
                sample_frontmatter("custom-skill", "Fixture custom skill"),
            ),
            (
                "tests/fixtures/custom-paths/skills/default-skill/SKILL.md",
                sample_frontmatter("default-skill", "Fixture default skill"),
            ),
            (
                "tests/fixtures/sample-plugin/skills/disabled-skill/SKILL.md",
                sample_frontmatter("disabled-skill", "Fixture disabled skill"),
            ),
            (
                "tests/fixtures/sample-plugin/skills/skill-one/SKILL.md",
                sample_frontmatter("skill-one", "Fixture sample skill"),
            ),
        ])
    }

    fn sample_and_example_skill_snapshot() -> GitHubRepoSnapshot {
        repo_snapshot(&[
            (
                "sample/skill-one/SKILL.md",
                sample_frontmatter("sample-skill", "Published sample skill"),
            ),
            (
                "samples/skill-two/SKILL.md",
                sample_frontmatter("samples-skill", "Published samples skill"),
            ),
            (
                "example/skill-three/SKILL.md",
                sample_frontmatter("example-skill", "Published example skill"),
            ),
            (
                "examples/skill-four/SKILL.md",
                sample_frontmatter("examples-skill", "Published examples skill"),
            ),
        ])
    }

    fn duplicate_name_snapshot() -> GitHubRepoSnapshot {
        repo_snapshot(&[
            (
                "skills/preferred/SKILL.md",
                sample_frontmatter("duplicate-skill", "Preferred"),
            ),
            (
                "packages/fallback/duplicate/SKILL.md",
                sample_frontmatter("duplicate-skill", "Fallback"),
            ),
        ])
    }

    fn plugin_json_grouped_snapshot() -> GitHubRepoSnapshot {
        repo_snapshot(&[
            (
                ".claude-plugin/plugin.json",
                r#"{
                  "name": "mattpocock-skills",
                  "skills": [
                    "./skills/engineering/ask-matt",
                    "skills/engineering/code-review"
                  ]
                }"#
                .to_string(),
            ),
            (
                "skills/engineering/ask-matt/SKILL.md",
                sample_frontmatter("ask-matt", "Ask Matt"),
            ),
            (
                "skills/engineering/code-review/SKILL.md",
                sample_frontmatter("code-review", "Review code"),
            ),
            (
                "skills/writing/blog-post/SKILL.md",
                sample_frontmatter("blog-post", "Write posts"),
            ),
        ])
    }

    fn manifest_hint_with_priority_snapshot() -> GitHubRepoSnapshot {
        repo_snapshot(&[
            (
                ".claude-plugin/plugin.json",
                r#"{
                  "name": "deep-plugin",
                  "skills": ["packages/hidden/deep-skill"]
                }"#
                .to_string(),
            ),
            (
                "skills/top-level/SKILL.md",
                sample_frontmatter("top-level", "Priority root skill"),
            ),
            (
                "packages/hidden/deep-skill/SKILL.md",
                sample_frontmatter("deep-skill", "Manifest-only skill"),
            ),
        ])
    }

    fn broken_manifest_hint_snapshot() -> GitHubRepoSnapshot {
        repo_snapshot(&[
            (
                ".claude-plugin/plugin.json",
                r#"{
                  "name": "broken-plugin",
                  "skills": [
                    "packages/missing-skill",
                    "packages/bad-frontmatter",
                    "../escape",
                    "/absolute/path",
                    "https://example.com/remote-skill"
                  ]
                }"#
                .to_string(),
            ),
            (
                "skills/top-level/SKILL.md",
                sample_frontmatter("top-level", "Priority root skill"),
            ),
            (
                "packages/bad-frontmatter/SKILL.md",
                "# invalid\n".to_string(),
            ),
        ])
    }

    fn marketplace_json_grouped_snapshot() -> GitHubRepoSnapshot {
        repo_snapshot(&[
            (
                ".claude-plugin/marketplace.json",
                r#"{
                  "metadata": { "pluginRoot": "plugins" },
                  "plugins": [
                    {
                      "name": "docs",
                      "source": "docs-plugin",
                      "skills": ["skills/write-docs"]
                    },
                    {
                      "name": "remote-object",
                      "source": { "type": "github", "repo": "owner/repo" },
                      "skills": ["skills/remote-skill"]
                    },
                    {
                      "name": "remote-string",
                      "source": "https://github.com/owner/repo",
                      "skills": ["skills/remote-skill"]
                    }
                  ]
                }"#
                .to_string(),
            ),
            (
                "skills/top-level/SKILL.md",
                sample_frontmatter("top-level", "Priority root skill"),
            ),
            (
                "plugins/docs-plugin/skills/write-docs/SKILL.md",
                sample_frontmatter("write-docs", "Write docs"),
            ),
            (
                "plugins/remote-plugin/skills/remote-skill/SKILL.md",
                sample_frontmatter("remote-skill", "Remote plugin skill"),
            ),
        ])
    }

    fn mixed_valid_invalid_snapshot() -> GitHubRepoSnapshot {
        repo_snapshot(&[
            (
                "skills/valid-skill/SKILL.md",
                sample_frontmatter("Valid Skill", "Valid"),
            ),
            (
                "skills/bad-frontmatter/SKILL.md",
                "# missing frontmatter\n".to_string(),
            ),
        ])
    }

    fn repository_archive(files: &[(&str, &[u8])]) -> Vec<u8> {
        let encoder = GzEncoder::new(Vec::new(), Compression::default());
        let mut builder = tar::Builder::new(encoder);
        for (path, content) in files {
            let archive_path = format!("repo-snapshot/{}", path);
            let mut header = tar::Header::new_gnu();
            header.set_size(content.len() as u64);
            header.set_mode(0o644);
            header.set_entry_type(tar::EntryType::Regular);
            header.set_cksum();
            builder
                .append_data(&mut header, archive_path, *content)
                .expect("append archive entry");
        }
        let encoder = builder.into_inner().expect("finalize tar");
        encoder.finish().expect("finalize gzip")
    }

    #[test]
    fn parse_github_source_normalizes_owner_repo_and_subpath() {
        let parsed = parse_github_source("https://github.com/Anthropics/Skills/content/skills/")
            .expect("parse");
        assert_eq!(parsed.owner, "anthropics");
        assert_eq!(parsed.repo, "skills");
        assert_eq!(parsed.branch, None);
        assert_eq!(parsed.source_path.as_deref(), Some("content/skills"));
    }

    #[test]
    fn parse_github_source_accepts_shorthand_repo_subpaths() {
        let parsed = parse_github_source("bahayonghang/my-claude-code-settings/content/skills")
            .expect("parse");
        assert_eq!(parsed.owner, "bahayonghang");
        assert_eq!(parsed.repo, "my-claude-code-settings");
        assert_eq!(parsed.source_path.as_deref(), Some("content/skills"));
    }

    #[test]
    fn parse_github_source_accepts_tree_urls() {
        let parsed = parse_github_source(
            "https://github.com/bahayonghang/my-claude-code-settings/tree/main/content/skills",
        )
        .expect("parse");
        assert_eq!(parsed.owner, "bahayonghang");
        assert_eq!(parsed.repo, "my-claude-code-settings");
        assert_eq!(parsed.branch.as_deref(), Some("main"));
        assert_eq!(parsed.source_path.as_deref(), Some("content/skills"));
    }

    #[test]
    fn parse_github_source_rejects_non_github_hosts() {
        let error = parse_github_source("https://gitlab.com/example/repo").unwrap_err();
        assert!(error.to_string().contains("github.com"));
    }

    #[test]
    fn parse_github_source_rejects_unsafe_subpaths() {
        let error = parse_github_source("owner/repo/../escape").unwrap_err();
        assert!(error.to_string().contains("not supported"));
    }

    #[test]
    fn sanitize_skill_id_collapses_symbols() {
        let skill_id = sanitize_skill_id("My Cool_Skill!").expect("sanitize");
        assert_eq!(skill_id, "my-cool-skill");
    }

    #[test]
    fn parse_frontmatter_requires_yaml_block() {
        assert!(parse_frontmatter("# nope").is_none());
        let parsed = parse_frontmatter(&sample_frontmatter("alpha", "desc")).expect("fm");
        assert_eq!(parsed.name, "alpha");
        assert_eq!(parsed.description.as_deref(), Some("desc"));
    }

    #[test]
    fn parse_frontmatter_allows_quoted_triple_dash_commands() {
        let parsed = parse_frontmatter(&planning_with_files_like_skill()).expect("frontmatter");

        assert_eq!(parsed.name, "planning-with-files-zh");
        assert_eq!(
            parsed.description.as_deref(),
            Some("Plan with task_plan.md, findings.md, and progress.md files.")
        );
    }

    #[test]
    fn parse_frontmatter_accepts_crlf_delimiters() {
        let content =
            "---\r\nname: crlf-skill\r\ndescription: CRLF frontmatter\r\n---\r\n# Body\r\n";

        let parsed = parse_frontmatter(content).expect("frontmatter");

        assert_eq!(parsed.name, "crlf-skill");
        assert_eq!(parsed.description.as_deref(), Some("CRLF frontmatter"));
    }

    #[test]
    fn parse_frontmatter_requires_independent_closing_delimiter() {
        let content =
            "---\nname: missing-close\ndescription: inline --- is not a closing delimiter\n";

        assert!(parse_frontmatter(content).is_none());
    }

    #[test]
    fn parse_frontmatter_bom_result_matches_scanner_entry_point() {
        // Both entry points share extract_frontmatter_block; a BOM-prefixed
        // SKILL.md must parse identically via Discover and Marketplace import.
        let content = "\u{feff}---\nname: bom-skill\ndescription: BOM guarded\n---\n# Body\n";

        let imported = parse_frontmatter(content).expect("github_import entry");
        let scanned =
            crate::services::scanner::parse_skill_md_content(content).expect("scanner entry");

        assert_eq!(imported.name, scanned.name);
        assert_eq!(imported.description, scanned.description);
        assert_eq!(scanned.name, "bom-skill");
        assert_eq!(scanned.description.as_deref(), Some("BOM guarded"));
    }

    #[test]
    fn classify_github_rate_limit_denial_returns_actionable_message() {
        let denial = GitHubAccessDenial {
            kind: GitHubAccessDenialKind::RateLimited {
                reset_at: Some("2026-04-17 12:34:56".to_string()),
                remaining: Some("0".to_string()),
            },
            operation: "inspecting the repository",
            status: reqwest::StatusCode::FORBIDDEN,
            github_message: Some("API rate limit exceeded for 1.2.3.4.".to_string()),
            used_auth: false,
        };

        let message = denial.to_string();

        assert!(message.contains("rate limit was exceeded"));
        assert!(message.contains("Retry later after 2026-04-17 12:34:56 UTC"));
        assert!(message.contains("authenticated GitHub requests"));
        assert!(message.contains("API rate limit exceeded"));
    }

    #[test]
    fn classify_github_permission_denial_returns_actionable_message() {
        let denial = GitHubAccessDenial {
            kind: GitHubAccessDenialKind::AuthenticationOrPermission,
            operation: "reading repository contents",
            status: reqwest::StatusCode::UNAUTHORIZED,
            github_message: Some("Requires authentication".to_string()),
            used_auth: false,
        };

        let message = denial.to_string();

        assert!(message.contains("denied access"));
        assert!(message.contains("require authentication"));
        assert!(message.contains("token/permissions are insufficient"));
        assert!(message.contains("Requires authentication"));
    }

    #[test]
    fn raw_url_to_repo_path_parses_github_raw_urls() {
        let parsed = raw_url_to_repo_path(
            "https://raw.githubusercontent.com/owner/repo/main/skills/demo/SKILL.md",
        )
        .expect("parsed");

        assert_eq!(parsed.repo.owner, "owner");
        assert_eq!(parsed.repo.repo, "repo");
        assert_eq!(parsed.repo.branch, "main");
        assert_eq!(parsed.file_path, "skills/demo/SKILL.md");
    }

    #[test]
    fn raw_url_to_repo_path_ignores_non_github_raw_hosts() {
        assert!(raw_url_to_repo_path("https://example.com/file.txt").is_none());
    }

    #[test]
    fn mirror_status_retry_excludes_auth_denials() {
        assert!(should_retry_via_mirror_status(
            GitHubFetchSurface::Api,
            reqwest::StatusCode::BAD_GATEWAY
        ));
        assert!(!should_retry_via_mirror_status(
            GitHubFetchSurface::Api,
            reqwest::StatusCode::FORBIDDEN
        ));
        assert!(!should_retry_via_mirror_status(
            GitHubFetchSurface::Raw,
            reqwest::StatusCode::TOO_MANY_REQUESTS
        ));
    }

    #[test]
    fn summarize_mirror_attempts_reports_all_failures() {
        let message = summarize_mirror_attempts(&[
            MirrorAttemptOutcome {
                status: None,
                error_message: "API mirror 'github' failed: timeout".to_string(),
            },
            MirrorAttemptOutcome {
                status: Some(reqwest::StatusCode::BAD_GATEWAY),
                error_message: "API mirror 'ghfast' returned HTTP 502".to_string(),
            },
        ]);

        assert!(message.contains("timeout"));
        assert!(message.contains("HTTP 502"));
    }

    #[test]
    fn snapshot_from_repository_archive_strips_archive_root_directory() {
        let archive = repository_archive(&[
            (
                "skills/demo/SKILL.md",
                sample_frontmatter("Demo", "Archive demo").as_bytes(),
            ),
            ("README.md", b"# readme\n"),
        ]);

        let snapshot = snapshot_from_repository_archive(&archive).expect("snapshot");

        assert!(snapshot.files.contains_key("skills/demo/SKILL.md"));
        assert!(snapshot.files.contains_key("README.md"));
    }

    #[test]
    fn snapshot_from_repository_archive_rejects_entry_over_budget() {
        let oversized = vec![b'a'; 9];
        let archive = repository_archive(&[("skills/demo/SKILL.md", oversized.as_slice())]);
        let budget = ResourceBudget {
            archive_entry_bytes: 8,
            ..ResourceBudget::default()
        };

        let err = snapshot_from_repository_archive_with_budget(&archive, budget).unwrap_err();

        assert!(err.to_string().contains("resource budget"));
    }

    #[test]
    fn snapshot_from_repository_archive_accepts_default_32mb_entry_budget() {
        let large_font = vec![0_u8; 18_948_244];
        let archive = repository_archive(&[("skills/demo/assets/font.ttf", large_font.as_slice())]);

        let snapshot = snapshot_from_repository_archive(&archive).expect("snapshot");

        assert_eq!(
            snapshot
                .files
                .get("skills/demo/assets/font.ttf")
                .map(Vec::len),
            Some(18_948_244)
        );
    }

    #[test]
    fn snapshot_from_repository_archive_rejects_expanded_contents_over_budget() {
        let first = vec![b'a'; 6];
        let second = vec![b'b'; 6];
        let archive = repository_archive(&[
            ("skills/demo/one.txt", first.as_slice()),
            ("skills/demo/two.txt", second.as_slice()),
        ]);
        let budget = ResourceBudget {
            archive_expanded_bytes: 10,
            archive_entry_bytes: 8,
            ..ResourceBudget::default()
        };

        let err = snapshot_from_repository_archive_with_budget(&archive, budget).unwrap_err();

        let err = err.to_string();
        assert!(err.contains("expanded archive contents"));
        assert!(err.contains("12 bytes > 10 bytes"));
    }

    #[tokio::test]
    async fn preview_marks_canonical_conflicts_without_writing() {
        let pool = setup_test_db().await;
        let central_root = tempdir().expect("central");
        sqlx::query("UPDATE agents SET global_skills_dir = ? WHERE id = 'central'")
            .bind(central_root.path().to_string_lossy().into_owned())
            .execute(&pool)
            .await
            .expect("update central");

        let existing_dir = central_root.path().join("twitterapi-io");
        std::fs::create_dir_all(&existing_dir).expect("mkdir");
        std::fs::write(
            existing_dir.join("SKILL.md"),
            sample_frontmatter("twitterapi-io", "existing"),
        )
        .expect("write skill");

        db::upsert_skill(
            &pool,
            &Skill {
                id: "twitterapi-io".to_string(),
                name: "twitterapi-io".to_string(),
                description: Some("existing".to_string()),
                file_path: existing_dir.join("SKILL.md").to_string_lossy().into_owned(),
                canonical_path: Some(existing_dir.to_string_lossy().into_owned()),
                is_central: true,
                source: Some("local".to_string()),
                content: None,
                scanned_at: Utc::now().to_rfc3339(),
                fs_created_at: None,
                fs_updated_at: None,
            },
        )
        .await
        .expect("upsert skill");

        let repo = GitHubRepoRef {
            owner: "dorukardahan".to_string(),
            repo: "twitterapi-io-skill".to_string(),
            branch: "main".to_string(),
            normalized_url: "https://github.com/dorukardahan/twitterapi-io-skill".to_string(),
        };
        let candidates = build_repo_skill_candidates_from_snapshot(&repo, &root_repo_snapshot())
            .expect("candidates");
        let preview = GitHubRepoPreview {
            repo,
            skills: build_preview_skills(&pool, &candidates)
                .await
                .expect("preview skills"),
            preview_workspace_id: None,
        };

        assert!(!preview.skills.is_empty());
        let conflict = preview
            .skills
            .iter()
            .find(|skill| skill.skill_id == "twitterapi-io")
            .and_then(|skill| skill.conflict.clone())
            .expect("conflict");
        assert_eq!(conflict.existing_skill_id, "twitterapi-io");

        let central_entries = std::fs::read_dir(central_root.path())
            .expect("read dir")
            .count();
        assert_eq!(central_entries, 1, "preview should not write to central");
    }

    #[tokio::test]
    async fn import_staging_allows_reclaiming_non_central_record_after_delete() {
        let pool = setup_test_db().await;
        let candidate = RemoteSkillCandidate {
            source_path: "skills/web-access".to_string(),
            skill_id: "web-access".to_string(),
            skill_name: "web-access".to_string(),
            description: Some("remote import".to_string()),
            plugin_name: None,
            root_directory: "skills".to_string(),
            skill_directory_name: "web-access".to_string(),
            download_url: "https://raw.githubusercontent.com/eze-is/web-access/main/SKILL.md"
                .to_string(),
        };

        db::upsert_skill(
            &pool,
            &Skill {
                id: "web-access".to_string(),
                name: "web-access".to_string(),
                description: Some("platform copy left after central delete".to_string()),
                file_path: "/tmp/codex/web-access/SKILL.md".to_string(),
                canonical_path: None,
                is_central: false,
                source: Some("copy".to_string()),
                content: None,
                scanned_at: Utc::now().to_rfc3339(),
                fs_created_at: None,
                fs_updated_at: None,
            },
        )
        .await
        .expect("seed non-central record");

        let preview = build_preview_skills(&pool, std::slice::from_ref(&candidate))
            .await
            .expect("preview");
        assert!(
            preview[0].conflict.is_none(),
            "non-central rows should not be presented as Central overwrite conflicts"
        );

        let (staging_ops, skipped_skills) = plan_import_staging(
            &pool,
            std::slice::from_ref(&candidate),
            vec![GitHubSkillImportSelection {
                source_path: candidate.source_path.clone(),
                resolution: DuplicateResolution::Overwrite,
                renamed_skill_id: None,
            }],
        )
        .await
        .expect("stage import");

        assert!(skipped_skills.is_empty());
        assert_eq!(staging_ops.len(), 1);
        assert_eq!(staging_ops[0].final_skill_id, "web-access");
        assert_eq!(staging_ops[0].resolution, DuplicateResolution::Overwrite);
    }

    #[tokio::test]
    async fn import_repo_skills_honors_skip_rename_and_overwrite() {
        let pool = setup_test_db().await;
        let snapshot = multi_skill_snapshot();
        let repo = GitHubRepoRef {
            owner: "anthropics".to_string(),
            repo: "skills".to_string(),
            branch: "main".to_string(),
            normalized_url: "https://github.com/anthropics/skills".to_string(),
        };

        let candidates =
            build_repo_skill_candidates_from_snapshot(&repo, &snapshot).expect("candidates");

        let agent_planner = candidates
            .iter()
            .find(|candidate| candidate.source_path == "skills/agent-planner")
            .expect("agent planner");
        let commit = candidates
            .iter()
            .find(|candidate| candidate.source_path == "skills/commit")
            .expect("commit");
        let code_review = candidates
            .iter()
            .find(|candidate| candidate.source_path == "skills/code-review")
            .expect("code review");

        db::upsert_skill(
            &pool,
            &Skill {
                id: agent_planner.skill_id.clone(),
                name: "Agent Planner".to_string(),
                description: Some("existing".to_string()),
                file_path: "/tmp/agent-planner/SKILL.md".to_string(),
                canonical_path: Some("/tmp/agent-planner".to_string()),
                is_central: true,
                source: Some("local".to_string()),
                content: None,
                scanned_at: Utc::now().to_rfc3339(),
                fs_created_at: None,
                fs_updated_at: None,
            },
        )
        .await
        .expect("seed rename conflict");
        db::upsert_skill(
            &pool,
            &Skill {
                id: commit.skill_id.clone(),
                name: "Commit".to_string(),
                description: Some("existing".to_string()),
                file_path: "/tmp/commit/SKILL.md".to_string(),
                canonical_path: Some("/tmp/commit".to_string()),
                is_central: true,
                source: Some("local".to_string()),
                content: None,
                scanned_at: Utc::now().to_rfc3339(),
                fs_created_at: None,
                fs_updated_at: None,
            },
        )
        .await
        .expect("seed skip conflict");
        db::upsert_skill(
            &pool,
            &Skill {
                id: code_review.skill_id.clone(),
                name: "Code Review".to_string(),
                description: Some("existing".to_string()),
                file_path: "/tmp/code-review/SKILL.md".to_string(),
                canonical_path: Some("/tmp/code-review".to_string()),
                is_central: true,
                source: Some("local".to_string()),
                content: None,
                scanned_at: Utc::now().to_rfc3339(),
                fs_created_at: None,
                fs_updated_at: None,
            },
        )
        .await
        .expect("seed overwrite conflict");

        let mut occupied = current_central_skill_ids(&pool).await.expect("occupied");
        assert!(occupied.contains(&agent_planner.skill_id));
        assert!(occupied.contains(&commit.skill_id));
        assert!(occupied.contains(&code_review.skill_id));

        let rename_target = sanitize_skill_id("agent-planner-imported").expect("rename target");
        assert!(
            !occupied.contains(&rename_target),
            "rename target should be available before import"
        );
        occupied.insert(rename_target.clone());

        assert!(
            occupied.contains(&rename_target),
            "rename should reserve the requested canonical id"
        );
        assert!(
            occupied.contains(&code_review.skill_id),
            "overwrite keeps the original canonical id occupied"
        );
        assert!(
            occupied.contains(&commit.skill_id),
            "skip leaves the existing canonical id occupied without needing a new id"
        );
    }

    #[test]
    fn inspect_snapshot_keeps_valid_candidates_and_reports_invalid_ones() {
        let repo = GitHubRepoRef {
            owner: "openai".to_string(),
            repo: "skills".to_string(),
            branch: "main".to_string(),
            normalized_url: "https://github.com/openai/skills".to_string(),
        };

        let inspected = inspect_repo_skill_candidates_from_snapshot_at_path(
            &repo,
            &mixed_valid_invalid_snapshot(),
            None,
        )
        .expect("inspect");

        assert_eq!(inspected.valid_candidates.len(), 1);
        assert_eq!(
            inspected.valid_candidates[0].source_path,
            "skills/valid-skill"
        );
        assert_eq!(inspected.invalid_candidates.len(), 1);
        assert_eq!(
            inspected.invalid_candidates[0].source_path,
            "skills/bad-frontmatter"
        );
        assert_eq!(
            inspected.invalid_candidates[0].reason,
            "invalid_frontmatter"
        );
    }

    #[tokio::test]
    async fn partial_import_continues_after_invalid_skill_and_preserves_successful_imports() {
        let pool = setup_test_db().await;
        let central_root = tempdir().expect("central");
        sqlx::query("UPDATE agents SET global_skills_dir = ? WHERE id = 'central'")
            .bind(central_root.path().to_string_lossy().into_owned())
            .execute(&pool)
            .await
            .expect("update central");

        let repo = GitHubRepoRef {
            owner: "openai".to_string(),
            repo: "skills".to_string(),
            branch: "main".to_string(),
            normalized_url: "https://github.com/openai/skills".to_string(),
        };
        let snapshot = mixed_valid_invalid_snapshot();
        let inspected = inspect_repo_skill_candidates_from_snapshot_at_path(&repo, &snapshot, None)
            .expect("inspect");

        let result = import_github_repo_skills_from_snapshot_partially(
            &pool,
            &repo,
            &snapshot,
            inspected,
            vec![
                GitHubSkillImportSelection {
                    source_path: "skills/valid-skill".to_string(),
                    resolution: DuplicateResolution::Overwrite,
                    renamed_skill_id: None,
                },
                GitHubSkillImportSelection {
                    source_path: "skills/bad-frontmatter".to_string(),
                    resolution: DuplicateResolution::Overwrite,
                    renamed_skill_id: None,
                },
            ],
            central_root.path(),
            None,
        )
        .await
        .expect("partial import");

        assert_eq!(result.imported_skills.len(), 1);
        assert_eq!(result.failed_skills.len(), 1);
        assert_eq!(
            result.failed_skills[0].source_path,
            "skills/bad-frontmatter"
        );
        assert!(db::get_skill_by_id(&pool, "valid-skill")
            .await
            .expect("db")
            .is_some());
        assert!(central_root
            .path()
            .join("valid-skill")
            .join("SKILL.md")
            .exists());
    }

    #[tokio::test]
    async fn partial_import_rejects_crafted_selection_for_filtered_generic_skill() {
        let pool = setup_test_db().await;
        let central_root = tempdir().expect("central");
        sqlx::query("UPDATE agents SET global_skills_dir = ? WHERE id = 'central'")
            .bind(central_root.path().to_string_lossy().into_owned())
            .execute(&pool)
            .await
            .expect("update central");

        let repo = GitHubRepoRef {
            owner: "panniantong".to_string(),
            repo: "agent-reach".to_string(),
            branch: "main".to_string(),
            normalized_url: "https://github.com/panniantong/agent-reach".to_string(),
        };
        let snapshot = repo_snapshot(&[(
            "agent_reach/skill/SKILL.md",
            sample_frontmatter("Agent Reach", "Generic container"),
        )]);
        let inspected = inspect_repo_skill_candidates_from_snapshot_at_path(&repo, &snapshot, None)
            .expect("inspect");

        assert!(inspected.valid_candidates.is_empty());

        let result = import_github_repo_skills_from_snapshot_partially(
            &pool,
            &repo,
            &snapshot,
            inspected,
            vec![GitHubSkillImportSelection {
                source_path: "agent_reach/skill".to_string(),
                resolution: DuplicateResolution::Overwrite,
                renamed_skill_id: None,
            }],
            central_root.path(),
            None,
        )
        .await
        .expect("partial import");

        assert!(result.imported_skills.is_empty());
        assert_eq!(result.failed_skills.len(), 1);
        assert_eq!(result.failed_skills[0].source_path, "agent_reach/skill");
        assert!(result.failed_skills[0]
            .error
            .contains("no longer available"));
        assert!(db::get_skill_by_id(&pool, "skill")
            .await
            .expect("db")
            .is_none());
        assert!(!central_root.path().join("skill").exists());
    }

    #[tokio::test]
    async fn partial_import_cleans_staging_dirs_after_per_skill_failure() {
        let pool = setup_test_db().await;
        let central_root = tempdir().expect("central");
        sqlx::query("UPDATE agents SET global_skills_dir = ? WHERE id = 'central'")
            .bind(central_root.path().to_string_lossy().into_owned())
            .execute(&pool)
            .await
            .expect("update central");

        let conflicting_dir = central_root.path().join("commit-imported");
        std::fs::create_dir_all(&conflicting_dir).expect("mkdir conflict");
        std::fs::write(conflicting_dir.join("sentinel.txt"), "keep").expect("write sentinel");

        let repo = GitHubRepoRef {
            owner: "anthropics".to_string(),
            repo: "skills".to_string(),
            branch: "main".to_string(),
            normalized_url: "https://github.com/anthropics/skills".to_string(),
        };
        let snapshot = multi_skill_snapshot();
        let inspected = inspect_repo_skill_candidates_from_snapshot_at_path(&repo, &snapshot, None)
            .expect("inspect");

        let result = import_github_repo_skills_from_snapshot_partially(
            &pool,
            &repo,
            &snapshot,
            inspected,
            vec![
                GitHubSkillImportSelection {
                    source_path: "skills/agent-planner".to_string(),
                    resolution: DuplicateResolution::Overwrite,
                    renamed_skill_id: None,
                },
                GitHubSkillImportSelection {
                    source_path: "skills/commit".to_string(),
                    resolution: DuplicateResolution::Rename,
                    renamed_skill_id: Some("commit-imported".to_string()),
                },
            ],
            central_root.path(),
            None,
        )
        .await
        .expect("partial import");

        assert_eq!(result.imported_skills.len(), 1);
        assert_eq!(result.failed_skills.len(), 1);
        assert_eq!(result.failed_skills[0].source_path, "skills/commit");
        assert!(central_root
            .path()
            .join("agent-planner")
            .join("SKILL.md")
            .exists());
        assert!(conflicting_dir.join("sentinel.txt").exists());

        let leaked_staging = std::fs::read_dir(central_root.path())
            .expect("read central")
            .filter_map(Result::ok)
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .filter(|name| {
                name.starts_with(".skillport-import-") || name.starts_with(".skillport-backup-")
            })
            .collect::<Vec<_>>();
        assert!(
            leaked_staging.is_empty(),
            "temporary staging directories should be cleaned up: {leaked_staging:?}"
        );
    }

    #[tokio::test]
    async fn full_import_restores_overwrite_target_when_db_assignment_fails() {
        let pool = setup_test_db().await;
        let central_root = tempdir().expect("central");
        sqlx::query("UPDATE agents SET global_skills_dir = ? WHERE id = 'central'")
            .bind(central_root.path().to_string_lossy().into_owned())
            .execute(&pool)
            .await
            .expect("update central");

        let repo = GitHubRepoRef {
            owner: "anthropics".to_string(),
            repo: "skills".to_string(),
            branch: "main".to_string(),
            normalized_url: "https://github.com/anthropics/skills".to_string(),
        };
        let snapshot = multi_skill_snapshot();
        let candidates =
            build_repo_skill_candidates_from_snapshot(&repo, &snapshot).expect("candidates");
        let source_path = "skills/code-review";
        let candidate = candidates
            .iter()
            .find(|candidate| candidate.source_path == source_path)
            .expect("code review candidate");

        let existing_dir = central_root.path().join(&candidate.skill_id);
        std::fs::create_dir_all(&existing_dir).expect("mkdir existing");
        std::fs::write(
            existing_dir.join("SKILL.md"),
            sample_frontmatter("Code Review", "existing description"),
        )
        .expect("write existing skill");
        std::fs::write(existing_dir.join("old-only.txt"), "keep").expect("write sentinel");
        db::upsert_skill(
            &pool,
            &Skill {
                id: candidate.skill_id.clone(),
                name: "Code Review".to_string(),
                description: Some("existing description".to_string()),
                file_path: existing_dir.join("SKILL.md").to_string_lossy().into_owned(),
                canonical_path: Some(existing_dir.to_string_lossy().into_owned()),
                is_central: true,
                source: Some("local".to_string()),
                content: None,
                scanned_at: Utc::now().to_rfc3339(),
                fs_created_at: None,
                fs_updated_at: None,
            },
        )
        .await
        .expect("seed existing skill");

        sqlx::query("DROP TABLE skill_repository_members")
            .execute(&pool)
            .await
            .expect("simulate repository assignment failure");

        let result = import_github_repo_skills_from_snapshot(
            &pool,
            &repo,
            &snapshot,
            &candidates,
            vec![GitHubSkillImportSelection {
                source_path: source_path.to_string(),
                resolution: DuplicateResolution::Overwrite,
                renamed_skill_id: None,
            }],
            central_root.path(),
            None,
        )
        .await;

        let error = result.expect_err("import should fail on DB assignment");
        let error = error.to_string();
        assert!(
            error.contains("skill_repository_members"),
            "error should identify the failed assignment table: {error}"
        );
        assert!(
            existing_dir.join("old-only.txt").exists(),
            "overwrite backup should be restored when DB assignment fails"
        );
        let restored =
            std::fs::read_to_string(existing_dir.join("SKILL.md")).expect("read restored SKILL.md");
        assert!(
            restored.contains("existing description"),
            "restored target should keep the original skill content"
        );

        let db_skill = db::get_skill_by_id(&pool, &candidate.skill_id)
            .await
            .expect("read skill")
            .expect("existing skill remains");
        assert_eq!(
            db_skill.description.as_deref(),
            Some("existing description")
        );
        assert_eq!(db_skill.source.as_deref(), Some("local"));

        let repository_id = db::github_repository_id(&repo.owner, &repo.repo, &repo.branch);
        let imported_repo_rows: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM skill_repositories WHERE id = ?")
                .bind(repository_id)
                .fetch_one(&pool)
                .await
                .expect("count repository rows");
        assert_eq!(
            imported_repo_rows, 0,
            "repository upsert should roll back with the failed assignment"
        );

        let leaked_staging = std::fs::read_dir(central_root.path())
            .expect("read central")
            .filter_map(Result::ok)
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .filter(|name| {
                name.starts_with(".skillport-import-") || name.starts_with(".skillport-backup-")
            })
            .collect::<Vec<_>>();
        assert!(
            leaked_staging.is_empty(),
            "temporary staging directories should be cleaned up: {leaked_staging:?}"
        );
    }

    #[tokio::test]
    async fn import_invalid_repo_leaves_central_storage_unchanged() {
        let pool = setup_test_db().await;
        let central_root = tempdir().expect("central");
        sqlx::query("UPDATE agents SET global_skills_dir = ? WHERE id = 'central'")
            .bind(central_root.path().to_string_lossy().into_owned())
            .execute(&pool)
            .await
            .expect("update central");

        let result = import_github_repo_skills_impl(
            &pool,
            &MockSecretStore::default(),
            "https://github.com/example/definitely-missing-repo",
            vec![GitHubSkillImportSelection {
                source_path: "skills/foo".to_string(),
                resolution: DuplicateResolution::Skip,
                renamed_skill_id: None,
            }],
            None,
        )
        .await;

        assert!(result.is_err());
        assert_eq!(
            std::fs::read_dir(central_root.path())
                .expect("read central")
                .count(),
            0
        );
        let central_skills = db::get_central_skills(&pool).await.expect("central skills");
        assert!(central_skills.is_empty());
    }

    #[tokio::test]
    async fn denied_import_selection_performs_no_writes_or_db_mutations() {
        let pool = setup_test_db().await;
        let central_root = tempdir().expect("central");
        sqlx::query("UPDATE agents SET global_skills_dir = ? WHERE id = 'central'")
            .bind(central_root.path().to_string_lossy().into_owned())
            .execute(&pool)
            .await
            .expect("update central");

        let before_skills = db::get_central_skills(&pool).await.expect("before skills");
        let before_entries = std::fs::read_dir(central_root.path())
            .expect("read central before")
            .count();

        let result = import_github_repo_skills_impl(
            &pool,
            &MockSecretStore::default(),
            "https://github.com/example/restricted-repo",
            vec![GitHubSkillImportSelection {
                source_path: "skills/private-skill".to_string(),
                resolution: DuplicateResolution::Overwrite,
                renamed_skill_id: None,
            }],
            None,
        )
        .await;

        let error = result.expect_err("denied import should fail");
        assert!(
            !error.to_string().trim().is_empty(),
            "failure should return an error message"
        );

        let after_skills = db::get_central_skills(&pool).await.expect("after skills");
        let after_entries = std::fs::read_dir(central_root.path())
            .expect("read central after")
            .count();
        assert_eq!(
            before_entries, after_entries,
            "denied import should not write files"
        );
        assert_eq!(
            before_skills.len(),
            after_skills.len(),
            "denied import should not mutate DB"
        );
    }

    #[tokio::test]
    async fn preview_top_level_skills_directory_discovers_candidates() {
        let pool = setup_test_db().await;
        let repo = GitHubRepoRef {
            owner: "anthropics".to_string(),
            repo: "skills".to_string(),
            branch: "main".to_string(),
            normalized_url: "https://github.com/anthropics/skills".to_string(),
        };
        let candidates = build_repo_skill_candidates_from_snapshot(&repo, &multi_skill_snapshot())
            .expect("candidates");
        let preview = GitHubRepoPreview {
            repo,
            skills: build_preview_skills(&pool, &candidates)
                .await
                .expect("skills"),
            preview_workspace_id: None,
        };

        assert!(preview
            .skills
            .iter()
            .any(|skill| skill.source_path.starts_with("skills/")));
    }

    #[tokio::test]
    async fn preview_namespaced_skills_directory_discovers_candidates() {
        let pool = setup_test_db().await;
        let repo = GitHubRepoRef {
            owner: "openai".to_string(),
            repo: "skills".to_string(),
            branch: "main".to_string(),
            normalized_url: "https://github.com/openai/skills".to_string(),
        };

        let candidates =
            build_repo_skill_candidates_from_snapshot(&repo, &namespaced_skill_snapshot())
                .expect("candidates");

        assert_eq!(
            candidates.len(),
            2,
            "expected two namespaced skill candidates"
        );

        let curated = candidates
            .iter()
            .find(|candidate| candidate.source_path == "skills/.curated/openai-docs")
            .expect("curated skill");
        assert_eq!(curated.root_directory, "skills/.curated");
        assert_eq!(curated.skill_directory_name, "openai-docs");
        assert_eq!(curated.skill_id, "openai-docs");

        let system = candidates
            .iter()
            .find(|candidate| candidate.source_path == "skills/.system/skill-creator")
            .expect("system skill");
        assert_eq!(system.root_directory, "skills/.system");
        assert_eq!(system.skill_directory_name, "skill-creator");
        assert_eq!(system.skill_id, "skill-creator");

        let preview = GitHubRepoPreview {
            repo,
            skills: build_preview_skills(&pool, &candidates)
                .await
                .expect("preview skills"),
            preview_workspace_id: None,
        };

        assert!(preview
            .skills
            .iter()
            .any(|skill| skill.source_path == "skills/.curated/openai-docs"));
        assert!(preview
            .skills
            .iter()
            .any(|skill| skill.source_path == "skills/.system/skill-creator"));
    }

    #[test]
    fn preview_content_skills_catalog_is_found_by_recursive_fallback() {
        let repo = GitHubRepoRef {
            owner: "bahayonghang".to_string(),
            repo: "my-claude-code-settings".to_string(),
            branch: "main".to_string(),
            normalized_url: "https://github.com/bahayonghang/my-claude-code-settings".to_string(),
        };

        let candidates =
            build_repo_skill_candidates_from_snapshot(&repo, &content_skills_snapshot())
                .expect("candidates");

        assert!(candidates.iter().any(|candidate| {
            candidate.source_path == "content/skills/development-workflows/code-auditor"
                && candidate.skill_id == "code-auditor"
                && candidate.root_directory == "content/skills/development-workflows"
        }));
        assert!(candidates.iter().any(|candidate| {
            candidate.source_path == "content/skills/git-github-collaboration/git-commit"
        }));
    }

    #[test]
    fn preview_content_skills_subpath_discovers_catalog() {
        let repo = GitHubRepoRef {
            owner: "bahayonghang".to_string(),
            repo: "my-claude-code-settings".to_string(),
            branch: "main".to_string(),
            normalized_url: "https://github.com/bahayonghang/my-claude-code-settings".to_string(),
        };

        let candidates = build_repo_skill_candidates_from_snapshot_at_path(
            &repo,
            &content_skills_snapshot(),
            Some("content/skills"),
        )
        .expect("candidates");

        assert_eq!(candidates.len(), 2);
        assert!(candidates
            .iter()
            .all(|candidate| candidate.source_path.starts_with("content/skills/")));
    }

    #[test]
    fn repo_skill_candidate_accepts_planning_with_files_style_frontmatter() {
        let repo = GitHubRepoRef {
            owner: "example".to_string(),
            repo: "skills".to_string(),
            branch: "main".to_string(),
            normalized_url: "https://github.com/example/skills".to_string(),
        };
        let snapshot = repo_snapshot(&[(
            "skills/planning-with-files-zh/SKILL.md",
            planning_with_files_like_skill(),
        )]);

        let candidates =
            build_repo_skill_candidates_from_snapshot_at_path(&repo, &snapshot, Some("skills"))
                .expect("candidates");

        assert_eq!(candidates.len(), 1);
        let candidate = &candidates[0];
        assert_eq!(candidate.source_path, "skills/planning-with-files-zh");
        assert_eq!(candidate.skill_id, "planning-with-files-zh");
        assert_eq!(candidate.skill_name, "planning-with-files-zh");
        assert_eq!(
            candidate.description.as_deref(),
            Some("Plan with task_plan.md, findings.md, and progress.md files.")
        );
    }

    #[test]
    fn preview_agent_specific_skill_roots_are_supported() {
        let repo = GitHubRepoRef {
            owner: "example".to_string(),
            repo: "agent-paths".to_string(),
            branch: "main".to_string(),
            normalized_url: "https://github.com/example/agent-paths".to_string(),
        };

        let candidates = build_repo_skill_candidates_from_snapshot(&repo, &agent_path_snapshot())
            .expect("candidates");

        assert!(candidates
            .iter()
            .any(|candidate| candidate.source_path == ".agents/skills/universal-skill"));
        assert!(candidates
            .iter()
            .any(|candidate| candidate.source_path == ".claude/skills/claude-skill"));
        assert!(candidates
            .iter()
            .any(|candidate| candidate.source_path == ".codex/skills/codex-skill"));
    }

    #[test]
    fn recursive_fallback_skips_large_generated_and_generic_skill_directories() {
        let repo = GitHubRepoRef {
            owner: "example".to_string(),
            repo: "fallback".to_string(),
            branch: "main".to_string(),
            normalized_url: "https://github.com/example/fallback".to_string(),
        };

        let candidates =
            build_repo_skill_candidates_from_snapshot(&repo, &recursive_fallback_snapshot())
                .expect("candidates");

        assert!(candidates.is_empty());
    }

    #[test]
    fn recursive_fallback_filters_generic_skill_id_without_hiding_real_candidates() {
        let repo = GitHubRepoRef {
            owner: "example".to_string(),
            repo: "mixed".to_string(),
            branch: "main".to_string(),
            normalized_url: "https://github.com/example/mixed".to_string(),
        };
        let snapshot = repo_snapshot(&[
            (
                "agent_reach/skill/SKILL.md",
                sample_frontmatter("fallback-skill", "Generic container"),
            ),
            (
                "plugins/compound-engineering/skills/ce-work/SKILL.md",
                sample_frontmatter("ce-work", "Real plugin skill"),
            ),
        ]);

        let candidates =
            build_repo_skill_candidates_from_snapshot(&repo, &snapshot).expect("candidates");

        assert_eq!(candidates.len(), 1);
        assert_eq!(
            candidates[0].source_path,
            "plugins/compound-engineering/skills/ce-work"
        );
        assert_eq!(candidates[0].skill_id, "ce-work");
    }

    #[test]
    fn recursive_fallback_skips_test_fixture_skill_directories() {
        let repo = GitHubRepoRef {
            owner: "everyinc".to_string(),
            repo: "compound-engineering-plugin".to_string(),
            branch: "main".to_string(),
            normalized_url: "https://github.com/everyinc/compound-engineering-plugin".to_string(),
        };

        let candidates =
            build_repo_skill_candidates_from_snapshot(&repo, &compound_plugin_like_snapshot())
                .expect("candidates");

        assert_eq!(candidates.len(), 1);
        assert_eq!(
            candidates[0].source_path,
            "plugins/compound-engineering/skills/ce-work"
        );
        assert_eq!(candidates[0].skill_id, "ce-work");
        assert!(candidates.iter().all(|candidate| {
            !matches!(
                candidate.skill_id.as_str(),
                "custom-skill" | "default-skill" | "disabled-skill" | "skill-one"
            )
        }));
    }

    #[test]
    fn recursive_fallback_keeps_sample_and_example_skill_directories() {
        let repo = GitHubRepoRef {
            owner: "example".to_string(),
            repo: "published-examples".to_string(),
            branch: "main".to_string(),
            normalized_url: "https://github.com/example/published-examples".to_string(),
        };

        let candidates =
            build_repo_skill_candidates_from_snapshot(&repo, &sample_and_example_skill_snapshot())
                .expect("candidates");
        let source_paths = candidates
            .iter()
            .map(|candidate| candidate.source_path.as_str())
            .collect::<Vec<_>>();

        assert_eq!(source_paths.len(), 4);
        assert!(source_paths.contains(&"sample/skill-one"));
        assert!(source_paths.contains(&"samples/skill-two"));
        assert!(source_paths.contains(&"example/skill-three"));
        assert!(source_paths.contains(&"examples/skill-four"));
    }

    #[test]
    fn duplicate_skill_names_keep_priority_manifest() {
        let repo = GitHubRepoRef {
            owner: "example".to_string(),
            repo: "duplicates".to_string(),
            branch: "main".to_string(),
            normalized_url: "https://github.com/example/duplicates".to_string(),
        };

        let candidates =
            build_repo_skill_candidates_from_snapshot(&repo, &duplicate_name_snapshot())
                .expect("candidates");

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].source_path, "skills/preferred");
        assert_eq!(candidates[0].description.as_deref(), Some("Preferred"));
    }

    #[tokio::test]
    async fn plugin_json_assigns_preview_grouping_without_persisted_import_metadata() {
        let pool = setup_test_db().await;
        let repo = GitHubRepoRef {
            owner: "mattpocock".to_string(),
            repo: "skills".to_string(),
            branch: "main".to_string(),
            normalized_url: "https://github.com/mattpocock/skills".to_string(),
        };

        let candidates =
            build_repo_skill_candidates_from_snapshot(&repo, &plugin_json_grouped_snapshot())
                .expect("candidates");
        let grouped = candidates
            .iter()
            .filter(|candidate| candidate.plugin_name.as_deref() == Some("mattpocock-skills"))
            .map(|candidate| candidate.source_path.as_str())
            .collect::<Vec<_>>();

        assert_eq!(grouped.len(), 2);
        assert!(grouped.contains(&"skills/engineering/ask-matt"));
        assert!(grouped.contains(&"skills/engineering/code-review"));
        let unlisted = candidates
            .iter()
            .find(|candidate| candidate.source_path == "skills/writing/blog-post")
            .expect("unlisted skill");
        assert_eq!(unlisted.plugin_name, None);

        let preview = build_preview_skills(&pool, &candidates)
            .await
            .expect("preview");
        assert!(preview.iter().any(|skill| {
            skill.source_path == "skills/engineering/ask-matt"
                && skill.plugin_name.as_deref() == Some("mattpocock-skills")
        }));
        let summary_json = serde_json::to_value(ImportedGitHubSkillSummary {
            source_path: "skills/engineering/ask-matt".to_string(),
            original_skill_id: "ask-matt".to_string(),
            imported_skill_id: "ask-matt".to_string(),
            skill_name: "ask-matt".to_string(),
            target_directory: "/tmp/ask-matt".to_string(),
            resolution: DuplicateResolution::Overwrite,
        })
        .expect("serialize summary");
        assert!(
            summary_json.get("pluginName").is_none(),
            "plugin grouping must stay preview-only"
        );
    }

    #[test]
    fn manifest_hints_find_deep_skills_without_suppressing_priority_discovery() {
        let repo = GitHubRepoRef {
            owner: "example".to_string(),
            repo: "mixed-layout".to_string(),
            branch: "main".to_string(),
            normalized_url: "https://github.com/example/mixed-layout".to_string(),
        };

        let candidates = build_repo_skill_candidates_from_snapshot(
            &repo,
            &manifest_hint_with_priority_snapshot(),
        )
        .expect("candidates");
        let source_paths = candidates
            .iter()
            .map(|candidate| candidate.source_path.as_str())
            .collect::<Vec<_>>();

        assert_eq!(
            source_paths,
            vec!["skills/top-level", "packages/hidden/deep-skill"]
        );
        let hinted = candidates
            .iter()
            .find(|candidate| candidate.source_path == "packages/hidden/deep-skill")
            .expect("hinted deep skill");
        assert_eq!(hinted.plugin_name.as_deref(), Some("deep-plugin"));
    }

    #[test]
    fn manifest_hints_drop_broken_and_unsafe_entries_without_failing() {
        let repo = GitHubRepoRef {
            owner: "example".to_string(),
            repo: "broken-hints".to_string(),
            branch: "main".to_string(),
            normalized_url: "https://github.com/example/broken-hints".to_string(),
        };

        let candidates =
            build_repo_skill_candidates_from_snapshot(&repo, &broken_manifest_hint_snapshot())
                .expect("broken manifest hints should not fail candidate building");

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].source_path, "skills/top-level");
        assert_eq!(candidates[0].plugin_name, None);
    }

    #[test]
    fn malformed_plugin_manifest_json_keeps_legacy_discovery() {
        let repo = GitHubRepoRef {
            owner: "example".to_string(),
            repo: "malformed-manifest".to_string(),
            branch: "main".to_string(),
            normalized_url: "https://github.com/example/malformed-manifest".to_string(),
        };
        let snapshot = repo_snapshot(&[
            (".claude-plugin/plugin.json", "{not valid json".to_string()),
            (
                "skills/top-level/SKILL.md",
                sample_frontmatter("top-level", "Priority root skill"),
            ),
        ]);

        let candidates =
            build_repo_skill_candidates_from_snapshot(&repo, &snapshot).expect("candidates");

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].source_path, "skills/top-level");
        assert_eq!(candidates[0].plugin_name, None);
    }

    #[test]
    fn marketplace_json_groups_local_plugin_skills_relative_to_plugin_directory() {
        let repo = GitHubRepoRef {
            owner: "example".to_string(),
            repo: "marketplace-manifest".to_string(),
            branch: "main".to_string(),
            normalized_url: "https://github.com/example/marketplace-manifest".to_string(),
        };

        let candidates =
            build_repo_skill_candidates_from_snapshot(&repo, &marketplace_json_grouped_snapshot())
                .expect("candidates");
        let source_paths = candidates
            .iter()
            .map(|candidate| candidate.source_path.as_str())
            .collect::<Vec<_>>();

        assert_eq!(
            source_paths,
            vec!["skills/top-level", "plugins/docs-plugin/skills/write-docs"]
        );
        let docs = candidates
            .iter()
            .find(|candidate| candidate.source_path == "plugins/docs-plugin/skills/write-docs")
            .expect("docs plugin skill");
        assert_eq!(docs.plugin_name.as_deref(), Some("docs"));
        assert!(!candidates
            .iter()
            .any(|candidate| candidate.source_path.contains("remote-plugin")));
    }

    #[test]
    fn manifest_hints_do_not_suppress_recursive_fallback_for_unlisted_skills() {
        let repo = GitHubRepoRef {
            owner: "mattpocock".to_string(),
            repo: "skills".to_string(),
            branch: "main".to_string(),
            normalized_url: "https://github.com/mattpocock/skills".to_string(),
        };

        let candidates =
            build_repo_skill_candidates_from_snapshot(&repo, &plugin_json_grouped_snapshot())
                .expect("candidates");

        assert!(candidates
            .iter()
            .any(|candidate| candidate.source_path == "skills/writing/blog-post"));
    }

    #[test]
    fn remote_manifest_discovery_preserves_snapshot_priority_order() {
        let paths = [
            "packages/fallback/skill/SKILL.md",
            ".agents/skills/universal/SKILL.md",
            "skills/agent-planner/SKILL.md",
            "SKILL.md",
        ];

        let manifests =
            discover_skill_manifests_from_paths(paths.iter().copied(), None).expect("manifests");

        assert_eq!(manifests[0].source_path, ".");
        assert!(manifests
            .iter()
            .any(|manifest| manifest.source_path == "skills/agent-planner"));
        assert!(manifests
            .iter()
            .any(|manifest| manifest.source_path == ".agents/skills/universal"));
        assert!(!manifests
            .iter()
            .any(|manifest| manifest.source_path == "packages/fallback/skill"));
    }

    #[test]
    fn remote_manifest_discovery_honors_source_subpath() {
        let paths = [
            "content/skills/code/SKILL.md",
            "content/skills/git/SKILL.md",
            "other/skills/ignored/SKILL.md",
        ];

        let manifests =
            discover_skill_manifests_from_paths(paths.iter().copied(), Some("content/skills"))
                .expect("manifests");

        assert_eq!(manifests.len(), 2);
        assert!(manifests
            .iter()
            .all(|manifest| manifest.source_path.starts_with("content/skills/")));
    }

    #[test]
    fn remote_import_script_uses_remote_copy_and_move_not_streamed_cat() {
        let script = remote_import_skill_script();

        assert!(script.contains("cp -a"));
        assert!(script.contains("mv \"$stage_dir\" \"$target_dir\""));
        assert!(!script.contains("cat >"));
    }

    #[test]
    fn preview_workspace_reuse_requires_matching_target_repo_and_path() {
        let repo = GitHubRepoRef {
            owner: "openai".to_string(),
            repo: "skills".to_string(),
            branch: "main".to_string(),
            normalized_url: "https://github.com/openai/skills".to_string(),
        };
        let now = Utc::now();
        let workspace = GitHubPreviewWorkspace {
            id: "workspace-1".to_string(),
            target_id: "ssh-demo".to_string(),
            repo: repo.clone(),
            source_path: Some("content/skills".to_string()),
            remote_workspace_dir: "/tmp/skillport-github-preview.abc123".to_string(),
            remote_repo_dir: "/tmp/skillport-github-preview.abc123/repo".to_string(),
            created_at: now,
            expires_at: now + Duration::minutes(30),
        };

        assert!(workspace.matches_source("ssh-demo", &repo, Some("content/skills")));
        assert!(!workspace.matches_source("ssh-other", &repo, Some("content/skills")));
        assert!(!workspace.matches_source("ssh-demo", &repo, Some("other")));

        let other_repo = GitHubRepoRef {
            repo: "other".to_string(),
            ..repo
        };
        assert!(!workspace.matches_source("ssh-demo", &other_repo, Some("content/skills")));
    }

    #[test]
    fn remote_workspace_download_script_puts_token_only_in_stdin_script() {
        let token = "ghp_secret_for_test";
        let script = remote_workspace_download_script(Some(token)).expect("script");
        let command = crate::targets::shell_quote("sh -s --");

        assert!(script.contains("curl.conf"));
        assert!(script.contains("Authorization: Bearer ghp_secret_for_test"));
        assert!(
            !command.contains(token),
            "ssh command string must not contain the GitHub token"
        );
    }

    #[test]
    fn remote_workspace_download_script_enforces_archive_budgets() {
        let script = remote_workspace_download_script(None).expect("script");

        assert!(script.contains("archive_limit=134217728"));
        assert!(script.contains("archive_files_limit=20000"));
        assert!(script.contains("archive_expanded_limit=268435456"));
        assert!(script.contains("archive_entry_limit=33554432"));
        assert!(script.contains("wc -c < \"$archive_file\""));
        assert!(script.contains("find \"$repo_dir\" -type f -print"));
        assert!(script.contains("GitHub repository archive entry"));
    }

    #[test]
    fn nested_import_copy_is_limited_to_selected_skill_directory() {
        let snapshot = content_skills_snapshot();
        let files = collect_snapshot_source_files(
            &snapshot,
            "content/skills/development-workflows/code-auditor",
        )
        .expect("files");

        assert!(files.iter().any(|file| file.relative_path == "SKILL.md"));
        assert!(files
            .iter()
            .any(|file| file.relative_path == "references/checklist.md"));
        assert!(!files
            .iter()
            .any(|file| file.repo_path.contains("git-commit")));
    }

    #[tokio::test]
    async fn github_pat_secret_store_is_trimmed_and_empty_values_are_ignored() {
        let pool = setup_test_db().await;
        let secrets = MockSecretStore::default();

        set_github_pat_impl(&pool, &secrets, "  test-token  ".to_string())
            .await
            .expect("set token");
        assert_eq!(
            github_direct_auth_from_secret_store(&pool, &secrets)
                .await
                .expect("read token"),
            Some("test-token".to_string())
        );
        assert_eq!(
            db::get_setting(&pool, LEGACY_GITHUB_PAT_SETTING_KEY)
                .await
                .expect("legacy token removed"),
            None
        );

        assert!(set_github_pat_impl(&pool, &secrets, "   ".to_string())
            .await
            .is_err());
        clear_github_pat_impl(&pool, &secrets)
            .await
            .expect("clear token");
        assert_eq!(
            github_direct_auth_from_secret_store(&pool, &secrets)
                .await
                .expect("read empty"),
            None
        );
    }

    #[tokio::test]
    async fn legacy_github_pat_migrates_to_secret_store_and_deletes_setting() {
        let pool = setup_test_db().await;
        let secrets = MockSecretStore::default();
        db::set_setting(&pool, LEGACY_GITHUB_PAT_SETTING_KEY, "  legacy-token  ")
            .await
            .expect("set legacy token");

        assert_eq!(
            github_direct_auth_from_secret_store(&pool, &secrets)
                .await
                .expect("read migrated token"),
            Some("legacy-token".to_string())
        );
        assert_eq!(
            secrets.get(GITHUB_PAT_SECRET_KEY).expect("secret read"),
            Some("legacy-token".to_string())
        );
        assert_eq!(
            db::get_setting(&pool, LEGACY_GITHUB_PAT_SETTING_KEY)
                .await
                .expect("legacy token removed"),
            None
        );
        assert_eq!(
            db::get_setting(&pool, GITHUB_PAT_MIGRATION_SETTING_KEY)
                .await
                .expect("migration marker"),
            Some("1".to_string())
        );
    }

    #[tokio::test]
    async fn legacy_github_pat_migration_failure_keeps_setting_and_marker_absent() {
        let pool = setup_test_db().await;
        let secrets = MockSecretStore::default();
        secrets.set_set_error(SecretError::Other("vault unavailable".to_string()));
        db::set_setting(&pool, LEGACY_GITHUB_PAT_SETTING_KEY, " legacy-token ")
            .await
            .expect("set legacy token");

        let result = github_direct_auth_from_secret_store(&pool, &secrets).await;

        assert_eq!(
            result.expect("legacy fallback remains readable"),
            Some("legacy-token".to_string())
        );
        assert_eq!(
            db::get_setting(&pool, LEGACY_GITHUB_PAT_SETTING_KEY)
                .await
                .expect("legacy token retained"),
            Some(" legacy-token ".to_string())
        );
        assert_eq!(
            db::get_setting(&pool, GITHUB_PAT_MIGRATION_SETTING_KEY)
                .await
                .expect("no migration marker"),
            None
        );
    }

    #[tokio::test]
    async fn get_github_pat_state_reports_legacy_config_when_migration_fails() {
        let pool = setup_test_db().await;
        let secrets = MockSecretStore::default();
        secrets.set_set_error(SecretError::Other("vault unavailable".to_string()));
        db::set_setting(&pool, LEGACY_GITHUB_PAT_SETTING_KEY, " legacy-token ")
            .await
            .expect("set legacy token");

        let state = get_github_pat_state_impl(&pool, &secrets)
            .await
            .expect("state");

        assert!(state.configured);
        assert_eq!(state.storage_state, SecretStorageState::Missing);
        assert!(state
            .error
            .as_deref()
            .is_some_and(|error| error.contains("vault unavailable")));
        assert_eq!(
            db::get_setting(&pool, LEGACY_GITHUB_PAT_SETTING_KEY)
                .await
                .expect("legacy token retained"),
            Some(" legacy-token ".to_string())
        );
    }

    #[tokio::test]
    async fn reveal_github_pat_returns_stored_secret() {
        let pool = setup_test_db().await;
        let secrets = MockSecretStore::with_value(GITHUB_PAT_SECRET_KEY, " github_pat_saved ");

        let revealed = reveal_github_pat_impl(&pool, &secrets)
            .await
            .expect("reveal token");

        assert_eq!(revealed.as_deref(), Some("github_pat_saved"));
    }

    #[tokio::test]
    async fn reveal_github_pat_returns_session_secret() {
        let pool = setup_test_db().await;
        let secrets = MockSecretStore::default();
        secrets.set_next_state(SecretStorageState::Session);
        secrets
            .set(GITHUB_PAT_SECRET_KEY, "github_pat_session")
            .expect("session secret");

        let revealed = reveal_github_pat_impl(&pool, &secrets)
            .await
            .expect("reveal token");

        assert_eq!(revealed.as_deref(), Some("github_pat_session"));
    }

    #[tokio::test]
    async fn reveal_github_pat_uses_legacy_fallback_when_migration_fails() {
        let pool = setup_test_db().await;
        let secrets = MockSecretStore::default();
        secrets.set_set_error(SecretError::Other("vault unavailable".to_string()));
        db::set_setting(&pool, LEGACY_GITHUB_PAT_SETTING_KEY, " legacy-token ")
            .await
            .expect("set legacy token");

        let revealed = reveal_github_pat_impl(&pool, &secrets)
            .await
            .expect("legacy fallback");

        assert_eq!(revealed.as_deref(), Some("legacy-token"));
    }

    #[tokio::test]
    async fn reveal_github_pat_returns_none_when_missing() {
        let pool = setup_test_db().await;
        let secrets = MockSecretStore::default();

        let revealed = reveal_github_pat_impl(&pool, &secrets)
            .await
            .expect("reveal missing");

        assert_eq!(revealed, None);
    }

    #[tokio::test]
    async fn startup_github_pat_migration_records_sanitized_failure_log() {
        let pool = setup_test_db().await;
        let secrets = MockSecretStore::default();
        secrets.set_set_error(SecretError::Other("vault unavailable".to_string()));
        db::set_setting(&pool, LEGACY_GITHUB_PAT_SETTING_KEY, " legacy-token ")
            .await
            .expect("set legacy token");

        migrate_github_pat_on_startup(&pool, &secrets)
            .await
            .expect("startup migration keeps app usable");

        let page = db::list_operation_logs(
            &pool,
            db::OperationLogFilter {
                action: Some("settings.github_pat_migration".to_string()),
                ..Default::default()
            },
        )
        .await
        .expect("list operation logs");
        assert_eq!(page.total, 1);
        let entry = &page.entries[0];
        assert_eq!(entry.status, "failed");
        assert_eq!(
            entry.error_summary.as_deref(),
            Some("Failed to migrate GitHub token: vault unavailable")
        );
        let details: Value =
            serde_json::from_str(entry.details_json.as_deref().expect("details json present"))
                .expect("details json");
        assert_eq!(details["legacySettingRetained"], true);
        assert_eq!(details["key"], LEGACY_GITHUB_PAT_SETTING_KEY);
        assert!(!entry
            .details_json
            .as_deref()
            .unwrap_or_default()
            .contains("legacy-token"));
    }

    #[tokio::test]
    async fn empty_legacy_github_pat_is_ignored_without_marker() {
        let pool = setup_test_db().await;
        let secrets = MockSecretStore::default();
        db::set_setting(&pool, LEGACY_GITHUB_PAT_SETTING_KEY, "   ")
            .await
            .expect("set empty legacy token");

        assert_eq!(
            github_direct_auth_from_secret_store(&pool, &secrets)
                .await
                .expect("read token"),
            None
        );
        assert_eq!(
            db::get_setting(&pool, GITHUB_PAT_MIGRATION_SETTING_KEY)
                .await
                .expect("no migration marker"),
            None
        );
    }

    #[tokio::test]
    async fn authenticated_api_fallback_does_not_forward_bearer_auth_to_mirror() {
        use std::io::{Read, Write};
        use std::net::TcpListener;
        use std::sync::{
            atomic::{AtomicUsize, Ordering},
            Arc, Mutex,
        };

        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let address = listener.local_addr().expect("addr");
        let requests = Arc::new(Mutex::new(Vec::<String>::new()));
        let accepted = Arc::new(AtomicUsize::new(0));
        let requests_clone = Arc::clone(&requests);
        let accepted_clone = Arc::clone(&accepted);

        let server = std::thread::spawn(move || {
            while accepted_clone.load(Ordering::SeqCst) < 2 {
                let (mut stream, _) = listener.accept().expect("accept");
                let mut buffer = [0_u8; 2048];
                let bytes_read = stream.read(&mut buffer).expect("read");
                let request_text = String::from_utf8_lossy(&buffer[..bytes_read]).to_string();
                requests_clone
                    .lock()
                    .expect("lock")
                    .push(request_text.clone());
                accepted_clone.fetch_add(1, Ordering::SeqCst);

                if request_text.contains("GET /direct") {
                    let response =
                        "HTTP/1.1 502 Bad Gateway\r\nContent-Length: 11\r\n\r\nbad gateway";
                    stream.write_all(response.as_bytes()).expect("write direct");
                } else {
                    let response = "HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok";
                    stream.write_all(response.as_bytes()).expect("write mirror");
                }
            }
        });

        let client = github_client().expect("client");
        let direct_url = format!("http://{}/direct", address);
        let mirror_url = format!("http://{}/mirror", address);

        let response = send_github_request_with_fallback(
            &client,
            GitHubFetchSurface::Api,
            |endpoint| {
                if endpoint.label == "github" {
                    direct_url.clone()
                } else {
                    mirror_url.clone()
                }
            },
            "direct request failed",
            Some("direct-token"),
        )
        .await
        .expect("fallback response");
        assert!(response.status().is_success());

        server.join().expect("server join");
        let captured = requests.lock().expect("captured");
        let direct_request = captured
            .iter()
            .find(|request| request.contains("GET /direct"))
            .expect("captured direct request");
        let mirror_request = captured
            .iter()
            .find(|request| request.contains("GET /mirror"))
            .expect("captured mirror request");
        assert!(
            direct_request.contains("authorization: Bearer direct-token")
                || direct_request.contains("Authorization: Bearer direct-token"),
            "direct github request should include bearer auth"
        );
        assert!(
            !mirror_request.contains("authorization: Bearer direct-token")
                && !mirror_request.contains("Authorization: Bearer direct-token"),
            "mirror request should not include bearer auth"
        );
    }

    #[tokio::test]
    async fn authenticated_raw_fallback_does_not_forward_bearer_auth_to_mirror() {
        use std::io::{Read, Write};
        use std::net::TcpListener;
        use std::sync::{
            atomic::{AtomicUsize, Ordering},
            Arc, Mutex,
        };

        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let address = listener.local_addr().expect("addr");
        let requests = Arc::new(Mutex::new(Vec::<String>::new()));
        let accepted = Arc::new(AtomicUsize::new(0));
        let requests_clone = Arc::clone(&requests);
        let accepted_clone = Arc::clone(&accepted);

        let server = std::thread::spawn(move || {
            while accepted_clone.load(Ordering::SeqCst) < 2 {
                let (mut stream, _) = listener.accept().expect("accept");
                let mut buffer = [0_u8; 2048];
                let bytes_read = stream.read(&mut buffer).expect("read");
                let request_text = String::from_utf8_lossy(&buffer[..bytes_read]).to_string();
                requests_clone
                    .lock()
                    .expect("lock")
                    .push(request_text.clone());
                accepted_clone.fetch_add(1, Ordering::SeqCst);

                if request_text.contains("GET /raw-direct") {
                    let response = "HTTP/1.1 503 Service Unavailable\r\nContent-Length: 19\r\n\r\nservice unavailable";
                    stream.write_all(response.as_bytes()).expect("write direct");
                } else {
                    let response = "HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok";
                    stream.write_all(response.as_bytes()).expect("write mirror");
                }
            }
        });

        let client = github_client().expect("client");
        let direct_url = format!("http://{}/raw-direct", address);
        let mirror_url = format!("http://{}/raw-mirror", address);

        let response = send_github_request_with_fallback(
            &client,
            GitHubFetchSurface::Raw,
            |endpoint| {
                if endpoint.label == "github" {
                    direct_url.clone()
                } else {
                    mirror_url.clone()
                }
            },
            "raw request failed",
            Some("direct-token"),
        )
        .await
        .expect("fallback response");
        assert!(response.status().is_success());

        server.join().expect("server join");
        let captured = requests.lock().expect("captured");
        let direct_request = captured
            .iter()
            .find(|request| request.contains("GET /raw-direct"))
            .expect("captured direct request");
        let mirror_request = captured
            .iter()
            .find(|request| request.contains("GET /raw-mirror"))
            .expect("captured mirror request");
        assert!(
            direct_request.contains("authorization: Bearer direct-token")
                || direct_request.contains("Authorization: Bearer direct-token"),
            "direct raw request should include bearer auth"
        );
        assert!(
            !mirror_request.contains("authorization: Bearer direct-token")
                && !mirror_request.contains("Authorization: Bearer direct-token"),
            "mirror raw request should not include bearer auth"
        );
    }

    #[tokio::test]
    async fn unauthenticated_rate_limit_retries_public_mirror_before_failing() {
        use std::io::{Read, Write};
        use std::net::TcpListener;
        use std::sync::{
            atomic::{AtomicUsize, Ordering},
            Arc, Mutex,
        };

        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let address = listener.local_addr().expect("addr");
        let requests = Arc::new(Mutex::new(Vec::<String>::new()));
        let accepted = Arc::new(AtomicUsize::new(0));
        let requests_clone = Arc::clone(&requests);
        let accepted_clone = Arc::clone(&accepted);

        let server = std::thread::spawn(move || {
            while accepted_clone.load(Ordering::SeqCst) < 2 {
                let (mut stream, _) = listener.accept().expect("accept");
                let mut buffer = [0_u8; 2048];
                let bytes_read = stream.read(&mut buffer).expect("read");
                let request_text = String::from_utf8_lossy(&buffer[..bytes_read]).to_string();
                let is_direct = request_text.contains("GET /direct");
                requests_clone.lock().expect("lock").push(request_text);
                accepted_clone.fetch_add(1, Ordering::SeqCst);

                if is_direct {
                    let response = concat!(
                        "HTTP/1.1 403 Forbidden\r\n",
                        "Content-Type: application/json\r\n",
                        "X-RateLimit-Remaining: 0\r\n",
                        "X-RateLimit-Reset: 1786576453\r\n",
                        "Content-Length: 48\r\n\r\n",
                        "{\"message\":\"API rate limit exceeded for 1.2.3.4\"}"
                    );
                    stream.write_all(response.as_bytes()).expect("write direct");
                } else {
                    let response = "HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok";
                    stream.write_all(response.as_bytes()).expect("write mirror");
                }
            }
        });

        let client = github_client().expect("client");
        let direct_url = format!("http://{}/direct", address);
        let mirror_url = format!("http://{}/mirror", address);

        let response = send_github_request_with_fallback(
            &client,
            GitHubFetchSurface::Api,
            |endpoint| {
                if endpoint.label == "github" {
                    direct_url.clone()
                } else {
                    mirror_url.clone()
                }
            },
            "request failed",
            None,
        )
        .await
        .expect("mirror retry response");
        assert!(response.status().is_success());

        server.join().expect("server join");
        let captured = requests.lock().expect("captured");
        assert!(captured
            .iter()
            .any(|request| request.contains("GET /direct")));
        assert!(captured
            .iter()
            .any(|request| request.contains("GET /mirror")));
    }
}
