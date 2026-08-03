//! `skill_tags`, `skill_tag_links`, `skill_ai_tag_reviews` CRUD — Phase 2c.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{QueryBuilder, Row, Sqlite, Transaction};
use uuid::Uuid;

use crate::db::repos::repositories_repo::normalize_repository_component;
use crate::db::sqlite_batch::{sqlite_rows_per_batch, validate_text_ids_exist, TextIdTable};
use crate::db::types::{DbPool, PendingAiTagReviewInput, SkillAiTagReview, SkillTag};
use crate::db::util::now_rfc3339;

pub async fn get_skill_tags(pool: &DbPool) -> Result<Vec<SkillTag>, sqlx::Error> {
    sqlx::query_as::<_, SkillTag>("SELECT * FROM skill_tags ORDER BY is_builtin DESC, name")
        .fetch_all(pool)
        .await
}

pub async fn get_skill_tag_by_id(
    pool: &DbPool,
    tag_id: &str,
) -> Result<Option<SkillTag>, sqlx::Error> {
    sqlx::query_as::<_, SkillTag>("SELECT * FROM skill_tags WHERE id = ?")
        .bind(tag_id)
        .fetch_optional(pool)
        .await
}

pub async fn get_skill_tag_by_name(
    pool: &DbPool,
    name: &str,
) -> Result<Option<SkillTag>, sqlx::Error> {
    sqlx::query_as::<_, SkillTag>("SELECT * FROM skill_tags WHERE name = ?")
        .bind(name)
        .fetch_optional(pool)
        .await
}

pub async fn create_skill_tag(
    pool: &DbPool,
    name: &str,
    description: Option<&str>,
    color: Option<&str>,
) -> Result<SkillTag, sqlx::Error> {
    let mut transaction = pool.begin().await?;
    let tag = create_skill_tag_in_transaction(&mut transaction, name, description, color).await?;
    transaction.commit().await?;
    Ok(tag)
}

pub fn derive_skill_tag_id(name: &str) -> String {
    let slug = normalize_repository_component(name);
    if !slug.is_empty() {
        return slug;
    }

    let digest = Sha256::digest(name.trim().as_bytes());
    let prefix = digest
        .iter()
        .take(8)
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("tag-{prefix}")
}

async fn create_skill_tag_in_transaction(
    transaction: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    name: &str,
    description: Option<&str>,
    color: Option<&str>,
) -> Result<SkillTag, sqlx::Error> {
    let trimmed_name = name.trim();
    if trimmed_name.is_empty() {
        return Err(sqlx::Error::InvalidArgument(
            "Tag name is required".to_string(),
        ));
    }

    let tag_id = derive_skill_tag_id(trimmed_name);
    let now = now_rfc3339();

    sqlx::query(
        "INSERT INTO skill_tags (id, name, description, color, is_builtin, created_at, updated_at)
         VALUES (?, ?, ?, ?, 0, ?, ?)
         ON CONFLICT DO NOTHING",
    )
    .bind(&tag_id)
    .bind(trimmed_name)
    .bind(description)
    .bind(color)
    .bind(&now)
    .bind(&now)
    .execute(&mut **transaction)
    .await?;

    if let Some(tag) = sqlx::query_as::<_, SkillTag>("SELECT * FROM skill_tags WHERE name = ?")
        .bind(trimmed_name)
        .fetch_optional(&mut **transaction)
        .await?
    {
        return Ok(tag);
    }

    let fallback_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO skill_tags (id, name, description, color, is_builtin, created_at, updated_at)
         VALUES (?, ?, ?, ?, 0, ?, ?)
         ON CONFLICT(name) DO NOTHING",
    )
    .bind(&fallback_id)
    .bind(trimmed_name)
    .bind(description)
    .bind(color)
    .bind(&now)
    .bind(&now)
    .execute(&mut **transaction)
    .await?;

    sqlx::query_as::<_, SkillTag>("SELECT * FROM skill_tags WHERE name = ?")
        .bind(trimmed_name)
        .fetch_optional(&mut **transaction)
        .await?
        .ok_or_else(|| sqlx::Error::InvalidArgument("Failed to retrieve created tag".to_string()))
}

pub async fn assign_skill_tags(
    pool: &DbPool,
    skill_ids: &[String],
    tag_ids: &[String],
    source: &str,
    confidence: Option<f64>,
    reason: Option<&str>,
) -> Result<(), sqlx::Error> {
    let mut transaction = pool.begin().await?;
    validate_text_ids_exist(&mut transaction, TextIdTable::SkillTags, "Tag", tag_ids).await?;
    validate_text_ids_exist(&mut transaction, TextIdTable::Skills, "Skill", skill_ids).await?;
    assign_skill_tags_in_transaction(
        &mut transaction,
        skill_ids,
        tag_ids,
        source,
        confidence,
        reason,
    )
    .await?;
    transaction.commit().await
}

async fn assign_skill_tags_in_transaction(
    transaction: &mut Transaction<'_, Sqlite>,
    skill_ids: &[String],
    tag_ids: &[String],
    source: &str,
    confidence: Option<f64>,
    reason: Option<&str>,
) -> Result<(), sqlx::Error> {
    let total_rows = skill_ids.len().checked_mul(tag_ids.len()).ok_or_else(|| {
        sqlx::Error::InvalidArgument("Too many skill tag assignments".to_string())
    })?;
    if total_rows == 0 {
        return Ok(());
    }

    let now = now_rfc3339();
    let rows_per_batch = sqlite_rows_per_batch(6)?;
    let mut start = 0;
    while start < total_rows {
        let end = start.saturating_add(rows_per_batch).min(total_rows);
        let mut builder = QueryBuilder::<Sqlite>::new(
            "INSERT INTO skill_tag_links
             (skill_id, tag_id, confidence, reason, source, added_at) ",
        );
        builder.push_values(start..end, |mut row, index| {
            let skill_id = &skill_ids[index / tag_ids.len()];
            let tag_id = &tag_ids[index % tag_ids.len()];
            row.push_bind(skill_id)
                .push_bind(tag_id)
                .push_bind(confidence)
                .push_bind(reason)
                .push_bind(source)
                .push_bind(&now);
        });
        builder.push(
            " ON CONFLICT(skill_id, tag_id) DO UPDATE SET
               confidence = excluded.confidence,
               reason = excluded.reason,
               source = excluded.source",
        );
        builder.build().execute(&mut **transaction).await?;
        start = end;
    }
    Ok(())
}

/// 删除指定 skill 的若干 tag 关联（只删传入的 tag_ids，其余保留）。
/// 与 `assign_skill_tags` 对称；空 tag_ids 为 no-op。
pub async fn unassign_skill_tags(
    pool: &DbPool,
    skill_id: &str,
    tag_ids: &[String],
) -> Result<(), sqlx::Error> {
    if tag_ids.is_empty() {
        return Ok(());
    }
    let placeholders = tag_ids.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
    let sql = format!(
        "DELETE FROM skill_tag_links WHERE skill_id = ? AND tag_id IN ({})",
        placeholders
    );
    let mut q = sqlx::query(&sql).bind(skill_id);
    for tag_id in tag_ids {
        q = q.bind(tag_id);
    }
    q.execute(pool).await?;
    Ok(())
}

pub async fn replace_skill_ai_tags(
    pool: &DbPool,
    skill_id: &str,
    suggestions: &[(String, f64, String)],
) -> Result<(), sqlx::Error> {
    let mut transaction = pool.begin().await?;
    let tag_ids = suggestions
        .iter()
        .map(|(tag_id, _, _)| tag_id.clone())
        .collect::<Vec<_>>();
    validate_text_ids_exist(&mut transaction, TextIdTable::SkillTags, "Tag", &tag_ids).await?;
    validate_text_ids_exist(
        &mut transaction,
        TextIdTable::Skills,
        "Skill",
        &[skill_id.to_string()],
    )
    .await?;

    sqlx::query("DELETE FROM skill_tag_links WHERE skill_id = ? AND source = 'ai'")
        .bind(skill_id)
        .execute(&mut *transaction)
        .await?;

    let rows_per_batch = sqlite_rows_per_batch(6)?;
    for chunk in suggestions.chunks(rows_per_batch) {
        let mut builder = QueryBuilder::<Sqlite>::new(
            "INSERT INTO skill_tag_links
             (skill_id, tag_id, confidence, reason, source, added_at) ",
        );
        let now = now_rfc3339();
        builder.push_values(chunk, |mut row, (tag_id, confidence, reason)| {
            row.push_bind(skill_id)
                .push_bind(tag_id)
                .push_bind(confidence)
                .push_bind(reason)
                .push_bind("ai")
                .push_bind(&now);
        });
        // Any remaining conflict is a manual link because AI links were removed above.
        builder.push(" ON CONFLICT(skill_id, tag_id) DO NOTHING");
        builder.build().execute(&mut *transaction).await?;
    }

    transaction.commit().await
}

pub async fn replace_pending_ai_tag_reviews(
    pool: &DbPool,
    skill_id: &str,
    suggestions: &[PendingAiTagReviewInput],
) -> Result<(), sqlx::Error> {
    let mut transaction = pool.begin().await?;
    let existing_tag_ids = suggestions
        .iter()
        .filter(|suggestion| {
            suggestion
                .proposed_name
                .as_deref()
                .is_none_or(|name| name.trim().is_empty())
        })
        .map(|suggestion| suggestion.tag_id.clone())
        .collect::<Vec<_>>();
    validate_text_ids_exist(
        &mut transaction,
        TextIdTable::SkillTags,
        "Tag",
        &existing_tag_ids,
    )
    .await?;
    validate_text_ids_exist(
        &mut transaction,
        TextIdTable::Skills,
        "Skill",
        &[skill_id.to_string()],
    )
    .await?;

    let now = now_rfc3339();
    sqlx::query("DELETE FROM skill_ai_tag_reviews WHERE skill_id = ? AND status = 'pending'")
        .bind(skill_id)
        .execute(&mut *transaction)
        .await?;

    let rows_per_batch = sqlite_rows_per_batch(8)?;
    for chunk in suggestions.chunks(rows_per_batch) {
        let mut builder = QueryBuilder::<Sqlite>::new(
            "INSERT INTO skill_ai_tag_reviews
             (skill_id, tag_id, confidence, reason, proposed_name, proposed_description,
              status, suggested_at, updated_at) ",
        );
        builder.push_values(chunk, |mut row, suggestion| {
            row.push_bind(skill_id)
                .push_bind(&suggestion.tag_id)
                .push_bind(suggestion.confidence)
                .push_bind(&suggestion.reason)
                .push_bind(&suggestion.proposed_name)
                .push_bind(&suggestion.proposed_description)
                .push("'pending'")
                .push_bind(&now)
                .push_bind(&now);
        });
        builder.push(
            " ON CONFLICT(skill_id, tag_id) DO UPDATE SET
               confidence = excluded.confidence,
               reason = excluded.reason,
               proposed_name = excluded.proposed_name,
               proposed_description = excluded.proposed_description,
               status = 'pending',
               updated_at = excluded.updated_at",
        );
        builder.build().execute(&mut *transaction).await?;
    }

    transaction.commit().await
}

pub async fn get_pending_ai_tag_reviews(
    pool: &DbPool,
) -> Result<Vec<SkillAiTagReview>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT
           r.skill_id,
           COALESCE(s.name, r.skill_id) AS skill_name,
           r.tag_id AS tag_id,
           COALESCE(t.name, r.proposed_name) AS tag_name,
           COALESCE(t.description, r.proposed_description) AS tag_description,
           t.color AS tag_color,
           COALESCE(t.is_builtin, 0) AS tag_is_builtin,
           COALESCE(t.created_at, r.suggested_at) AS tag_created_at,
           COALESCE(t.updated_at, r.updated_at) AS tag_updated_at,
           t.group_id AS tag_group_id,
           r.confidence,
           r.reason,
           r.suggested_at,
           r.updated_at,
           (t.id IS NULL AND NULLIF(TRIM(r.proposed_name), '') IS NOT NULL) AS is_proposal
         FROM skill_ai_tag_reviews r
         LEFT JOIN skill_tags t ON t.id = r.tag_id
         LEFT JOIN skills s ON s.id = r.skill_id
         WHERE r.status = 'pending'
           AND (t.id IS NOT NULL OR NULLIF(TRIM(r.proposed_name), '') IS NOT NULL)
         ORDER BY r.updated_at DESC, skill_name, tag_name",
    )
    .fetch_all(pool)
    .await?;

    rows.into_iter()
        .map(|row| {
            Ok(SkillAiTagReview {
                skill_id: row.get("skill_id"),
                skill_name: row.get("skill_name"),
                tag: SkillTag {
                    id: row.get("tag_id"),
                    name: row.get("tag_name"),
                    description: row.get("tag_description"),
                    color: row.get("tag_color"),
                    is_builtin: row.get("tag_is_builtin"),
                    created_at: row.get("tag_created_at"),
                    updated_at: row.get("tag_updated_at"),
                    group_id: row.get("tag_group_id"),
                },
                confidence: row.get("confidence"),
                reason: row
                    .get::<Option<String>, _>("reason")
                    .unwrap_or_else(|| "AI 低置信度建议".to_string()),
                suggested_at: row.get("suggested_at"),
                updated_at: row.get("updated_at"),
                is_proposal: row.get("is_proposal"),
            })
        })
        .collect()
}

pub async fn accept_ai_tag_reviews(
    pool: &DbPool,
    skill_id: &str,
    tag_ids: &[String],
) -> Result<(), sqlx::Error> {
    if tag_ids.is_empty() {
        return Err(sqlx::Error::InvalidArgument(
            "No review tags selected".to_string(),
        ));
    }

    let mut transaction = pool.begin().await?;
    for tag_id in tag_ids {
        let review = sqlx::query(
            "SELECT confidence, reason, proposed_name, proposed_description
             FROM skill_ai_tag_reviews
             WHERE skill_id = ? AND tag_id = ? AND status = 'pending'",
        )
        .bind(skill_id)
        .bind(tag_id)
        .fetch_optional(&mut *transaction)
        .await?;

        let confidence = review
            .as_ref()
            .map(|row| row.get::<f64, _>("confidence"))
            .unwrap_or(1.0);
        let reason = review
            .as_ref()
            .and_then(|row| row.get::<Option<String>, _>("reason"))
            .unwrap_or_else(|| "人工复核确认".to_string());

        let actual_tag_id = if let Some(proposed_name) = review
            .as_ref()
            .and_then(|row| row.get::<Option<String>, _>("proposed_name"))
            .filter(|name| !name.trim().is_empty())
        {
            let description = review
                .as_ref()
                .and_then(|row| row.get::<Option<String>, _>("proposed_description"));
            create_skill_tag_in_transaction(
                &mut transaction,
                &proposed_name,
                description.as_deref(),
                None,
            )
            .await?
            .id
        } else {
            let exists: bool =
                sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM skill_tags WHERE id = ?)")
                    .bind(tag_id)
                    .fetch_one(&mut *transaction)
                    .await?;
            if !exists {
                return Err(sqlx::Error::InvalidArgument(format!(
                    "Tag '{}' not found",
                    tag_id
                )));
            }
            tag_id.clone()
        };

        let now = now_rfc3339();
        sqlx::query(
            "INSERT INTO skill_tag_links
             (skill_id, tag_id, confidence, reason, source, added_at)
             VALUES (?, ?, ?, ?, 'ai', ?)
             ON CONFLICT(skill_id, tag_id) DO UPDATE SET
               confidence = excluded.confidence,
               reason = excluded.reason,
               source = excluded.source",
        )
        .bind(skill_id)
        .bind(actual_tag_id)
        .bind(confidence)
        .bind(reason)
        .bind(now)
        .execute(&mut *transaction)
        .await?;
    }

    let now = now_rfc3339();
    for tag_id in tag_ids {
        sqlx::query(
            "UPDATE skill_ai_tag_reviews
             SET status = 'accepted', updated_at = ?
             WHERE skill_id = ? AND tag_id = ? AND status = 'pending'",
        )
        .bind(&now)
        .bind(skill_id)
        .bind(tag_id)
        .execute(&mut *transaction)
        .await?;
    }

    sqlx::query(
        "UPDATE skill_ai_tag_reviews
         SET status = 'skipped', updated_at = ?
         WHERE skill_id = ? AND status = 'pending'",
    )
    .bind(&now)
    .bind(skill_id)
    .execute(&mut *transaction)
    .await?;

    transaction.commit().await?;
    Ok(())
}

pub async fn skip_ai_tag_reviews(pool: &DbPool, skill_id: &str) -> Result<(), sqlx::Error> {
    let now = now_rfc3339();
    sqlx::query(
        "UPDATE skill_ai_tag_reviews
         SET status = 'skipped', updated_at = ?
         WHERE skill_id = ? AND status = 'pending'",
    )
    .bind(&now)
    .bind(skill_id)
    .execute(pool)
    .await
    .map(|_| ())
}

pub async fn get_skill_tags_for_skill(
    pool: &DbPool,
    skill_id: &str,
) -> Result<Vec<SkillTag>, sqlx::Error> {
    sqlx::query_as::<_, SkillTag>(
        "SELECT t.* FROM skill_tags t
         JOIN skill_tag_links l ON t.id = l.tag_id
         WHERE l.skill_id = ?
         ORDER BY t.is_builtin DESC, t.name",
    )
    .bind(skill_id)
    .fetch_all(pool)
    .await
}

pub async fn get_skill_tags_for_skills(
    pool: &DbPool,
    skill_ids: &[String],
) -> Result<HashMap<String, Vec<SkillTag>>, sqlx::Error> {
    if skill_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let placeholders = skill_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let sql = format!(
        "SELECT
            l.skill_id AS skill_id,
            t.id AS tag_id,
            t.name AS tag_name,
            t.description AS tag_description,
            t.color AS tag_color,
            t.is_builtin AS tag_is_builtin,
            t.created_at AS tag_created_at,
            t.updated_at AS tag_updated_at,
            t.group_id AS tag_group_id
         FROM skill_tag_links l
         JOIN skill_tags t ON t.id = l.tag_id
         WHERE l.skill_id IN ({})
         ORDER BY l.skill_id, t.is_builtin DESC, t.name",
        placeholders
    );
    let mut query = sqlx::query(&sql);
    for skill_id in skill_ids {
        query = query.bind(skill_id);
    }

    let rows = query.fetch_all(pool).await?;
    let mut grouped: HashMap<String, Vec<SkillTag>> = HashMap::new();
    for row in rows {
        let skill_id: String = row.try_get("skill_id")?;
        grouped.entry(skill_id).or_default().push(SkillTag {
            id: row.try_get("tag_id")?,
            name: row.try_get("tag_name")?,
            description: row.try_get("tag_description")?,
            color: row.try_get("tag_color")?,
            is_builtin: row.try_get("tag_is_builtin")?,
            created_at: row.try_get("tag_created_at")?,
            updated_at: row.try_get("tag_updated_at")?,
            group_id: row.try_get("tag_group_id")?,
        });
    }

    Ok(grouped)
}

/// 仪表盘「中央库热门标签」聚合的一行。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CentralTopTag {
    pub id: String,
    pub name: String,
    pub count: u32,
}

const MAX_CENTRAL_TOP_TAGS_LIMIT: u32 = 50;

/// 中央库（`is_central = 1`）技能的 tag 使用 Top-N。
///
/// - 只统计能 JOIN 到 central skill 的 link：非 central 副本与迁移前遗留的
///   orphan link 都不计入；
/// - 排除占位 tag `uncategorized`（`UNCATEGORIZED_TAG_ID`）；
/// - 排序 `count DESC, name ASC`，并列确定性，前端直接渲染。
pub async fn list_central_top_tags(
    pool: &DbPool,
    limit: u32,
) -> Result<Vec<CentralTopTag>, sqlx::Error> {
    let limit = i64::from(limit.clamp(1, MAX_CENTRAL_TOP_TAGS_LIMIT));
    let rows = sqlx::query(
        "SELECT t.id, t.name, COUNT(*) AS count
         FROM skill_tag_links l
         JOIN skills s ON s.id = l.skill_id AND s.is_central = 1
         JOIN skill_tags t ON t.id = l.tag_id
         WHERE l.tag_id != 'uncategorized'
         GROUP BY t.id, t.name
         ORDER BY count DESC, t.name ASC
         LIMIT ?",
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;

    rows.into_iter()
        .map(|row| {
            Ok(CentralTopTag {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                count: row.try_get::<i64, _>("count")?.max(0) as u32,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{upsert_skill, UNCATEGORIZED_TAG_ID};
    use crate::test_support::{central_skill_row, mem_pool, mem_pool_single_conn};
    use std::path::Path;

    async fn add_skill(pool: &DbPool, id: &str, is_central: bool) {
        let mut skill = central_skill_row(id, Path::new(&format!("/tmp/{id}")));
        skill.is_central = is_central;
        upsert_skill(pool, &skill).await.unwrap();
    }

    async fn link_tag(pool: &DbPool, skill_id: &str, tag_id: &str) {
        assign_skill_tags(
            pool,
            &[skill_id.to_string()],
            &[tag_id.to_string()],
            "manual",
            None,
            None,
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn central_top_tags_excludes_non_central_and_orphan_links() {
        let pool = mem_pool_single_conn().await;
        let alpha = create_skill_tag(&pool, "alpha", None, None).await.unwrap();
        add_skill(&pool, "central-one", true).await;
        add_skill(&pool, "platform-copy", false).await;
        link_tag(&pool, "central-one", &alpha.id).await;
        link_tag(&pool, "platform-copy", &alpha.id).await;
        // Semantic corruption fixture: migrated production connections keep FK
        // enabled, so disable it explicitly on this single test connection.
        sqlx::query("PRAGMA foreign_keys = OFF")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO skill_tag_links
             (skill_id, tag_id, confidence, reason, source, added_at)
             VALUES ('ghost-skill', ?, NULL, NULL, 'manual', ?)",
        )
        .bind(&alpha.id)
        .bind(now_rfc3339())
        .execute(&pool)
        .await
        .unwrap();

        let top = list_central_top_tags(&pool, 10).await.unwrap();

        assert_eq!(top.len(), 1);
        assert_eq!(top[0].id, alpha.id);
        assert_eq!(top[0].count, 1);
    }

    #[tokio::test]
    async fn central_top_tags_excludes_uncategorized() {
        let pool = mem_pool().await;
        add_skill(&pool, "central-one", true).await;
        link_tag(&pool, "central-one", UNCATEGORIZED_TAG_ID).await;

        let top = list_central_top_tags(&pool, 10).await.unwrap();

        assert!(top.is_empty());
    }

    #[tokio::test]
    async fn central_top_tags_orders_by_count_desc_then_name_asc() {
        let pool = mem_pool().await;
        let beta = create_skill_tag(&pool, "beta", None, None).await.unwrap();
        let delta = create_skill_tag(&pool, "delta", None, None).await.unwrap();
        let gamma = create_skill_tag(&pool, "gamma", None, None).await.unwrap();
        for skill_id in ["s1", "s2", "s3"] {
            add_skill(&pool, skill_id, true).await;
        }
        // 多技能共享同一 tag：gamma 计数 3。
        for skill_id in ["s1", "s2", "s3"] {
            link_tag(&pool, skill_id, &gamma.id).await;
        }
        // beta / delta 并列 2：按 name ASC（beta < delta）。
        for skill_id in ["s1", "s2"] {
            link_tag(&pool, skill_id, &beta.id).await;
            link_tag(&pool, skill_id, &delta.id).await;
        }

        let top = list_central_top_tags(&pool, 10).await.unwrap();

        let summary: Vec<(&str, u32)> = top
            .iter()
            .map(|tag| (tag.name.as_str(), tag.count))
            .collect();
        assert_eq!(summary, [("gamma", 3), ("beta", 2), ("delta", 2)]);
    }

    #[tokio::test]
    async fn central_top_tags_applies_limit_and_clamps_range() {
        let pool = mem_pool().await;
        add_skill(&pool, "central-one", true).await;
        for index in 0..55 {
            let tag = create_skill_tag(&pool, &format!("tag-{index:02}"), None, None)
                .await
                .unwrap();
            link_tag(&pool, "central-one", &tag.id).await;
        }

        // limit 截断生效。
        let top3 = list_central_top_tags(&pool, 3).await.unwrap();
        assert_eq!(top3.len(), 3);

        // clamp 范围 1..=50：0 → 1，999 → 50。
        let clamped_low = list_central_top_tags(&pool, 0).await.unwrap();
        assert_eq!(clamped_low.len(), 1);
        let clamped_high = list_central_top_tags(&pool, 999).await.unwrap();
        assert_eq!(clamped_high.len(), 50);
    }
}
