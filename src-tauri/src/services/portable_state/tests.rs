use super::*;
use crate::db::{self, Skill};
use crate::services::github_import::DuplicateResolution;
use sqlx::SqlitePool;
use std::collections::{HashMap, HashSet};
use std::sync::{atomic::AtomicBool, Arc};

async fn setup_test_db() -> DbPool {
    let pool = SqlitePool::connect(":memory:").await.unwrap();
    db::init_database(&pool).await.unwrap();
    pool
}

fn github_source(path: &str) -> PortableCentralSkillSource {
    github_source_for_repo("openai", "skills", "main", path)
}

fn github_source_for_repo(
    owner: &str,
    repo: &str,
    branch: &str,
    path: &str,
) -> PortableCentralSkillSource {
    PortableCentralSkillSource {
        source_type: "github".to_string(),
        owner: owner.to_string(),
        repo: repo.to_string(),
        branch: branch.to_string(),
        url: format!("https://github.com/{owner}/{repo}"),
        source_path: path.to_string(),
    }
}

fn manifest_with_skill(id: &str, path: &str) -> SkillportStateManifest {
    SkillportStateManifest {
        kind: EXPORT_KIND.to_string(),
        version: EXPORT_VERSION,
        exported_at: "2026-04-25T00:00:00Z".to_string(),
        exported_from: ExportedFrom {
            app: "SkillPort".to_string(),
        },
        github_sources: vec![PortableGithubSource {
            name: "OpenAI Skills".to_string(),
            source_type: "github".to_string(),
            url: "https://github.com/openai/skills".to_string(),
            is_enabled: true,
        }],
        central_skills: vec![PortableCentralSkill {
            id: id.to_string(),
            name: id.to_string(),
            description: Some("demo".to_string()),
            source: github_source(path),
            tags: vec![PortableSkillTag {
                name: "Docs".to_string(),
                description: None,
                color: Some("#111111".to_string()),
            }],
        }],
        unrestorable_skills: Vec::new(),
    }
}

#[tokio::test]
async fn export_empty_state_produces_manifest() {
    let pool = setup_test_db().await;
    let json = export_skillport_state_impl(&pool, None, None)
        .await
        .unwrap();
    let manifest = parse_manifest(&json).unwrap();
    assert_eq!(manifest.kind, EXPORT_KIND);
    assert_eq!(manifest.version, EXPORT_VERSION);
    assert!(manifest.github_sources.is_empty());
}

#[tokio::test]
async fn export_includes_github_skill_and_unrestorable_local_skill() {
    let pool = setup_test_db().await;
    let github = Skill {
        id: "openai-docs".to_string(),
        name: "openai-docs".to_string(),
        description: Some("docs".to_string()),
        file_path: "/tmp/openai-docs/SKILL.md".to_string(),
        canonical_path: Some("/tmp/openai-docs".to_string()),
        is_central: true,
        source: Some("github:openai/skills".to_string()),
        content: None,
        scanned_at: "2026-04-25T00:00:00Z".to_string(),
        fs_created_at: None,
        fs_updated_at: None,
    };
    db::upsert_skill(&pool, &github).await.unwrap();
    db::assign_github_repository_to_skill(
        &pool,
        "openai",
        "skills",
        "main",
        "https://github.com/openai/skills",
        "openai-docs",
        "skills/openai-docs",
    )
    .await
    .unwrap();
    let tag = db::create_skill_tag(&pool, "Docs", None, Some("#111111"))
        .await
        .unwrap();
    db::assign_skill_tags(
        &pool,
        &["openai-docs".to_string()],
        &[tag.id],
        "manual",
        None,
        None,
    )
    .await
    .unwrap();
    let local = Skill {
        id: "local-skill".to_string(),
        name: "local-skill".to_string(),
        description: None,
        file_path: "/tmp/local-skill/SKILL.md".to_string(),
        canonical_path: Some("/tmp/local-skill".to_string()),
        is_central: true,
        source: None,
        content: None,
        scanned_at: "2026-04-25T00:00:00Z".to_string(),
        fs_created_at: None,
        fs_updated_at: None,
    };
    db::upsert_skill(&pool, &local).await.unwrap();

    let manifest = parse_manifest(
        &export_skillport_state_impl(&pool, None, None)
            .await
            .unwrap(),
    )
    .unwrap();

    assert_eq!(manifest.central_skills.len(), 1);
    assert_eq!(manifest.github_sources.len(), 1);
    assert_eq!(manifest.github_sources[0].name, "openai/skills");
    assert_eq!(
        manifest.github_sources[0].url,
        "https://github.com/openai/skills"
    );
    assert_eq!(
        manifest.central_skills[0].source.source_path,
        "skills/openai-docs/SKILL.md"
    );
    assert_eq!(manifest.central_skills[0].tags[0].name, "Docs");
    assert_eq!(manifest.unrestorable_skills.len(), 1);
}

#[tokio::test]
async fn export_counts_distinct_github_repositories_backing_central_skills() {
    let pool = setup_test_db().await;
    for (id, name) in [
        ("alpha-one", "Alpha One"),
        ("alpha-two", "Alpha Two"),
        ("beta-one", "Beta One"),
    ] {
        db::upsert_skill(
            &pool,
            &Skill {
                id: id.to_string(),
                name: name.to_string(),
                description: None,
                file_path: format!("/tmp/{id}/SKILL.md"),
                canonical_path: Some(format!("/tmp/{id}")),
                is_central: true,
                source: Some("github:test/source".to_string()),
                content: None,
                scanned_at: "2026-04-25T00:00:00Z".to_string(),
                fs_created_at: None,
                fs_updated_at: None,
            },
        )
        .await
        .unwrap();
    }
    db::assign_github_repository_to_skill(
        &pool,
        "example",
        "alpha-skills",
        "main",
        "https://github.com/example/alpha-skills",
        "alpha-one",
        "skills/alpha-one",
    )
    .await
    .unwrap();
    db::assign_github_repository_to_skill(
        &pool,
        "example",
        "alpha-skills",
        "main",
        "https://github.com/example/alpha-skills",
        "alpha-two",
        "skills/alpha-two",
    )
    .await
    .unwrap();
    db::assign_github_repository_to_skill(
        &pool,
        "example",
        "beta-skills",
        "main",
        "https://github.com/example/beta-skills",
        "beta-one",
        "skills/beta-one",
    )
    .await
    .unwrap();

    let manifest = parse_manifest(
        &export_skillport_state_impl(&pool, None, None)
            .await
            .unwrap(),
    )
    .unwrap();
    let urls = manifest
        .github_sources
        .iter()
        .map(|source| source.url.as_str())
        .collect::<Vec<_>>();

    assert_eq!(
        urls,
        vec![
            "https://github.com/example/alpha-skills",
            "https://github.com/example/beta-skills",
        ]
    );
    assert_eq!(manifest.central_skills.len(), 3);
}

#[test]
fn parse_manifest_rejects_invalid_kind_and_version() {
    let invalid_kind = r#"{"kind":"other","version":1,"exportedAt":"","exportedFrom":{"app":"SkillPort"},"githubSources":[],"centralSkills":[],"unrestorableSkills":[]}"#;
    assert!(parse_manifest(invalid_kind).unwrap_err().contains("kind"));

    let invalid_version = r#"{"kind":"skillport/state-export","version":2,"exportedAt":"","exportedFrom":{"app":"SkillPort"},"githubSources":[],"centralSkills":[],"unrestorableSkills":[]}"#;
    assert!(parse_manifest(invalid_version)
        .unwrap_err()
        .contains("version"));
}

#[tokio::test]
async fn ensure_github_sources_skips_duplicates() {
    let pool = setup_test_db().await;
    let mut manifest = manifest_with_skill("openai-docs", "skills/openai-docs/SKILL.md");
    manifest.github_sources[0].url = "https://github.com/example/portable-skills".to_string();

    let first = ensure_github_sources(&pool, &manifest.github_sources)
        .await
        .unwrap();
    let second = ensure_github_sources(&pool, &manifest.github_sources)
        .await
        .unwrap();

    assert_eq!(first, (1, 0));
    assert_eq!(second, (0, 1));
}

#[tokio::test]
async fn ensure_github_sources_skips_duplicate_sources_in_same_manifest() {
    let pool = setup_test_db().await;
    let mut manifest = manifest_with_skill("openai-docs", "skills/openai-docs/SKILL.md");
    manifest.github_sources[0].url = "https://github.com/example/portable-skills".to_string();
    let mut duplicate_source = manifest.github_sources[0].clone();
    duplicate_source.name = "OpenAI Skills Duplicate".to_string();
    duplicate_source.url = "https://github.com/example/portable-skills.git".to_string();
    manifest.github_sources.push(duplicate_source);

    let result = ensure_github_sources(&pool, &manifest.github_sources)
        .await
        .unwrap();

    assert_eq!(result, (1, 1));
}

#[tokio::test]
async fn preview_reports_ready_conflict_missing_and_unrestorable() {
    let pool = setup_test_db().await;
    let existing = Skill {
        id: "conflict-skill".to_string(),
        name: "conflict-skill".to_string(),
        description: None,
        file_path: "/tmp/conflict-skill/SKILL.md".to_string(),
        canonical_path: Some("/tmp/conflict-skill".to_string()),
        is_central: true,
        source: None,
        content: None,
        scanned_at: "2026-04-25T00:00:00Z".to_string(),
        fs_created_at: None,
        fs_updated_at: None,
    };
    db::upsert_skill(&pool, &existing).await.unwrap();

    let mut manifest = manifest_with_skill("ready-skill", "skills/ready-skill/SKILL.md");
    manifest.central_skills.push(PortableCentralSkill {
        id: "conflict-skill".to_string(),
        name: "conflict-skill".to_string(),
        description: None,
        source: github_source("skills/conflict-skill/SKILL.md"),
        tags: Vec::new(),
    });
    manifest.central_skills.push(PortableCentralSkill {
        id: "missing-skill".to_string(),
        name: "missing-skill".to_string(),
        description: None,
        source: github_source("skills/missing-skill/SKILL.md"),
        tags: Vec::new(),
    });
    manifest
        .unrestorable_skills
        .push(PortableUnrestorableSkill {
            id: "local-only".to_string(),
            name: "local-only".to_string(),
            reason: "source_unknown".to_string(),
        });

    let mut paths = HashSet::new();
    paths.insert("skills/ready-skill".to_string());
    paths.insert("skills/conflict-skill".to_string());
    let mut catalog = HashMap::new();
    catalog.insert(
        repo_key(&github_source("skills/ready-skill/SKILL.md")),
        RemoteCatalogEntry {
            valid_source_paths: paths,
            invalid_candidates: HashMap::new(),
            repo_error: None,
        },
    );

    let preview = preview_skillport_state_import_impl(&pool, &manifest, Some(&catalog), None, None)
        .await
        .unwrap();

    assert_eq!(preview.summary.ready, 1);
    assert_eq!(preview.summary.conflicts, 1);
    assert_eq!(preview.summary.missing, 1);
    assert_eq!(preview.summary.unrestorable, 1);
}

#[tokio::test]
async fn preview_reports_internal_duplicate_skills_and_sources() {
    let pool = setup_test_db().await;
    let mut manifest = manifest_with_skill("dup-skill", "skills/dup-skill/SKILL.md");
    manifest.github_sources.push(PortableGithubSource {
        name: "OpenAI Skills Duplicate".to_string(),
        source_type: "github".to_string(),
        url: "https://github.com/openai/skills.git".to_string(),
        is_enabled: true,
    });
    manifest
        .central_skills
        .push(manifest.central_skills[0].clone());
    manifest.central_skills.push(PortableCentralSkill {
        id: "dup-skill".to_string(),
        name: "dup-skill-alt".to_string(),
        description: None,
        source: github_source("skills/dup-skill-alt/SKILL.md"),
        tags: Vec::new(),
    });

    let mut paths = HashSet::new();
    paths.insert("skills/dup-skill".to_string());
    paths.insert("skills/dup-skill-alt".to_string());
    let mut catalog = HashMap::new();
    catalog.insert(
        repo_key(&github_source("skills/dup-skill/SKILL.md")),
        RemoteCatalogEntry {
            valid_source_paths: paths,
            invalid_candidates: HashMap::new(),
            repo_error: None,
        },
    );

    let preview = preview_skillport_state_import_impl(&pool, &manifest, Some(&catalog), None, None)
        .await
        .unwrap();

    assert_eq!(preview.summary.sources_duplicate, 1);
    assert_eq!(preview.summary.duplicate_skipped, 1);
    assert_eq!(preview.summary.conflicts, 1);
    assert!(preview.skills.iter().any(|skill| {
        skill.status == SkillPreviewStatus::DuplicateSkipped
            && skill.reason.as_deref() == Some("duplicate_in_json")
    }));
    assert!(preview.skills.iter().any(|skill| {
        skill.status == SkillPreviewStatus::Conflict
            && skill.reason.as_deref() == Some("duplicate_skill_id_different_source")
    }));
}

#[tokio::test]
async fn preview_reports_invalid_remote_skill_and_repo_unavailable_as_unrestorable() {
    let pool = setup_test_db().await;
    let invalid_source = github_source_for_repo(
        "openai",
        "skills",
        "main",
        "skills/bad-frontmatter/SKILL.md",
    );
    let repo_error_source =
        github_source_for_repo("other", "skills", "main", "skills/network-error/SKILL.md");
    let manifest = SkillportStateManifest {
        kind: EXPORT_KIND.to_string(),
        version: EXPORT_VERSION,
        exported_at: "2026-04-25T00:00:00Z".to_string(),
        exported_from: ExportedFrom {
            app: "SkillPort".to_string(),
        },
        github_sources: vec![],
        central_skills: vec![
            PortableCentralSkill {
                id: "bad-frontmatter".to_string(),
                name: "bad-frontmatter".to_string(),
                description: None,
                source: invalid_source.clone(),
                tags: Vec::new(),
            },
            PortableCentralSkill {
                id: "network-error".to_string(),
                name: "network-error".to_string(),
                description: None,
                source: repo_error_source.clone(),
                tags: Vec::new(),
            },
        ],
        unrestorable_skills: Vec::new(),
    };

    let mut catalog = HashMap::new();
    catalog.insert(
        repo_key(&invalid_source),
        RemoteCatalogEntry {
            valid_source_paths: HashSet::new(),
            invalid_candidates: HashMap::from([(
                "skills/bad-frontmatter".to_string(),
                RemoteCatalogInvalidCandidate {
                    reason: "invalid_frontmatter".to_string(),
                    detail: "Skill 'skills/bad-frontmatter' is missing valid frontmatter."
                        .to_string(),
                },
            )]),
            repo_error: None,
        },
    );
    catalog.insert(
        repo_key(&repo_error_source),
        RemoteCatalogEntry {
            valid_source_paths: HashSet::new(),
            invalid_candidates: HashMap::new(),
            repo_error: Some("GitHub rate limit was exceeded".to_string()),
        },
    );

    let preview = preview_skillport_state_import_impl(&pool, &manifest, Some(&catalog), None, None)
        .await
        .unwrap();

    let invalid = preview
        .skills
        .iter()
        .find(|skill| skill.id == "bad-frontmatter")
        .expect("invalid skill");
    assert_eq!(invalid.status, SkillPreviewStatus::Unrestorable);
    assert_eq!(invalid.reason.as_deref(), Some("invalid_frontmatter"));
    assert_eq!(
        invalid.detail.as_deref(),
        Some("Skill 'skills/bad-frontmatter' is missing valid frontmatter.")
    );

    let repo_failure = preview
        .skills
        .iter()
        .find(|skill| skill.id == "network-error")
        .expect("repo failure skill");
    assert_eq!(repo_failure.status, SkillPreviewStatus::Unrestorable);
    assert_eq!(repo_failure.reason.as_deref(), Some("repo_unavailable"));
    assert_eq!(
        repo_failure.detail.as_deref(),
        Some("GitHub rate limit was exceeded")
    );
    assert_eq!(preview.summary.unrestorable, 2);
}

#[tokio::test]
async fn build_import_groups_applies_skip_overwrite_and_rename() {
    let pool = setup_test_db().await;
    let mut manifest = manifest_with_skill("new-skill", "skills/new-skill/SKILL.md");
    manifest.central_skills.push(PortableCentralSkill {
        id: "renamed-skill".to_string(),
        name: "renamed-skill".to_string(),
        description: None,
        source: github_source("skills/renamed-skill/SKILL.md"),
        tags: Vec::new(),
    });
    manifest.central_skills.push(PortableCentralSkill {
        id: "skipped-skill".to_string(),
        name: "skipped-skill".to_string(),
        description: None,
        source: github_source("skills/skipped-skill/SKILL.md"),
        tags: Vec::new(),
    });

    let (groups, result) = build_import_groups(
        &pool,
        &manifest,
        vec![
            SkillportStateImportResolution {
                skill_id: "renamed-skill".to_string(),
                source_path: None,
                resolution: DuplicateResolution::Rename,
                renamed_skill_id: Some("renamed-skill-copy".to_string()),
            },
            SkillportStateImportResolution {
                skill_id: "skipped-skill".to_string(),
                source_path: None,
                resolution: DuplicateResolution::Skip,
                renamed_skill_id: None,
            },
        ],
    )
    .await
    .unwrap();

    assert_eq!(result.skipped_skills, vec!["skipped-skill"]);
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].selections.len(), 2);
    assert_eq!(
        groups[0].selections[0].resolution,
        DuplicateResolution::Overwrite
    );
    assert_eq!(
        groups[0].selections[1].resolution,
        DuplicateResolution::Rename
    );
}

#[tokio::test]
async fn build_import_groups_skips_exact_duplicate_entries() {
    let pool = setup_test_db().await;
    let mut manifest = manifest_with_skill("dup-skill", "skills/dup-skill/SKILL.md");
    manifest
        .central_skills
        .push(manifest.central_skills[0].clone());

    let (groups, result) = build_import_groups(&pool, &manifest, Vec::new())
        .await
        .unwrap();

    assert_eq!(result.skipped_skills, vec!["dup-skill"]);
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].selections.len(), 1);
    assert_eq!(groups[0].selections[0].source_path, "skills/dup-skill");
}

#[tokio::test]
async fn build_import_groups_requires_resolution_for_duplicate_id_with_different_source() {
    let pool = setup_test_db().await;
    let mut manifest = manifest_with_skill("dup-skill", "skills/dup-skill/SKILL.md");
    manifest.central_skills.push(PortableCentralSkill {
        id: "dup-skill".to_string(),
        name: "dup-skill-alt".to_string(),
        description: None,
        source: github_source("skills/dup-skill-alt/SKILL.md"),
        tags: Vec::new(),
    });

    let (groups, result) = build_import_groups(&pool, &manifest, Vec::new())
        .await
        .unwrap();

    assert_eq!(result.skipped_skills, vec!["dup-skill"]);
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].selections.len(), 1);
    assert_eq!(groups[0].selections[0].source_path, "skills/dup-skill");

    let (groups, result) = build_import_groups(
        &pool,
        &manifest,
        vec![SkillportStateImportResolution {
            skill_id: "dup-skill".to_string(),
            source_path: Some("skills/dup-skill-alt/SKILL.md".to_string()),
            resolution: DuplicateResolution::Rename,
            renamed_skill_id: Some("dup-skill-alt-copy".to_string()),
        }],
    )
    .await
    .unwrap();

    assert!(result.skipped_skills.is_empty());
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].selections.len(), 2);
    assert_eq!(
        groups[0].selections[1].renamed_skill_id.as_deref(),
        Some("dup-skill-alt-copy")
    );
}

#[tokio::test]
async fn import_cancelled_before_groups_returns_partial_cancelled_result() {
    let pool = setup_test_db().await;
    let manifest = manifest_with_skill("cancelled-skill", "skills/cancelled-skill/SKILL.md");
    let cancel = Arc::new(AtomicBool::new(true));

    let secrets = crate::secrets::MockSecretStore::default();
    let result =
        import_skillport_state_impl(&pool, &secrets, &manifest, Vec::new(), None, Some(&cancel))
            .await
            .unwrap();

    assert!(result.cancelled);
    assert_eq!(result.skipped_skills, vec!["cancelled-skill"]);
    assert!(result.failed_skills.is_empty());
}

#[tokio::test]
async fn restore_skill_tags_creates_and_assigns_tags() {
    let pool = setup_test_db().await;
    let skill = Skill {
        id: "tagged".to_string(),
        name: "tagged".to_string(),
        description: None,
        file_path: "/tmp/tagged/SKILL.md".to_string(),
        canonical_path: Some("/tmp/tagged".to_string()),
        is_central: true,
        source: None,
        content: None,
        scanned_at: "2026-04-25T00:00:00Z".to_string(),
        fs_created_at: None,
        fs_updated_at: None,
    };
    db::upsert_skill(&pool, &skill).await.unwrap();

    let count = restore_skill_tags(
        &pool,
        "tagged",
        &[PortableSkillTag {
            name: "Portable".to_string(),
            description: None,
            color: None,
        }],
    )
    .await
    .unwrap();
    let tags = db::get_skill_tags_for_skill(&pool, "tagged").await.unwrap();

    assert_eq!(count, 1);
    assert!(tags.iter().any(|tag| tag.name == "Portable"));
}
