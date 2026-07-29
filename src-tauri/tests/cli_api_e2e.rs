//! Public `skillport_lib::cli_api` integration contracts.

mod common;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use skillport_lib::cli_api::{CliApiError, CliContext};
use skillport_lib::db::DbPool;
use skillport_lib::secrets::{SecretError, SecretStorageState, SecretStore};

use common::{fresh_db, seed_central_skill};

struct DenySecretStore;

impl SecretStore for DenySecretStore {
    fn get(&self, _key: &str) -> Result<Option<String>, SecretError> {
        panic!("CLI integration contract unexpectedly read a secret")
    }

    fn set(&self, _key: &str, _value: &str) -> Result<SecretStorageState, SecretError> {
        panic!("CLI integration contract unexpectedly wrote a secret")
    }

    fn delete(&self, _key: &str) -> Result<(), SecretError> {
        panic!("CLI integration contract unexpectedly deleted a secret")
    }

    fn state(&self, _key: &str) -> Result<SecretStorageState, SecretError> {
        panic!("CLI integration contract unexpectedly inspected a secret")
    }
}

fn cli_context(pool: DbPool) -> CliContext {
    CliContext::new(pool, Arc::new(DenySecretStore))
}

async fn set_agent_dir(pool: &DbPool, agent_id: &str, path: &Path) {
    sqlx::query("UPDATE agents SET global_skills_dir = ? WHERE id = ?")
        .bind(path.to_string_lossy().as_ref())
        .bind(agent_id)
        .execute(pool)
        .await
        .unwrap();
}

fn assert_invalid<T>(result: Result<T, CliApiError>) {
    let error = match result {
        Ok(_) => panic!("expected invalid input"),
        Err(error) => error,
    };
    assert!(matches!(error, CliApiError::InvalidInput(_)));
    assert_eq!(error.code(), "input.invalid");
    assert_eq!(error.exit_code(), 2);
}

#[tokio::test]
async fn public_identity_flows_from_list_and_show_into_dry_run_sync() {
    let pool = fresh_db().await;
    let temp = tempfile::tempdir().unwrap();
    let agent_root = temp.path().join("codex-skills");
    set_agent_dir(&pool, "codex", &agent_root).await;
    let skill = seed_central_skill(&pool, &temp.path().join("central/demo"), "demo", "Demo").await;
    let context = cli_context(pool);

    let listed = context.list_skills().await.unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].uid, skill.uid);
    assert_eq!(listed[0].id, skill.id);

    let shown = context.show_skill(&skill.uid).await.unwrap();
    assert_eq!(shown.uid, skill.uid);
    assert_eq!(shown.id, skill.id);

    let output = context
        .sync_skills(
            vec![skill.uid],
            false,
            vec!["codex".to_string()],
            "copy",
            true,
        )
        .await
        .unwrap();
    assert!(output.dry_run);
    assert!(output.result.is_none());
    assert_eq!(output.plans.len(), 1);
    assert_eq!(output.plans[0].id, "demo");
    assert_eq!(output.plans[0].agent_id, "codex");
    assert_eq!(output.plans[0].method, "copy");
    assert_eq!(
        PathBuf::from(&output.plans[0].target_path),
        agent_root.join("demo")
    );
}

#[tokio::test]
async fn dry_run_sync_deduplicates_uid_and_id_references() {
    let pool = fresh_db().await;
    let temp = tempfile::tempdir().unwrap();
    set_agent_dir(&pool, "codex", &temp.path().join("codex-skills")).await;
    let skill = seed_central_skill(&pool, &temp.path().join("central/demo"), "demo", "Demo").await;
    let context = cli_context(pool);

    let output = context
        .sync_skills(
            vec![skill.uid, skill.id],
            false,
            vec!["codex".to_string()],
            "copy",
            true,
        )
        .await
        .unwrap();

    assert_eq!(output.plans.len(), 1);
}

#[tokio::test]
async fn ambiguous_name_preserves_the_public_error_contract() {
    let pool = fresh_db().await;
    let temp = tempfile::tempdir().unwrap();
    seed_central_skill(
        &pool,
        &temp.path().join("central/alpha"),
        "alpha",
        "Shared Name",
    )
    .await;
    seed_central_skill(
        &pool,
        &temp.path().join("central/beta"),
        "beta",
        "Shared Name",
    )
    .await;
    let context = cli_context(pool);

    let error = context.show_skill("Shared Name").await.unwrap_err();
    assert!(matches!(error, CliApiError::Ambiguous(_)));
    assert_eq!(error.code(), "skill.ambiguous");
    assert_eq!(error.exit_code(), 3);
}

#[tokio::test]
async fn invalid_sync_selections_fail_before_external_side_effects() {
    let context = cli_context(fresh_db().await);

    assert_invalid(
        context
            .sync_skills(vec!["demo".to_string()], true, vec![], "copy", true)
            .await,
    );
    assert_invalid(
        context
            .sync_skills(vec![], false, vec![], "copy", true)
            .await,
    );
    assert_invalid(
        context
            .sync_skills(vec![], true, vec![], "unsupported", true)
            .await,
    );
}
