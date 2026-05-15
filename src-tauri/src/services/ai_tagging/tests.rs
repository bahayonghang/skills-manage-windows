use super::{
    bulk_suggest_skill_tags_impl, map_ai_suggestions, parse_ai_tag_suggestions,
    AiTagProgressPayload, AiTagProgressStatus,
};
use crate::db::{self, DbPool, Skill, SkillTag, UNCATEGORIZED_TAG_ID};
use crate::secrets::{MockSecretStore, AI_API_KEY_SECRET_KEY};
use sqlx::sqlite::SqlitePoolOptions;
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

fn make_skill(id: &str, name: &str) -> Skill {
    Skill {
        id: id.to_string(),
        name: name.to_string(),
        description: Some(format!("{name} description")),
        file_path: format!("/tmp/{id}/SKILL.md"),
        canonical_path: Some(format!("/tmp/{id}")),
        is_central: true,
        source: Some("test".to_string()),
        content: Some(format!("# {name}\nTest skill content")),
        scanned_at: "2026-04-24T00:00:00Z".to_string(),
    }
}

async fn setup_test_db() -> DbPool {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect(":memory:")
        .await
        .expect("db");
    db::init_database(&pool).await.expect("init");
    pool
}
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
        r#"{"tags":[{"tag":"编程与 Agent 工程","confidence":0.91,"reason":"开发工具"}]}"#,
    )
    .expect("parse");
    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0].tag, "编程与 Agent 工程");
}

#[test]
fn maps_unknown_ai_tags_to_uncategorized() {
    let tags = vec![
        tag("programming-agent-engineering", "编程与 Agent 工程"),
        tag("uncategorized", "未分类"),
    ];
    let parsed =
        parse_ai_tag_suggestions(r#"{"tags":[{"tag":"不存在","confidence":0.8,"reason":"测试"}]}"#)
            .expect("parse");
    let mapped = map_ai_suggestions("skill-a", &tags, parsed).expect("map");
    assert_eq!(mapped[0].tag.id, "uncategorized");
}

#[tokio::test]
async fn bulk_ai_tagging_emits_progress_limits_parallelism_and_continues_on_failure() {
    let pool = setup_test_db().await;
    let secrets = test_ai_secret();
    let response = r#"{"tags":[{"tag":"编程与 Agent 工程","confidence":0.9,"reason":"开发工具"},{"tag":"未分类","confidence":0.4,"reason":"不确定"}]}"#;
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
        .any(|tag| tag.id == "programming-agent-engineering"));
    let reviews = db::get_pending_ai_tag_reviews(&pool)
        .await
        .expect("reviews");
    assert!(reviews
        .iter()
        .any(|review| review.tag.id == UNCATEGORIZED_TAG_ID));
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
    let response = r#"{"tags":[{"tag":"编程与 Agent 工程","confidence":0.9,"reason":"开发工具"}]}"#;
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
