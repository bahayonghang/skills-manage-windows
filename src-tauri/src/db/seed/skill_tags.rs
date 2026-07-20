use std::collections::HashSet;

use crate::db::types::{DbPool, ACADEMIC_RESEARCH_WRITING_TAG_ID, UNCATEGORIZED_TAG_ID};

pub(super) async fn seed_builtin_skill_tags(pool: &DbPool, now: &str) -> Result<(), sqlx::Error> {
    for (id, name, description, color) in builtin_skill_tags() {
        seed_builtin_skill_tag(pool, id, name, description, color, now).await?;
    }
    prune_obsolete_builtin_skill_tags(pool).await
}

async fn seed_builtin_skill_tag(
    pool: &DbPool,
    id: &str,
    name: &str,
    description: &str,
    color: &str,
    now: &str,
) -> Result<(), sqlx::Error> {
    let conflicts = sqlx::query_as::<_, (String, String, bool)>(
        "SELECT id, name, is_builtin FROM skill_tags WHERE id = ? OR name = ?",
    )
    .bind(id)
    .bind(name)
    .fetch_all(pool)
    .await?;

    let existing_id = conflicts
        .iter()
        .find(|(existing_id, _, _)| existing_id == id);
    if let Some((_, _, is_builtin)) = existing_id {
        if !is_builtin {
            return Ok(());
        }

        let name_is_taken = conflicts
            .iter()
            .any(|(existing_id, existing_name, _)| existing_id != id && existing_name == name);
        if name_is_taken {
            sqlx::query(
                "UPDATE skill_tags
                 SET description = ?, color = ?, is_builtin = 1, updated_at = ?
                 WHERE id = ? AND is_builtin = 1",
            )
            .bind(description)
            .bind(color)
            .bind(now)
            .bind(id)
            .execute(pool)
            .await?;
        } else {
            sqlx::query(
                "UPDATE skill_tags
                 SET name = ?, description = ?, color = ?, is_builtin = 1, updated_at = ?
                 WHERE id = ? AND is_builtin = 1",
            )
            .bind(name)
            .bind(description)
            .bind(color)
            .bind(now)
            .bind(id)
            .execute(pool)
            .await?;
        }
        return Ok(());
    }

    if !conflicts.is_empty() {
        return Ok(());
    }

    sqlx::query(
        "INSERT INTO skill_tags
         (id, name, description, color, is_builtin, created_at, updated_at)
         VALUES (?, ?, ?, ?, 1, ?, ?)",
    )
    .bind(id)
    .bind(name)
    .bind(description)
    .bind(color)
    .bind(now)
    .bind(now)
    .execute(pool)
    .await?;

    Ok(())
}

async fn prune_obsolete_builtin_skill_tags(pool: &DbPool) -> Result<(), sqlx::Error> {
    let current_ids: HashSet<&str> = builtin_skill_tags()
        .into_iter()
        .map(|(id, _, _, _)| id)
        .collect();
    let obsolete_ids: Vec<(String,)> =
        sqlx::query_as::<_, (String,)>("SELECT id FROM skill_tags WHERE is_builtin = 1")
            .fetch_all(pool)
            .await?
            .into_iter()
            .filter(|(id,)| !current_ids.contains(id.as_str()))
            .collect();
    if obsolete_ids.is_empty() {
        return Ok(());
    }

    let mut tx = pool.begin().await?;
    for (id,) in obsolete_ids {
        sqlx::query("DELETE FROM skill_tag_links WHERE tag_id = ?")
            .bind(&id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM skill_ai_tag_reviews WHERE tag_id = ?")
            .bind(&id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM skill_tags WHERE id = ? AND is_builtin = 1")
            .bind(&id)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    Ok(())
}

pub(crate) fn builtin_skill_tags() -> Vec<(&'static str, &'static str, &'static str, &'static str)>
{
    vec![
        (
            ACADEMIC_RESEARCH_WRITING_TAG_ID,
            "学术研究与写作",
            "Paper search, academic writing, slides, and research workflows.",
            "#0891b2",
        ),
        (
            "frontend-development",
            "前端开发",
            "Web UI, React/Vue, CSS, component and page building.",
            "#3b82f6",
        ),
        (
            "backend-development",
            "后端开发",
            "Server-side APIs, databases, business logic, system services.",
            "#8b5cf6",
        ),
        (
            "devops-deployment",
            "DevOps 与部署",
            "CI/CD, containers, infrastructure, release and ops automation.",
            "#f97316",
        ),
        (
            "testing-quality",
            "测试与质量",
            "Test writing, code review, linting, QA workflows.",
            "#22c55e",
        ),
        (
            "docs-writing",
            "文档与写作",
            "Technical docs, README, blogs, general writing and editing.",
            "#eab308",
        ),
        (
            "data-analysis",
            "数据与分析",
            "Data processing, SQL, visualization, reports and analytics.",
            "#14b8a6",
        ),
        (
            "design-ui",
            "设计与 UI",
            "Visual design, prototyping, design systems, UX polish.",
            "#ec4899",
        ),
        (
            "ai-prompt-engineering",
            "AI 与提示工程",
            "LLM prompts, agents, RAG, model integration workflows.",
            "#6366f1",
        ),
        (
            "productivity-tools",
            "效率与工具",
            "Automation scripts, CLI helpers, personal productivity.",
            "#64748b",
        ),
        (
            "office-documents",
            "办公文档",
            "Word/Excel/PPT/PDF creation and manipulation.",
            "#a16207",
        ),
        // System fallback retained for smart views and AI fallback, not a
        // visible ordinary category.
        (
            UNCATEGORIZED_TAG_ID,
            "未分类",
            "Fallback category for skills that still need review.",
            "#71717a",
        ),
    ]
}
