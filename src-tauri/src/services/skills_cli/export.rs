//! Atomic Skills CLI inventory export writer.
//!
//! The renderer supplies a path and a serialized v1 snapshot. This module owns
//! schema validation and same-directory temp + flush/sync + persist.

use std::collections::BTreeSet;
use std::io::Write;
use std::path::{Path, PathBuf};

use tempfile::Builder;

use crate::fs_util::run_blocking_fs_with;

use super::SkillsCliError;

const EXPORT_LIMIT: usize = 1_048_576;
const TEMP_PREFIX: &str = ".skillport-skills-cli-export-";
const SKILL_FIELDS: [&str; 10] = [
    "name",
    "source",
    "sourceType",
    "sourceUrl",
    "installKind",
    "canonicalPath",
    "folderHash",
    "installedAt",
    "updatedAt",
    "placements",
];
const PLACEMENT_FIELDS: [&str; 3] = ["agentId", "displayName", "state"];
const ENVELOPE_FIELDS: [&str; 4] = ["schemaVersion", "scope", "skillCount", "skills"];
const INSTALL_KINDS: [&str; 3] = ["canonical", "copy", "missing"];
const PLACEMENT_STATES: [&str; 5] = [
    "managed_link",
    "direct_copy",
    "missing",
    "conflict",
    "unavailable",
];

pub(crate) async fn export_inventory(path: PathBuf, json: String) -> Result<(), SkillsCliError> {
    run_blocking_fs_with(
        "Skills CLI inventory export",
        move || export_inventory_sync(&path, &json),
        SkillsCliError::task_join,
    )
    .await
}

fn export_inventory_sync(path: &Path, json: &str) -> Result<(), SkillsCliError> {
    validate_json_extension(path)?;
    if json.len() > EXPORT_LIMIT {
        return Err(SkillsCliError::ExportInvalid);
    }
    validate_export_document(json)?;
    write_atomic(path, json)
}

fn validate_json_extension(path: &Path) -> Result<(), SkillsCliError> {
    let is_json = path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("json"));
    if is_json {
        Ok(())
    } else {
        Err(SkillsCliError::ExportInvalid)
    }
}

fn validate_export_document(json: &str) -> Result<(), SkillsCliError> {
    let value: serde_json::Value =
        serde_json::from_str(json).map_err(|_| SkillsCliError::ExportInvalid)?;
    let object = value.as_object().ok_or(SkillsCliError::ExportInvalid)?;
    if !keys_exactly(object, &ENVELOPE_FIELDS) {
        return Err(SkillsCliError::ExportInvalid);
    }
    if object
        .get("schemaVersion")
        .and_then(serde_json::Value::as_u64)
        != Some(1)
    {
        return Err(SkillsCliError::ExportInvalid);
    }
    if !object
        .get("scope")
        .is_some_and(serde_json::Value::is_string)
    {
        return Err(SkillsCliError::ExportInvalid);
    }
    let skills = object
        .get("skills")
        .and_then(serde_json::Value::as_array)
        .ok_or(SkillsCliError::ExportInvalid)?;
    let skill_count = object
        .get("skillCount")
        .and_then(serde_json::Value::as_u64)
        .ok_or(SkillsCliError::ExportInvalid)?;
    if skill_count != skills.len() as u64 {
        return Err(SkillsCliError::ExportInvalid);
    }
    for skill in skills {
        validate_skill(skill)?;
    }
    Ok(())
}

fn validate_skill(value: &serde_json::Value) -> Result<(), SkillsCliError> {
    let object = value.as_object().ok_or(SkillsCliError::ExportInvalid)?;
    if !keys_exactly(object, &SKILL_FIELDS) {
        return Err(SkillsCliError::ExportInvalid);
    }
    require_string(object, "name")?;
    require_nullable_string(object, "source")?;
    require_nullable_string(object, "sourceType")?;
    require_nullable_string(object, "sourceUrl")?;
    require_nullable_string(object, "canonicalPath")?;
    require_nullable_string(object, "folderHash")?;
    require_nullable_string(object, "installedAt")?;
    require_nullable_string(object, "updatedAt")?;
    let kind = object
        .get("installKind")
        .and_then(serde_json::Value::as_str)
        .ok_or(SkillsCliError::ExportInvalid)?;
    if !INSTALL_KINDS.contains(&kind) {
        return Err(SkillsCliError::ExportInvalid);
    }
    let placements = object
        .get("placements")
        .and_then(serde_json::Value::as_array)
        .ok_or(SkillsCliError::ExportInvalid)?;
    for placement in placements {
        validate_placement(placement)?;
    }
    Ok(())
}

fn validate_placement(value: &serde_json::Value) -> Result<(), SkillsCliError> {
    let object = value.as_object().ok_or(SkillsCliError::ExportInvalid)?;
    if !keys_exactly(object, &PLACEMENT_FIELDS) {
        return Err(SkillsCliError::ExportInvalid);
    }
    require_string(object, "agentId")?;
    require_string(object, "displayName")?;
    let state = object
        .get("state")
        .and_then(serde_json::Value::as_str)
        .ok_or(SkillsCliError::ExportInvalid)?;
    if !PLACEMENT_STATES.contains(&state) {
        return Err(SkillsCliError::ExportInvalid);
    }
    Ok(())
}

fn keys_exactly(object: &serde_json::Map<String, serde_json::Value>, expected: &[&str]) -> bool {
    let actual: BTreeSet<&str> = object.keys().map(String::as_str).collect();
    let wanted: BTreeSet<&str> = expected.iter().copied().collect();
    actual == wanted
}

fn require_string(
    object: &serde_json::Map<String, serde_json::Value>,
    key: &str,
) -> Result<(), SkillsCliError> {
    object
        .get(key)
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.is_empty())
        .map(|_| ())
        .ok_or(SkillsCliError::ExportInvalid)
}

fn require_nullable_string(
    object: &serde_json::Map<String, serde_json::Value>,
    key: &str,
) -> Result<(), SkillsCliError> {
    match object.get(key) {
        Some(serde_json::Value::Null) | Some(serde_json::Value::String(_)) => Ok(()),
        _ => Err(SkillsCliError::ExportInvalid),
    }
}

fn write_atomic(path: &Path, json: &str) -> Result<(), SkillsCliError> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let mut temp = Builder::new()
        .prefix(TEMP_PREFIX)
        .tempfile_in(parent)
        .map_err(|_| SkillsCliError::ExportFailed)?;
    temp.write_all(json.as_bytes())
        .map_err(|_| SkillsCliError::ExportFailed)?;
    temp.flush().map_err(|_| SkillsCliError::ExportFailed)?;
    temp.as_file()
        .sync_all()
        .map_err(|_| SkillsCliError::ExportFailed)?;
    temp.persist(path)
        .map_err(|_| SkillsCliError::ExportFailed)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn valid_json() -> String {
        serde_json::json!({
            "schemaVersion": 1,
            "scope": "all",
            "skillCount": 1,
            "skills": [{
                "name": "demo",
                "source": "owner/repo",
                "sourceType": "github",
                "sourceUrl": "https://github.com/owner/repo",
                "installKind": "canonical",
                "canonicalPath": null,
                "folderHash": null,
                "installedAt": null,
                "updatedAt": null,
                "placements": [{
                    "agentId": "cursor",
                    "displayName": "Cursor",
                    "state": "direct_copy"
                }]
            }]
        })
        .to_string()
    }

    #[test]
    fn rejects_oversize_payload_before_parse() {
        let huge = "a".repeat(EXPORT_LIMIT + 1);
        assert!(matches!(
            export_inventory_sync(Path::new("out.json"), &huge),
            Err(SkillsCliError::ExportInvalid)
        ));
    }

    #[test]
    fn rejects_invalid_json() {
        assert!(matches!(
            export_inventory_sync(Path::new("out.json"), "{not-json"),
            Err(SkillsCliError::ExportInvalid)
        ));
    }

    #[test]
    fn rejects_non_json_extension() {
        assert!(matches!(
            export_inventory_sync(Path::new("out.txt"), &valid_json()),
            Err(SkillsCliError::ExportInvalid)
        ));
    }

    #[test]
    fn rejects_unknown_skill_field() {
        let mut json: serde_json::Value = serde_json::from_str(&valid_json()).unwrap();
        json["skills"][0]["secret"] = serde_json::json!("token");
        assert!(matches!(
            validate_export_document(&json.to_string()),
            Err(SkillsCliError::ExportInvalid)
        ));
    }

    #[test]
    fn atomically_replaces_existing_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("export.json");
        std::fs::write(&path, "old").unwrap();
        export_inventory_sync(&path, &valid_json()).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), valid_json());
    }

    #[test]
    fn persist_failure_keeps_old_target_and_cleans_temp() {
        let dir = TempDir::new().unwrap();
        let destination = dir.path().join("export.json");
        std::fs::create_dir(&destination).unwrap();
        assert!(export_inventory_sync(&destination, &valid_json()).is_err());
        let leftovers = std::fs::read_dir(dir.path())
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name().to_string_lossy().starts_with(TEMP_PREFIX))
            .count();
        assert_eq!(leftovers, 0);
        assert!(destination.is_dir());
    }
}
