//! Collection JSON export/import implementations.
//!
//! Moved verbatim out of `commands/collections.rs`; the Tauri command shells
//! stay in the parent module and call into these impls.

use serde::{Deserialize, Serialize};

use crate::db::{self, Collection, DbPool};

use super::get_collection_or_err;

/// Export format for a collection, matching the spec in docs/desktop-design.md.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectionExport {
    pub version: u32,
    pub name: String,
    pub description: Option<String>,
    /// Skill IDs (the skill names/identifiers, not UUIDs).
    pub skills: Vec<String>,
    pub created_at: String,
    pub exported_from: String,
}

/// Export a collection to a JSON string matching the spec in docs/desktop-design.md.
pub async fn export_collection_impl(pool: &DbPool, collection_id: &str) -> Result<String, String> {
    let collection = get_collection_or_err(pool, collection_id).await?;

    let skills = db::get_collection_skills(pool, collection_id)
        .await
        .map_err(|e| e.to_string())?;
    let skill_ids: Vec<String> = skills.into_iter().map(|s| s.id).collect();

    let export = CollectionExport {
        version: 1,
        name: collection.name,
        description: collection.description,
        skills: skill_ids,
        created_at: collection.created_at,
        exported_from: "SkillPort".to_string(),
    };

    serde_json::to_string_pretty(&export).map_err(|e| e.to_string())
}

/// Import a collection from a JSON string.
///
/// Creates a new collection with the given name/description and links any
/// skills whose IDs exist in the database. Skills that are not found are
/// silently skipped (they may not yet be scanned on this machine).
///
/// Returns the newly created collection.
pub async fn import_collection_impl(pool: &DbPool, json: &str) -> Result<Collection, String> {
    let export: CollectionExport =
        serde_json::from_str(json).map_err(|e| format!("Invalid collection JSON: {}", e))?;

    if export.name.trim().is_empty() {
        return Err("Imported collection name cannot be empty".to_string());
    }

    // Create the collection.
    let collection = db::create_collection(pool, &export.name, export.description.as_deref())
        .await
        .map_err(|e| e.to_string())?;

    // Link skills that exist in the local database.
    for skill_id in &export.skills {
        // Only add the skill if it exists in the local DB; silently skip otherwise.
        if let Ok(Some(_)) = db::get_skill_by_id(pool, skill_id).await {
            db::add_skill_to_collection(pool, &collection.id, skill_id)
                .await
                .map_err(|e| e.to_string())?;
        }
    }

    Ok(collection)
}
