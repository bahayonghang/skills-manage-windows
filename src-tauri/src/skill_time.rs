use chrono::{DateTime, Utc};
use std::fs::Metadata;
use std::time::SystemTime;

use crate::db;

fn system_time_to_rfc3339(time: SystemTime) -> String {
    let datetime: DateTime<Utc> = time.into();
    datetime.to_rfc3339()
}

fn metadata_created_at(metadata: Option<&Metadata>) -> Option<String> {
    metadata
        .and_then(|metadata| metadata.created().ok())
        .map(system_time_to_rfc3339)
}

fn metadata_updated_at(metadata: Option<&Metadata>) -> Option<String> {
    metadata
        .and_then(|metadata| metadata.modified().ok())
        .map(system_time_to_rfc3339)
}

pub fn filesystem_timestamps_from_metadata(
    directory_metadata: Option<&Metadata>,
    file_metadata: Option<&Metadata>,
) -> (Option<String>, Option<String>) {
    let created_at =
        metadata_created_at(directory_metadata).or_else(|| metadata_created_at(file_metadata));
    let updated_at =
        metadata_updated_at(file_metadata).or_else(|| metadata_updated_at(directory_metadata));
    (created_at, updated_at)
}

/// Return filesystem-backed creation/update timestamps for a skill.
///
/// Falls back to `skill.scanned_at` when the platform or filesystem cannot
/// provide the requested metadata. Scan-time cache values are used first so
/// list APIs do not synchronously stat every row on the hot path.
pub fn skill_filesystem_timestamps(skill: &db::Skill) -> (String, String) {
    let mut created_at = skill.fs_created_at.clone();
    let mut updated_at = skill.fs_updated_at.clone();

    if created_at.is_none() || updated_at.is_none() {
        let directory_metadata = skill
            .canonical_path
            .as_deref()
            .and_then(|path| std::fs::metadata(path).ok());
        let file_metadata = std::fs::metadata(&skill.file_path).ok();
        let (metadata_created_at, metadata_updated_at) = filesystem_timestamps_from_metadata(
            directory_metadata.as_ref(),
            file_metadata.as_ref(),
        );
        if created_at.is_none() {
            created_at = metadata_created_at;
        }
        if updated_at.is_none() {
            updated_at = metadata_updated_at;
        }
    }

    (
        created_at.unwrap_or_else(|| skill.scanned_at.clone()),
        updated_at.unwrap_or_else(|| skill.scanned_at.clone()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn skill_with_timestamp_cache(
        fs_created_at: Option<&str>,
        fs_updated_at: Option<&str>,
    ) -> db::Skill {
        db::Skill {
            id: "cached".to_string(),
            uid: "cached-uid".to_string(),
            name: "Cached".to_string(),
            description: None,
            file_path: "Z:/missing/skill/SKILL.md".to_string(),
            canonical_path: Some("Z:/missing/skill".to_string()),
            is_central: true,
            source: None,
            content: None,
            scanned_at: "2026-05-18T00:00:00Z".to_string(),
            fs_created_at: fs_created_at.map(str::to_string),
            fs_updated_at: fs_updated_at.map(str::to_string),
        }
    }

    #[test]
    fn skill_filesystem_timestamps_prefers_cached_values() {
        let skill =
            skill_with_timestamp_cache(Some("2026-05-17T01:00:00Z"), Some("2026-05-17T02:00:00Z"));

        assert_eq!(
            skill_filesystem_timestamps(&skill),
            (
                "2026-05-17T01:00:00Z".to_string(),
                "2026-05-17T02:00:00Z".to_string()
            )
        );
    }

    #[test]
    fn skill_filesystem_timestamps_falls_back_to_scanned_at_when_missing() {
        let skill = skill_with_timestamp_cache(None, None);

        assert_eq!(
            skill_filesystem_timestamps(&skill),
            (
                "2026-05-18T00:00:00Z".to_string(),
                "2026-05-18T00:00:00Z".to_string()
            )
        );
    }
}
