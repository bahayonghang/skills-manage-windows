//! Skill explanation cache I/O against the `skill_explanations` table.
//!
//! Stored shape: `(skill_id, explanation, lang, model, created_at, updated_at)`,
//! with `created_at` preserved across upserts so the original generation
//! timestamp survives refreshes. Empty rows from older builds are treated as
//! corruption and dropped on read.

pub(crate) fn explanation_has_content(explanation: &str) -> bool {
    !explanation.trim().is_empty()
}

pub(crate) async fn delete_cached_skill_explanation(
    pool: &crate::db::DbPool,
    skill_id: &str,
    lang: &str,
) -> Result<(), String> {
    sqlx::query("DELETE FROM skill_explanations WHERE skill_id = ? AND lang = ?")
        .bind(skill_id)
        .bind(lang)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub(crate) async fn load_cached_skill_explanation(
    pool: &crate::db::DbPool,
    skill_id: &str,
    lang: &str,
) -> Result<Option<String>, String> {
    use sqlx::Row;

    let row =
        sqlx::query("SELECT explanation FROM skill_explanations WHERE skill_id = ? AND lang = ?")
            .bind(skill_id)
            .bind(lang)
            .fetch_optional(pool)
            .await
            .map_err(|e| e.to_string())?;

    match row {
        Some(row) => {
            let explanation: String = row.get("explanation");
            if explanation_has_content(&explanation) {
                Ok(Some(explanation))
            } else {
                // Older builds could persist empty strings. Treat them as cache
                // corruption so the next request re-generates a fresh explanation.
                delete_cached_skill_explanation(pool, skill_id, lang).await?;
                Ok(None)
            }
        }
        None => Ok(None),
    }
}

pub(crate) async fn cache_skill_explanation(
    pool: &crate::db::DbPool,
    skill_id: &str,
    lang: &str,
    model: &str,
    explanation: &str,
) -> Result<(), String> {
    if !explanation_has_content(explanation) {
        return Err("AI explanation returned no content.".to_string());
    }

    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT OR REPLACE INTO skill_explanations (skill_id, explanation, lang, model, created_at, updated_at)
         VALUES (?, ?, ?, ?,
            COALESCE((SELECT created_at FROM skill_explanations WHERE skill_id = ? AND lang = ?), ?),
            ?)",
    )
    .bind(skill_id)
    .bind(explanation)
    .bind(lang)
    .bind(model)
    .bind(skill_id)
    .bind(lang)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await
    .map_err(|e| format!("Failed to cache AI explanation: {}", e))?;

    Ok(())
}
