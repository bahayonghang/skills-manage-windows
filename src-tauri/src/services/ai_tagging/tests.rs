use super::{
    build_tagging_prompt, bulk_suggest_skill_tags_impl, map_ai_suggestions,
    parse_ai_tag_suggestions, resolve_ai_suggestions, AiTagProgressPayload, AiTagProgressStatus,
};
use crate::db::{
    self, DbPool, Skill, SkillTag, ACADEMIC_RESEARCH_WRITING_TAG_ID, UNCATEGORIZED_TAG_ID,
};
use crate::secrets::{MockSecretStore, AI_API_KEY_SECRET_KEY};
use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc, Mutex,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::time::{sleep, Duration};

fn tag(id: &str, name: &str) -> SkillTag {
    SkillTag {
        id: id.to_string(),
        name: name.to_string(),
        description: None,
        color: None,
        is_builtin: true,
        created_at: "2026-04-24T00:00:00Z".to_string(),
        updated_at: "2026-04-24T00:00:00Z".to_string(),
        group_id: None,
    }
}

fn custom_tag(id: &str, name: &str, description: &str) -> SkillTag {
    SkillTag {
        id: id.to_string(),
        name: name.to_string(),
        description: Some(description.to_string()),
        color: None,
        is_builtin: false,
        created_at: "2026-04-24T00:00:00Z".to_string(),
        updated_at: "2026-04-24T00:00:00Z".to_string(),
        group_id: None,
    }
}

fn make_skill(id: &str, name: &str) -> Skill {
    Skill {
        id: id.to_string(),
        uid: format!("{id}-uid"),
        name: name.to_string(),
        description: Some(format!("{name} description")),
        file_path: format!("/tmp/{id}/SKILL.md"),
        canonical_path: Some(format!("/tmp/{id}")),
        is_central: true,
        source: Some("test".to_string()),
        content: Some(format!("# {name}\nTest skill content")),
        scanned_at: "2026-04-24T00:00:00Z".to_string(),
        fs_created_at: None,
        fs_updated_at: None,
    }
}

use crate::test_support::mem_pool_single_conn as setup_test_db;

fn test_ai_secret() -> MockSecretStore {
    MockSecretStore::with_value(AI_API_KEY_SECRET_KEY, "test-key")
}

async fn configure_ai(pool: &DbPool, api_url: &str) {
    db::set_setting(pool, "ai_provider", "custom")
        .await
        .expect("provider");
    db::set_setting(pool, "ai_api_url__custom", api_url)
        .await
        .expect("api url");
    db::set_setting(pool, "ai_model__custom", "test-model")
        .await
        .expect("model");
    db::set_setting(pool, "ai_tag_concurrency", "4")
        .await
        .expect("tag concurrency");
    db::set_setting(pool, "ai_tag_interval_ms", "0")
        .await
        .expect("tag interval");
    db::set_setting(pool, "ai_tag_stop_on_rate_limit", "true")
        .await
        .expect("tag stop on rate limit");
}

async fn spawn_ai_server(
    response_text: &'static str,
    fail_first: bool,
) -> (String, Arc<AtomicUsize>, Arc<AtomicUsize>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let address = listener.local_addr().expect("addr");
    let current = Arc::new(AtomicUsize::new(0));
    let max_seen = Arc::new(AtomicUsize::new(0));
    let request_count = Arc::new(AtomicUsize::new(0));
    let current_for_task = Arc::clone(&current);
    let max_for_task = Arc::clone(&max_seen);
    let count_for_task = Arc::clone(&request_count);

    tokio::spawn(async move {
        loop {
            let Ok((mut socket, _)) = listener.accept().await else {
                break;
            };
            let current = Arc::clone(&current_for_task);
            let max_seen = Arc::clone(&max_for_task);
            let request_count = Arc::clone(&count_for_task);
            tokio::spawn(async move {
                let now = current.fetch_add(1, Ordering::SeqCst) + 1;
                max_seen.fetch_max(now, Ordering::SeqCst);

                let mut buffer = [0_u8; 4096];
                let _ = socket.read(&mut buffer).await;
                sleep(Duration::from_millis(80)).await;

                let index = request_count.fetch_add(1, Ordering::SeqCst);
                let (status, body) = if fail_first && index == 0 {
                    (
                        "500 Internal Server Error",
                        "{\"error\":\"boom\"}".to_string(),
                    )
                } else {
                    let escaped = response_text.replace('"', "\\\"");
                    (
                        "200 OK",
                        format!("{{\"content\":[{{\"text\":\"{}\"}}]}}", escaped),
                    )
                };
                let response = format!(
                        "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                        body.len()
                    );
                let _ = socket.write_all(response.as_bytes()).await;
                let _ = socket.shutdown().await;
                current.fetch_sub(1, Ordering::SeqCst);
            });
        }
    });

    (format!("http://{address}/v1/messages"), current, max_seen)
}

#[test]
fn parses_tag_json_envelope() {
    let parsed = parse_ai_tag_suggestions(
        r#"{"tags":[{"tag":"academic-research-writing","confidence":0.91,"reason":"研究写作"}]}"#,
    )
    .expect("parse");
    assert_eq!(parsed.tags.len(), 1);
    assert_eq!(parsed.tags[0].tag, ACADEMIC_RESEARCH_WRITING_TAG_ID);
    assert!(parsed.new_tag.is_none());
}

#[test]
fn parses_proposal_mixed_empty_and_legacy_responses() {
    let proposal = parse_ai_tag_suggestions(
        r#"{"tags":[],"new_tag":{"name":"安全审计","description":"Security audits.","confidence":0.9,"reason":"缺少分类"}}"#,
    )
    .expect("proposal");
    assert!(proposal.tags.is_empty());
    assert_eq!(proposal.new_tag.unwrap().name, "安全审计");

    let mixed = parse_ai_tag_suggestions(
        r#"{"tags":[{"tag":"frontend-development"}],"new_tag":{"name":"WebGL","description":"WebGL workflows."}}"#,
    )
    .expect("mixed");
    assert_eq!(mixed.tags.len(), 1);
    assert!(mixed.new_tag.is_some());

    let empty = parse_ai_tag_suggestions(r#"{"tags":[]}"#).expect("empty");
    assert!(empty.tags.is_empty());
    assert!(empty.new_tag.is_none());

    let legacy = parse_ai_tag_suggestions(
        r#"[{"tag":"backend-development","confidence":0.8,"reason":"后端"}]"#,
    )
    .expect("legacy");
    assert_eq!(legacy.tags[0].tag, "backend-development");
    assert!(legacy.new_tag.is_none());
}

#[test]
fn proposal_name_or_id_collision_downgrades_to_existing_suggestion() {
    let tags = vec![
        custom_tag("security-audit", "安全审计", "Security audits."),
        tag(UNCATEGORIZED_TAG_ID, "未分类"),
    ];
    let parsed = parse_ai_tag_suggestions(
        r#"{"tags":[],"new_tag":{"name":" 安全审计 ","description":"Duplicate.","confidence":0.92,"reason":"安全"}}"#,
    )
    .expect("parse");

    let resolved = resolve_ai_suggestions("skill-a", &tags, parsed).expect("resolve");
    assert!(resolved.proposals.is_empty());
    assert_eq!(resolved.suggestions.len(), 1);
    assert_eq!(resolved.suggestions[0].tag.id, "security-audit");
    assert_eq!(resolved.suggestions[0].confidence, 0.92);
}

#[test]
fn build_prompt_includes_all_classifiable_tags_and_excludes_uncategorized() {
    let tags = vec![
        tag(ACADEMIC_RESEARCH_WRITING_TAG_ID, "学术研究与写作"),
        custom_tag(
            "literature-review",
            "文献综述",
            "Systematic literature review workflows.",
        ),
        tag("frontend-development", "前端开发"),
        tag("backend-development", "后端开发"),
        tag(UNCATEGORIZED_TAG_ID, "未分类"),
    ];

    let prompt = build_tagging_prompt(
        "paper-helper",
        Some("Helps write related work"),
        "# paper-helper\nResearch notes",
        &tags,
    );

    assert!(prompt.contains("id: academic-research-writing"));
    assert!(prompt.contains("id: literature-review"));
    assert!(prompt.contains("kind: custom"));
    assert!(prompt.contains("Systematic literature review workflows."));
    assert!(prompt.contains("id: frontend-development"));
    assert!(prompt.contains("id: backend-development"));
    assert!(prompt.contains("只能输出候选列表中的 tag id"));
    assert!(prompt.contains("{\"tags\":[]}"));
    assert!(!prompt.contains("uncategorized"));
    assert!(!prompt.contains("未分类"));
}

#[test]
fn maps_unknown_ai_tags_to_uncategorized() {
    let tags = vec![
        tag(ACADEMIC_RESEARCH_WRITING_TAG_ID, "学术研究与写作"),
        tag(UNCATEGORIZED_TAG_ID, "未分类"),
    ];
    let parsed =
        parse_ai_tag_suggestions(r#"{"tags":[{"tag":"不存在","confidence":0.8,"reason":"测试"}]}"#)
            .expect("parse");
    let mapped = map_ai_suggestions("skill-a", &tags, parsed.tags).expect("map");
    assert_eq!(mapped[0].tag.id, UNCATEGORIZED_TAG_ID);
    assert_eq!(mapped[0].confidence, 0.2);
}

#[test]
fn maps_empty_ai_tags_to_uncategorized() {
    let tags = vec![
        tag(ACADEMIC_RESEARCH_WRITING_TAG_ID, "学术研究与写作"),
        tag(UNCATEGORIZED_TAG_ID, "未分类"),
    ];
    let parsed = parse_ai_tag_suggestions(r#"{"tags":[]}"#).expect("parse");
    let mapped = map_ai_suggestions("skill-a", &tags, parsed.tags).expect("map");
    assert_eq!(mapped[0].tag.id, UNCATEGORIZED_TAG_ID);
    assert_eq!(mapped[0].confidence, 0.2);
}

#[test]
fn ignores_uncategorized_returned_by_model_as_primary_tag() {
    let tags = vec![
        tag(ACADEMIC_RESEARCH_WRITING_TAG_ID, "学术研究与写作"),
        tag(UNCATEGORIZED_TAG_ID, "未分类"),
    ];
    let parsed = parse_ai_tag_suggestions(
        r#"{"tags":[{"tag":"uncategorized","confidence":0.95,"reason":"模型直返"}]}"#,
    )
    .expect("parse");
    let mapped = map_ai_suggestions("skill-a", &tags, parsed.tags).expect("map");
    assert_eq!(mapped[0].tag.id, UNCATEGORIZED_TAG_ID);
    assert_eq!(mapped[0].confidence, 0.2);
}

#[tokio::test]
async fn bulk_ai_tagging_emits_progress_limits_parallelism_and_continues_on_failure() {
    let pool = setup_test_db().await;
    let secrets = test_ai_secret();
    let response =
        r#"{"tags":[{"tag":"academic-research-writing","confidence":0.9,"reason":"研究写作"}]}"#;
    let (api_url, _current, max_seen) = spawn_ai_server(response, true).await;
    configure_ai(&pool, &api_url).await;

    for index in 0..6 {
        db::upsert_skill(
            &pool,
            &make_skill(&format!("skill-{index}"), &format!("Skill {index}")),
        )
        .await
        .expect("skill");
    }

    let events: Arc<Mutex<Vec<AiTagProgressPayload>>> = Arc::new(Mutex::new(Vec::new()));
    let events_for_emit = Arc::clone(&events);
    let results = bulk_suggest_skill_tags_impl(
        &pool,
        &secrets,
        (0..6).map(|index| format!("skill-{index}")).collect(),
        "job-test".to_string(),
        Arc::new(AtomicBool::new(false)),
        move |payload| {
            events_for_emit.lock().expect("events").push(payload);
        },
    )
    .await
    .expect("bulk");

    assert_eq!(results.len(), 6);
    assert!(results.iter().any(|result| !result.succeeded));
    assert!(results.iter().any(|result| result.succeeded));
    assert!(max_seen.load(Ordering::SeqCst) <= 4);
    assert!(max_seen.load(Ordering::SeqCst) > 1);

    {
        let captured = events.lock().expect("events");
        assert_eq!(
            captured.first().map(|event| event.status),
            Some(AiTagProgressStatus::Started)
        );
        assert_eq!(
            captured.last().map(|event| event.status),
            Some(AiTagProgressStatus::Completed)
        );
        assert!(captured
            .iter()
            .any(|event| event.status == AiTagProgressStatus::Running));
        assert!(captured
            .iter()
            .any(|event| event.status == AiTagProgressStatus::Failed));
    }

    let tags = db::get_skill_tags_for_skill(&pool, "skill-1")
        .await
        .expect("tags");
    assert!(tags
        .iter()
        .any(|tag| tag.id == ACADEMIC_RESEARCH_WRITING_TAG_ID));
}

#[tokio::test]
async fn proposal_is_persisted_for_review_without_tag_or_fallback_link() {
    let pool = setup_test_db().await;
    let secrets = test_ai_secret();
    let response = r#"{"tags":[],"new_tag":{"name":"Security","description":"Security auditing workflows.","confidence":0.95,"reason":"安全审计"}}"#;
    let (api_url, _current, _max_seen) = spawn_ai_server(response, false).await;
    configure_ai(&pool, &api_url).await;
    db::upsert_skill(&pool, &make_skill("proposal-skill", "Proposal Skill"))
        .await
        .expect("skill");

    let results = bulk_suggest_skill_tags_impl(
        &pool,
        &secrets,
        vec!["proposal-skill".to_string()],
        "job-proposal".to_string(),
        Arc::new(AtomicBool::new(false)),
        |_| {},
    )
    .await
    .expect("bulk");

    assert_eq!(results[0].proposals.len(), 1);
    assert_eq!(results[0].low_confidence_count, 1);
    assert!(db::get_skill_tag_by_name(&pool, "Security")
        .await
        .unwrap()
        .is_none());
    assert!(db::get_skill_tags_for_skill(&pool, "proposal-skill")
        .await
        .unwrap()
        .is_empty());
    let reviews = db::get_pending_ai_tag_reviews(&pool).await.unwrap();
    assert_eq!(reviews.len(), 1);
    assert!(reviews[0].is_proposal);
    assert_eq!(reviews[0].tag.name, "Security");
}

#[tokio::test]
async fn bulk_ai_tagging_requires_configuration_before_writing() {
    let pool = setup_test_db().await;
    let secrets = MockSecretStore::default();
    db::upsert_skill(&pool, &make_skill("skill-a", "Skill A"))
        .await
        .expect("skill");

    let result = bulk_suggest_skill_tags_impl(
        &pool,
        &secrets,
        vec!["skill-a".to_string()],
        "job-test".to_string(),
        Arc::new(AtomicBool::new(false)),
        |_| {},
    )
    .await;

    assert!(result
        .expect_err("missing setting")
        .to_string()
        .starts_with("ai.missing_api_key:"));
    let tags = db::get_skill_tags_for_skill(&pool, "skill-a")
        .await
        .expect("tags");
    assert!(tags.is_empty());
}

#[tokio::test]
async fn bulk_ai_tagging_can_be_cancelled_before_requests_start() {
    let pool = setup_test_db().await;
    let secrets = test_ai_secret();
    let response =
        r#"{"tags":[{"tag":"academic-research-writing","confidence":0.9,"reason":"研究写作"}]}"#;
    let (api_url, _current, _max_seen) = spawn_ai_server(response, false).await;
    configure_ai(&pool, &api_url).await;
    db::upsert_skill(&pool, &make_skill("skill-a", "Skill A"))
        .await
        .expect("skill");

    let cancel_flag = Arc::new(AtomicBool::new(true));
    let events: Arc<Mutex<Vec<AiTagProgressPayload>>> = Arc::new(Mutex::new(Vec::new()));
    let events_for_emit = Arc::clone(&events);
    let results = bulk_suggest_skill_tags_impl(
        &pool,
        &secrets,
        vec!["skill-a".to_string()],
        "job-cancel".to_string(),
        cancel_flag,
        move |payload| {
            events_for_emit.lock().expect("events").push(payload);
        },
    )
    .await
    .expect("bulk");

    assert_eq!(results.len(), 1);
    assert!(!results[0].succeeded);
    assert!(results[0]
        .error
        .as_deref()
        .unwrap_or_default()
        .contains("canceled"));
    assert_eq!(
        events
            .lock()
            .expect("events")
            .last()
            .map(|event| event.status),
        Some(AiTagProgressStatus::Cancelled)
    );
    let tags = db::get_skill_tags_for_skill(&pool, "skill-a")
        .await
        .expect("tags");
    assert!(tags.is_empty());
}
