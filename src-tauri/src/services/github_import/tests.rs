use super::*;
#[cfg(test)]
pub(super) mod suite {
    use super::*;
    use crate::secrets::{
        MockSecretStore, SecretError, SecretStorageState, SecretStore, GITHUB_PAT_SECRET_KEY,
    };
    use crate::services::resource_budget::ResourceBudget;
    use flate2::{write::GzEncoder, Compression};
    use serde_json::Value;
    use std::collections::HashMap;
    use std::io::{Read, Write};
    use std::net::{SocketAddr, TcpListener};
    use std::sync::{Arc, Mutex};
    use tempfile::tempdir;

    async fn setup_test_db() -> DbPool {
        let (pool, dir) = crate::test_support::file_pool().await;
        // 历史行为：泄漏 TempDir 让 db 文件活过测试生命周期。
        std::mem::forget(dir);
        pool
    }

    fn ssh_test_target(id: &str) -> ActiveTarget {
        ActiveTarget::Ssh(Box::new(crate::targets::RemoteTargetConfig {
            id: id.to_string(),
            label: format!("SSH {id}"),
            host: "example.com".to_string(),
            username: "alice".to_string(),
            port: 22,
            auth_method: crate::targets::SshAuthMethod::Key,
            key_path: "~/.ssh/id_ed25519".to_string(),
            credential_key: None,
            protected_password: None,
            password: None,
            remote_home: "/home/alice".to_string(),
            remote_os: "Linux".to_string(),
            symlink_enabled: true,
        }))
    }

    fn fake_ssh_connection(
        id: &str,
        runner: Arc<crate::test_support::FakeRunner>,
    ) -> ConnectedRemoteTarget {
        ConnectedRemoteTarget::Ssh(crate::targets::ConnectedSshTarget::for_tests_with_runner(
            crate::targets::RemoteTargetConfig {
                id: id.to_string(),
                label: format!("SSH {id}"),
                host: "example.com".to_string(),
                username: "alice".to_string(),
                port: 22,
                auth_method: crate::targets::SshAuthMethod::Key,
                key_path: "~/.ssh/id_ed25519".to_string(),
                credential_key: None,
                protected_password: None,
                password: None,
                remote_home: "/home/alice".to_string(),
                remote_os: "Linux".to_string(),
                symlink_enabled: true,
            },
            runner,
        ))
    }

    fn register_expired_remote_snapshot(target_id: &str) -> String {
        register_expired_remote_snapshot_with_kind(target_id, TargetKind::Ssh)
    }

    fn register_expired_remote_snapshot_with_kind(
        target_id: &str,
        target_kind: TargetKind,
    ) -> String {
        let now = Utc::now();
        let mut reservation = match reserve_remote_preview_snapshot(
            target_id,
            target_kind,
            now - Duration::minutes(2),
        )
        .expect("reserve remote preview")
        {
            RemoteReservationAttempt::Reserved(reservation) => reservation,
            other => panic!("expected remote reservation, got {other:?}"),
        };
        let id = reservation.preview_id().to_string();
        let mut snapshot = remote_test_snapshot(None);
        snapshot.id = id.clone();
        snapshot.target_id = target_id.to_string();
        snapshot.target_kind = target_kind;
        snapshot.created_at = now - Duration::minutes(2);
        snapshot.expires_at = now - Duration::minutes(1);
        if let PreviewSnapshotStorage::Remote(workspace) = &mut snapshot.storage {
            workspace.remote_workspace_dir = format!("/tmp/{target_id}");
            workspace.remote_repo_dir = format!("/tmp/{target_id}/repo");
        }
        reservation.fill(snapshot).expect("fill remote preview");
        id
    }

    /// A registered remote snapshot bound to `ssh-demo` / `openai/skills@main`.
    fn remote_test_snapshot(source_path: Option<&str>) -> PreviewSnapshot {
        let now = Utc::now();
        PreviewSnapshot {
            id: "github-preview-binding".to_string(),
            target_id: "ssh-demo".to_string(),
            target_kind: TargetKind::Ssh,
            repo: GitHubRepoRef {
                owner: "openai".to_string(),
                repo: "skills".to_string(),
                branch: "main".to_string(),
                normalized_url: "https://github.com/openai/skills".to_string(),
            },
            source_path: source_path.map(str::to_string),
            resolved_commit_sha: "a".repeat(40),
            snapshot_digest: "sha256-v1:binding".to_string(),
            files: Vec::new(),
            candidates: Vec::new(),
            created_at: now,
            expires_at: now + Duration::minutes(30),
            storage: PreviewSnapshotStorage::Remote(GitHubPreviewWorkspace {
                remote_workspace_dir: "/tmp/skillport-github-preview.abc123".to_string(),
                remote_repo_dir: "/tmp/skillport-github-preview.abc123/repo".to_string(),
            }),
        }
    }

    /// A registered local snapshot for the given retained repository snapshot.
    fn local_test_snapshot(
        repo: &GitHubRepoRef,
        source_path: Option<&str>,
        snapshot: GitHubRepoSnapshot,
        candidates: Vec<PreviewSnapshotCandidate>,
    ) -> PreviewSnapshot {
        let now = Utc::now();
        let files = snapshot_files_from_local(&snapshot);
        PreviewSnapshot {
            id: "github-preview-local".to_string(),
            target_id: ActiveTarget::Local.id().to_string(),
            target_kind: TargetKind::Local,
            repo: repo.clone(),
            source_path: source_path.map(str::to_string),
            resolved_commit_sha: "b".repeat(40),
            snapshot_digest: repository_snapshot_digest(&files),
            files,
            candidates,
            created_at: now,
            expires_at: now + Duration::minutes(30),
            storage: PreviewSnapshotStorage::Local(Arc::new(snapshot)),
        }
    }

    fn test_mirror_endpoints(direct_url: String, mirror_url: String) -> Vec<GitHubMirrorEndpoint> {
        let direct_url: &'static str = Box::leak(direct_url.into_boxed_str());
        let mirror_url: &'static str = Box::leak(mirror_url.into_boxed_str());
        vec![
            GitHubMirrorEndpoint {
                label: "github",
                api_base: direct_url,
                raw_base: direct_url,
            },
            GitHubMirrorEndpoint {
                label: "mirror-one",
                api_base: mirror_url,
                raw_base: mirror_url,
            },
            GitHubMirrorEndpoint {
                label: "mirror-two",
                api_base: mirror_url,
                raw_base: mirror_url,
            },
        ]
    }

    fn spawn_http_sequence<F>(
        build_responses: F,
    ) -> (String, Arc<Mutex<Vec<String>>>, std::thread::JoinHandle<()>)
    where
        F: FnOnce(SocketAddr) -> Vec<Vec<u8>>,
    {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let address = listener.local_addr().expect("addr");
        let responses = build_responses(address);
        let requests = Arc::new(Mutex::new(Vec::new()));
        let captured_requests = Arc::clone(&requests);
        let server = std::thread::spawn(move || {
            for response in responses {
                let (mut stream, _) = listener.accept().expect("accept");
                let mut buffer = [0_u8; 4096];
                let bytes_read = stream.read(&mut buffer).expect("read");
                captured_requests
                    .lock()
                    .expect("lock")
                    .push(String::from_utf8_lossy(&buffer[..bytes_read]).to_string());
                stream.write_all(&response).expect("write response");
            }
        });

        (format!("http://{address}"), requests, server)
    }

    fn http_response(status: &str, locations: &[String], body: &[u8]) -> Vec<u8> {
        let locations = locations
            .iter()
            .map(|location| format!("Location: {location}\r\n"))
            .collect::<String>();
        let headers = format!(
            "HTTP/1.1 {status}\r\n{locations}Content-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        let mut response = headers.into_bytes();
        response.extend_from_slice(body);
        response
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

    fn root_package_snapshot() -> GitHubRepoSnapshot {
        repo_snapshot(&[
            (
                "SKILL.md",
                sample_frontmatter("huashu-design", "root package skill"),
            ),
            ("README.md", "# Huashu Design\n".to_string()),
            ("assets/example.txt", "asset\n".to_string()),
            ("references/guide.md", "# Guide\n".to_string()),
            ("scripts/run.py", "print('ok')\n".to_string()),
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

    fn repository_level_singular_skill_snapshot() -> GitHubRepoSnapshot {
        repo_snapshot(&[
            (
                "skill/SKILL.md",
                sample_frontmatter("kill-ai-slop", "Find and remove AI slop"),
            ),
            (
                "skill/references/detection.md",
                "# Detection patterns\n".to_string(),
            ),
            (
                "website/src/pages/index.astro",
                "<main>Website content</main>\n".to_string(),
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
    fn reconcile_selected_branch_preserves_default_and_explicit_selection() {
        assert_eq!(
            reconcile_selected_branch(None, None).expect("default branch"),
            None
        );
        assert_eq!(
            reconcile_selected_branch(None, Some("   ")).expect("blank branch"),
            None
        );
        assert_eq!(
            reconcile_selected_branch(None, Some("  dev  ")).expect("explicit branch"),
            Some("dev".to_string())
        );
        assert_eq!(
            reconcile_selected_branch(Some("dev"), None).expect("URL branch"),
            Some("dev".to_string())
        );
        assert_eq!(
            reconcile_selected_branch(Some("dev"), Some("dev")).expect("matching branches"),
            Some("dev".to_string())
        );
    }

    #[test]
    fn reconcile_selected_branch_rejects_conflicts_and_unsafe_names() {
        assert!(matches!(
            reconcile_selected_branch(Some("dev"), Some("main")),
            Err(GithubImportError::BranchSelectionConflict)
        ));

        for branch in ["feature/foo", r"feature\foo", "dev\nnext"] {
            assert!(
                matches!(
                    reconcile_selected_branch(None, Some(branch)),
                    Err(GithubImportError::InvalidBranchSelection)
                ),
                "branch should be rejected: {branch:?}"
            );
        }
    }

    #[test]
    fn parse_github_source_rejects_non_github_hosts() {
        let error = parse_github_source("https://gitlab.com/example/repo").unwrap_err();
        assert!(error.to_string().contains("github.com"));
    }

    #[test]
    fn parse_github_source_rejects_authority_and_url_suffix_controls() {
        for source in [
            "https://user@github.com/example/repo",
            "https://github.com:444/example/repo",
            "https://github.com/example/repo?download=1",
            "https://github.com/example/repo#readme",
        ] {
            assert!(
                matches!(
                    parse_github_source(source),
                    Err(GithubImportError::InvalidRepoUrl)
                ),
                "source should be rejected: {source}"
            );
        }
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
    fn repo_file_path_mapping_distinguishes_root_and_nested_sources() {
        assert_eq!(
            repo_file_relative_to_source("references/guide.md", ".").as_deref(),
            Some("references/guide.md")
        );
        assert_eq!(
            repo_file_relative_to_source(
                "skills/agent-browser/references/guide.md",
                "skills/agent-browser",
            )
            .as_deref(),
            Some("references/guide.md")
        );
        assert_eq!(
            repo_file_relative_to_source("skill-data/core/SKILL.md", "skills/agent-browser"),
            None
        );
        assert_eq!(
            repo_file_relative_to_source("skills/agent-browser", "skills/agent-browser"),
            None
        );
    }

    #[tokio::test]
    async fn preview_file_manifest_uses_root_content_boundary_and_serializes_camel_case() {
        let pool = setup_test_db().await;
        let snapshot = root_package_snapshot();
        let repo = GitHubRepoRef {
            owner: "alchaincyf".to_string(),
            repo: "huashu-design".to_string(),
            branch: "master".to_string(),
            normalized_url: "https://github.com/alchaincyf/huashu-design".to_string(),
        };
        let candidates =
            build_repo_skill_candidates_from_snapshot(&repo, &snapshot).expect("root candidate");
        let mut previews = build_preview_skills(&pool, &candidates)
            .await
            .expect("preview skills");

        assert!(serde_json::to_value(&previews[0])
            .expect("serialize preview")
            .get("files")
            .is_none());

        let candidate_digests = attach_preview_file_manifests(
            &mut previews,
            &snapshot_preview_repository_files(&snapshot),
        )
        .expect("attach files");

        let files = previews[0].files.as_ref().expect("file manifest");
        assert_eq!(
            files
                .iter()
                .map(|file| file.path.as_str())
                .collect::<Vec<_>>(),
            vec![
                "README.md",
                "SKILL.md",
                "assets/example.txt",
                "references/guide.md",
                "scripts/run.py",
            ]
        );
        assert_eq!(candidate_digests.len(), 1);
        assert!(candidate_digests[0]
            .content_digest
            .starts_with("sha256-v1:"));
        let serialized = serde_json::to_value(&previews[0]).expect("serialize preview");
        assert_eq!(serialized["files"][0]["path"], "README.md");
        assert_eq!(serialized["files"][0]["byteLen"], 16);
        assert!(serialized["files"][0].get("byte_len").is_none());
        assert!(serialized["files"][0]["sha256"]
            .as_str()
            .expect("sha256 field")
            .starts_with("sha256-v1:"));
    }

    #[tokio::test]
    async fn preview_file_manifest_limits_nested_candidate_to_its_source_subtree() {
        let pool = setup_test_db().await;
        let snapshot = content_skills_snapshot();
        let repo = GitHubRepoRef {
            owner: "example".to_string(),
            repo: "skills".to_string(),
            branch: "main".to_string(),
            normalized_url: "https://github.com/example/skills".to_string(),
        };
        let candidates =
            build_repo_skill_candidates_from_snapshot(&repo, &snapshot).expect("nested candidates");
        let candidate = candidates
            .iter()
            .find(|candidate| candidate.skill_id == "code-auditor")
            .expect("code-auditor candidate")
            .clone();
        let mut previews = build_preview_skills(&pool, &[candidate])
            .await
            .expect("preview skills");

        attach_preview_file_manifests(&mut previews, &snapshot_preview_repository_files(&snapshot))
            .expect("attach files");

        let files = previews[0].files.as_ref().expect("file manifest");
        assert_eq!(
            files
                .iter()
                .map(|file| file.path.as_str())
                .collect::<Vec<_>>(),
            vec!["SKILL.md", "references/checklist.md"]
        );
    }

    #[test]
    fn remote_preview_file_manifest_parser_is_stable_and_budgeted() {
        let guide_digest = "1".repeat(64);
        let skill_digest = "2".repeat(64);
        let output = format!(
            "references/guide.md\x005\x00{guide_digest}\x00SKILL.md\x0012\x00{skill_digest}\x00"
        );
        let files = parse_remote_preview_repository_files(&output).expect("parse manifest");

        assert_eq!(
            files
                .iter()
                .map(|file| (file.repo_path.as_str(), file.byte_len))
                .collect::<Vec<_>>(),
            vec![("SKILL.md", 12), ("references/guide.md", 5)]
        );
        assert_eq!(files[0].sha256[0], 0x22);
        assert_eq!(files[1].sha256[0], 0x11);
        assert!(REMOTE_PREVIEW_FILE_INVENTORY_SCRIPT.contains("find \"$repo_dir\" -type f"));
        assert!(REMOTE_PREVIEW_FILE_INVENTORY_SCRIPT.contains("wc -c < \"$file\""));
        assert!(REMOTE_PREVIEW_FILE_INVENTORY_SCRIPT.contains("printf \"%s\\0%s\\0%s\\0\""));
        assert!(REMOTE_PREVIEW_FILE_INVENTORY_SCRIPT.contains("sha256sum"));
    }

    #[test]
    fn remote_preview_file_manifest_parser_rejects_malformed_or_duplicate_records() {
        let digest = "3".repeat(64);
        for malformed in [
            format!("SKILL.md\x0012\x00{digest}"),
            format!("SKILL.md\x0012\x00{digest}\x00SKILL.md\x0012\x00{digest}\x00"),
            format!("SKILL.md\x00not-a-size\x00{digest}\x00"),
            "SKILL.md\x0012\x00short-digest\x00".to_string(),
            format!("SKILL.md\x0012\x00{}\x00", "z".repeat(64)),
        ] {
            assert!(
                matches!(
                    parse_remote_preview_repository_files(&malformed),
                    Err(GithubImportError::RemotePreviewInvalidFileManifest)
                ),
                "expected rejection for {malformed:?}"
            );
        }
    }

    #[tokio::test]
    async fn remote_preview_file_inventory_uses_one_fake_runner_script_call() {
        let runner = Arc::new(crate::test_support::FakeRunner::new());
        runner.push_success(&format!(
            "SKILL.md\x0012\x00{}\x00references/guide.md\x005\x00{}\x00",
            "a".repeat(64),
            "b".repeat(64)
        ));
        let connection =
            ConnectedRemoteTarget::Ssh(crate::targets::ConnectedSshTarget::for_tests_with_runner(
                crate::targets::RemoteTargetConfig {
                    id: "ssh-preview".to_string(),
                    label: "SSH Preview".to_string(),
                    host: "example.com".to_string(),
                    username: "alice".to_string(),
                    port: 22,
                    auth_method: crate::targets::SshAuthMethod::Key,
                    key_path: "~/.ssh/id_ed25519".to_string(),
                    credential_key: None,
                    protected_password: None,
                    password: None,
                    remote_home: "/home/alice".to_string(),
                    remote_os: "Linux".to_string(),
                    symlink_enabled: true,
                },
                runner.clone(),
            ));

        let files = remote_preview_repository_files(&connection, "/tmp/preview/repo")
            .await
            .expect("remote inventory");

        assert_eq!(files.len(), 2);
        let calls = runner.calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(
            calls[0].stdin.as_deref(),
            Some(REMOTE_PREVIEW_FILE_INVENTORY_SCRIPT.as_bytes())
        );
        assert_eq!(
            calls[0].args.last().map(String::as_str),
            Some("sh -s -- '/tmp/preview/repo'")
        );
    }

    #[test]
    fn preview_file_manifest_fails_closed_without_root_skill_markdown() {
        let mut previews = vec![GitHubSkillPreview {
            source_path: ".".to_string(),
            skill_id: "demo".to_string(),
            skill_name: "Demo".to_string(),
            description: None,
            plugin_name: None,
            root_directory: "/".to_string(),
            skill_directory_name: "demo".to_string(),
            download_url: "https://example.com/SKILL.md".to_string(),
            conflict: None,
            files: None,
        }];
        let repository_files = vec![PreviewSnapshotFile {
            repo_path: "README.md".to_string(),
            byte_len: 4,
            sha256: [0_u8; 32],
        }];

        assert!(matches!(
            attach_preview_file_manifests(&mut previews, &repository_files),
            Err(GithubImportError::PreviewFileManifestIncomplete(path)) if path == "."
        ));
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
            used_auth: false,
        };

        let message = denial.to_string();

        assert!(message.contains("rate limit was exceeded"));
        assert!(message.contains("Retry later after 2026-04-17 12:34:56 UTC"));
        assert!(message.contains("authenticated GitHub requests"));
        assert!(!message.contains("API rate limit exceeded"));
        assert!(!message.contains("1.2.3.4"));
    }

    #[test]
    fn classify_github_permission_denial_returns_actionable_message() {
        let denial = GitHubAccessDenial {
            kind: GitHubAccessDenialKind::AuthenticationOrPermission,
            operation: "reading repository contents",
            status: reqwest::StatusCode::UNAUTHORIZED,
            used_auth: false,
        };

        let message = denial.to_string();

        assert!(message.contains("denied access"));
        assert!(message.contains("require authentication"));
        assert!(message.contains("token/permissions are insufficient"));
        assert!(!message.contains("Requires authentication"));
    }

    #[test]
    fn structured_raw_url_ignores_renderer_normalized_url_authority() {
        let endpoint = GITHUB_MIRROR_ENDPOINTS.first().expect("github endpoint");
        let repo = GitHubRepoRef {
            owner: "owner".to_string(),
            repo: "repo".to_string(),
            branch: "main#fragment".to_string(),
            normalized_url: "http://169.254.169.254/latest/meta-data".to_string(),
        };
        let url = raw_file_url(endpoint, &repo, "skills/demo/SKILL.md");
        let parsed = reqwest::Url::parse(&url).expect("built raw URL");

        assert_eq!(parsed.scheme(), "https");
        assert_eq!(parsed.host_str(), Some("raw.githubusercontent.com"));
        assert!(parsed.fragment().is_none());
        assert!(parsed.path().contains("main%23fragment"));
        validate_github_endpoint_request(endpoint, GitHubFetchSurface::Raw, &url)
            .expect("fixed endpoint URL");
    }

    #[test]
    fn endpoint_policy_rejects_ssrf_url_matrix() {
        let endpoint = GITHUB_MIRROR_ENDPOINTS.first().expect("github endpoint");
        for url in [
            "http://raw.githubusercontent.com/owner/repo/main/SKILL.md",
            "file:///etc/passwd",
            "ftp://raw.githubusercontent.com/file",
            "https://127.0.0.1/SKILL.md",
            "https://[::1]/SKILL.md",
            "https://10.0.0.1/SKILL.md",
            "https://172.16.0.1/SKILL.md",
            "https://192.168.0.1/SKILL.md",
            "https://169.254.169.254/latest/meta-data",
            "https://raw.githubusercontent.com.evil.example/SKILL.md",
            "https://user@raw.githubusercontent.com/owner/repo/main/SKILL.md",
            "https://raw.githubusercontent.com:444/owner/repo/main/SKILL.md",
            "https://raw.githubusercontent.com/owner/repo/main/SKILL.md#fragment",
        ] {
            assert!(
                validate_github_endpoint_request(endpoint, GitHubFetchSurface::Raw, url).is_err(),
                "dangerous URL should be rejected: {url}"
            );
        }
    }

    #[tokio::test]
    async fn owning_target_cleanup_failure_remains_pending_until_retry_ack() {
        let target_id = "ssh-cleanup-retry";
        let preview_id = register_expired_remote_snapshot(target_id);
        let runner = Arc::new(crate::test_support::FakeRunner::new());
        runner.push_output(1, "", "remove failed");
        let connection = fake_ssh_connection(target_id, Arc::clone(&runner));

        let tickets = sweep_preview_snapshots_for_target(target_id, Utc::now());
        assert_eq!(tickets.len(), 1);
        assert!(!cleanup_preview_tickets_for_connection(&connection, tickets).await);
        assert!(matches!(
            lookup_preview_snapshot(&preview_id, Utc::now()),
            Err(GithubImportError::PreviewCleanupPending)
        ));
        assert!(matches!(
            acquire_import_lease(&preview_id, Utc::now()),
            Err(GithubImportError::PreviewCleanupPending)
        ));

        runner.push_success("");
        let retry = sweep_preview_snapshots_for_target(target_id, Utc::now());
        assert_eq!(retry.len(), 1);
        assert!(cleanup_preview_tickets_for_connection(&connection, retry).await);
        assert!(!preview_snapshot_is_registered(&preview_id));
        assert_eq!(runner.calls().len(), 2);
    }

    #[tokio::test]
    async fn target_a_connection_never_removes_target_b_workspace() {
        let target_a = "ssh-cleanup-a";
        let target_b = "ssh-cleanup-b";
        let preview_a = register_expired_remote_snapshot(target_a);
        let preview_b = register_expired_remote_snapshot(target_b);
        let runner_a = Arc::new(crate::test_support::FakeRunner::new());
        let runner_b = Arc::new(crate::test_support::FakeRunner::new());
        runner_a.push_success("");
        runner_b.push_success("");
        let connection_a = fake_ssh_connection(target_a, Arc::clone(&runner_a));
        let connection_b = fake_ssh_connection(target_b, Arc::clone(&runner_b));

        let tickets_a = sweep_preview_snapshots_for_target(target_a, Utc::now());
        assert!(cleanup_preview_tickets_for_connection(&connection_a, tickets_a).await);
        assert!(!preview_snapshot_is_registered(&preview_a));
        assert!(preview_snapshot_is_registered(&preview_b));
        assert_eq!(runner_a.calls().len(), 1);
        assert!(runner_b.calls().is_empty());

        let tickets_b = sweep_preview_snapshots_for_target(target_b, Utc::now());
        assert!(cleanup_preview_tickets_for_connection(&connection_b, tickets_b).await);
        assert!(!preview_snapshot_is_registered(&preview_b));
        assert_eq!(runner_b.calls().len(), 1);
    }

    #[tokio::test]
    async fn connection_kind_mismatch_never_removes_or_acknowledges_workspace() {
        let target_id = "same-id-different-kind";
        let preview_id = register_expired_remote_snapshot_with_kind(target_id, TargetKind::Wsl);
        let runner = Arc::new(crate::test_support::FakeRunner::new());
        runner.push_success("");
        let connection = fake_ssh_connection(target_id, Arc::clone(&runner));

        let tickets = sweep_preview_snapshots_for_target(target_id, Utc::now());
        assert_eq!(tickets.len(), 1);
        assert!(!cleanup_preview_tickets_for_connection(&connection, tickets.clone()).await);
        assert!(runner.calls().is_empty());
        assert!(matches!(
            lookup_preview_snapshot(&preview_id, Utc::now()),
            Err(GithubImportError::PreviewCleanupPending)
        ));

        assert!(ack_preview_snapshot_cleanup(&tickets[0]));
    }

    #[test]
    fn built_in_endpoint_urls_satisfy_their_declared_policies() {
        let repo = GitHubRepoRef {
            owner: "owner".to_string(),
            repo: "repo".to_string(),
            branch: "main".to_string(),
            normalized_url: "https://github.com/owner/repo".to_string(),
        };

        for endpoint in GITHUB_MIRROR_ENDPOINTS {
            let api_url =
                github_endpoint_url(endpoint, GitHubFetchSurface::Api, "/repos/owner/repo");
            validate_github_endpoint_request(endpoint, GitHubFetchSurface::Api, &api_url)
                .unwrap_or_else(|error| panic!("{} API URL: {error}", endpoint.label));

            let raw_url = raw_file_url(endpoint, &repo, "skills/demo/SKILL.md");
            validate_github_endpoint_request(endpoint, GitHubFetchSurface::Raw, &raw_url)
                .unwrap_or_else(|error| panic!("{} raw URL: {error}", endpoint.label));
        }
    }

    #[test]
    fn structured_repo_components_reject_authority_and_path_injection() {
        for (field, value) in [
            ("owner", "https://127.0.0.1"),
            ("owner", "owner/other"),
            ("repo", "repo\\other"),
            ("branch", "main/other"),
            ("branch", ".."),
        ] {
            let mut repo = GitHubRepoRef {
                owner: "owner".to_string(),
                repo: "repo".to_string(),
                branch: "main".to_string(),
                normalized_url: "https://github.com/owner/repo".to_string(),
            };
            match field {
                "owner" => repo.owner = value.to_string(),
                "repo" => repo.repo = value.to_string(),
                "branch" => repo.branch = value.to_string(),
                _ => unreachable!(),
            }
            assert!(validate_repo_ref(&repo).is_err(), "{field}={value}");
        }
    }

    #[tokio::test]
    async fn github_client_does_not_follow_redirects() {
        use std::io::{Read, Write};
        use std::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let address = listener.local_addr().expect("address");
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept");
            let mut buffer = [0_u8; 1024];
            let _ = stream.read(&mut buffer).expect("read request");
            stream
                .write_all(
                    b"HTTP/1.1 302 Found\r\nLocation: http://127.0.0.1:9/private\r\nContent-Length: 0\r\n\r\n",
                )
                .expect("write redirect");
        });

        let response = github_client()
            .expect("client")
            .get(format!("http://{address}/redirect"))
            .send()
            .await
            .expect("redirect response");

        assert_eq!(response.status(), reqwest::StatusCode::FOUND);
        server.join().expect("server join");
    }

    #[tokio::test]
    async fn chunked_raw_body_stops_at_the_streaming_budget() {
        use std::io::{Read, Write};
        use std::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let address = listener.local_addr().expect("address");
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept");
            let mut buffer = [0_u8; 1024];
            let _ = stream.read(&mut buffer).expect("read request");
            stream
                .write_all(
                    b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n6\r\nabcdef\r\n",
                )
                .expect("write first chunk");
        });

        let response = github_client()
            .expect("client")
            .get(format!("http://{address}/chunked"))
            .send()
            .await
            .expect("chunked response");
        assert_eq!(response.content_length(), None);
        let budget = ResourceBudget {
            file_bytes: 5,
            ..ResourceBudget::default_skill()
        };

        let error =
            read_raw_response_with_budget(response, budget, RawBytesBudget::Metadata, "SKILL.md")
                .await
                .expect_err("cap+1 chunk should fail before EOF");

        assert!(matches!(error, GithubImportError::Budget(_)));
        server.join().expect("server join");
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

    #[tokio::test]
    async fn github_transport_error_summary_omits_request_url() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind closed port");
        let address = listener.local_addr().expect("closed port address");
        let server = std::thread::spawn(move || {
            let (stream, _) = listener.accept().expect("accept request");
            drop(stream);
        });
        let secret = "transport-secret";
        let request_url = format!("http://{address}/private?token={secret}");
        let error = reqwest::Client::builder()
            .no_proxy()
            .build()
            .expect("client")
            .get(&request_url)
            .send()
            .await
            .expect_err("closed port should fail");

        let summary = sanitized_github_transport_error(&error);
        assert!(!summary.contains(secret));
        assert!(!summary.contains(&request_url));
        assert!(!summary.contains(&address.to_string()));
        server.join().expect("server join");
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
    fn archive_redirect_validator_accepts_only_exact_codeload_location() {
        let repo = GitHubRepoRef {
            owner: "openai".to_string(),
            repo: "skills".to_string(),
            branch: "main".to_string(),
            normalized_url: "https://github.com/openai/skills".to_string(),
        };
        let expected = "https://codeload.github.com/openai/skills/legacy.tar.gz/refs/heads/main";

        assert_eq!(
            validate_archive_redirect_url(expected, &repo)
                .expect("valid codeload redirect")
                .as_str(),
            expected
        );
        assert!(validate_archive_redirect_url(
            "https://codeload.github.com:443/openai/skills/legacy.tar.gz/refs/heads/main",
            &repo,
        )
        .is_ok());
        assert!(validate_archive_redirect_url(
            "https://codeload.github.com/OpenAI/SKILLS/legacy.tar.gz/refs/heads/main",
            &repo,
        )
        .is_ok());

        let commit_sha = "A1234567890ABCDEF1234567890ABCDEF1234567";
        let pinned_repo = GitHubRepoRef {
            branch: commit_sha.to_string(),
            ..repo.clone()
        };
        assert!(validate_archive_redirect_url(
            &format!("https://codeload.github.com/openai/skills/legacy.tar.gz/{commit_sha}"),
            &pinned_repo,
        )
        .is_ok());
        for rejected in [
            format!(
                "https://codeload.github.com/openai/skills/legacy.tar.gz/refs/heads/{commit_sha}"
            ),
            "https://codeload.github.com/openai/skills/legacy.tar.gz/a1234567890abcdef1234567890abcdef1234567"
                .to_string(),
        ] {
            assert!(matches!(
                validate_archive_redirect_url(&rejected, &pinned_repo),
                Err(GithubImportError::ArchiveRedirectRejected)
            ));
        }

        for rejected in [
            "http://codeload.github.com/openai/skills/legacy.tar.gz/refs/heads/main",
            "https://user@codeload.github.com/openai/skills/legacy.tar.gz/refs/heads/main",
            "https://codeload.github.com:444/openai/skills/legacy.tar.gz/refs/heads/main",
            "https://codeload.github.com.evil.example/openai/skills/legacy.tar.gz/refs/heads/main",
            "https://127.0.0.1/openai/skills/legacy.tar.gz/refs/heads/main",
            "https://169.254.169.254/openai/skills/legacy.tar.gz/refs/heads/main",
            "https://codeload.github.com/other/skills/legacy.tar.gz/refs/heads/main",
            "https://codeload.github.com/openai/other/legacy.tar.gz/refs/heads/main",
            "https://codeload.github.com/openai/skills/legacy.tar.gz/refs/heads/dev",
            "https://codeload.github.com/openai/skills/legacy.tar.gz/refs/heads/main/extra",
            "https://codeload.github.com/openai/other/../skills/legacy.tar.gz/refs/heads/main",
            "https://codeload.github.com/openai/other/%2e%2e/skills/legacy.tar.gz/refs/heads/main",
            r"https://codeload.github.com/openai/other\..\skills/legacy.tar.gz/refs/heads/main",
            "https://codeload.github.com/openai/skills/legacy.tar.gz/refs/heads/main?token=secret",
            "https://codeload.github.com/openai/skills/legacy.tar.gz/refs/heads/main#fragment",
            "https://codeload.github.com/openai%2fother/skills/legacy.tar.gz/refs/heads/main",
            "https://codeload.github.com/openai%5cother/skills/legacy.tar.gz/refs/heads/main",
            "https://@codeload.github.com/openai/skills/legacy.tar.gz/refs/heads/main",
            "https://codeload.github.com/openai/skills/legacy.tar.gz/main",
            "/openai/skills/legacy.tar.gz/refs/heads/main",
            "not a url",
        ] {
            assert!(
                matches!(
                    validate_archive_redirect_url(rejected, &repo),
                    Err(GithubImportError::ArchiveRedirectRejected)
                ),
                "unexpectedly accepted {rejected}",
            );
        }
    }

    #[test]
    fn archive_numeric_redirect_validator_accepts_only_exact_same_ref_api_location() {
        let repo = GitHubRepoRef {
            owner: "legacy-owner".to_string(),
            repo: "legacy-repo".to_string(),
            branch: "main".to_string(),
            normalized_url: "https://github.com/legacy-owner/legacy-repo".to_string(),
        };
        let expected = "https://api.github.com/repositories/123456789/tarball/main";

        assert_eq!(
            validate_archive_api_redirect_url(expected, &repo)
                .expect("valid numeric API redirect")
                .as_str(),
            expected
        );
        assert!(validate_archive_api_redirect_url(
            "https://api.github.com:443/repositories/123456789/tarball/main",
            &repo,
        )
        .is_ok());

        for rejected in [
            "https://api.github.com/repositories/not-a-number/tarball/main",
            "https://api.github.com/repositories/0/tarball/main",
            "https://api.github.com/repositories/18446744073709551616/tarball/main",
            "https://api.github.com/repositories/+1/tarball/main",
            "https://api.github.com/repositories/1/tarball/dev",
            "https://api.github.com/repositories/1/tarball/main/extra",
            "https://api.github.com/repositories/1/../1/tarball/main",
            "https://api.github.com/repositories/1/%2e%2e/1/tarball/main",
            r"https://api.github.com/repositories\999\..\1\tarball\main",
            "https://api.github.com/repositories%2f1/tarball/main",
            "https://api.github.com/repositories%5c1/tarball/main",
            "https://user@api.github.com/repositories/1/tarball/main",
            "https://@api.github.com/repositories/1/tarball/main",
            "https://api.github.com/repositories/1/tarball/main?token=secret",
            "https://api.github.com/repositories/1/tarball/main#fragment",
            "http://api.github.com/repositories/1/tarball/main",
            "https://api.github.com:444/repositories/1/tarball/main",
            "https://api.github.com.evil.example/repositories/1/tarball/main",
            "/repositories/1/tarball/main",
        ] {
            assert!(
                matches!(
                    validate_archive_api_redirect_url(rejected, &repo),
                    Err(GithubImportError::ArchiveRedirectRejected)
                ),
                "unexpectedly accepted {rejected}",
            );
        }
    }

    #[test]
    fn archive_redirect_headers_require_exactly_one_location() {
        use reqwest::header::{HeaderMap, HeaderValue, LOCATION};

        let repo = GitHubRepoRef {
            owner: "openai".to_string(),
            repo: "skills".to_string(),
            branch: "main".to_string(),
            normalized_url: "https://github.com/openai/skills".to_string(),
        };
        let location = HeaderValue::from_static(
            "https://codeload.github.com/openai/skills/legacy.tar.gz/refs/heads/main",
        );

        assert!(matches!(
            validate_archive_redirect_headers(&HeaderMap::new(), &repo),
            Err(GithubImportError::ArchiveRedirectRejected)
        ));

        let mut duplicate = HeaderMap::new();
        duplicate.append(LOCATION, location.clone());
        duplicate.append(LOCATION, location);
        assert!(matches!(
            validate_archive_redirect_headers(&duplicate, &repo),
            Err(GithubImportError::ArchiveRedirectRejected)
        ));
    }

    #[tokio::test]
    async fn archive_redirect_request_rejects_invalid_structured_repo_before_transport() {
        let repo = GitHubRepoRef {
            owner: "openai".to_string(),
            repo: "skills".to_string(),
            branch: "feature/unsafe".to_string(),
            normalized_url: "https://github.com/openai/skills".to_string(),
        };
        let endpoints = test_mirror_endpoints(
            "http://127.0.0.1:9".to_string(),
            "http://127.0.0.1:10".to_string(),
        );

        let error = download_repository_archive_with_test_endpoints(
            &github_client().expect("client"),
            &repo,
            None,
            ResourceBudget::default_skill(),
            &endpoints,
            "http://127.0.0.1:11",
        )
        .await
        .expect_err("invalid structured ref must fail before transport");

        assert!(matches!(
            error,
            GithubImportError::InvalidRepoComponent {
                field: "branch",
                ..
            }
        ));
    }

    #[tokio::test]
    async fn archive_redirect_follows_one_hop_without_forwarding_bearer() {
        use std::io::{Read, Write};
        use std::net::TcpListener;
        use std::sync::{Arc, Mutex};

        let commit_sha = "a1234567890abcdef1234567890abcdef1234567";
        for (branch, redirect_path) in [
            (
                "main",
                "/openai/skills/legacy.tar.gz/refs/heads/main".to_string(),
            ),
            (
                commit_sha,
                format!("/openai/skills/legacy.tar.gz/{commit_sha}"),
            ),
        ] {
            let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
            let address = listener.local_addr().expect("addr");
            let archive = repository_archive(&[(
                "skills/demo/SKILL.md",
                b"---\nname: demo\ndescription: demo\n---\n",
            )]);
            let requests = Arc::new(Mutex::new(Vec::<String>::new()));
            let requests_clone = Arc::clone(&requests);

            let server = std::thread::spawn(move || {
                for request_index in 0..2 {
                    let (mut stream, _) = listener.accept().expect("accept");
                    let mut buffer = [0_u8; 4096];
                    let bytes_read = stream.read(&mut buffer).expect("read");
                    requests_clone
                        .lock()
                        .expect("lock")
                        .push(String::from_utf8_lossy(&buffer[..bytes_read]).to_string());

                    if request_index == 0 {
                        let location = format!("http://{address}{redirect_path}");
                        let response = format!(
                            "HTTP/1.1 302 Found\r\nLocation: {location}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                        );
                        stream
                            .write_all(response.as_bytes())
                            .expect("write redirect");
                    } else {
                        let headers = format!(
                            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                            archive.len()
                        );
                        stream.write_all(headers.as_bytes()).expect("write headers");
                        stream.write_all(&archive).expect("write archive");
                    }
                }
            });

            let repo = GitHubRepoRef {
                owner: "openai".to_string(),
                repo: "skills".to_string(),
                branch: branch.to_string(),
                normalized_url: "https://github.com/openai/skills".to_string(),
            };
            let base_url = format!("http://{address}");
            let endpoints = test_mirror_endpoints(base_url.clone(), format!("{base_url}/mirror"));
            let client = github_client().expect("client");
            let bytes = download_repository_archive_with_test_endpoints(
                &client,
                &repo,
                Some("direct-token"),
                ResourceBudget::default_skill(),
                &endpoints,
                &base_url,
            )
            .await
            .expect("download redirected archive");
            let snapshot = snapshot_from_repository_archive(&bytes).expect("snapshot");
            assert!(snapshot.files.contains_key("skills/demo/SKILL.md"));

            server.join().expect("server join");
            let captured = requests.lock().expect("captured");
            assert_eq!(captured.len(), 2);
            assert!(captured[0]
                .to_ascii_lowercase()
                .contains("authorization: bearer direct-token"));
            assert!(!captured[1].to_ascii_lowercase().contains("authorization:"));
        }
    }

    #[tokio::test]
    async fn archive_redirect_follows_trusted_numeric_canonicalization_with_scoped_bearer() {
        let archive = repository_archive(&[(
            "skills/demo/SKILL.md",
            b"---\nname: demo\ndescription: demo\n---\n",
        )]);
        let (base_url, requests, server) = spawn_http_sequence(move |address| {
            let base_url = format!("http://{address}");
            vec![
                http_response(
                    "301 Moved Permanently",
                    &[format!("{base_url}/repositories/123/tarball/main")],
                    &[],
                ),
                http_response(
                    "302 Found",
                    &[format!(
                        "{base_url}/CanonicalOwner/renamed-repo/legacy.tar.gz/refs/heads/main"
                    )],
                    &[],
                ),
                http_response("200 OK", &[], &archive),
            ]
        });
        let repo = GitHubRepoRef {
            owner: "legacy-owner".to_string(),
            repo: "legacy-repo".to_string(),
            branch: "main".to_string(),
            normalized_url: "https://github.com/legacy-owner/legacy-repo".to_string(),
        };
        let endpoints = test_mirror_endpoints(base_url.clone(), format!("{base_url}/mirror"));

        let bytes = download_repository_archive_with_test_endpoints(
            &github_client().expect("client"),
            &repo,
            Some("direct-token"),
            ResourceBudget::default_skill(),
            &endpoints,
            &base_url,
        )
        .await
        .expect("download renamed repository archive");
        let snapshot = snapshot_from_repository_archive(&bytes).expect("snapshot");
        assert!(snapshot.files.contains_key("skills/demo/SKILL.md"));

        server.join().expect("server join");
        let captured = requests.lock().expect("captured");
        assert_eq!(captured.len(), 3);
        assert!(captured[0].starts_with("GET /repos/legacy-owner/legacy-repo/tarball/main "));
        assert!(captured[1].starts_with("GET /repositories/123/tarball/main "));
        assert!(captured[2]
            .starts_with("GET /CanonicalOwner/renamed-repo/legacy.tar.gz/refs/heads/main "));
        assert!(captured[0]
            .to_ascii_lowercase()
            .contains("authorization: bearer direct-token"));
        assert!(captured[1]
            .to_ascii_lowercase()
            .contains("authorization: bearer direct-token"));
        assert!(!captured[2].to_ascii_lowercase().contains("authorization:"));
    }

    #[tokio::test]
    async fn archive_redirect_rejects_numeric_canonicalization_from_a_mirror() {
        let (direct_url, _, direct_server) =
            spawn_http_sequence(|_| vec![http_response("502 Bad Gateway", &[], &[])]);
        let (mirror_url, mirror_requests, mirror_server) = spawn_http_sequence(|address| {
            vec![http_response(
                "301 Moved Permanently",
                &[format!("http://{address}/repositories/123/tarball/main")],
                &[],
            )]
        });
        let repo = GitHubRepoRef {
            owner: "legacy-owner".to_string(),
            repo: "legacy-repo".to_string(),
            branch: "main".to_string(),
            normalized_url: "https://github.com/legacy-owner/legacy-repo".to_string(),
        };
        let endpoints = test_mirror_endpoints(direct_url, mirror_url.clone());

        let error = download_repository_archive_with_test_endpoints(
            &github_client().expect("client"),
            &repo,
            Some("direct-token"),
            ResourceBudget::default_skill(),
            &endpoints,
            &mirror_url,
        )
        .await
        .expect_err("mirror 301 must not authorize canonicalization");

        direct_server.join().expect("direct server join");
        mirror_server.join().expect("mirror server join");
        assert_eq!(mirror_requests.lock().expect("captured").len(), 1);
        assert!(matches!(error, GithubImportError::ArchiveRedirectRejected));
    }

    #[tokio::test]
    async fn archive_redirect_rejects_non_302_numeric_hop() {
        let (base_url, requests, server) = spawn_http_sequence(|address| {
            vec![
                http_response(
                    "301 Moved Permanently",
                    &[format!("http://{address}/repositories/123/tarball/main")],
                    &[],
                ),
                http_response("200 OK", &[], &[]),
            ]
        });
        let repo = GitHubRepoRef {
            owner: "legacy-owner".to_string(),
            repo: "legacy-repo".to_string(),
            branch: "main".to_string(),
            normalized_url: "https://github.com/legacy-owner/legacy-repo".to_string(),
        };
        let endpoints = test_mirror_endpoints(base_url.clone(), format!("{base_url}/mirror"));

        let error = download_repository_archive_with_test_endpoints(
            &github_client().expect("client"),
            &repo,
            None,
            ResourceBudget::default_skill(),
            &endpoints,
            &base_url,
        )
        .await
        .expect_err("numeric API hop must return 302");

        server.join().expect("server join");
        assert_eq!(requests.lock().expect("captured").len(), 2);
        assert!(matches!(error, GithubImportError::ArchiveRedirectRejected));
    }

    #[tokio::test]
    async fn archive_redirect_rejects_a_redirect_after_numeric_codeload() {
        let (base_url, requests, server) = spawn_http_sequence(|address| {
            let base_url = format!("http://{address}");
            vec![
                http_response(
                    "301 Moved Permanently",
                    &[format!("{base_url}/repositories/123/tarball/main")],
                    &[],
                ),
                http_response(
                    "302 Found",
                    &[format!(
                        "{base_url}/CanonicalOwner/renamed-repo/legacy.tar.gz/refs/heads/main"
                    )],
                    &[],
                ),
                http_response("302 Found", &[format!("{base_url}/another-redirect")], &[]),
            ]
        });
        let repo = GitHubRepoRef {
            owner: "legacy-owner".to_string(),
            repo: "legacy-repo".to_string(),
            branch: "main".to_string(),
            normalized_url: "https://github.com/legacy-owner/legacy-repo".to_string(),
        };
        let endpoints = test_mirror_endpoints(base_url.clone(), format!("{base_url}/mirror"));

        let error = download_repository_archive_with_test_endpoints(
            &github_client().expect("client"),
            &repo,
            None,
            ResourceBudget::default_skill(),
            &endpoints,
            &base_url,
        )
        .await
        .expect_err("codeload redirect must terminate the chain");

        server.join().expect("server join");
        assert_eq!(requests.lock().expect("captured").len(), 3);
        assert!(matches!(error, GithubImportError::ArchiveRedirectRejected));
    }

    #[tokio::test]
    async fn archive_redirect_rejects_a_second_redirect() {
        use std::io::{Read, Write};
        use std::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let address = listener.local_addr().expect("addr");
        let server = std::thread::spawn(move || {
            for _ in 0..2 {
                let (mut stream, _) = listener.accept().expect("accept");
                let mut buffer = [0_u8; 4096];
                let _ = stream.read(&mut buffer).expect("read");
                let location =
                    format!("http://{address}/openai/skills/legacy.tar.gz/refs/heads/main");
                let response = format!(
                    "HTTP/1.1 302 Found\r\nLocation: {location}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                );
                stream
                    .write_all(response.as_bytes())
                    .expect("write redirect");
            }
        });

        let repo = GitHubRepoRef {
            owner: "openai".to_string(),
            repo: "skills".to_string(),
            branch: "main".to_string(),
            normalized_url: "https://github.com/openai/skills".to_string(),
        };
        let base_url = format!("http://{address}");
        let endpoints = test_mirror_endpoints(base_url.clone(), format!("{base_url}/mirror"));
        let client = github_client().expect("client");
        let error = download_repository_archive_with_test_endpoints(
            &client,
            &repo,
            None,
            ResourceBudget::default_skill(),
            &endpoints,
            &base_url,
        )
        .await
        .expect_err("second redirect must fail closed");

        server.join().expect("server join");
        assert!(matches!(error, GithubImportError::ArchiveRedirectRejected));
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
                uid: "twitterapi-io-uid".to_string(),
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
            preview_id: "github-preview-test".to_string(),
            resolved_commit_sha: "0".repeat(40),
            snapshot_digest: "sha256-v1:test".to_string(),
            expires_at: Utc::now().to_rfc3339(),
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
                uid: "web-access-uid".to_string(),
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
                uid: format!("{}-uid", agent_planner.skill_id),
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
                uid: format!("{}-uid", commit.skill_id),
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
                uid: format!("{}-uid", code_review.skill_id),
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
                uid: format!("{}-uid", candidate.skill_id),
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
            preview_id: "github-preview-test".to_string(),
            resolved_commit_sha: "0".repeat(40),
            snapshot_digest: "sha256-v1:test".to_string(),
            expires_at: Utc::now().to_rfc3339(),
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
            preview_id: "github-preview-test".to_string(),
            resolved_commit_sha: "0".repeat(40),
            snapshot_digest: "sha256-v1:test".to_string(),
            expires_at: Utc::now().to_rfc3339(),
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
    fn repository_level_singular_skill_directory_uses_repository_identity() {
        let repo = GitHubRepoRef {
            owner: "yetone".to_string(),
            repo: "kill-ai-slop".to_string(),
            branch: "main".to_string(),
            normalized_url: "https://github.com/yetone/kill-ai-slop".to_string(),
        };
        let snapshot = repository_level_singular_skill_snapshot();

        let candidates = build_repo_skill_candidates_from_snapshot(&repo, &snapshot)
            .expect("repository candidates");
        let subpath_candidates =
            build_repo_skill_candidates_from_snapshot_at_path(&repo, &snapshot, Some("skill"))
                .expect("subpath candidates");

        assert_eq!(candidates, subpath_candidates);
        assert_eq!(candidates.len(), 1);
        let candidate = &candidates[0];
        assert_eq!(candidate.source_path, "skill");
        assert_eq!(candidate.skill_id, "kill-ai-slop");
        assert_eq!(candidate.skill_name, "kill-ai-slop");
        assert_eq!(
            candidate.description.as_deref(),
            Some("Find and remove AI slop")
        );
        assert_eq!(candidate.root_directory, "/");
        assert_eq!(candidate.skill_directory_name, "skill");
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
    fn snapshot_binding_requires_matching_target_repo_and_source_path() {
        let snapshot = remote_test_snapshot(Some("content/skills"));
        let ssh_demo = ssh_test_target("ssh-demo");
        let ssh_other = ssh_test_target("ssh-other");

        assert!(validate_snapshot_binding(
            &snapshot,
            &ssh_demo,
            "https://github.com/openai/skills/tree/main/content/skills",
        )
        .is_ok());
        assert!(matches!(
            validate_snapshot_binding(
                &snapshot,
                &ssh_other,
                "https://github.com/openai/skills/tree/main/content/skills",
            ),
            Err(GithubImportError::PreviewTargetChanged)
        ));
        assert!(matches!(
            validate_snapshot_binding(
                &snapshot,
                &ActiveTarget::Local,
                "https://github.com/openai/skills/tree/main/content/skills",
            ),
            Err(GithubImportError::PreviewTargetChanged)
        ));
        assert!(matches!(
            validate_snapshot_binding(
                &snapshot,
                &ssh_demo,
                "https://github.com/openai/skills/tree/main/other",
            ),
            Err(GithubImportError::PreviewWorkspaceMismatch)
        ));
        assert!(matches!(
            validate_snapshot_binding(
                &snapshot,
                &ssh_demo,
                "https://github.com/openai/other/tree/main/content/skills",
            ),
            Err(GithubImportError::PreviewWorkspaceMismatch)
        ));
        assert!(matches!(
            validate_snapshot_binding(
                &snapshot,
                &ssh_demo,
                "https://github.com/openai/skills/tree/dev/content/skills",
            ),
            Err(GithubImportError::PreviewWorkspaceMismatch)
        ));
    }

    #[test]
    fn snapshot_binding_accepts_a_root_source_without_branch_segment() {
        let snapshot = remote_test_snapshot(None);
        let ssh_demo = ssh_test_target("ssh-demo");

        assert!(validate_snapshot_binding(
            &snapshot,
            &ssh_demo,
            "https://github.com/openai/skills"
        )
        .is_ok());
        assert!(matches!(
            validate_snapshot_binding(
                &snapshot,
                &ssh_demo,
                "https://github.com/openai/skills/tree/main/content/skills",
            ),
            Err(GithubImportError::PreviewWorkspaceMismatch)
        ));
    }

    #[test]
    fn snapshot_binding_reconciles_an_explicit_branch_with_the_preview() {
        let snapshot = remote_test_snapshot(Some("content/skills"));
        let ssh_demo = ssh_test_target("ssh-demo");
        let repo_url = "https://github.com/openai/skills/content/skills";

        assert!(validate_snapshot_binding_with_branch(
            &snapshot,
            &ssh_demo,
            repo_url,
            Some("main"),
        )
        .is_ok());
        assert!(matches!(
            validate_snapshot_binding_with_branch(&snapshot, &ssh_demo, repo_url, Some("dev"),),
            Err(GithubImportError::PreviewWorkspaceMismatch)
        ));
        assert!(matches!(
            validate_snapshot_binding_with_branch(
                &snapshot,
                &ssh_demo,
                "https://github.com/openai/skills/tree/main/content/skills",
                Some("dev"),
            ),
            Err(GithubImportError::BranchSelectionConflict)
        ));
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

    #[test]
    fn root_import_copy_includes_all_descendant_files() {
        let files = collect_snapshot_source_files(&root_package_snapshot(), ".").expect("files");
        let relative_paths = files
            .iter()
            .map(|file| file.relative_path.as_str())
            .collect::<Vec<_>>();

        assert_eq!(
            relative_paths,
            vec![
                "README.md",
                "SKILL.md",
                "assets/example.txt",
                "references/guide.md",
                "scripts/run.py",
            ]
        );
    }

    #[tokio::test]
    async fn root_repository_import_writes_complete_package_and_preserves_assignment() {
        let pool = setup_test_db().await;
        let central_root = tempdir().expect("central");
        let repo = GitHubRepoRef {
            owner: "alchaincyf".to_string(),
            repo: "huashu-design".to_string(),
            branch: "master".to_string(),
            normalized_url: "https://github.com/alchaincyf/huashu-design".to_string(),
        };
        let snapshot = root_package_snapshot();
        let candidates =
            build_repo_skill_candidates_from_snapshot(&repo, &snapshot).expect("candidates");

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].source_path, ".");

        let result = import_github_repo_skills_from_snapshot(
            &pool,
            &repo,
            &snapshot,
            &candidates,
            vec![GitHubSkillImportSelection {
                source_path: ".".to_string(),
                resolution: DuplicateResolution::Overwrite,
                renamed_skill_id: None,
            }],
            central_root.path(),
            None,
            None,
        )
        .await
        .expect("root import");

        assert_eq!(result.imported_skills.len(), 1);
        let target = central_root.path().join("huashu-design");
        assert!(target.join("SKILL.md").exists());
        assert_eq!(
            std::fs::read_to_string(target.join("references/guide.md")).unwrap(),
            "# Guide\n"
        );
        assert_eq!(
            std::fs::read_to_string(target.join("scripts/run.py")).unwrap(),
            "print('ok')\n"
        );
        assert!(target.join("assets/example.txt").exists());

        let assignment = db::get_skill_repository_assignment(&pool, "huashu-design")
            .await
            .expect("assignment");
        assert_eq!(assignment.source_path.as_deref(), Some("."));
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
        let endpoints = test_mirror_endpoints(direct_url.clone(), mirror_url.clone());

        let response = send_github_request_with_test_endpoints(
            &client,
            GitHubFetchSurface::Api,
            &endpoints,
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
        let endpoints = test_mirror_endpoints(direct_url.clone(), mirror_url.clone());

        let response = send_github_request_with_test_endpoints(
            &client,
            GitHubFetchSurface::Raw,
            &endpoints,
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
        let endpoints = test_mirror_endpoints(direct_url.clone(), mirror_url.clone());

        let response = send_github_request_with_test_endpoints(
            &client,
            GitHubFetchSurface::Api,
            &endpoints,
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

    // ── Tree manifest fast-path parser (Commit 1 of task
    // `07-18-github-import-manifest-fast-path`). Parity/fallback tests that
    // require mock HTTP (F2-F8) land in Commit 2 with the dispatcher; these
    // cover the pure parser surface so the acquisition-layer primitive is
    // fully specified before it is wired into preview/import.
    mod tree_manifest_parser {
        use super::super::tree_manifest::{
            classify_tree_entry, parse_tree_response, read_tree_response_with_budget,
            RepositoryFileKind,
        };
        use super::*;
        use crate::services::resource_budget::{ResourceBudget, DEFAULT_TREE_ENTRIES};

        fn sample_repo() -> GitHubRepoRef {
            GitHubRepoRef {
                owner: "owner".to_string(),
                repo: "repo".to_string(),
                branch: "main".to_string(),
                normalized_url: "https://github.com/owner/repo".to_string(),
            }
        }

        fn tree_entry(path: &str, mode: &str, type_field: &str, size: Option<u64>) -> String {
            let size_json = match size {
                Some(size) => format!(",\"size\":{size}"),
                None => String::new(),
            };
            format!(
                "{{\"path\":\"{path}\",\"mode\":\"{mode}\",\"type\":\"{type_field}\"{size_json}}}"
            )
        }

        fn tree_response(entries: &[String], truncated: bool) -> String {
            let joined = entries.join(",");
            format!(
                "{{\"sha\":\"abc\",\"url\":\"https://api.github.com/\",\"tree\":[{joined}],\"truncated\":{truncated}}}",
                truncated = truncated
            )
        }

        #[tokio::test]
        async fn chunked_tree_body_stops_at_limit_before_eof() {
            use std::io::{Read, Write};
            use std::net::TcpListener;
            use std::sync::mpsc;

            let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
            let address = listener.local_addr().expect("address");
            let (release_tx, release_rx) = mpsc::channel();
            let server = std::thread::spawn(move || {
                let (mut stream, _) = listener.accept().expect("accept");
                let mut request = [0_u8; 1024];
                let _ = stream.read(&mut request).expect("read request");
                stream
                    .write_all(
                        b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n11\r\n{\"tree\":[]}xxxxxx\r\n",
                    )
                    .expect("write over-limit chunk");
                release_rx.recv().expect("reader returned before EOF");
            });

            let response = github_client()
                .expect("client")
                .get(format!("http://{address}/tree"))
                .send()
                .await
                .expect("chunked response");
            let budget = ResourceBudget {
                tree_response_bytes: 16,
                ..ResourceBudget::default_skill()
            };

            let error = read_tree_response_with_budget(response, &sample_repo(), budget)
                .await
                .expect_err("cap+1 chunk should fail before EOF");
            assert!(matches!(error, GithubImportError::Budget(_)));
            release_tx.send(()).unwrap();
            server.join().expect("server join");
        }

        #[test]
        fn classify_regular_blob_modes_are_regular() {
            assert_eq!(
                classify_tree_entry("100644", "blob"),
                Some(RepositoryFileKind::RegularBlob)
            );
            assert_eq!(
                classify_tree_entry("100755", "blob"),
                Some(RepositoryFileKind::RegularBlob)
            );
        }

        #[test]
        fn classify_symlink_and_gitlink_are_skipped_kinds() {
            assert_eq!(
                classify_tree_entry("120000", "blob"),
                Some(RepositoryFileKind::SymlinkBlob)
            );
            assert_eq!(
                classify_tree_entry("160000", "commit"),
                Some(RepositoryFileKind::Gitlink)
            );
        }

        #[test]
        fn classify_tree_node_returns_none_like_archive_is_file_filter() {
            assert_eq!(classify_tree_entry("040000", "tree"), None);
        }

        #[test]
        fn classify_unknown_mode_falls_back_to_other() {
            assert_eq!(
                classify_tree_entry("100000", "blob"),
                Some(RepositoryFileKind::Other)
            );
            assert_eq!(
                classify_tree_entry("040000", "blob"),
                Some(RepositoryFileKind::Other)
            );
            assert_eq!(
                classify_tree_entry("100644", "commit"),
                Some(RepositoryFileKind::Other)
            );
        }

        #[test]
        fn parse_regular_blob_entries_builds_manifest() {
            let body = tree_response(
                &[
                    tree_entry("SKILL.md", "100644", "blob", Some(40)),
                    tree_entry("README.md", "100644", "blob", Some(12)),
                    tree_entry("references/guide.md", "100755", "blob", Some(8)),
                    // Directory node — skipped, like archive's is_file() filter.
                    tree_entry("references", "040000", "tree", None),
                ],
                false,
            );
            let manifest =
                parse_tree_response(&body, &sample_repo(), ResourceBudget::default_skill())
                    .expect("regular blobs parse");

            assert_eq!(
                manifest
                    .regular_files
                    .iter()
                    .map(|f| f.repo_path.as_str())
                    .collect::<Vec<_>>(),
                vec!["README.md", "SKILL.md", "references/guide.md"]
            );
            assert_eq!(manifest.regular_files[0].byte_len, 12);
            assert_eq!(manifest.regular_files[1].byte_len, 40);
            assert_eq!(manifest.regular_files[2].byte_len, 8);
            assert_eq!(manifest.regular_total_bytes(), 60);
            assert_eq!(
                manifest.regular_files.len(),
                manifest.regular_paths().count()
            );
        }

        #[test]
        fn parse_symlink_and_gitlink_are_recorded_in_skipped_not_regular() {
            let body = tree_response(
                &[
                    tree_entry("SKILL.md", "100644", "blob", Some(40)),
                    // symlink blob — must NOT become a candidate or raw download.
                    tree_entry("link", "120000", "blob", Some(11)),
                    // submodule gitlink — must NOT become a candidate.
                    tree_entry("vendor/submod", "160000", "commit", None),
                ],
                false,
            );
            let manifest =
                parse_tree_response(&body, &sample_repo(), ResourceBudget::default_skill())
                    .expect("symlink/gitlink parse");

            let regular: Vec<_> = manifest
                .regular_files
                .iter()
                .map(|f| f.repo_path.as_str())
                .collect();
            assert_eq!(regular, vec!["SKILL.md"]);
            let skipped_kinds: Vec<_> = manifest
                .skipped
                .iter()
                .map(|f| (f.repo_path.as_str(), f.kind))
                .collect();
            assert_eq!(
                skipped_kinds,
                vec![
                    ("link", RepositoryFileKind::SymlinkBlob),
                    ("vendor/submod", RepositoryFileKind::Gitlink),
                ]
            );
        }

        #[test]
        fn parse_truncated_tree_returns_typed_fallback_error() {
            let body = tree_response(&[tree_entry("SKILL.md", "100644", "blob", Some(40))], true);
            let error = parse_tree_response(&body, &sample_repo(), ResourceBudget::default_skill())
                .expect_err("truncated tree should error");
            assert!(matches!(error, GithubImportError::TreeManifestTruncated));
        }

        #[test]
        fn parse_regular_blob_without_size_returns_typed_integrity_error() {
            let body = tree_response(&[tree_entry("SKILL.md", "100644", "blob", None)], false);
            let error = parse_tree_response(&body, &sample_repo(), ResourceBudget::default_skill())
                .expect_err("missing size should error");
            assert!(
                matches!(error, GithubImportError::TreeManifestEntryMissingSize(ref path) if path == "SKILL.md")
            );
        }

        #[test]
        fn parse_unknown_mode_returns_typed_unsupported_fallback_error() {
            let body = tree_response(
                &[
                    tree_entry("SKILL.md", "100644", "blob", Some(40)),
                    tree_entry("weird.txt", "100000", "blob", Some(7)),
                ],
                false,
            );
            let error = parse_tree_response(&body, &sample_repo(), ResourceBudget::default_skill())
                .expect_err("unknown mode should error");
            match error {
                GithubImportError::TreeManifestUnsupportedMode { path, mode } => {
                    assert_eq!(path, "weird.txt");
                    assert_eq!(mode, "100000");
                }
                other => panic!("expected TreeManifestUnsupportedMode, got {other:?}"),
            }
        }

        #[test]
        fn parse_over_entry_budget_returns_typed_budget_error() {
            let mut entries = Vec::new();
            for index in 0..(DEFAULT_TREE_ENTRIES + 1) {
                let path = format!("file{index}.md");
                entries.push(tree_entry(&path, "100644", "blob", Some(1)));
            }
            let body = tree_response(&entries, false);
            let error = parse_tree_response(&body, &sample_repo(), ResourceBudget::default_skill())
                .expect_err("entry budget should error");
            assert!(
                matches!(error, GithubImportError::TreeManifestEntryBudgetExceeded(limit) if limit == DEFAULT_TREE_ENTRIES)
            );
        }

        #[test]
        fn parse_over_expanded_byte_budget_returns_budget_error() {
            // Use a budget with tiny expanded-byte limit so summed sizes overflow.
            let budget = ResourceBudget {
                archive_expanded_bytes: 10,
                ..ResourceBudget::default_skill()
            };
            let body = tree_response(&[tree_entry("SKILL.md", "100644", "blob", Some(40))], false);
            let error = parse_tree_response(&body, &sample_repo(), budget)
                .expect_err("byte budget should error");
            assert!(matches!(error, GithubImportError::Budget(_)));
        }

        #[test]
        fn parse_over_response_body_budget_returns_budget_error() {
            let budget = ResourceBudget {
                tree_response_bytes: 16,
                ..ResourceBudget::default_skill()
            };
            let body = tree_response(&[tree_entry("SKILL.md", "100644", "blob", Some(40))], false);
            let error = parse_tree_response(&body, &sample_repo(), budget)
                .expect_err("response budget should error");
            assert!(matches!(error, GithubImportError::Budget(_)));
        }

        #[test]
        fn parse_malformed_json_returns_parse_error() {
            let body = "{ not valid json ";
            let error = parse_tree_response(body, &sample_repo(), ResourceBudget::default_skill())
                .expect_err("malformed json should error");
            assert!(matches!(error, GithubImportError::Parse(_)));
        }

        #[test]
        fn parse_rejects_unsafe_repo_path_through_shared_path_policy() {
            // Traversal segment — must be rejected by the shared `normalize_repo_path`
            // (same path policy as the archive parser's `is_safe_repo_relative_path`).
            let body = tree_response(
                &[tree_entry("../escape.md", "100644", "blob", Some(40))],
                false,
            );
            let error = parse_tree_response(&body, &sample_repo(), ResourceBudget::default_skill())
                .expect_err("traversal path should error");
            assert!(matches!(error, GithubImportError::UnsupportedRepoPath(_)));
        }

        #[test]
        fn parse_strips_leading_trailing_slash_in_repo_path() {
            // GitHub tree paths may arrive with leading slashes for some
            // mirror responses; `normalize_repo_path` must collapse them so
            // downstream discovery sees a canonical relative path.
            let body = tree_response(
                &[tree_entry(
                    "/skills/demo/SKILL.md",
                    "100644",
                    "blob",
                    Some(40),
                )],
                false,
            );
            let manifest =
                parse_tree_response(&body, &sample_repo(), ResourceBudget::default_skill())
                    .expect("leading slash normalizes");
            assert_eq!(manifest.regular_files[0].repo_path, "skills/demo/SKILL.md");
        }

        #[test]
        fn parse_duplicate_path_returns_integrity_error() {
            let body = tree_response(
                &[
                    tree_entry("SKILL.md", "100644", "blob", Some(40)),
                    tree_entry("SKILL.md", "100644", "blob", Some(40)),
                ],
                false,
            );
            let error = parse_tree_response(&body, &sample_repo(), ResourceBudget::default_skill())
                .expect_err("duplicate path should error");
            // Duplicate is surfaced via the `MissingSize` integrity path
            // (archive parser would have silently overwritten the HashMap
            // entry; we fail closed to expose upstream anomalies).
            assert!(matches!(
                error,
                GithubImportError::TreeManifestEntryMissingSize(_)
            ));
        }
    }

    // ── Tree fast-path dispatcher integration (Commit 2 of task
    // `07-18-github-import-manifest-fast-path`). Covers the parity contract
    // (tree vs archive produce identical candidate/preview/repository-file
    // output for the same fixture) and the fallback classification matrix
    // (F5–F8, F11–F14). Full HTTP-level dispatch tests (mock tree/raw/archive
    // servers) require an injectable endpoint seam planned with the Commit 3
    // diagnostics/import work; the pure parity + classifier tests here cover
    // the behavioral acceptance criteria.
    mod tree_fast_path_dispatcher {
        use super::super::plugin_manifest::{
            effective_source_root, plugin_manifest_discovery_from_manifest_bytes,
            plugin_manifest_discovery_from_snapshot,
        };
        use super::super::preview::snapshot_preview_repository_files;
        use super::super::tree_import::{download_tree_selection, plan_tree_selection};
        use super::super::tree_manifest::{
            fallback_reason_for, AcquisitionMode, FallbackReason, RepositoryFileKind,
            RepositoryFileMeta, RepositoryManifest,
        };
        use super::*;

        fn sample_repo() -> GitHubRepoRef {
            GitHubRepoRef {
                owner: "owner".to_string(),
                repo: "repo".to_string(),
                branch: "main".to_string(),
                normalized_url: "https://github.com/owner/repo".to_string(),
            }
        }

        fn direct_endpoint() -> &'static GitHubMirrorEndpoint {
            GITHUB_MIRROR_ENDPOINTS.first().expect("github endpoint")
        }

        /// Build the `RepositoryManifest` a real `git/trees?recursive=1` call
        /// would return for the same fixture snapshot: every regular archive
        /// file becomes a `RegularBlob` with its byte length, mirroring the
        /// archive parser's `is_file()` filter.
        fn tree_manifest_from_snapshot(
            repo: &GitHubRepoRef,
            snapshot: &GitHubRepoSnapshot,
        ) -> RepositoryManifest {
            let regular_files = snapshot
                .files
                .iter()
                .map(|(path, bytes)| RepositoryFileMeta {
                    repo_path: path.clone(),
                    byte_len: bytes.len() as u64,
                    kind: RepositoryFileKind::RegularBlob,
                })
                .collect::<Vec<_>>();
            RepositoryManifest {
                repo: repo.clone(),
                regular_files,
                skipped: Vec::new(),
            }
        }

        /// Re-implement the archive candidate path's discovery + candidate
        /// construction so the tree path can be compared against it without
        /// touching HTTP or the DB.
        fn archive_candidates(
            repo: &GitHubRepoRef,
            snapshot: &GitHubRepoSnapshot,
            source_path: Option<&str>,
        ) -> Vec<RemoteSkillCandidate> {
            build_repo_skill_candidates_from_snapshot_at_path(repo, snapshot, source_path)
                .expect("archive candidates")
        }

        /// Re-implement the tree fast-path's candidate construction (the HTTP-free
        /// core of `try_build_preview_from_tree_manifest`) using snapshot bytes
        /// as the simulated raw fetch. This must match `archive_candidates`
        /// exactly because discovery + frontmatter + filter are shared.
        fn tree_candidates(
            repo: &GitHubRepoRef,
            snapshot: &GitHubRepoSnapshot,
            manifest: &RepositoryManifest,
            source_path: Option<&str>,
        ) -> Vec<RemoteSkillCandidate> {
            let plugin_discovery = plugin_manifest_discovery_from_snapshot(snapshot, source_path)
                .expect("plugin discovery");
            let manifests = discover_skill_manifests_from_paths_with_plugin_discovery(
                manifest.regular_paths(),
                source_path,
                &plugin_discovery,
            )
            .expect("tree discovery");
            let endpoint = direct_endpoint();
            let mut candidates = Vec::with_capacity(manifests.len());
            let mut seen_names = HashSet::new();
            for skill_manifest in manifests {
                let raw = snapshot
                    .files
                    .get(&skill_manifest.skill_md_path)
                    .expect("snapshot has skill md")
                    .clone();
                let candidate = build_remote_skill_candidate(repo, &skill_manifest, raw, endpoint)
                    .expect("tree candidate");
                if is_generic_remote_skill_candidate(&candidate) {
                    continue;
                }
                if !seen_names.insert(candidate.skill_name.clone()) {
                    continue;
                }
                candidates.push(candidate);
            }
            candidates
        }

        /// Build preview skills (without DB conflict lookup) for parity checks.
        fn preview_skills_without_conflicts(
            candidates: &[RemoteSkillCandidate],
        ) -> Vec<GitHubSkillPreview> {
            candidates
                .iter()
                .map(|candidate| GitHubSkillPreview {
                    source_path: candidate.source_path.clone(),
                    skill_id: candidate.skill_id.clone(),
                    skill_name: candidate.skill_name.clone(),
                    description: candidate.description.clone(),
                    plugin_name: candidate.plugin_name.clone(),
                    root_directory: candidate.root_directory.clone(),
                    skill_directory_name: candidate.skill_directory_name.clone(),
                    download_url: candidate.download_url.clone(),
                    conflict: None,
                    files: None,
                })
                .collect()
        }

        #[test]
        fn fallback_matrix_classifies_acquisition_errors() {
            // F7 — truncated tree.
            assert_eq!(
                fallback_reason_for(&GithubImportError::TreeManifestTruncated),
                Some(FallbackReason::Truncated)
            );
            // F12 — unsupported mode.
            assert_eq!(
                fallback_reason_for(&GithubImportError::TreeManifestUnsupportedMode {
                    path: "x".to_string(),
                    mode: "100000".to_string(),
                }),
                Some(FallbackReason::Unsupported)
            );
            // F5/F6 — rate limit + access denial (parity).
            assert_eq!(
                fallback_reason_for(&GithubImportError::RateLimited("rl".to_string())),
                Some(FallbackReason::Denied)
            );
            assert_eq!(
                fallback_reason_for(&GithubImportError::AccessDenied("denied".to_string())),
                Some(FallbackReason::Denied)
            );
            // F8 — transport / mirror failure.
            assert_eq!(
                fallback_reason_for(&GithubImportError::Http("transport".to_string())),
                Some(FallbackReason::Transport)
            );
            assert_eq!(
                fallback_reason_for(&GithubImportError::RepoNotFound),
                Some(FallbackReason::Transport)
            );
            // F13/F14 — budget (entry count, expanded bytes, response body).
            assert_eq!(
                fallback_reason_for(&GithubImportError::TreeManifestEntryBudgetExceeded(10)),
                Some(FallbackReason::Budget)
            );
            assert_eq!(
                fallback_reason_for(&GithubImportError::TreeManifestSizeOverflow),
                Some(FallbackReason::Budget)
            );
            assert!(matches!(
                fallback_reason_for(&GithubImportError::Budget(
                    crate::services::resource_budget::BudgetExceeded::new("tree", 1, 0)
                )),
                Some(FallbackReason::Budget)
            ));
            // F11 — missing size integrity gap.
            assert_eq!(
                fallback_reason_for(&GithubImportError::TreeManifestEntryMissingSize(
                    "p".to_string()
                )),
                Some(FallbackReason::Integrity)
            );
            assert_eq!(
                fallback_reason_for(&GithubImportError::Parse("bad tree json".to_string())),
                Some(FallbackReason::Integrity)
            );
        }

        #[test]
        fn fallback_matrix_does_not_swallow_domain_errors() {
            // Invalid candidate is a domain error — archive would produce the
            // same failure, so the dispatcher must NOT fall back.
            assert_eq!(
                fallback_reason_for(&GithubImportError::InvalidCandidate(
                    "bad frontmatter".to_string()
                )),
                None
            );
            assert_eq!(
                fallback_reason_for(&GithubImportError::NoImportableSkills),
                None
            );
        }

        #[test]
        fn acquisition_errors_select_archive_fallback() {
            assert_eq!(
                fallback_reason_for(&GithubImportError::TreeManifestTruncated),
                Some(FallbackReason::Truncated)
            );
            assert_eq!(
                fallback_reason_for(&GithubImportError::RateLimited("rl".to_string())),
                Some(FallbackReason::Denied)
            );
        }

        #[test]
        fn domain_errors_are_not_acquisition_fallbacks() {
            // Domain errors are not acquisition fallbacks: the archive path
            // would surface the same failure, so the dispatcher must propagate
            // them instead of switching acquisition mode.
            assert_eq!(
                fallback_reason_for(&GithubImportError::InvalidCandidate("bad".to_string())),
                None
            );
            assert_eq!(
                fallback_reason_for(&GithubImportError::NoImportableSkills),
                None
            );
        }

        /// Preview acquisition must retain real bytes, so the tree fast-path
        /// downloads every candidate subtree. Feeding the fixture bytes in as
        /// already-fetched files keeps this HTTP-free while still exercising the
        /// real planner and its integrity checks.
        fn tree_selection_snapshot(
            snapshot: &GitHubRepoSnapshot,
            manifest: &RepositoryManifest,
            candidates: &[RemoteSkillCandidate],
        ) -> Option<GitHubRepoSnapshot> {
            let selections = candidates
                .iter()
                .map(|candidate| GitHubSkillImportSelection {
                    source_path: candidate.source_path.clone(),
                    resolution: DuplicateResolution::Overwrite,
                    renamed_skill_id: None,
                })
                .collect::<Vec<_>>();
            let plan =
                plan_tree_selection(manifest, candidates, &selections).expect("selection plan");
            if plan.mode == AcquisitionMode::Archive {
                return None;
            }
            let client = reqwest::Client::new();
            Some(
                tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("runtime")
                    .block_on(download_tree_selection(
                        &client,
                        &sample_repo(),
                        &plan,
                        None,
                        snapshot.files.clone(),
                    ))
                    .expect("tree selection download"),
            )
        }

        #[test]
        fn tree_selection_repository_files_match_archive_for_candidate_subtrees() {
            for snapshot in [
                multi_skill_snapshot(),
                namespaced_skill_snapshot(),
                content_skills_snapshot(),
            ] {
                let repo = sample_repo();
                let manifest = tree_manifest_from_snapshot(&repo, &snapshot);
                let candidates = archive_candidates(&repo, &snapshot, None);
                let Some(tree_snapshot) =
                    tree_selection_snapshot(&snapshot, &manifest, &candidates)
                else {
                    continue;
                };

                let tree_files = snapshot_preview_repository_files(&tree_snapshot);
                let archive_files = snapshot_preview_repository_files(&snapshot);
                for candidate in &candidates {
                    let mut tree_skills =
                        preview_skills_without_conflicts(std::slice::from_ref(candidate));
                    let mut archive_skills =
                        preview_skills_without_conflicts(std::slice::from_ref(candidate));
                    attach_preview_file_manifests(&mut tree_skills, &tree_files)
                        .expect("tree attach");
                    attach_preview_file_manifests(&mut archive_skills, &archive_files)
                        .expect("archive attach");
                    assert_eq!(
                        tree_skills, archive_skills,
                        "tree selection must reproduce the archive file manifest"
                    );
                }
            }
        }

        #[test]
        fn root_candidates_route_preview_acquisition_to_archive() {
            for snapshot in [root_repo_snapshot(), root_package_snapshot()] {
                let repo = sample_repo();
                let manifest = tree_manifest_from_snapshot(&repo, &snapshot);
                let candidates = archive_candidates(&repo, &snapshot, None);
                assert!(
                    tree_selection_snapshot(&snapshot, &manifest, &candidates).is_none(),
                    "a root candidate must select archive acquisition"
                );
            }
        }

        #[test]
        fn tree_discovery_matches_archive_discovery_for_fixtures() {
            for snapshot in [
                root_repo_snapshot(),
                root_package_snapshot(),
                multi_skill_snapshot(),
                namespaced_skill_snapshot(),
                content_skills_snapshot(),
            ] {
                let repo = sample_repo();
                let manifest = tree_manifest_from_snapshot(&repo, &snapshot);
                let plugin_discovery = plugin_manifest_discovery_from_snapshot(&snapshot, None)
                    .expect("plugin discovery");
                let tree_manifests = discover_skill_manifests_from_paths_with_plugin_discovery(
                    manifest.regular_paths(),
                    None,
                    &plugin_discovery,
                )
                .expect("tree discovery");
                let archive_manifests = discover_skill_manifests_from_paths_with_plugin_discovery(
                    snapshot.files.keys().map(String::as_str),
                    None,
                    &plugin_discovery,
                )
                .expect("archive discovery");
                assert_eq!(
                    tree_manifests, archive_manifests,
                    "discovery input set must be identical for tree and archive"
                );
            }
        }

        #[test]
        fn tree_candidates_match_archive_candidates_for_fixtures() {
            for snapshot in [
                root_repo_snapshot(),
                root_package_snapshot(),
                multi_skill_snapshot(),
                namespaced_skill_snapshot(),
                content_skills_snapshot(),
            ] {
                let repo = sample_repo();
                let manifest = tree_manifest_from_snapshot(&repo, &snapshot);
                let archive = archive_candidates(&repo, &snapshot, None);
                let tree = tree_candidates(&repo, &snapshot, &manifest, None);
                assert_eq!(
                    tree, archive,
                    "tree fast-path candidates must match archive candidates"
                );
            }
        }

        #[test]
        fn plugin_discovery_bytes_path_matches_snapshot_path() {
            // The tree fast-path re-derives plugin discovery from raw bytes via
            // `plugin_manifest_discovery_from_manifest_bytes`; it must match the
            // archive path's `plugin_manifest_discovery_from_snapshot` for the
            // same fixture bytes.
            let snapshot = repo_snapshot(&[
                (
                    ".claude-plugin/plugin.json",
                    r#"{"name":"demo","skills":["skills/a/SKILL.md"]}"#.to_string(),
                ),
                ("skills/a/SKILL.md", sample_frontmatter("a", "a skill")),
                ("skills/b/SKILL.md", sample_frontmatter("b", "b skill")),
            ]);
            let repo = sample_repo();
            let manifest = tree_manifest_from_snapshot(&repo, &snapshot);

            let base_path = effective_source_root(None).expect("base path");
            let plugin_json_path =
                join_repo_path(&base_path, ".claude-plugin/plugin.json").expect("join");
            let marketplace_path =
                join_repo_path(&base_path, ".claude-plugin/marketplace.json").expect("join");

            let plugin_json = manifest
                .regular_paths()
                .any(|p| p == plugin_json_path)
                .then(|| snapshot.files.get(&plugin_json_path).cloned())
                .flatten();
            let marketplace = manifest
                .regular_paths()
                .any(|p| p == marketplace_path)
                .then(|| snapshot.files.get(&marketplace_path).cloned())
                .flatten();

            let bytes_discovery = plugin_manifest_discovery_from_manifest_bytes(
                &base_path,
                plugin_json.as_deref(),
                marketplace.as_deref(),
            );
            let snapshot_discovery = plugin_manifest_discovery_from_snapshot(&snapshot, None)
                .expect("snapshot discovery");
            assert_eq!(
                bytes_discovery.explicit_skill_paths,
                snapshot_discovery.explicit_skill_paths
            );
            assert_eq!(
                bytes_discovery.plugin_by_source_path,
                snapshot_discovery.plugin_by_source_path
            );
        }

        #[test]
        fn tree_manifest_from_snapshot_excludes_no_archive_files() {
            // Sanity: the parity helper converts every archive regular file to
            // a RegularBlob — so the fixture must produce a manifest whose
            // regular path set equals the archive file key set.
            let snapshot = multi_skill_snapshot();
            let repo = sample_repo();
            let manifest = tree_manifest_from_snapshot(&repo, &snapshot);
            let mut manifest_paths = manifest
                .regular_files
                .iter()
                .map(|f| f.repo_path.clone())
                .collect::<Vec<_>>();
            manifest_paths.sort();
            let mut snapshot_paths = snapshot.files.keys().cloned().collect::<Vec<_>>();
            snapshot_paths.sort();
            assert_eq!(manifest_paths, snapshot_paths);
        }

        #[test]
        fn tree_import_plan_downloads_only_selected_subtree_union() {
            let snapshot = multi_skill_snapshot();
            let repo = sample_repo();
            let manifest = tree_manifest_from_snapshot(&repo, &snapshot);
            let candidates = archive_candidates(&repo, &snapshot, None);
            let selections = candidates
                .iter()
                .filter(|candidate| candidate.source_path != ".")
                .take(2)
                .map(|candidate| GitHubSkillImportSelection {
                    source_path: candidate.source_path.clone(),
                    resolution: DuplicateResolution::Overwrite,
                    renamed_skill_id: None,
                })
                .collect::<Vec<_>>();

            let plan = plan_tree_selection(&manifest, &candidates, &selections)
                .expect("selected subtree plan");

            assert_eq!(plan.mode, AcquisitionMode::TreeRaw);
            let planned_paths = plan
                .files
                .iter()
                .map(|file| file.repo_path.as_str())
                .collect::<Vec<_>>();
            assert!(!planned_paths.is_empty());
            assert_eq!(
                planned_paths.len(),
                planned_paths.iter().collect::<HashSet<_>>().len(),
                "overlapping selections must download each repo file once"
            );
            assert!(planned_paths
                .iter()
                .all(|path| selections.iter().any(|selection| {
                    repo_file_relative_to_source(path, &selection.source_path).is_some()
                })));
        }

        #[test]
        fn tree_import_plan_routes_root_skill_to_archive() {
            let snapshot = root_repo_snapshot();
            let repo = sample_repo();
            let manifest = tree_manifest_from_snapshot(&repo, &snapshot);
            let candidates = archive_candidates(&repo, &snapshot, None);
            let selections = vec![GitHubSkillImportSelection {
                source_path: ".".to_string(),
                resolution: DuplicateResolution::Overwrite,
                renamed_skill_id: None,
            }];

            let plan = plan_tree_selection(&manifest, &candidates, &selections).expect("root plan");

            assert_eq!(plan.mode, AcquisitionMode::Archive);
            assert_eq!(plan.fallback_reason, Some(FallbackReason::Threshold));
        }

        #[test]
        fn tree_import_plan_avoids_raw_request_amplification() {
            let repo = sample_repo();
            let mut regular_files = vec![RepositoryFileMeta::new(
                "skills/demo/SKILL.md",
                32,
                RepositoryFileKind::RegularBlob,
            )];
            regular_files.extend((0..65).map(|index| {
                RepositoryFileMeta::new(
                    &format!("skills/demo/references/{index}.md"),
                    16,
                    RepositoryFileKind::RegularBlob,
                )
            }));
            let manifest = RepositoryManifest {
                repo,
                regular_files,
                skipped: Vec::new(),
            };
            let candidates = vec![RemoteSkillCandidate {
                source_path: "skills/demo".to_string(),
                skill_id: "demo".to_string(),
                skill_name: "Demo".to_string(),
                description: None,
                plugin_name: None,
                root_directory: "skills".to_string(),
                skill_directory_name: "demo".to_string(),
                download_url: "https://example.invalid/SKILL.md".to_string(),
            }];
            let selections = vec![GitHubSkillImportSelection {
                source_path: "skills/demo".to_string(),
                resolution: DuplicateResolution::Overwrite,
                renamed_skill_id: None,
            }];

            let plan =
                plan_tree_selection(&manifest, &candidates, &selections).expect("amplified plan");

            assert_eq!(plan.mode, AcquisitionMode::Archive);
            assert_eq!(plan.fallback_reason, Some(FallbackReason::Threshold));
        }

        #[test]
        fn raw_size_mismatch_is_an_integrity_fallback() {
            let error = GithubImportError::RepoFileSizeMismatch {
                path: "skills/demo/SKILL.md".to_string(),
                expected: 10,
                actual: 9,
            };
            assert_eq!(fallback_reason_for(&error), Some(FallbackReason::Integrity));
        }

        #[tokio::test]
        async fn tree_import_reuses_already_fetched_candidate_metadata() {
            let repo = sample_repo();
            let bytes = sample_frontmatter("demo", "demo skill").into_bytes();
            let plan = super::super::tree_import::TreeSelectionPlan {
                mode: AcquisitionMode::TreeRaw,
                fallback_reason: None,
                files: vec![RepositoryFileMeta::new(
                    "skills/demo/SKILL.md",
                    bytes.len() as u64,
                    RepositoryFileKind::RegularBlob,
                )],
                selected_bytes: bytes.len() as u64,
            };
            let fetched = HashMap::from([("skills/demo/SKILL.md".to_string(), bytes.clone())]);

            let snapshot = download_tree_selection(
                &github_client().expect("client"),
                &repo,
                &plan,
                None,
                fetched,
            )
            .await
            .expect("metadata-only selection needs no network request");

            assert_eq!(snapshot.files.get("skills/demo/SKILL.md"), Some(&bytes));
        }

        // `try_fetch_tree_manifest` reachability is enforced by `cargo check`
        // (non-test build) which fails on dead code now that the
        // `#![allow(dead_code)]` was removed from `tree_manifest.rs`.
    }

    // ── Immutable preview snapshot contract (task
    // `07-24-github-preview-snapshot`). Covers digest v1 framing, registry
    // lifecycle, snapshot-only reads/import, integrity fail-closed behavior,
    // and per-skill provenance persistence.
    mod preview_snapshot {
        use super::*;

        fn digest_entry(path: &str, bytes: &[u8]) -> DigestFileEntry {
            DigestFileEntry {
                path: path.to_string(),
                byte_len: bytes.len() as u64,
                sha256: file_sha256(bytes),
            }
        }

        fn demo_repo() -> GitHubRepoRef {
            GitHubRepoRef {
                owner: "anthropics".to_string(),
                repo: "skills".to_string(),
                branch: "main".to_string(),
                normalized_url: "https://github.com/anthropics/skills".to_string(),
            }
        }

        #[test]
        fn digest_is_stable_and_independent_of_input_order() {
            let entries = vec![
                digest_entry("SKILL.md", b"skill"),
                digest_entry("references/guide.md", b"guide"),
                digest_entry("assets/logo.png", b"logo"),
            ];
            let mut reversed = entries.clone();
            reversed.reverse();

            let forward = aggregate_digest(REPOSITORY_SNAPSHOT_DIGEST_DOMAIN, &entries);
            let backward = aggregate_digest(REPOSITORY_SNAPSHOT_DIGEST_DOMAIN, &reversed);

            assert_eq!(forward, backward);
            assert!(forward.starts_with("sha256-v1:"));
            assert_eq!(forward.len(), "sha256-v1:".len() + 64);
        }

        #[test]
        fn digest_domains_separate_repository_and_skill_content() {
            let entries = vec![digest_entry("SKILL.md", b"skill")];
            assert_ne!(
                aggregate_digest(REPOSITORY_SNAPSHOT_DIGEST_DOMAIN, &entries),
                aggregate_digest(SKILL_CONTENT_DIGEST_DOMAIN, &entries)
            );
        }

        #[test]
        fn digest_framing_prevents_path_boundary_collisions() {
            let left = vec![digest_entry("a/b", b"x"), digest_entry("c", b"y")];
            let right = vec![digest_entry("a", b"x"), digest_entry("b/c", b"y")];
            assert_ne!(
                aggregate_digest(REPOSITORY_SNAPSHOT_DIGEST_DOMAIN, &left),
                aggregate_digest(REPOSITORY_SNAPSHOT_DIGEST_DOMAIN, &right)
            );
        }

        #[test]
        fn digest_detects_content_and_length_tampering() {
            let base = vec![digest_entry("SKILL.md", b"skill")];
            let tampered_content = vec![digest_entry("SKILL.md", b"skil1")];
            let mut tampered_length = base.clone();
            tampered_length[0].byte_len += 1;

            let baseline = aggregate_digest(REPOSITORY_SNAPSHOT_DIGEST_DOMAIN, &base);
            assert_ne!(
                baseline,
                aggregate_digest(REPOSITORY_SNAPSHOT_DIGEST_DOMAIN, &tampered_content)
            );
            assert_ne!(
                baseline,
                aggregate_digest(REPOSITORY_SNAPSHOT_DIGEST_DOMAIN, &tampered_length)
            );
        }

        #[test]
        fn repository_digest_ignores_hashmap_insertion_order() {
            let files = [
                ("SKILL.md", sample_frontmatter("demo", "demo skill")),
                ("references/guide.md", "# guide\n".to_string()),
                ("assets/example.txt", "asset\n".to_string()),
            ];
            let forward = repo_snapshot(&files);
            let mut reversed_files = files.to_vec();
            reversed_files.reverse();
            let backward = repo_snapshot(&reversed_files);

            assert_eq!(
                repository_snapshot_digest(&snapshot_files_from_local(&forward)),
                repository_snapshot_digest(&snapshot_files_from_local(&backward))
            );
        }

        #[test]
        fn commit_sha_validation_requires_a_full_hex_sha() {
            assert!(validate_commit_sha(&"a".repeat(40)).is_ok());
            for invalid in ["", "abc", &"a".repeat(39), &"a".repeat(41), &"z".repeat(40)] {
                assert!(
                    matches!(
                        validate_commit_sha(invalid),
                        Err(GithubImportError::PreviewCommitUnresolved)
                    ),
                    "expected rejection for {invalid:?}"
                );
            }
        }

        #[test]
        fn pinned_repo_ref_only_replaces_the_acquisition_ref() {
            let repo = demo_repo();
            let sha = "c".repeat(40);
            let pinned = pinned_repo_ref(&repo, &sha);

            assert_eq!(pinned.branch, sha);
            assert_eq!(pinned.owner, repo.owner);
            assert_eq!(pinned.repo, repo.repo);
            assert_eq!(pinned.normalized_url, repo.normalized_url);
            // The pinned ref must be usable as an acquisition ref.
            assert!(validate_repo_ref(&pinned).is_ok());
        }

        fn register_local_snapshot(id: &str, snapshot: GitHubRepoSnapshot) -> PreviewSnapshot {
            let mut registered = local_test_snapshot(&demo_repo(), None, snapshot, Vec::new());
            registered.id = id.to_string();
            register_preview_snapshot(registered.clone()).expect("register local snapshot");
            registered
        }

        #[test]
        fn registry_lookup_rejects_unknown_and_expired_tokens() {
            let id = "github-preview-lookup";
            let registered = register_local_snapshot(id, root_repo_snapshot());

            assert!(lookup_preview_snapshot(id, Utc::now()).is_ok());
            assert!(matches!(
                lookup_preview_snapshot("github-preview-unknown", Utc::now()),
                Err(GithubImportError::PreviewSnapshotMissing)
            ));
            assert!(matches!(
                lookup_preview_snapshot(id, registered.expires_at),
                Err(GithubImportError::PreviewWorkspaceExpired)
            ));

            assert!(discard_preview_snapshot(id).is_some());
            assert!(!preview_snapshot_is_registered(id));
        }

        #[test]
        fn import_lease_is_exclusive_and_released_for_retry() {
            let id = "github-preview-lease";
            register_local_snapshot(id, root_repo_snapshot());

            assert!(acquire_import_lease(id, Utc::now()).is_ok());
            assert!(matches!(
                acquire_import_lease(id, Utc::now()),
                Err(GithubImportError::PreviewSnapshotBusy)
            ));

            // Failure releases the lease and keeps the snapshot for a retry.
            assert!(release_import_lease(id).is_none());
            assert!(preview_snapshot_is_registered(id));
            assert!(acquire_import_lease(id, Utc::now()).is_ok());

            // Success consumes the token; every later read or import fails.
            assert!(consume_preview_snapshot(id).is_none());
            assert!(!preview_snapshot_is_registered(id));
            assert!(matches!(
                lookup_preview_snapshot(id, Utc::now()),
                Err(GithubImportError::PreviewSnapshotMissing)
            ));
            assert!(matches!(
                acquire_import_lease(id, Utc::now()),
                Err(GithubImportError::PreviewSnapshotMissing)
            ));
        }

        #[test]
        fn discard_during_an_import_lease_is_deferred_until_release() {
            let id = "github-preview-deferred-discard";
            register_local_snapshot(id, root_repo_snapshot());
            assert!(acquire_import_lease(id, Utc::now()).is_ok());

            // The in-flight import still owns the storage.
            assert!(discard_preview_snapshot(id).is_none());
            assert!(preview_snapshot_is_registered(id));

            // Releasing the lease applies the pending discard and hands the
            // snapshot back so the caller can release remote storage.
            assert!(release_import_lease(id).is_none());
            assert!(!preview_snapshot_is_registered(id));
        }

        /// Register an already-expired snapshot.
        ///
        /// The target is unique to this test so target-scoped pruning cannot
        /// observe snapshots registered by parallel lifecycle tests.
        fn register_expired_local_snapshot(id: &str, target_id: &str) -> PreviewSnapshot {
            let mut registered =
                local_test_snapshot(&demo_repo(), None, root_repo_snapshot(), Vec::new());
            registered.id = id.to_string();
            registered.target_id = target_id.to_string();
            registered.created_at = Utc::now() - Duration::minutes(60);
            registered.expires_at = Utc::now() - Duration::minutes(30);
            register_preview_snapshot(registered.clone()).expect("register expired snapshot");
            registered
        }

        #[test]
        fn expiry_pruning_removes_only_unleased_snapshots() {
            let leased = "github-preview-prune-leased";
            let expired = "github-preview-prune-expired";
            let target_id = "github-preview-prune-expired-target";
            register_expired_local_snapshot(leased, target_id);
            register_expired_local_snapshot(expired, target_id);
            // An expired snapshot can never start a new import.
            assert!(matches!(
                acquire_import_lease(leased, Utc::now()),
                Err(GithubImportError::PreviewSnapshotMissing)
            ));

            let pruned = prune_expired_preview_snapshots_for_target(target_id, Utc::now());
            assert!(pruned.iter().any(|snapshot| snapshot.id == expired));
            assert!(pruned.iter().all(|snapshot| snapshot.id != leased));
            assert!(!preview_snapshot_is_registered(expired));
            assert!(!preview_snapshot_is_registered(leased));
        }

        #[tokio::test]
        async fn pruning_never_removes_a_snapshot_with_an_active_import_lease() {
            let id = "github-preview-prune-active-lease";
            let target_id = "github-preview-prune-active-target";
            // A short TTL keeps the prune clock at "now", so snapshots
            // registered by tests running in parallel are never pruned.
            let mut registered =
                local_test_snapshot(&demo_repo(), None, root_repo_snapshot(), Vec::new());
            registered.id = id.to_string();
            registered.target_id = target_id.to_string();
            registered.expires_at = Utc::now() + Duration::milliseconds(50);
            register_preview_snapshot(registered).expect("register leased snapshot");
            assert!(acquire_import_lease(id, Utc::now()).is_ok());

            tokio::time::sleep(std::time::Duration::from_millis(120)).await;
            let pruned = prune_expired_preview_snapshots_for_target(target_id, Utc::now());
            assert!(pruned.iter().all(|snapshot| snapshot.id != id));
            assert!(preview_snapshot_is_registered(id));

            assert!(release_import_lease(id).is_none());
            assert!(discard_preview_snapshot(id).is_some());
        }

        #[tokio::test]
        async fn snapshot_reads_return_preview_bytes_and_never_consume_the_token() {
            let id = "github-preview-markdown";
            let snapshot_files = root_repo_snapshot();
            let mut registered =
                local_test_snapshot(&demo_repo(), None, snapshot_files, Vec::new());
            registered.id = id.to_string();
            registered.candidates = vec![PreviewSnapshotCandidate {
                source_path: ".".to_string(),
                content_digest: "sha256-v1:candidate".to_string(),
            }];
            register_preview_snapshot(registered).expect("register markdown snapshot");

            for _ in 0..2 {
                let markdown = fetch_github_skill_markdown_from_snapshot(
                    &ActiveTarget::Local,
                    id,
                    &demo_repo(),
                    ".",
                )
                .await
                .expect("markdown read");
                assert!(markdown.contains("twitterapi-io"));
            }
            assert!(preview_snapshot_is_registered(id));

            // A candidate that was never previewed cannot be read.
            assert!(matches!(
                fetch_github_skill_markdown_from_snapshot(
                    &ActiveTarget::Local,
                    id,
                    &demo_repo(),
                    "skills/other",
                )
                .await,
                Err(GithubImportError::SelectionUnavailable(path)) if path == "skills/other"
            ));

            // A different repository is a binding mismatch.
            let other_repo = GitHubRepoRef {
                repo: "other".to_string(),
                ..demo_repo()
            };
            assert!(matches!(
                fetch_github_skill_markdown_from_snapshot(
                    &ActiveTarget::Local,
                    id,
                    &other_repo,
                    ".",
                )
                .await,
                Err(GithubImportError::PreviewWorkspaceMismatch)
            ));

            // A different target is a target change.
            assert!(matches!(
                fetch_github_skill_markdown_from_snapshot(
                    &ssh_test_target("ssh-demo"),
                    id,
                    &demo_repo(),
                    ".",
                )
                .await,
                Err(GithubImportError::PreviewTargetChanged)
            ));

            assert!(discard_preview_snapshot(id).is_some());
        }

        #[tokio::test]
        async fn snapshot_read_fails_closed_when_storage_content_changed() {
            let id = "github-preview-read-integrity";
            let mut registered =
                local_test_snapshot(&demo_repo(), None, root_repo_snapshot(), Vec::new());
            registered.id = id.to_string();
            registered.candidates = vec![PreviewSnapshotCandidate {
                source_path: ".".to_string(),
                content_digest: "sha256-v1:candidate".to_string(),
            }];
            // Simulate a manifest that no longer matches the retained bytes.
            for file in &mut registered.files {
                if file.repo_path == "SKILL.md" {
                    file.sha256 = [0_u8; 32];
                }
            }
            register_preview_snapshot(registered).expect("register integrity snapshot");

            assert!(matches!(
                fetch_github_skill_markdown_from_snapshot(
                    &ActiveTarget::Local,
                    id,
                    &demo_repo(),
                    ".",
                )
                .await,
                Err(GithubImportError::PreviewSnapshotIntegrity)
            ));

            assert!(discard_preview_snapshot(id).is_some());
        }

        #[tokio::test]
        async fn integrity_verification_rejects_a_changed_retained_snapshot() {
            let mut registered =
                local_test_snapshot(&demo_repo(), None, root_repo_snapshot(), Vec::new());
            assert!(verify_snapshot_integrity(&ActiveTarget::Local, &registered)
                .await
                .is_ok());

            registered.snapshot_digest = "sha256-v1:not-the-registered-digest".to_string();
            assert!(matches!(
                verify_snapshot_integrity(&ActiveTarget::Local, &registered).await,
                Err(GithubImportError::PreviewSnapshotIntegrity)
            ));
        }

        #[tokio::test]
        async fn import_uses_preview_bytes_and_persists_per_skill_provenance() {
            let pool = setup_test_db().await;
            let central_root = tempdir().expect("central root");
            crate::test_support::set_agent_dir(&pool, "central", central_root.path()).await;

            let repo = demo_repo();
            let snapshot_files = repo_snapshot(&[
                (
                    "skills/demo/SKILL.md",
                    sample_frontmatter("Demo Skill", "preview-time content"),
                ),
                ("skills/demo/references/guide.md", "# guide\n".to_string()),
            ]);
            let candidates = build_repo_skill_candidates_from_snapshot(&repo, &snapshot_files)
                .expect("candidates");
            let mut skills = build_preview_skills(&pool, &candidates)
                .await
                .expect("preview skills");
            let repository_files = snapshot_preview_repository_files(&snapshot_files);
            let snapshot_candidates =
                attach_preview_file_manifests(&mut skills, &repository_files).expect("manifests");

            let id = "github-preview-import";
            let mut registered =
                local_test_snapshot(&repo, None, snapshot_files, snapshot_candidates.clone());
            registered.id = id.to_string();
            let expected_commit = registered.resolved_commit_sha.clone();
            register_preview_snapshot(registered).expect("register import snapshot");

            let result = import_github_repo_skills_from_preview(
                &pool,
                &ActiveTarget::Local,
                id,
                "https://github.com/anthropics/skills",
                vec![GitHubSkillImportSelection {
                    source_path: "skills/demo".to_string(),
                    resolution: DuplicateResolution::Overwrite,
                    renamed_skill_id: None,
                }],
                None,
            )
            .await
            .expect("import from preview snapshot");

            assert_eq!(result.imported_skills.len(), 1);
            let imported_id = result.imported_skills[0].imported_skill_id.clone();
            let installed = central_root.path().join(&imported_id);
            assert!(installed.join("SKILL.md").exists());
            assert!(installed.join("references/guide.md").exists());
            assert!(std::fs::read_to_string(installed.join("SKILL.md"))
                .expect("read imported SKILL.md")
                .contains("preview-time content"));

            let (commit, digest) = db::get_skill_repository_provenance(&pool, &imported_id)
                .await
                .expect("provenance query")
                .expect("membership row");
            assert_eq!(commit.as_deref(), Some(expected_commit.as_str()));
            assert_eq!(
                digest.as_deref(),
                snapshot_candidates
                    .iter()
                    .find(|candidate| candidate.source_path == "skills/demo")
                    .map(|candidate| candidate.content_digest.as_str())
            );

            // Success consumes the token immediately.
            assert!(!preview_snapshot_is_registered(id));
            assert!(matches!(
                import_github_repo_skills_from_preview(
                    &pool,
                    &ActiveTarget::Local,
                    id,
                    "https://github.com/anthropics/skills",
                    vec![GitHubSkillImportSelection {
                        source_path: "skills/demo".to_string(),
                        resolution: DuplicateResolution::Overwrite,
                        renamed_skill_id: None,
                    }],
                    None,
                )
                .await,
                Err(GithubImportError::PreviewSnapshotMissing)
            ));
        }

        /// Renamed imports must record provenance under the final skill ID, and
        /// a skipped selection must not create or touch a membership row.
        #[tokio::test]
        async fn renamed_import_records_provenance_and_skip_writes_nothing() {
            let pool = setup_test_db().await;
            let central_root = tempdir().expect("central root");
            crate::test_support::set_agent_dir(&pool, "central", central_root.path()).await;

            let repo = demo_repo();
            let snapshot_files = repo_snapshot(&[
                (
                    "skills/agent-planner/SKILL.md",
                    sample_frontmatter("Agent Planner", "planner"),
                ),
                (
                    "skills/commit/SKILL.md",
                    sample_frontmatter("Commit", "commit"),
                ),
            ]);
            let candidates = build_repo_skill_candidates_from_snapshot(&repo, &snapshot_files)
                .expect("candidates");
            let mut skills = build_preview_skills(&pool, &candidates)
                .await
                .expect("preview skills");
            let repository_files = snapshot_preview_repository_files(&snapshot_files);
            let snapshot_candidates =
                attach_preview_file_manifests(&mut skills, &repository_files).expect("manifests");

            let id = "github-preview-rename-skip";
            let mut registered =
                local_test_snapshot(&repo, None, snapshot_files, snapshot_candidates.clone());
            registered.id = id.to_string();
            let expected_commit = registered.resolved_commit_sha.clone();
            register_preview_snapshot(registered).expect("register renamed import snapshot");

            let result = import_github_repo_skills_from_preview(
                &pool,
                &ActiveTarget::Local,
                id,
                "https://github.com/anthropics/skills",
                vec![
                    GitHubSkillImportSelection {
                        source_path: "skills/agent-planner".to_string(),
                        resolution: DuplicateResolution::Rename,
                        renamed_skill_id: Some("planner-renamed".to_string()),
                    },
                    GitHubSkillImportSelection {
                        source_path: "skills/commit".to_string(),
                        resolution: DuplicateResolution::Skip,
                        renamed_skill_id: None,
                    },
                ],
                None,
            )
            .await
            .expect("import from preview snapshot");

            assert_eq!(result.imported_skills.len(), 1);
            assert_eq!(
                result.imported_skills[0].imported_skill_id,
                "planner-renamed"
            );
            assert_eq!(result.skipped_skills, vec!["skills/commit".to_string()]);

            // Provenance follows the final (renamed) skill ID.
            let (commit, digest) = db::get_skill_repository_provenance(&pool, "planner-renamed")
                .await
                .expect("provenance query")
                .expect("membership row");
            assert_eq!(commit.as_deref(), Some(expected_commit.as_str()));
            assert_eq!(
                digest.as_deref(),
                snapshot_candidates
                    .iter()
                    .find(|candidate| candidate.source_path == "skills/agent-planner")
                    .map(|candidate| candidate.content_digest.as_str())
            );

            // The skipped selection wrote no skill and no membership row.
            assert!(db::get_skill_by_id(&pool, "commit")
                .await
                .expect("db")
                .is_none());
            assert!(db::get_skill_repository_provenance(&pool, "commit")
                .await
                .expect("provenance query")
                .is_none());
        }

        #[tokio::test]
        async fn import_fails_closed_on_binding_mismatch_and_keeps_the_token() {
            let pool = setup_test_db().await;
            let central_root = tempdir().expect("central root");
            crate::test_support::set_agent_dir(&pool, "central", central_root.path()).await;

            let repo = demo_repo();
            let snapshot_files = repo_snapshot(&[(
                "skills/demo/SKILL.md",
                sample_frontmatter("Demo Skill", "preview-time content"),
            )]);
            let candidates = build_repo_skill_candidates_from_snapshot(&repo, &snapshot_files)
                .expect("candidates");
            let mut skills = build_preview_skills(&pool, &candidates)
                .await
                .expect("preview skills");
            let repository_files = snapshot_preview_repository_files(&snapshot_files);
            let snapshot_candidates =
                attach_preview_file_manifests(&mut skills, &repository_files).expect("manifests");

            let id = "github-preview-binding-mismatch";
            let mut registered =
                local_test_snapshot(&repo, None, snapshot_files, snapshot_candidates);
            registered.id = id.to_string();
            register_preview_snapshot(registered).expect("register binding snapshot");

            let selections = vec![GitHubSkillImportSelection {
                source_path: "skills/demo".to_string(),
                resolution: DuplicateResolution::Overwrite,
                renamed_skill_id: None,
            }];

            assert!(matches!(
                import_github_repo_skills_from_preview(
                    &pool,
                    &ActiveTarget::Local,
                    id,
                    "https://github.com/anthropics/other",
                    selections.clone(),
                    None,
                )
                .await,
                Err(GithubImportError::PreviewWorkspaceMismatch)
            ));
            assert!(std::fs::read_dir(central_root.path())
                .expect("read central root")
                .next()
                .is_none());

            assert!(matches!(
                import_github_repo_skills_from_preview_with_branch(
                    &pool,
                    &ActiveTarget::Local,
                    id,
                    "https://github.com/anthropics/skills/tree/main",
                    Some("dev"),
                    selections.clone(),
                    None,
                )
                .await,
                Err(GithubImportError::BranchSelectionConflict)
            ));
            assert!(matches!(
                import_github_repo_skills_from_preview_with_branch(
                    &pool,
                    &ActiveTarget::Local,
                    id,
                    "https://github.com/anthropics/skills",
                    Some("dev"),
                    selections.clone(),
                    None,
                )
                .await,
                Err(GithubImportError::PreviewWorkspaceMismatch)
            ));

            assert!(matches!(
                import_github_repo_skills_from_preview(
                    &pool,
                    &ActiveTarget::Local,
                    id,
                    "https://github.com/anthropics/skills",
                    vec![GitHubSkillImportSelection {
                        source_path: "skills/never-previewed".to_string(),
                        resolution: DuplicateResolution::Overwrite,
                        renamed_skill_id: None,
                    }],
                    None,
                )
                .await,
                Err(GithubImportError::SelectionUnavailable(path)) if path == "skills/never-previewed"
            ));

            // Every binding/selection failure released the lease, so the same
            // preview can retry with its matching explicit branch.
            assert!(preview_snapshot_is_registered(id));
            let result = import_github_repo_skills_from_preview_with_branch(
                &pool,
                &ActiveTarget::Local,
                id,
                "https://github.com/anthropics/skills",
                Some("main"),
                selections,
                None,
            )
            .await
            .expect("retry with the same preview token");
            assert_eq!(result.imported_skills.len(), 1);
            assert!(!preview_snapshot_is_registered(id));
        }

        #[tokio::test]
        async fn import_fails_before_mutation_when_the_snapshot_digest_changed() {
            let pool = setup_test_db().await;
            let central_root = tempdir().expect("central root");
            crate::test_support::set_agent_dir(&pool, "central", central_root.path()).await;

            let repo = demo_repo();
            let snapshot_files = repo_snapshot(&[(
                "skills/demo/SKILL.md",
                sample_frontmatter("Demo Skill", "preview-time content"),
            )]);
            let candidates = build_repo_skill_candidates_from_snapshot(&repo, &snapshot_files)
                .expect("candidates");
            let mut skills = build_preview_skills(&pool, &candidates)
                .await
                .expect("preview skills");
            let repository_files = snapshot_preview_repository_files(&snapshot_files);
            let snapshot_candidates =
                attach_preview_file_manifests(&mut skills, &repository_files).expect("manifests");

            let id = "github-preview-import-integrity";
            let mut registered =
                local_test_snapshot(&repo, None, snapshot_files, snapshot_candidates);
            registered.id = id.to_string();
            registered.snapshot_digest = "sha256-v1:stale".to_string();
            register_preview_snapshot(registered).expect("register digest snapshot");

            assert!(matches!(
                import_github_repo_skills_from_preview(
                    &pool,
                    &ActiveTarget::Local,
                    id,
                    "https://github.com/anthropics/skills",
                    vec![GitHubSkillImportSelection {
                        source_path: "skills/demo".to_string(),
                        resolution: DuplicateResolution::Overwrite,
                        renamed_skill_id: None,
                    }],
                    None,
                )
                .await,
                Err(GithubImportError::PreviewSnapshotIntegrity)
            ));
            assert!(std::fs::read_dir(central_root.path())
                .expect("read central root")
                .next()
                .is_none());
            let skills = db::get_central_skills(&pool).await.expect("skills");
            assert!(skills.is_empty(), "integrity failure must not write the DB");

            assert!(discard_preview_snapshot(id).is_some());
        }

        /// The remote inventory and the local snapshot must agree byte for byte,
        /// so an SSH/WSL preview and a Local preview of the same content produce
        /// the same registered digest.
        #[test]
        fn remote_inventory_digest_matches_the_local_snapshot_digest() {
            let snapshot = repo_snapshot(&[
                ("SKILL.md", sample_frontmatter("demo", "demo skill")),
                ("references/guide.md", "# guide\n".to_string()),
            ]);
            let local_files = snapshot_files_from_local(&snapshot);

            let mut inventory = String::new();
            for file in &local_files {
                inventory.push_str(&format!(
                    "{}\0{}\0{}\0",
                    file.repo_path,
                    file.byte_len,
                    hex_encode(&file.sha256)
                ));
            }
            let remote_files =
                parse_remote_preview_repository_files(&inventory).expect("remote inventory");

            assert_eq!(remote_files, local_files);
            assert_eq!(
                repository_snapshot_digest(&remote_files),
                repository_snapshot_digest(&local_files)
            );
        }

        /// Structural guard for the core invariant: the confirmed import must not
        /// be able to acquire anything. If this module ever regains a client,
        /// repo-source resolution, or archive download, a moved branch could
        /// silently change what is written to Central.
        #[test]
        fn preview_import_module_cannot_acquire_repository_content() {
            let source = include_str!("snapshot_import.rs");
            for forbidden in [
                "download_repo_snapshot",
                "resolve_repo_source",
                "github_client",
                "try_prepare_tree_import",
                "fetch_raw",
                "create_remote_preview_workspace",
            ] {
                assert!(
                    !source.contains(forbidden),
                    "preview-driven import must not reference {forbidden}"
                );
            }
        }

        #[test]
        fn snapshot_lifecycle_errors_use_stable_ipc_codes_without_leaking_details() {
            for (error, code) in [
                (GithubImportError::PreviewSnapshotMissing, "preview_missing"),
                (
                    GithubImportError::PreviewWorkspaceExpired,
                    "preview_expired",
                ),
                (
                    GithubImportError::PreviewWorkspaceMismatch,
                    "preview_mismatch",
                ),
                (GithubImportError::PreviewTargetChanged, "preview_mismatch"),
                (
                    GithubImportError::PreviewSnapshotIntegrity,
                    "preview_integrity",
                ),
                (GithubImportError::PreviewSnapshotBusy, "preview_busy"),
                (GithubImportError::PreviewCapacity, "preview_capacity"),
                (
                    GithubImportError::PreviewCleanupPending,
                    "preview_cleanup_pending",
                ),
                (
                    GithubImportError::PreviewCommitUnresolved,
                    "preview_commit_unresolved",
                ),
            ] {
                let envelope = error.to_ipc_error();
                assert_eq!(error.preview_snapshot_code(), Some(code));
                assert!(
                    envelope.starts_with(&format!("github_import.{code}:")),
                    "unexpected envelope: {envelope}"
                );
                for leaked in ["github-preview-", "/tmp/", "sha256-v1:", "ghp_"] {
                    assert!(
                        !envelope.contains(leaked),
                        "envelope leaked {leaked}: {envelope}"
                    );
                }
            }

            // Uncoded errors keep their historical Display text so existing
            // toasts do not change.
            let other = GithubImportError::NoSelections;
            assert_eq!(other.preview_snapshot_code(), None);
            assert_eq!(other.to_ipc_error(), other.to_string());
        }

        #[test]
        fn no_importable_skills_uses_stable_ipc_code_without_dynamic_details() {
            let error = GithubImportError::NoImportableSkills;
            let envelope = error.to_ipc_error();
            assert_eq!(error.preview_snapshot_code(), None);
            assert_eq!(error.ipc_code(), Some("no_importable_skills"));
            assert!(envelope.starts_with("github_import.no_importable_skills:"));
            for leaked in ["github-preview-", "/tmp/", "sha256-v1:", "ghp_"] {
                assert!(
                    !envelope.contains(leaked),
                    "envelope leaked {leaked}: {envelope}"
                );
            }
        }

        #[test]
        fn branch_selection_errors_use_stable_ipc_codes_without_dynamic_details() {
            for (error, code) in [
                (GithubImportError::InvalidBranchSelection, "branch_invalid"),
                (
                    GithubImportError::BranchSelectionConflict,
                    "branch_conflict",
                ),
            ] {
                let envelope = error.to_ipc_error();
                assert_eq!(error.ipc_code(), Some(code));
                assert!(envelope.starts_with(&format!("github_import.{code}:")));
                for leaked in ["feature/private", "ghp_", "github-preview-"] {
                    assert!(!envelope.contains(leaked));
                }
            }
        }
    }
}
