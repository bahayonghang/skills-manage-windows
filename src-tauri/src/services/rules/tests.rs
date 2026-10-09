use super::*;
use tempfile::TempDir;

fn fixture() -> (TempDir, RulesService) {
    let dir = tempfile::tempdir().unwrap();
    let service = RulesService::with_home(dir.path().to_path_buf(), [true, true]);
    (dir, service)
}
fn native(service: &RulesService, tool: RuleTool, name: &str, bytes: &[u8]) {
    let root = service.tool_root(tool);
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join(name), bytes).unwrap();
}
fn import_all(service: &RulesService) -> RulesImportResult {
    let preview = service.preview_import().unwrap();
    service
        .import(
            &preview
                .entries
                .into_iter()
                .map(|entry| RuleImportEntry {
                    name: entry.name,
                    source_fingerprints: entry.source_fingerprints,
                })
                .collect::<Vec<_>>(),
        )
        .unwrap()
}

#[test]
fn rejects_flat_name_escapes_reserved_names_and_case_duplicate() {
    let (_dir, service) = fixture();
    for name in [
        "../escape.md",
        "a/b.md",
        "a\\b.md",
        "RULES.md",
        "RULES@project.md",
        "CON.md",
        "lpt1.md",
        "COM¹.md",
        "COM².test.md",
        "LPT³.md",
        ".hidden.md",
        "x .md",
        "x..md",
        "a.txt",
    ] {
        assert!(validate_name(name).is_err(), "{name}");
    }
    service
        .create("中文 name.md", "description", "body")
        .unwrap();
    assert!(matches!(
        service.create("中文 NAME.md", "", "new"),
        Err(RulesError::TargetConflict)
    ));
}

#[test]
fn preserves_claude_body_bytes_and_both_originals_and_is_idempotent() {
    let (_dir, service) = fixture();
    let claude = b"# Sample\r\n\r\nbody\r\n";
    let omp = b"---\nalwaysApply: true\ndescription: 'Existing description'\n---\n# Sample\r\n\r\nbody\r\n";
    native(&service, RuleTool::ClaudeCode, "sample.md", claude);
    native(&service, RuleTool::Omp, "sample.md", omp);
    let first = import_all(&service);
    assert_eq!(first.entries[0].status, RuleImportStatus::Imported);
    let detail = service.read("sample.md").unwrap();
    assert_eq!(detail.body.as_bytes(), claude);
    assert_eq!(detail.description, "Existing description");
    for (tool, bytes) in [
        (RuleTool::ClaudeCode, claude.as_slice()),
        (RuleTool::Omp, omp.as_slice()),
    ] {
        assert_eq!(
            fs::read(service.backup_path_for_test(&first.operation_id, tool.id(), "sample.md"))
                .unwrap(),
            bytes
        );
        assert_eq!(
            fs::read(service.tool_root(tool).join("sample.md")).unwrap(),
            bytes
        );
    }
    assert_eq!(
        import_all(&service).entries[0].status,
        RuleImportStatus::Same
    );
    assert_eq!(service.list().unwrap().rules.len(), 1);
}

#[test]
fn import_conflicts_conditions_and_stale_preview_do_not_write() {
    let (_dir, service) = fixture();
    native(&service, RuleTool::ClaudeCode, "conflict.md", b"Claude");
    native(&service, RuleTool::Omp, "conflict.md", b"OMP");
    native(
        &service,
        RuleTool::ClaudeCode,
        "conditional.md",
        b"---\npaths: ['src/**']\n---\nconditional",
    );
    let preview = service.preview_import().unwrap();
    assert_eq!(preview.entries[0].status, RuleImportStatus::Unsupported);
    assert_eq!(preview.entries[1].status, RuleImportStatus::Conflict);
    assert!(import_all(&service)
        .entries
        .iter()
        .all(|e| e.status != RuleImportStatus::Imported));
    native(&service, RuleTool::ClaudeCode, "stale.md", b"old");
    let entry = service
        .preview_import()
        .unwrap()
        .entries
        .into_iter()
        .find(|e| e.name == "stale.md")
        .unwrap();
    native(&service, RuleTool::ClaudeCode, "stale.md", b"external edit");
    let result = service
        .import(&[RuleImportEntry {
            name: entry.name,
            source_fingerprints: entry.source_fingerprints,
        }])
        .unwrap();
    assert_eq!(
        result.entries[0].error_code.as_deref(),
        Some("rules.revision_conflict")
    );
    assert!(!service.root().join("stale.md").exists());
}

#[test]
fn real_file_links_share_updates_and_disable_preserves_library() {
    let (_dir, service) = fixture();
    let detail = service
        .create("中文 space.md", "Description", "# Title\nfirst\n")
        .unwrap();
    for tool in RuleTool::ALL {
        service.set_enabled(&detail.name, tool, true, None).unwrap();
        let path = service.tool_root(tool).join(&detail.name);
        assert!(fs::symlink_metadata(&path)
            .unwrap()
            .file_type()
            .is_symlink());
        assert!(!fs::read_link(&path).unwrap().is_absolute());
        assert!(points_to(&path, &service.root().join(&detail.name)));
    }
    let saved = service
        .save(&detail.name, "# Title\nupdated\n", &detail.revision)
        .unwrap();
    for tool in RuleTool::ALL {
        assert_eq!(
            fs::read(service.tool_root(tool).join(&detail.name)).unwrap(),
            saved.source.as_bytes()
        );
    }
    service
        .set_enabled(&detail.name, RuleTool::ClaudeCode, false, None)
        .unwrap();
    assert!(service.root().join(&detail.name).exists());
    assert!(service.tool_root(RuleTool::Omp).join(&detail.name).exists());
}

#[test]
fn same_native_file_requires_confirmed_fingerprint_and_external_links_are_preserved() {
    let (dir, service) = fixture();
    let detail = service.create("rule.md", "", "body").unwrap();
    native(&service, RuleTool::ClaudeCode, &detail.name, b"body");
    assert!(matches!(
        service.set_enabled(&detail.name, RuleTool::ClaudeCode, true, None),
        Err(RulesError::RevisionConflict)
    ));
    service
        .set_enabled(
            &detail.name,
            RuleTool::ClaudeCode,
            true,
            Some(&fingerprint(b"body")),
        )
        .unwrap();
    let other = dir.path().join("other.md");
    fs::write(&other, "body").unwrap();
    let root = service.tool_root(RuleTool::Omp);
    fs::create_dir_all(&root).unwrap();
    create_file_link(&other, &root.join(&detail.name)).unwrap();
    assert!(matches!(
        service.set_enabled(&detail.name, RuleTool::Omp, true, None),
        Err(RulesError::TargetConflict)
    ));
    assert!(matches!(
        service.set_enabled(&detail.name, RuleTool::Omp, false, None),
        Err(RulesError::TargetConflict)
    ));
    assert_eq!(fs::read_link(root.join(&detail.name)).unwrap(), other);
}

#[test]
fn failed_save_and_external_revision_preserve_contents() {
    let (_dir, mut service) = fixture();
    let detail = service.create("rule.md", "", "original").unwrap();
    service.fault = Some("save");
    assert!(service.save(&detail.name, "new", &detail.revision).is_err());
    assert_eq!(service.read(&detail.name).unwrap().source, detail.source);
    service.fault = None;
    fs::write(service.root().join(&detail.name), "external").unwrap();
    assert!(matches!(
        service.save(&detail.name, "new", &detail.revision),
        Err(RulesError::RevisionConflict)
    ));
    assert_eq!(
        fs::read(service.root().join(&detail.name)).unwrap(),
        b"external"
    );
}

#[test]
fn failed_takeover_can_restore_exact_original_and_blocks_external_changes() {
    let (_dir, mut service) = fixture();
    let original = b"body\r\n";
    let detail = service.create("rule.md", "", "body\r\n").unwrap();
    native(&service, RuleTool::ClaudeCode, &detail.name, original);
    service.fault = Some("link_verify");
    assert!(service
        .set_enabled(
            &detail.name,
            RuleTool::ClaudeCode,
            true,
            Some(&fingerprint(original))
        )
        .is_err());
    let pending = service.list().unwrap().recovery_operations.remove(0);
    assert_eq!(fs::read(pending.backup_path.unwrap()).unwrap(), original);
    service.fault = None;
    let central_path = service.root().join(&detail.name);
    fs::write(&central_path, "external").unwrap();
    assert!(matches!(
        service.recover(&pending.operation_id),
        Err(RulesError::RecoveryBlocked)
    ));
    fs::write(&central_path, detail.source).unwrap();
    service.recover(&pending.operation_id).unwrap();
    let destination = service.tool_root(RuleTool::ClaudeCode).join(&detail.name);
    assert!(!fs::symlink_metadata(&destination)
        .unwrap()
        .file_type()
        .is_symlink());
    assert_eq!(fs::read(destination).unwrap(), original);
}

#[test]
fn backup_receipt_and_commit_failures_preserve_original() {
    for phase in ["backup", "receipt", "link_commit"] {
        let (_dir, mut service) = fixture();
        service.create("rule.md", "", "body").unwrap();
        native(&service, RuleTool::ClaudeCode, "rule.md", b"body");
        service.fault = Some(phase);
        assert!(service
            .set_enabled(
                "rule.md",
                RuleTool::ClaudeCode,
                true,
                Some(&fingerprint(b"body"))
            )
            .is_err());
        assert_eq!(
            fs::read(service.tool_root(RuleTool::ClaudeCode).join("rule.md")).unwrap(),
            b"body"
        );
        assert!(
            !fs::symlink_metadata(service.tool_root(RuleTool::ClaudeCode).join("rule.md"))
                .unwrap()
                .file_type()
                .is_symlink()
        );
    }
}

#[test]
fn partial_delete_and_disable_recover_without_overwriting_external_entries() {
    let (_dir, mut service) = fixture();
    let detail = service.create("rule.md", "", "body").unwrap();
    for tool in RuleTool::ALL {
        service.set_enabled("rule.md", tool, true, None).unwrap();
    }
    service.fault = Some("delete_first_link");
    assert!(service.delete("rule.md", &detail.revision).is_err());
    assert!(service.root().join("rule.md").exists());
    let recovery = service.list().unwrap().recovery_operations.remove(0);
    native(&service, RuleTool::ClaudeCode, "rule.md", b"external");
    service.fault = None;
    assert!(matches!(
        service.recover(&recovery.operation_id),
        Err(RulesError::RecoveryBlocked)
    ));
    assert_eq!(
        fs::read(service.tool_root(RuleTool::ClaudeCode).join("rule.md")).unwrap(),
        b"external"
    );
    fs::remove_file(service.tool_root(RuleTool::ClaudeCode).join("rule.md")).unwrap();
    service.recover(&recovery.operation_id).unwrap();
    service.fault = Some("disable_complete");
    assert!(service
        .set_enabled("rule.md", RuleTool::Omp, false, None)
        .is_err());
    let recovery = service.list().unwrap().recovery_operations.remove(0);
    service.fault = None;
    service.recover(&recovery.operation_id).unwrap();
    assert!(points_to(
        &service.tool_root(RuleTool::Omp).join("rule.md"),
        &service.root().join("rule.md")
    ));
}

#[test]
fn committed_delete_retains_recoverable_central_bytes_and_native_files() {
    let (_dir, service) = fixture();
    let detail = service.create("rule.md", "", "body").unwrap();
    service
        .set_enabled("rule.md", RuleTool::ClaudeCode, true, None)
        .unwrap();
    native(&service, RuleTool::Omp, "rule.md", b"target-only body");
    let deleted = service.delete("rule.md", &detail.revision).unwrap();
    assert!(!service.root().join("rule.md").exists());
    assert_eq!(
        fs::read(service.tool_root(RuleTool::Omp).join("rule.md")).unwrap(),
        b"target-only body"
    );
    service.recover(&deleted.operation_id).unwrap();
    assert_eq!(service.read("rule.md").unwrap().source, detail.source);
}

#[test]
fn containment_rejects_central_file_and_parent_links_and_budget_is_bounded() {
    let (dir, service) = fixture();
    fs::create_dir_all(service.root()).unwrap();
    let outside = dir.path().join("outside.md");
    fs::write(&outside, "private").unwrap();
    create_file_link(&outside, &service.root().join("escape.md")).unwrap();
    assert!(matches!(
        service.read("escape.md"),
        Err(RulesError::TargetConflict)
    ));
    fs::remove_file(service.root().join("escape.md")).unwrap();
    native(
        &service,
        RuleTool::ClaudeCode,
        "large.md",
        &vec![b'x'; ResourceBudget::default_skill().file_bytes as usize + 1],
    );
    assert_eq!(
        service.preview_import().unwrap().entries[0]
            .error_code
            .as_deref(),
        Some("rules.budget_exceeded")
    );
    let state = service.root().join(".state");
    #[cfg(windows)]
    std::os::windows::fs::symlink_dir(dir.path(), &state).unwrap();
    #[cfg(not(windows))]
    std::os::unix::fs::symlink(dir.path(), &state).unwrap();
    assert!(matches!(service.list(), Err(RulesError::TargetConflict)));
}

#[test]
fn rejects_nonlocal_and_custom_configuration_without_touching_roots() {
    for target_id in ["ssh:a", "wsl:b", "", "../local"] {
        assert!(matches!(
            RulesService::ensure_local(target_id),
            Err(RulesError::LocalOnly)
        ));
    }
    assert!(RulesService::ensure_local("local").is_ok());
    assert!(!crate::paths::rule_tool_uses_default_root_with(
        false,
        |key| (key == "CLAUDE_CONFIG_DIR").then(|| "custom".into())
    ));
    for variable in [
        "PI_CODING_AGENT_DIR",
        "PI_CONFIG_DIR",
        "OMP_PROFILE",
        "PI_PROFILE",
    ] {
        assert!(!crate::paths::rule_tool_uses_default_root_with(
            true,
            |key| (key == variable).then(|| "custom".into())
        ));
    }
    let dir = tempfile::tempdir().unwrap();
    let service = RulesService::with_home(dir.path().to_path_buf(), [false, false]);
    assert!(matches!(
        service.set_enabled("rule.md", RuleTool::ClaudeCode, true, None),
        Err(RulesError::Unsupported)
    ));
    assert!(!service.root().exists());
}

#[test]
fn scans_skip_reserved_files_and_nested_rule_directories() {
    let (_dir, service) = fixture();
    service.create("managed.md", "", "body").unwrap();
    fs::write(service.root().join("RULES.md"), "sticky context").unwrap();
    fs::create_dir(service.root().join("nested.md")).unwrap();
    fs::write(service.root().join("nested.md").join("child.md"), "nested").unwrap();
    assert_eq!(service.list().unwrap().rules.len(), 1);
    native(
        &service,
        RuleTool::ClaudeCode,
        "RULES.md",
        b"sticky context",
    );
    fs::create_dir(service.tool_root(RuleTool::ClaudeCode).join("nested.md")).unwrap();
    assert!(service.preview_import().unwrap().entries.is_empty());
    assert_eq!(
        fs::read(service.root().join("RULES.md")).unwrap(),
        b"sticky context"
    );
}

#[test]
fn imports_from_supported_tool_when_other_tool_uses_custom_root() {
    let dir = tempfile::tempdir().unwrap();
    let service = RulesService::with_home(dir.path().to_path_buf(), [true, false]);
    native(&service, RuleTool::ClaudeCode, "rule.md", b"body\n");
    let result = import_all(&service);
    assert_eq!(result.entries[0].status, RuleImportStatus::Imported);
    assert_eq!(service.read("rule.md").unwrap().body, "body\n");
    assert!(!service.tool_root(RuleTool::Omp).exists());
    assert_eq!(
        service.read("rule.md").unwrap().targets[1].state,
        RuleTargetState::Unsupported
    );
}

#[test]
fn central_files_without_always_apply_metadata_are_read_only() {
    let (_dir, service) = fixture();
    let detail = service.create("rule.md", "", "body").unwrap();
    fs::write(service.root().join(&detail.name), b"body").unwrap();
    let external = service.read(&detail.name).unwrap();
    assert_eq!(external.compatibility, RuleCompatibility::Unsupported);
    assert!(matches!(
        service.save(&detail.name, "edited", &external.revision),
        Err(RulesError::Unsupported)
    ));
    assert!(matches!(
        service.set_enabled(&detail.name, RuleTool::Omp, true, None),
        Err(RulesError::Unsupported)
    ));
    assert!(!service.tool_root(RuleTool::Omp).exists());
}

#[test]
fn corrupt_receipt_phase_and_duplicate_tools_block_recovery() {
    for (phase, tools) in [
        ("unknown", vec![RuleTool::ClaudeCode]),
        ("prepared", vec![RuleTool::ClaudeCode, RuleTool::ClaudeCode]),
    ] {
        let (_dir, mut service) = fixture();
        service.create("rule.md", "", "body").unwrap();
        service.fault = Some("link_verify");
        assert!(service
            .set_enabled("rule.md", RuleTool::ClaudeCode, true, None)
            .is_err());
        service.fault = None;
        let operation = service.list().unwrap().recovery_operations.remove(0);
        let path = service
            .root()
            .join(".state")
            .join("operations")
            .join(format!("{}.json", operation.operation_id));
        let mut receipt: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        receipt["phase"] = serde_json::json!(phase);
        receipt["tools"] = serde_json::json!(tools);
        fs::write(&path, serde_json::to_vec(&receipt).unwrap()).unwrap();
        assert!(matches!(
            service.recover(&operation.operation_id),
            Err(RulesError::RecoveryBlocked)
        ));
        assert!(points_to(
            &service.tool_root(RuleTool::ClaudeCode).join("rule.md"),
            &service.root().join("rule.md")
        ));
    }
}

#[test]
fn import_atomic_commit_failure_keeps_original_and_verified_backup() {
    let (_dir, mut service) = fixture();
    native(&service, RuleTool::ClaudeCode, "rule.md", b"original\r\n");
    service.fault = Some("import_commit");
    let result = import_all(&service);
    assert_eq!(result.entries[0].status, RuleImportStatus::Failed);
    assert_eq!(
        fs::read(service.tool_root(RuleTool::ClaudeCode).join("rule.md")).unwrap(),
        b"original\r\n"
    );
    assert_eq!(
        fs::read(service.backup_path_for_test(&result.operation_id, "claude-code", "rule.md"))
            .unwrap(),
        b"original\r\n"
    );
    assert!(!service.root().join("rule.md").exists());
}

#[test]
fn unrelated_broken_link_has_no_ownership_and_is_retained() {
    let (dir, service) = fixture();
    service.create("rule.md", "", "body").unwrap();
    let tool = RuleTool::ClaudeCode;
    fs::create_dir_all(service.tool_root(tool)).unwrap();
    let destination = service.tool_root(tool).join("rule.md");
    let missing = dir.path().join("external-missing.md");
    create_file_link(&missing, &destination).unwrap();
    assert_eq!(
        service.read("rule.md").unwrap().targets[0].state,
        RuleTargetState::Broken
    );
    assert!(matches!(
        service.set_enabled("rule.md", tool, false, None),
        Err(RulesError::TargetConflict)
    ));
    assert_eq!(fs::read_link(destination).unwrap(), missing);
}

#[cfg(windows)]
#[test]
fn occupied_native_file_rejects_takeover_and_preserves_original() {
    use std::os::windows::fs::OpenOptionsExt;
    let (_dir, service) = fixture();
    service.create("rule.md", "", "body").unwrap();
    native(&service, RuleTool::ClaudeCode, "rule.md", b"body");
    let destination = service.tool_root(RuleTool::ClaudeCode).join("rule.md");
    let handle = fs::OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(&destination)
        .unwrap();
    assert!(service
        .set_enabled(
            "rule.md",
            RuleTool::ClaudeCode,
            true,
            Some(&fingerprint(b"body"))
        )
        .is_err());
    drop(handle);
    assert_eq!(fs::read(destination).unwrap(), b"body");
}

impl RulesService {
    fn backup_path_for_test(&self, id: &str, tool: &str, name: &str) -> PathBuf {
        self.root().join(".backups").join(id).join(tool).join(name)
    }
}
