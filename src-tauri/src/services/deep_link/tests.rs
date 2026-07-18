use super::{
    parse_import_deep_link, parse_import_intent_from_argv, parse_os_import_deep_link, ImportIntent,
    PendingImportIntentQueue,
};

fn encode_source(source: &str) -> String {
    urlencoding::encode(source).into_owned()
}

fn import_uri(source: &str) -> String {
    format!("skillport://import?source={}", encode_source(source))
}

#[test]
fn parser_accepts_and_normalizes_repo_branch_and_subpath_sources() {
    let cases = [
        (
            "https://github.com/Owner/Repo",
            "https://github.com/owner/repo",
        ),
        (
            "https://github.com/Owner/Repo/tree/Main/skills/demo",
            "https://github.com/owner/repo/tree/Main/skills/demo",
        ),
        (
            "https://www.github.com/Owner/Repo/skills/demo",
            "https://github.com/owner/repo/skills/demo",
        ),
    ];

    for (source, expected) in cases {
        let intent = parse_import_deep_link(&import_uri(source)).expect("valid import URI");
        assert_eq!(intent.source, expected, "source: {source}");
    }
}

#[test]
fn parser_rejects_noncanonical_actions_and_parameters() {
    let source = encode_source("https://github.com/owner/repo");
    let cases = [
        format!("skillport://share?source={source}"),
        format!("skillport://import/extra?source={source}"),
        format!("skillport://import?source={source}#fragment"),
        "skillport://import".to_string(),
        "skillport://import?source=".to_string(),
        format!("skillport://import?source={source}&source={source}"),
        format!("skillport://import?source={source}&target=cursor"),
        format!("skillport://import?source={source}&token=secret"),
        "skillport://import?source=https://github.com/owner/repo".to_string(),
    ];

    for uri in cases {
        assert!(parse_import_deep_link(&uri).is_err(), "accepted: {uri}");
    }
}

#[test]
fn parser_rejects_unsafe_or_non_github_sources() {
    let cases = [
        "http://github.com/owner/repo",
        "https://gitlab.com/owner/repo",
        "https://user@github.com/owner/repo",
        "https://user:password@github.com/owner/repo",
        "https://github.com:443/owner/repo",
        "https://github.com/owner/repo?token=secret",
        "https://github.com/owner/repo#readme",
        "file:///C:/Users/example/skill",
        "file://server/share/skill",
        "https://github.com/owner/repo\\escape",
        "https://github.com/owner/repo/%2e%2e/escape",
        "https://github.com/owner/repo/%252e%252e/escape",
        "https://github.com/owner/repo/%0Aescape",
    ];

    for source in cases {
        let uri = import_uri(source);
        assert!(parse_import_deep_link(&uri).is_err(), "accepted: {source}");
    }
}

#[test]
fn parser_enforces_utf8_byte_limit_and_redacts_errors() {
    let malicious = format!(
        "skillport://import?token=super-secret&source={}",
        encode_source("https://user:password@github.com/owner/repo")
    );
    let error = parse_import_deep_link(&malicious).expect_err("must reject credentials");
    let display = error.to_string();
    assert!(!display.contains("super-secret"));
    assert!(!display.contains("password"));
    assert!(!display.contains("owner/repo"));

    let oversized = format!("skillport://import?source={}", "a".repeat(4096));
    assert!(parse_import_deep_link(&oversized).is_err());

    let unicode_oversized = format!("skillport://import?source={}", "界".repeat(1400));
    assert!(unicode_oversized.len() > 4096);
    assert!(parse_import_deep_link(&unicode_oversized).is_err());
}

fn intent(repo: &str) -> ImportIntent {
    ImportIntent {
        source: format!("https://github.com/owner/{repo}"),
    }
}

#[test]
fn queue_is_fifo_deduplicated_and_ready_is_idempotent() {
    let mut queue = PendingImportIntentQueue::default();

    assert!(queue.enqueue(intent("one")).emit_now.is_none());
    assert!(queue.enqueue(intent("two")).emit_now.is_none());
    assert!(queue.enqueue(intent("one")).duplicate);

    assert_eq!(queue.mark_ready(), vec![intent("one"), intent("two")]);
    assert!(queue.mark_ready().is_empty());

    let outcome = queue.enqueue(intent("three"));
    assert_eq!(outcome.emit_now, Some(intent("three")));
    assert!(!outcome.duplicate);
}

#[test]
fn queue_capacity_is_eight_and_overflow_drops_oldest() {
    let mut queue = PendingImportIntentQueue::default();

    for index in 1..=8 {
        let outcome = queue.enqueue(intent(&index.to_string()));
        assert!(!outcome.dropped_oldest);
    }
    let overflow = queue.enqueue(intent("9"));
    assert!(overflow.dropped_oldest);

    let drained = queue.mark_ready();
    assert_eq!(drained.len(), 8);
    assert_eq!(drained.first(), Some(&intent("2")));
    assert_eq!(drained.last(), Some(&intent("9")));
}

#[test]
fn warm_instance_argv_accepts_exactly_one_canonical_import_uri() {
    let uri = import_uri("https://github.com/Owner/Repo/tree/Main/skills/demo");
    let argv = vec![
        "C:\\Program Files\\SkillPort\\skillport.exe".to_string(),
        uri,
    ];

    let parsed = parse_import_intent_from_argv(&argv).expect("valid warm instance argv");

    assert_eq!(
        parsed,
        ImportIntent {
            source: "https://github.com/owner/repo/tree/Main/skills/demo".to_string(),
        }
    );
}

#[test]
fn os_transport_normalizes_root_slash_before_using_the_canonical_parser() {
    let source = encode_source("https://github.com/Owner/Repo");
    let os_uri = format!("skillport://import/?source={source}");

    assert!(parse_import_deep_link(&os_uri).is_err());
    assert_eq!(
        parse_os_import_deep_link(&os_uri).expect("valid OS-normalized URI"),
        intent("repo")
    );
}

#[test]
fn warm_instance_argv_rejects_missing_extra_and_malformed_arguments_without_echoing_them() {
    let valid_uri = import_uri("https://github.com/owner/repo");
    let malicious_uri = "skillport://import?token=super-secret".to_string();
    let cases = [
        vec!["skillport.exe".to_string()],
        vec![
            "skillport.exe".to_string(),
            valid_uri,
            "--unexpected".to_string(),
        ],
        vec!["skillport.exe".to_string(), malicious_uri],
    ];

    for argv in cases {
        let error = parse_import_intent_from_argv(&argv).expect_err("must reject argv");
        let display = error.to_string();
        assert!(!display.contains("super-secret"));
        assert!(!display.contains("--unexpected"));
    }
}
