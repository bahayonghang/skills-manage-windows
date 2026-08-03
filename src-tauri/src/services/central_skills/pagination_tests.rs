use std::cell::Cell;
use std::path::Path;
use std::time::Duration;

use sqlx::{QueryBuilder, Row, Sqlite};

use super::error::CentralSkillsError;
use super::query::{
    get_central_skills_page_impl, get_central_skills_page_reference_impl,
    get_central_skills_page_with_observer,
};
use super::types::CentralSkillsPageRequest;
use crate::db::{self, DbPool, Skill};
use crate::test_support::mem_pool;

pub(super) const LARGE_FIXTURE_SKILLS: usize = 5_000;

async fn seed_page_skill(pool: &DbPool, skill: &Skill) {
    db::upsert_skill(pool, skill)
        .await
        .expect("seed pagination skill");
}

fn page_skill(
    id: &str,
    name: &str,
    description: &str,
    scanned_at: &str,
    created_at: Option<&str>,
    updated_at: Option<&str>,
) -> Skill {
    Skill {
        id: id.to_string(),
        uid: format!("{id}-uid"),
        name: name.to_string(),
        description: Some(description.to_string()),
        file_path: format!("Z:/missing/{id}/SKILL.md"),
        canonical_path: Some(format!("Z:/missing/{id}")),
        is_central: true,
        source: Some("native".to_string()),
        content: None,
        scanned_at: scanned_at.to_string(),
        fs_created_at: created_at.map(str::to_string),
        fs_updated_at: updated_at.map(str::to_string),
    }
}

async fn seed_deterministic_pagination_fixture(pool: &DbPool) {
    let now = "2026-08-03T00:00:00Z";
    for (id, name) in [("repo-a", "Repository A"), ("repo-b", "Repository B")] {
        sqlx::query(
            "INSERT INTO skill_repositories
             (id, name, source_type, owner, repo, branch, url, pinned, is_unknown,
              created_at, updated_at, last_synced_at)
             VALUES (?, ?, 'github', 'fixture', ?, 'main', NULL, 0, 0, ?, ?, NULL)",
        )
        .bind(id)
        .bind(name)
        .bind(id)
        .bind(now)
        .bind(now)
        .execute(pool)
        .await
        .expect("seed pagination repository");
    }

    let skills = [
        page_skill(
            "case-upper",
            "Alpha",
            "UPPER NeedLE",
            "2026-01-05T00:00:00Z",
            None,
            None,
        ),
        page_skill(
            "case-lower",
            "alpha",
            "plain text",
            "2026-01-02T00:00:00Z",
            Some("2026-01-01T00:00:00Z"),
            Some("2026-01-02T01:00:00Z"),
        ),
        page_skill(
            "literal-percent",
            "100% Tool",
            "literal percent",
            "2026-01-03T00:00:00Z",
            Some("2026-01-03T00:00:00Z"),
            Some("2026-01-03T01:00:00Z"),
        ),
        page_skill(
            "literal-underscore",
            "under_score",
            "literal underscore",
            "2026-01-04T00:00:00Z",
            Some("2026-01-04T00:00:00Z"),
            Some("2026-01-04T01:00:00Z"),
        ),
        page_skill(
            "literal-backslash",
            "slash\\tool",
            "literal backslash",
            "2026-01-05T00:00:00Z",
            Some("2026-01-05T00:00:00Z"),
            Some("2026-01-05T01:00:00Z"),
        ),
        page_skill(
            "tie-a",
            "Tie",
            "stable tie",
            "2026-01-06T00:00:00Z",
            Some("2026-01-06T00:00:00Z"),
            Some("2026-01-06T01:00:00Z"),
        ),
        page_skill(
            "tie-b",
            "Tie",
            "stable tie",
            "2026-01-06T00:00:00Z",
            Some("2026-01-06T00:00:00Z"),
            Some("2026-01-06T01:00:00Z"),
        ),
        page_skill(
            "repo-b-no-tag",
            "Zulu",
            "no tags",
            "2026-01-07T00:00:00Z",
            Some("2026-01-07T00:00:00Z"),
            Some("2026-01-07T01:00:00Z"),
        ),
    ];
    for skill in &skills {
        seed_page_skill(pool, skill).await;
    }

    let ignored = Skill {
        is_central: false,
        ..page_skill(
            "not-central",
            "Needle outside Central",
            "ignored",
            now,
            None,
            None,
        )
    };
    seed_page_skill(pool, &ignored).await;

    for (skill_id, repository_id) in [
        ("case-lower", "repo-a"),
        ("literal-percent", "repo-a"),
        ("literal-underscore", "repo-b"),
        ("literal-backslash", db::LOCAL_UNKNOWN_REPOSITORY_ID),
        ("tie-a", "repo-a"),
        ("tie-b", "repo-a"),
        ("repo-b-no-tag", "repo-b"),
    ] {
        sqlx::query(
            "INSERT INTO skill_repository_members
             (skill_id, repository_id, source_path, added_at, updated_at)
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(skill_id)
        .bind(repository_id)
        .bind(format!("skills/{skill_id}"))
        .bind(now)
        .bind(now)
        .execute(pool)
        .await
        .expect("seed pagination repository assignment");
    }

    for (skill_id, tag_id) in [
        ("case-lower", "backend-development"),
        ("literal-percent", db::UNCATEGORIZED_TAG_ID),
        ("literal-underscore", "testing-quality"),
        ("literal-backslash", db::UNCATEGORIZED_TAG_ID),
        ("literal-backslash", "backend-development"),
        ("tie-a", "backend-development"),
        ("tie-b", "testing-quality"),
    ] {
        sqlx::query(
            "INSERT INTO skill_tag_links
             (skill_id, tag_id, confidence, reason, source, added_at)
             VALUES (?, ?, NULL, NULL, 'manual', ?)",
        )
        .bind(skill_id)
        .bind(tag_id)
        .bind(now)
        .execute(pool)
        .await
        .expect("seed pagination tag assignment");
    }

    sqlx::query(
        "INSERT INTO skill_installations
         (skill_id, agent_id, installed_path, link_type, symlink_target, created_at)
         VALUES ('case-lower', 'claude-code', 'C:/agents/claude/case-lower',
                 'symlink', 'C:/central/case-lower', ?)",
    )
    .bind(now)
    .execute(pool)
    .await
    .expect("seed pagination installation");
}

pub(super) fn ids(page: &super::types::CentralSkillsPage) -> Vec<&str> {
    page.items.iter().map(|skill| skill.id.as_str()).collect()
}

#[test]
fn production_page_route_cannot_use_unpaged_listing_or_filesystem_timestamp_authority() {
    let source = include_str!("query.rs");
    let page_start = source
        .find("pub async fn get_central_skills_page_impl")
        .expect("production page entrypoint");
    let reference_start = source
        .find("pub(super) async fn get_central_skills_page_reference_impl")
        .expect("test reference entrypoint");
    let production_page = &source[page_start..reference_start];

    assert!(!production_page.contains("get_central_skills_impl("));
    assert!(!production_page.contains("TimestampAuthority::Filesystem"));
    assert!(production_page.contains("TimestampAuthority::Persisted"));
}

async fn assert_reference_equivalent(pool: &DbPool, request: CentralSkillsPageRequest) {
    let reference = get_central_skills_page_reference_impl(pool, request.clone())
        .await
        .expect("reference page");
    let sql = get_central_skills_page_impl(pool, request)
        .await
        .expect("SQL page");
    assert_eq!(sql.total, reference.total);
    assert_eq!(ids(&sql), ids(&reference));
}

#[tokio::test]
async fn sql_page_matches_reference_across_filter_sort_and_pagination_matrix() {
    let pool = mem_pool().await;
    seed_deterministic_pagination_fixture(&pool).await;

    let requests = [
        CentralSkillsPageRequest::default(),
        CentralSkillsPageRequest {
            query: Some(" needle ".to_string()),
            ..Default::default()
        },
        CentralSkillsPageRequest {
            source: vec!["repo-a".to_string(), "repo-b".to_string()],
            ..Default::default()
        },
        CentralSkillsPageRequest {
            source: vec!["repo-a".to_string(), "unassigned".to_string()],
            ..Default::default()
        },
        CentralSkillsPageRequest {
            tags: vec![
                "backend-development".to_string(),
                "testing-quality".to_string(),
            ],
            ..Default::default()
        },
        CentralSkillsPageRequest {
            tags: vec![
                db::UNCATEGORIZED_TAG_ID.to_string(),
                "testing-quality".to_string(),
            ],
            ..Default::default()
        },
        CentralSkillsPageRequest {
            install_state: Some("installed".to_string()),
            ..Default::default()
        },
        CentralSkillsPageRequest {
            install_state: Some("notInstalled".to_string()),
            ..Default::default()
        },
        CentralSkillsPageRequest {
            query: Some("tool".to_string()),
            source: vec!["repo-a".to_string(), "repo-b".to_string()],
            tags: vec![
                db::UNCATEGORIZED_TAG_ID.to_string(),
                "testing-quality".to_string(),
            ],
            install_state: Some("unlinked".to_string()),
            sort: Some("updated_at:desc".to_string()),
            limit: Some(3),
            offset: Some(0),
        },
        CentralSkillsPageRequest {
            limit: Some(2),
            offset: Some(2),
            ..Default::default()
        },
        CentralSkillsPageRequest {
            limit: Some(0),
            offset: Some(-50),
            ..Default::default()
        },
        CentralSkillsPageRequest {
            limit: Some(900),
            offset: Some(50_000),
            ..Default::default()
        },
    ];
    for request in requests {
        assert_reference_equivalent(&pool, request).await;
    }

    for field in ["name", "createdAt", "created_at", "updatedAt", "updated_at"] {
        for direction in ["asc", "desc"] {
            assert_reference_equivalent(
                &pool,
                CentralSkillsPageRequest {
                    sort: Some(format!("{field}:{direction}")),
                    ..Default::default()
                },
            )
            .await;
        }
    }
    for sort in ["unknown:desc", "updatedAt:sideways", "missing-colon"] {
        assert_reference_equivalent(
            &pool,
            CentralSkillsPageRequest {
                sort: Some(sort.to_string()),
                ..Default::default()
            },
        )
        .await;
    }
}

#[tokio::test]
async fn query_uses_ascii_case_insensitive_literal_contains() {
    let pool = mem_pool().await;
    seed_deterministic_pagination_fixture(&pool).await;

    for (query, expected) in [
        ("needle", vec!["case-upper"]),
        ("%", vec!["literal-percent"]),
        ("_", vec!["literal-underscore"]),
        ("\\", vec!["literal-backslash"]),
    ] {
        let page = get_central_skills_page_impl(
            &pool,
            CentralSkillsPageRequest {
                query: Some(query.to_string()),
                ..Default::default()
            },
        )
        .await
        .expect("literal query page");
        assert_eq!(ids(&page), expected, "query {query:?}");
    }
}

#[tokio::test]
async fn source_tag_and_install_special_values_preserve_or_semantics() {
    let pool = mem_pool().await;
    seed_deterministic_pagination_fixture(&pool).await;

    let unassigned = get_central_skills_page_impl(
        &pool,
        CentralSkillsPageRequest {
            source: vec!["unassigned".to_string()],
            ..Default::default()
        },
    )
    .await
    .expect("unassigned page");
    assert_eq!(ids(&unassigned), vec!["case-upper", "literal-backslash"]);

    let uncategorized = get_central_skills_page_impl(
        &pool,
        CentralSkillsPageRequest {
            tags: vec![db::UNCATEGORIZED_TAG_ID.to_string()],
            ..Default::default()
        },
    )
    .await
    .expect("uncategorized page");
    assert_eq!(
        ids(&uncategorized),
        vec!["literal-percent", "case-upper", "repo-b-no-tag",]
    );

    for alias in ["linked", "installed"] {
        let page = get_central_skills_page_impl(
            &pool,
            CentralSkillsPageRequest {
                install_state: Some(alias.to_string()),
                ..Default::default()
            },
        )
        .await
        .expect("linked alias page");
        assert_eq!(ids(&page), vec!["case-lower"]);
    }
    for alias in ["unlinked", "not_installed", "notInstalled"] {
        let page = get_central_skills_page_impl(
            &pool,
            CentralSkillsPageRequest {
                install_state: Some(alias.to_string()),
                ..Default::default()
            },
        )
        .await
        .expect("unlinked alias page");
        assert_eq!(page.total, 7);
    }
}

#[tokio::test]
async fn shared_root_agents_make_every_central_skill_linked() {
    let pool = mem_pool().await;
    seed_deterministic_pagination_fixture(&pool).await;
    let shared = Path::new("C:/shared-central-skills");
    crate::test_support::set_agent_dir(&pool, "central", shared).await;
    crate::test_support::set_agent_dir(&pool, "cursor", shared).await;

    let linked = get_central_skills_page_impl(
        &pool,
        CentralSkillsPageRequest {
            install_state: Some("linked".to_string()),
            ..Default::default()
        },
    )
    .await
    .expect("shared-root linked page");
    assert_eq!(linked.total, 8);
    assert!(linked
        .items
        .iter()
        .all(|skill| skill.linked_agents.contains(&"cursor".to_string())));

    let unlinked = get_central_skills_page_impl(
        &pool,
        CentralSkillsPageRequest {
            install_state: Some("unlinked".to_string()),
            ..Default::default()
        },
    )
    .await
    .expect("shared-root unlinked page");
    assert_eq!(unlinked.total, 0);
    assert!(unlinked.items.is_empty());
}

#[tokio::test]
async fn page_uses_persisted_timestamp_fallback_and_stable_id_ties() {
    let pool = mem_pool().await;
    seed_deterministic_pagination_fixture(&pool).await;

    let legacy = get_central_skills_page_impl(
        &pool,
        CentralSkillsPageRequest {
            query: Some("case-upper".to_string()),
            sort: Some("createdAt:asc".to_string()),
            ..Default::default()
        },
    )
    .await
    .expect("legacy timestamp page");
    assert_eq!(legacy.items[0].created_at, "2026-01-05T00:00:00Z");
    assert_eq!(legacy.items[0].updated_at, "2026-01-05T00:00:00Z");

    for (sort, expected) in [
        ("updatedAt:asc", vec!["tie-a", "tie-b"]),
        ("updatedAt:desc", vec!["tie-b", "tie-a"]),
    ] {
        let page = get_central_skills_page_impl(
            &pool,
            CentralSkillsPageRequest {
                query: Some("stable tie".to_string()),
                sort: Some(sort.to_string()),
                ..Default::default()
            },
        )
        .await
        .expect("stable tie page");
        assert_eq!(ids(&page), expected);
    }
}

#[tokio::test]
async fn normalized_filters_are_deduplicated_and_bounded() {
    let pool = mem_pool().await;
    seed_deterministic_pagination_fixture(&pool).await;

    let duplicates = vec!["repo-a".to_string(); 101];
    let page = get_central_skills_page_impl(
        &pool,
        CentralSkillsPageRequest {
            source: duplicates,
            ..Default::default()
        },
    )
    .await
    .expect("deduplicated source page");
    assert_eq!(page.total, 4);

    for field in ["source", "tags"] {
        let values = (0..=100)
            .map(|index| format!("value-{index}"))
            .collect::<Vec<_>>();
        let request = if field == "source" {
            CentralSkillsPageRequest {
                source: values,
                ..Default::default()
            }
        } else {
            CentralSkillsPageRequest {
                tags: values,
                ..Default::default()
            }
        };
        let error = get_central_skills_page_impl(&pool, request)
            .await
            .expect_err("101 unique values must fail");
        assert!(matches!(
            error,
            CentralSkillsError::PageFilterValuesExceeded {
                field: error_field,
                limit: 100
            } if error_field == field
        ));
    }
}

pub(super) async fn seed_large_pagination_fixture(pool: &DbPool) {
    let mut transaction = pool.begin().await.expect("begin large fixture");
    let now = "2026-08-03T00:00:00Z";

    sqlx::query(
        "INSERT INTO skill_repositories
         (id, name, source_type, owner, repo, branch, url, pinned, is_unknown,
          created_at, updated_at, last_synced_at)
         VALUES ('benchmark-repo', 'Benchmark', 'github', 'fixture', 'skills', 'main',
                 'https://example.invalid/fixture/skills', 0, 0, ?, ?, NULL)",
    )
    .bind(now)
    .bind(now)
    .execute(&mut *transaction)
    .await
    .expect("seed benchmark repository");

    for start in (0..LARGE_FIXTURE_SKILLS).step_by(75) {
        let end = (start + 75).min(LARGE_FIXTURE_SKILLS);
        let mut builder = QueryBuilder::<Sqlite>::new(
            "INSERT INTO skills
             (id, uid, name, description, file_path, canonical_path, is_central, source,
              content, scanned_at, fs_created_at, fs_updated_at) ",
        );
        builder.push_values(start..end, |mut row, index| {
            let id = format!("benchmark-skill-{index:05}");
            let name = format!("Benchmark Skill {index:05}");
            let path = format!("C:/benchmark/{id}/SKILL.md");
            let directory = format!("C:/benchmark/{id}");
            let timestamp = format!("2026-07-{:02}T{:02}:00:00Z", index % 28 + 1, index % 24);
            row.push_bind(id.clone())
                .push_bind(format!("{id}-uid"))
                .push_bind(name.clone())
                .push_bind(format!("Description for {name}"))
                .push_bind(path)
                .push_bind(directory)
                .push_bind(true)
                .push_bind("native")
                .push_bind(Option::<String>::None)
                .push_bind(now)
                .push_bind(timestamp.clone())
                .push_bind(timestamp);
        });
        builder
            .build()
            .execute(&mut *transaction)
            .await
            .expect("seed benchmark skills");
    }

    for start in (0..LARGE_FIXTURE_SKILLS).step_by(150) {
        let end = (start + 150).min(LARGE_FIXTURE_SKILLS);
        let mut repository_builder = QueryBuilder::<Sqlite>::new(
            "INSERT INTO skill_repository_members
             (skill_id, repository_id, source_path, added_at, updated_at) ",
        );
        repository_builder.push_values(start..end, |mut row, index| {
            row.push_bind(format!("benchmark-skill-{index:05}"))
                .push_bind("benchmark-repo")
                .push_bind(format!("skills/{index:05}"))
                .push_bind(now)
                .push_bind(now);
        });
        repository_builder
            .build()
            .execute(&mut *transaction)
            .await
            .expect("seed benchmark repository members");

        for agent_id in ["claude-code", "cursor"] {
            let mut installation_builder = QueryBuilder::<Sqlite>::new(
                "INSERT INTO skill_installations
                 (skill_id, agent_id, installed_path, link_type, symlink_target, created_at) ",
            );
            installation_builder.push_values(start..end, |mut row, index| {
                let id = format!("benchmark-skill-{index:05}");
                row.push_bind(id.clone())
                    .push_bind(agent_id)
                    .push_bind(format!("C:/agents/{agent_id}/{id}"))
                    .push_bind("symlink")
                    .push_bind(format!("C:/benchmark/{id}"))
                    .push_bind(now);
            });
            installation_builder
                .build()
                .execute(&mut *transaction)
                .await
                .expect("seed benchmark installations");
        }

        for tag_id in ["backend-development", "testing-quality"] {
            let mut tag_builder = QueryBuilder::<Sqlite>::new(
                "INSERT INTO skill_tag_links
                 (skill_id, tag_id, confidence, reason, source, added_at) ",
            );
            tag_builder.push_values(start..end, |mut row, index| {
                row.push_bind(format!("benchmark-skill-{index:05}"))
                    .push_bind(tag_id)
                    .push_bind(Option::<f64>::None)
                    .push_bind(Option::<String>::None)
                    .push_bind("manual")
                    .push_bind(now);
            });
            tag_builder
                .build()
                .execute(&mut *transaction)
                .await
                .expect("seed benchmark tags");
        }
    }

    transaction.commit().await.expect("commit large fixture");
}

pub(super) fn percentile(
    samples: &mut [Duration],
    numerator: usize,
    denominator: usize,
) -> Duration {
    samples.sort_unstable();
    let index = samples
        .len()
        .saturating_mul(numerator)
        .div_ceil(denominator)
        .saturating_sub(1)
        .min(samples.len().saturating_sub(1));
    samples[index]
}

#[tokio::test]
async fn large_page_enriches_only_page_rows_and_batches_relation_queries() {
    let pool = mem_pool().await;
    seed_large_pagination_fixture(&pool).await;
    let observed_rows = Cell::new(usize::MAX);
    let page = get_central_skills_page_with_observer(
        &pool,
        CentralSkillsPageRequest {
            sort: Some("updatedAt:desc".to_string()),
            limit: Some(25),
            offset: Some(2_500),
            ..Default::default()
        },
        |rows| observed_rows.set(rows.len()),
    )
    .await
    .expect("large SQL page");
    assert_eq!(page.total, LARGE_FIXTURE_SKILLS);
    assert_eq!(page.items.len(), 25);
    assert_eq!(observed_rows.get(), 25);

    let first_501_ids = (0..501)
        .map(|index| format!("benchmark-skill-{index:05}"))
        .collect::<Vec<_>>();
    assert_eq!(
        db::get_skill_installations_for_skills(&pool, &first_501_ids)
            .await
            .expect("batched installations")
            .len(),
        501
    );
    assert_eq!(
        db::get_skill_repository_assignments_for_skills(&pool, &first_501_ids)
            .await
            .expect("batched repositories")
            .len(),
        501
    );
    assert_eq!(
        db::get_skill_tags_for_skills(&pool, &first_501_ids)
            .await
            .expect("batched tags")
            .len(),
        501
    );

    let empty_observed_rows = Cell::new(usize::MAX);
    let empty = get_central_skills_page_with_observer(
        &pool,
        CentralSkillsPageRequest {
            limit: Some(25),
            offset: Some(50_000),
            ..Default::default()
        },
        |rows| empty_observed_rows.set(rows.len()),
    )
    .await
    .expect("empty SQL page");
    assert_eq!(empty.total, LARGE_FIXTURE_SKILLS);
    assert!(empty.items.is_empty());
    assert_eq!(empty_observed_rows.get(), 0);
}

fn explain_filter(sort: db::CentralSkillPageSort) -> db::CentralSkillPageQuery {
    db::CentralSkillPageQuery {
        query: None,
        sources: Vec::new(),
        include_unassigned: false,
        tags: Vec::new(),
        include_uncategorized: false,
        install: db::CentralSkillInstallFilter::All,
        has_shared_root_agent: false,
        sort,
        descending: false,
        limit: 25,
        offset: 0,
    }
}

#[tokio::test]
async fn explain_query_plan_covers_pagination_query_shapes() {
    let pool = mem_pool().await;
    seed_deterministic_pagination_fixture(&pool).await;
    let legacy_details =
        sqlx::query("EXPLAIN QUERY PLAN SELECT * FROM skills WHERE is_central = 1")
            .fetch_all(&pool)
            .await
            .expect("explain legacy full load")
            .into_iter()
            .map(|row| {
                row.try_get::<String, _>("detail")
                    .expect("legacy plan detail")
            })
            .collect::<Vec<_>>();
    assert!(!legacy_details.is_empty());
    eprintln!(
        "central-pagination plan=legacy-full-load: {}",
        legacy_details.join(" | ")
    );

    let mut cases = Vec::new();
    cases.push(("name", explain_filter(db::CentralSkillPageSort::Name)));
    cases.push((
        "updated-time",
        explain_filter(db::CentralSkillPageSort::UpdatedAt),
    ));

    let mut source = explain_filter(db::CentralSkillPageSort::Name);
    source.sources.push("repo-a".to_string());
    source.include_unassigned = true;
    cases.push(("source", source));

    let mut tag = explain_filter(db::CentralSkillPageSort::Name);
    tag.tags.push("backend-development".to_string());
    tag.include_uncategorized = true;
    cases.push(("tag", tag));

    let mut install = explain_filter(db::CentralSkillPageSort::Name);
    install.install = db::CentralSkillInstallFilter::Linked;
    cases.push(("install", install));

    let mut contains = explain_filter(db::CentralSkillPageSort::Name);
    contains.query = Some("tool".to_string());
    cases.push(("contains", contains));

    for (label, filter) in cases {
        let details = db::explain_central_skills_page(&pool, &filter)
            .await
            .expect("explain page query");
        assert!(!details.is_empty(), "missing plan for {label}");
        assert!(
            details.iter().any(|detail| detail.contains("skills")),
            "plan for {label} must access skills: {details:?}"
        );
        eprintln!("central-pagination plan={label}: {}", details.join(" | "));
    }
}
