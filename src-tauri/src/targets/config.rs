use std::collections::{HashMap, HashSet};

use chrono::Utc;
use serde_json::Value;
use sha2::{Digest, Sha256};

use super::*;

const SSH_FIELDS: &[&str] = &[
    "id",
    "label",
    "host",
    "username",
    "port",
    "authMethod",
    "keyPath",
    "credentialKey",
    "protectedPassword",
    "remoteHome",
    "remoteOs",
    "symlinkEnabled",
];
const WSL_FIELDS: &[&str] = &[
    "id",
    "label",
    "distribution",
    "remoteHome",
    "remoteOs",
    "symlinkEnabled",
];

#[derive(Debug)]
pub(super) struct TargetConfigSnapshot {
    pub(super) ssh_targets: Vec<RemoteTargetConfig>,
    pub(super) wsl_targets: Vec<WslTargetConfig>,
    pub(super) active_target_id: String,
    pub(super) quarantine_status: TargetConfigQuarantineStatus,
}

#[derive(Clone, Copy)]
enum QuarantineReason {
    InvalidJson,
    InvalidSchema,
    DuplicateId,
    ReservedId,
}

impl QuarantineReason {
    fn as_str(self) -> &'static str {
        match self {
            Self::InvalidJson => "invalid_json",
            Self::InvalidSchema => "invalid_schema",
            Self::DuplicateId => "duplicate_id",
            Self::ReservedId => "reserved_id",
        }
    }
}

pub(super) async fn load_target_config_snapshot(
    local_db: &DbPool,
) -> Result<TargetConfigSnapshot, TargetsError> {
    let keys = [
        TARGETS_SETTING_KEY.to_string(),
        WSL_TARGETS_SETTING_KEY.to_string(),
        ACTIVE_TARGET_SETTING_KEY.to_string(),
        TARGET_CONFIG_QUARANTINE_SETTING_KEY.to_string(),
    ];
    let values = db::get_settings(local_db, &keys).await?;
    let ssh_raw = setting_value(&values, TARGETS_SETTING_KEY);
    let wsl_raw = setting_value(&values, WSL_TARGETS_SETTING_KEY);
    let mut status =
        parse_quarantine_status(setting_value(&values, TARGET_CONFIG_QUARANTINE_SETTING_KEY));
    let mut updates = HashMap::new();
    let mut new_incidents = Vec::new();

    let ssh_targets = match parse_ssh_targets(ssh_raw) {
        Ok(targets) => targets,
        Err(reason) => {
            let incident = quarantine_incident(TargetConfigDomain::Ssh, reason, ssh_raw);
            upsert_incident(&mut status, incident.clone());
            updates.insert(TARGETS_SETTING_KEY.to_string(), "[]".to_string());
            new_incidents.push(incident);
            Vec::new()
        }
    };

    let wsl_targets = match parse_wsl_targets(wsl_raw) {
        Ok(targets) => targets,
        Err(reason) => {
            let incident = quarantine_incident(TargetConfigDomain::Wsl, reason, wsl_raw);
            upsert_incident(&mut status, incident.clone());
            updates.insert(WSL_TARGETS_SETTING_KEY.to_string(), "[]".to_string());
            new_incidents.push(incident);
            Vec::new()
        }
    };

    let configured_active_id = setting_value(&values, ACTIVE_TARGET_SETTING_KEY).trim();
    let mut active_target_id = if configured_active_id.is_empty() {
        LOCAL_TARGET_ID.to_string()
    } else {
        configured_active_id.to_string()
    };
    let active_is_valid = active_target_id == LOCAL_TARGET_ID
        || ssh_targets
            .iter()
            .any(|target| target.id == active_target_id)
        || wsl_targets
            .iter()
            .any(|target| target.id == active_target_id);
    let active_target_reset = !active_is_valid;
    if active_target_reset {
        active_target_id = LOCAL_TARGET_ID.to_string();
        status.active_target_reset = true;
        updates.insert(
            ACTIVE_TARGET_SETTING_KEY.to_string(),
            LOCAL_TARGET_ID.to_string(),
        );
    }

    if !new_incidents.is_empty() || active_target_reset {
        updates.insert(
            TARGET_CONFIG_QUARANTINE_SETTING_KEY.to_string(),
            serde_json::to_string(&status)?,
        );
    }

    if !updates.is_empty() {
        db::set_settings(local_db, &updates).await?;
        for incident in &new_incidents {
            tracing::warn!(
                domain = domain_name(incident.domain),
                reason_code = incident.reason_code,
                source_bytes = incident.source_bytes,
                source_sha256 = incident.source_sha256,
                "Quarantined invalid target configuration"
            );
        }
    }

    Ok(TargetConfigSnapshot {
        ssh_targets,
        wsl_targets,
        active_target_id,
        quarantine_status: status,
    })
}

pub(super) async fn persist_target_deletion_settings(
    local_db: &DbPool,
    ssh_targets: &[RemoteTargetConfig],
    wsl_targets: &[WslTargetConfig],
    reset_active: bool,
) -> Result<HashMap<String, Option<String>>, TargetsError> {
    let snapshot = db::get_settings(
        local_db,
        &[
            TARGETS_SETTING_KEY.to_string(),
            WSL_TARGETS_SETTING_KEY.to_string(),
            ACTIVE_TARGET_SETTING_KEY.to_string(),
        ],
    )
    .await?;
    let mut updates = HashMap::from([
        (
            TARGETS_SETTING_KEY.to_string(),
            serde_json::to_string(ssh_targets)?,
        ),
        (
            WSL_TARGETS_SETTING_KEY.to_string(),
            serde_json::to_string(wsl_targets)?,
        ),
    ]);
    if reset_active {
        updates.insert(
            ACTIVE_TARGET_SETTING_KEY.to_string(),
            LOCAL_TARGET_ID.to_string(),
        );
    }
    db::set_settings(local_db, &updates).await?;
    Ok(snapshot)
}

pub(super) async fn restore_target_settings(
    local_db: &DbPool,
    snapshot: &HashMap<String, Option<String>>,
) -> Result<(), sqlx::Error> {
    let mut transaction = local_db.begin().await?;
    for (key, value) in snapshot {
        match value {
            Some(value) => {
                sqlx::query("INSERT OR REPLACE INTO settings (key, value) VALUES (?, ?)")
                    .bind(key)
                    .bind(value)
                    .execute(&mut *transaction)
                    .await?;
            }
            None => {
                sqlx::query("DELETE FROM settings WHERE key = ?")
                    .bind(key)
                    .execute(&mut *transaction)
                    .await?;
            }
        }
    }
    transaction.commit().await
}

pub async fn get_target_config_quarantine_status_impl(
    local_db: &DbPool,
) -> Result<TargetConfigQuarantineStatus, TargetsError> {
    let raw = db::get_setting(local_db, TARGET_CONFIG_QUARANTINE_SETTING_KEY).await?;
    Ok(parse_quarantine_status(raw.as_deref().unwrap_or_default()))
}

pub async fn recover_target_config(
    local_db: &DbPool,
) -> Result<TargetConfigQuarantineStatus, TargetsError> {
    Ok(load_target_config_snapshot(local_db)
        .await?
        .quarantine_status)
}

fn setting_value<'a>(values: &'a HashMap<String, Option<String>>, key: &str) -> &'a str {
    values
        .get(key)
        .and_then(Option::as_deref)
        .unwrap_or_default()
}

fn parse_quarantine_status(raw: &str) -> TargetConfigQuarantineStatus {
    serde_json::from_str(raw)
        .ok()
        .filter(is_valid_quarantine_status)
        .unwrap_or_default()
}

fn is_valid_quarantine_status(status: &TargetConfigQuarantineStatus) -> bool {
    if status.version != 1 || status.incidents.len() > 2 {
        return false;
    }
    let mut domains = HashSet::new();
    status.incidents.iter().all(|incident| {
        domains.insert(incident.domain)
            && matches!(
                incident.reason_code.as_str(),
                "invalid_json" | "invalid_schema" | "duplicate_id" | "reserved_id"
            )
            && chrono::DateTime::parse_from_rfc3339(&incident.detected_at).is_ok()
            && incident.source_sha256.len() == 64
            && incident
                .source_sha256
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
    })
}

fn parse_ssh_targets(raw: &str) -> Result<Vec<RemoteTargetConfig>, QuarantineReason> {
    if raw.trim().is_empty() {
        return Ok(Vec::new());
    }
    let value: Value = serde_json::from_str(raw).map_err(|_| QuarantineReason::InvalidJson)?;
    validate_object_fields(&value, SSH_FIELDS)?;
    let targets: Vec<RemoteTargetConfig> =
        serde_json::from_value(value).map_err(|_| QuarantineReason::InvalidSchema)?;
    validate_target_ids(targets.iter().map(|target| target.id.as_str()))?;
    for target in &targets {
        if any_empty([
            target.label.as_str(),
            target.host.as_str(),
            target.username.as_str(),
            target.remote_home.as_str(),
            target.remote_os.as_str(),
        ]) || target.port == 0
            || (target.auth_method == SshAuthMethod::Key && target.key_path.trim().is_empty())
        {
            return Err(QuarantineReason::InvalidSchema);
        }
    }
    Ok(targets)
}

fn parse_wsl_targets(raw: &str) -> Result<Vec<WslTargetConfig>, QuarantineReason> {
    if raw.trim().is_empty() {
        return Ok(Vec::new());
    }
    let value: Value = serde_json::from_str(raw).map_err(|_| QuarantineReason::InvalidJson)?;
    validate_object_fields(&value, WSL_FIELDS)?;
    let targets: Vec<WslTargetConfig> =
        serde_json::from_value(value).map_err(|_| QuarantineReason::InvalidSchema)?;
    validate_target_ids(targets.iter().map(|target| target.id.as_str()))?;
    for target in &targets {
        if any_empty([
            target.label.as_str(),
            target.distribution.as_str(),
            target.remote_home.as_str(),
            target.remote_os.as_str(),
        ]) {
            return Err(QuarantineReason::InvalidSchema);
        }
    }
    Ok(targets)
}

fn validate_object_fields(value: &Value, allowed: &[&str]) -> Result<(), QuarantineReason> {
    let items = value.as_array().ok_or(QuarantineReason::InvalidSchema)?;
    for item in items {
        let object = item.as_object().ok_or(QuarantineReason::InvalidSchema)?;
        if object.keys().any(|key| !allowed.contains(&key.as_str())) {
            return Err(QuarantineReason::InvalidSchema);
        }
    }
    Ok(())
}

fn validate_target_ids<'a>(ids: impl Iterator<Item = &'a str>) -> Result<(), QuarantineReason> {
    let mut seen = HashSet::new();
    for id in ids {
        let id = id.trim();
        if id == LOCAL_TARGET_ID {
            return Err(QuarantineReason::ReservedId);
        }
        if !is_allowed_target_id_token(id) {
            return Err(QuarantineReason::InvalidSchema);
        }
        if !seen.insert(id) {
            return Err(QuarantineReason::DuplicateId);
        }
    }
    Ok(())
}

fn any_empty<const N: usize>(values: [&str; N]) -> bool {
    values.iter().any(|value| value.trim().is_empty())
}

fn quarantine_incident(
    domain: TargetConfigDomain,
    reason: QuarantineReason,
    raw: &str,
) -> TargetConfigQuarantineIncident {
    TargetConfigQuarantineIncident {
        domain,
        detected_at: Utc::now().to_rfc3339(),
        reason_code: reason.as_str().to_string(),
        source_bytes: raw.len() as u64,
        source_sha256: crate::hashing::encode_lower_hex(Sha256::digest(raw.as_bytes()).as_ref()),
    }
}

fn upsert_incident(
    status: &mut TargetConfigQuarantineStatus,
    incident: TargetConfigQuarantineIncident,
) {
    if let Some(existing) = status
        .incidents
        .iter_mut()
        .find(|existing| existing.domain == incident.domain)
    {
        if existing.source_sha256 != incident.source_sha256 {
            *existing = incident;
        }
    } else {
        status.incidents.push(incident);
        status.incidents.sort_by_key(|item| match item.domain {
            TargetConfigDomain::Ssh => 0,
            TargetConfigDomain::Wsl => 1,
        });
    }
}

fn domain_name(domain: TargetConfigDomain) -> &'static str {
    match domain {
        TargetConfigDomain::Ssh => "ssh",
        TargetConfigDomain::Wsl => "wsl",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::mem_pool as setup_test_db;

    fn valid_ssh(id: &str) -> Value {
        serde_json::json!({
            "id": id,
            "label": "Lab",
            "host": "lab.example",
            "username": "alice",
            "port": 22,
            "authMethod": "key",
            "keyPath": "C:/Users/alice/.ssh/id_ed25519",
            "credentialKey": null,
            "remoteHome": "/home/alice",
            "remoteOs": "linux",
            "symlinkEnabled": false
        })
    }

    fn valid_wsl(id: &str) -> Value {
        serde_json::json!({
            "id": id,
            "label": "Ubuntu",
            "distribution": "Ubuntu-24.04",
            "remoteHome": "/home/alice",
            "remoteOs": "linux",
            "symlinkEnabled": true
        })
    }

    #[tokio::test]
    async fn corrupt_ssh_is_quarantined_without_touching_healthy_wsl() {
        let pool = setup_test_db().await;
        let secret_raw = r#"[{"id":"ssh-secret","password":"plaintext-secret","protectedPassword":"dpapi-secret","host":"private-host"}]"#;
        let healthy_wsl = serde_json::to_string(&vec![valid_wsl("wsl-good")]).unwrap();
        db::set_setting(&pool, TARGETS_SETTING_KEY, secret_raw)
            .await
            .unwrap();
        db::set_setting(&pool, WSL_TARGETS_SETTING_KEY, &healthy_wsl)
            .await
            .unwrap();
        db::set_setting(&pool, ACTIVE_TARGET_SETTING_KEY, "ssh-secret")
            .await
            .unwrap();

        let snapshot = load_target_config_snapshot(&pool).await.unwrap();

        assert!(snapshot.ssh_targets.is_empty());
        assert_eq!(snapshot.wsl_targets.len(), 1);
        assert_eq!(snapshot.active_target_id, LOCAL_TARGET_ID);
        assert_eq!(
            db::get_setting(&pool, WSL_TARGETS_SETTING_KEY)
                .await
                .unwrap()
                .as_deref(),
            Some(healthy_wsl.as_str())
        );
        assert_eq!(
            db::get_setting(&pool, TARGETS_SETTING_KEY)
                .await
                .unwrap()
                .as_deref(),
            Some("[]")
        );
        let metadata = serde_json::to_string(&snapshot.quarantine_status).unwrap();
        for secret in [
            "plaintext-secret",
            "dpapi-secret",
            "private-host",
            "protectedPassword",
        ] {
            assert!(!metadata.contains(secret));
        }
        assert!(snapshot.quarantine_status.active_target_reset);
        assert_eq!(
            snapshot.quarantine_status.incidents[0].domain,
            TargetConfigDomain::Ssh
        );
        assert_eq!(
            snapshot.quarantine_status.incidents[0].source_bytes,
            secret_raw.len() as u64
        );
        assert_eq!(
            snapshot.quarantine_status.incidents[0].source_sha256.len(),
            64
        );

        let listed = TargetRegistry::default().list_targets(&pool).await.unwrap();
        assert_eq!(
            listed
                .iter()
                .map(|target| target.id.as_str())
                .collect::<Vec<_>>(),
            vec![LOCAL_TARGET_ID, "wsl-good"]
        );
    }

    #[tokio::test]
    async fn corrupt_wsl_is_quarantined_without_touching_healthy_ssh() {
        let pool = setup_test_db().await;
        let healthy_ssh = serde_json::to_string(&vec![valid_ssh("ssh-good")]).unwrap();
        db::set_setting(&pool, TARGETS_SETTING_KEY, &healthy_ssh)
            .await
            .unwrap();
        db::set_setting(&pool, WSL_TARGETS_SETTING_KEY, "not-json")
            .await
            .unwrap();
        db::set_setting(&pool, ACTIVE_TARGET_SETTING_KEY, "ssh-good")
            .await
            .unwrap();

        let snapshot = load_target_config_snapshot(&pool).await.unwrap();

        assert_eq!(snapshot.ssh_targets.len(), 1);
        assert!(snapshot.wsl_targets.is_empty());
        assert_eq!(snapshot.active_target_id, "ssh-good");
        assert_eq!(
            db::get_setting(&pool, TARGETS_SETTING_KEY)
                .await
                .unwrap()
                .as_deref(),
            Some(healthy_ssh.as_str())
        );
        assert_eq!(
            snapshot.quarantine_status.incidents[0].domain,
            TargetConfigDomain::Wsl
        );
        assert_eq!(
            snapshot.quarantine_status.incidents[0].reason_code,
            "invalid_json"
        );
    }

    #[tokio::test]
    async fn duplicate_digest_preserves_the_original_incident_timestamp() {
        let pool = setup_test_db().await;
        let corrupt = "{same-corrupt-config";
        db::set_setting(&pool, TARGETS_SETTING_KEY, corrupt)
            .await
            .unwrap();
        let first = load_target_config_snapshot(&pool).await.unwrap();
        let detected_at = first.quarantine_status.incidents[0].detected_at.clone();

        db::set_setting(&pool, TARGETS_SETTING_KEY, corrupt)
            .await
            .unwrap();
        let second = load_target_config_snapshot(&pool).await.unwrap();

        assert_eq!(second.quarantine_status.incidents.len(), 1);
        assert_eq!(
            second.quarantine_status.incidents[0].detected_at,
            detected_at
        );
    }

    #[tokio::test]
    async fn quarantine_transaction_failure_keeps_all_original_settings() {
        let pool = setup_test_db().await;
        let corrupt = "{cannot-commit";
        db::set_setting(&pool, TARGETS_SETTING_KEY, corrupt)
            .await
            .unwrap();
        db::set_setting(&pool, ACTIVE_TARGET_SETTING_KEY, "ssh-missing")
            .await
            .unwrap();
        sqlx::query(&format!(
            "CREATE TRIGGER reject_target_quarantine BEFORE INSERT ON settings \
             WHEN NEW.key = '{TARGETS_SETTING_KEY}' AND NEW.value = '[]' \
             BEGIN SELECT RAISE(ABORT, 'test transaction failure'); END"
        ))
        .execute(&pool)
        .await
        .unwrap();

        assert!(load_target_config_snapshot(&pool).await.is_err());
        assert_eq!(
            db::get_setting(&pool, TARGETS_SETTING_KEY)
                .await
                .unwrap()
                .as_deref(),
            Some(corrupt)
        );
        assert_eq!(
            db::get_setting(&pool, ACTIVE_TARGET_SETTING_KEY)
                .await
                .unwrap()
                .as_deref(),
            Some("ssh-missing")
        );
        assert_eq!(
            db::get_setting(&pool, TARGET_CONFIG_QUARANTINE_SETTING_KEY)
                .await
                .unwrap(),
            None
        );
    }

    #[tokio::test]
    async fn quarantine_status_survives_subsequent_loads() {
        let pool = setup_test_db().await;
        db::set_setting(&pool, WSL_TARGETS_SETTING_KEY, "[")
            .await
            .unwrap();

        load_target_config_snapshot(&pool).await.unwrap();
        let first = get_target_config_quarantine_status_impl(&pool)
            .await
            .unwrap();
        let second = get_target_config_quarantine_status_impl(&pool)
            .await
            .unwrap();

        assert_eq!(first, second);
        assert_eq!(second.incidents.len(), 1);
        assert_eq!(second.incidents[0].domain, TargetConfigDomain::Wsl);
    }

    #[tokio::test]
    async fn duplicate_and_reserved_ids_use_stable_reason_codes() {
        let pool = setup_test_db().await;
        let duplicate = serde_json::to_string(&vec![valid_wsl("same"), valid_wsl("same")]).unwrap();
        db::set_setting(&pool, WSL_TARGETS_SETTING_KEY, &duplicate)
            .await
            .unwrap();
        load_target_config_snapshot(&pool).await.unwrap();
        let duplicate_status = get_target_config_quarantine_status_impl(&pool)
            .await
            .unwrap();
        assert_eq!(duplicate_status.incidents[0].reason_code, "duplicate_id");

        let reserved = serde_json::to_string(&vec![valid_wsl(LOCAL_TARGET_ID)]).unwrap();
        db::set_setting(&pool, WSL_TARGETS_SETTING_KEY, &reserved)
            .await
            .unwrap();
        load_target_config_snapshot(&pool).await.unwrap();
        let reserved_status = get_target_config_quarantine_status_impl(&pool)
            .await
            .unwrap();
        assert_eq!(reserved_status.incidents[0].reason_code, "reserved_id");
    }

    #[tokio::test]
    async fn legacy_protected_password_remains_loadable() {
        let pool = setup_test_db().await;
        let mut target = valid_ssh("ssh-password");
        let object = target.as_object_mut().unwrap();
        object.insert(
            "authMethod".to_string(),
            Value::String("password".to_string()),
        );
        object.insert("keyPath".to_string(), Value::String(String::new()));
        object.insert(
            "credentialKey".to_string(),
            Value::String("legacy-credential".to_string()),
        );
        object.insert(
            "protectedPassword".to_string(),
            Value::String("legacy-dpapi-payload".to_string()),
        );
        db::set_setting(
            &pool,
            TARGETS_SETTING_KEY,
            &serde_json::to_string(&vec![target]).unwrap(),
        )
        .await
        .unwrap();

        let snapshot = load_target_config_snapshot(&pool).await.unwrap();

        assert_eq!(snapshot.ssh_targets.len(), 1);
        assert_eq!(
            snapshot.ssh_targets[0].protected_password.as_deref(),
            Some("legacy-dpapi-payload")
        );
        assert!(snapshot.quarantine_status.incidents.is_empty());
    }

    #[tokio::test]
    async fn untrusted_quarantine_metadata_is_not_returned_over_ipc() {
        let pool = setup_test_db().await;
        let malicious = serde_json::json!({
            "version": 1,
            "incidents": [{
                "domain": "ssh",
                "detectedAt": "plaintext-secret",
                "reasonCode": "parser said protectedPassword=secret",
                "sourceBytes": 12,
                "sourceSha256": "not-a-digest"
            }],
            "activeTargetReset": true
        });
        db::set_setting(
            &pool,
            TARGET_CONFIG_QUARANTINE_SETTING_KEY,
            &malicious.to_string(),
        )
        .await
        .unwrap();

        let status = get_target_config_quarantine_status_impl(&pool)
            .await
            .unwrap();
        let serialized = serde_json::to_string(&status).unwrap();

        assert_eq!(status, TargetConfigQuarantineStatus::default());
        assert!(!serialized.contains("plaintext-secret"));
        assert!(!serialized.contains("protectedPassword"));
    }

    fn target_id_parity_cases() -> [(&'static str, bool); 6] {
        [
            ("../escape", false),
            ("a/b", false),
            ("a\\b", false),
            (" ", false),
            ("ssh-demo\n1", false),
            ("ssh-demo_1", true),
        ]
    }

    #[test]
    fn validate_target_ids_matches_sanitize_target_id_matrix() {
        for (id, accepted) in target_id_parity_cases() {
            assert_eq!(
                validate_target_ids(std::iter::once(id)).is_ok(),
                accepted,
                "validate_target_ids({id:?})"
            );
            assert_eq!(
                super::super::sanitize_target_id(id).is_ok(),
                accepted,
                "sanitize_target_id({id:?})"
            );
        }
    }

    #[tokio::test]
    async fn hostile_target_ids_are_quarantined_for_ssh_and_wsl() {
        for id in ["../escape", "a/b", "a\\b"] {
            let pool = setup_test_db().await;
            let ssh = serde_json::to_string(&vec![valid_ssh(id)]).unwrap();
            db::set_setting(&pool, TARGETS_SETTING_KEY, &ssh)
                .await
                .unwrap();
            let snapshot = load_target_config_snapshot(&pool).await.unwrap();
            assert!(
                snapshot.ssh_targets.is_empty(),
                "ssh id {id:?} must be quarantined before cache paths"
            );
            assert_eq!(
                snapshot.quarantine_status.incidents[0].reason_code,
                "invalid_schema"
            );

            let pool = setup_test_db().await;
            let wsl = serde_json::to_string(&vec![valid_wsl(id)]).unwrap();
            db::set_setting(&pool, WSL_TARGETS_SETTING_KEY, &wsl)
                .await
                .unwrap();
            let snapshot = load_target_config_snapshot(&pool).await.unwrap();
            assert!(
                snapshot.wsl_targets.is_empty(),
                "wsl id {id:?} must be quarantined before cache paths"
            );
            assert_eq!(
                snapshot.quarantine_status.incidents[0].reason_code,
                "invalid_schema"
            );
        }
    }
}
