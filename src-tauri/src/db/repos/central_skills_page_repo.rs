//! SQL-backed filtering, counting, ordering, and pagination for Central skills.

use sqlx::{QueryBuilder, Sqlite};

#[cfg(test)]
use sqlx::Row;

use crate::db::types::{DbPool, Skill, UNCATEGORIZED_TAG_ID};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CentralSkillInstallFilter {
    All,
    Linked,
    Unlinked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CentralSkillPageSort {
    Name,
    CreatedAt,
    UpdatedAt,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CentralSkillPageQuery {
    pub query: Option<String>,
    pub sources: Vec<String>,
    pub include_unassigned: bool,
    pub tags: Vec<String>,
    pub include_uncategorized: bool,
    pub install: CentralSkillInstallFilter,
    pub has_shared_root_agent: bool,
    pub sort: CentralSkillPageSort,
    pub descending: bool,
    pub limit: i64,
    pub offset: i64,
}

fn push_central_skill_page_predicates<'args>(
    builder: &mut QueryBuilder<'args, Sqlite>,
    filter: &'args CentralSkillPageQuery,
) {
    builder.push(" WHERE s.is_central = 1");

    if let Some(query) = &filter.query {
        builder
            .push(" AND (instr(lower(s.name), ")
            .push_bind(query)
            .push(") > 0 OR instr(lower(coalesce(s.description, '')), ")
            .push_bind(query)
            .push(") > 0 OR instr(lower(s.id), ")
            .push_bind(query)
            .push(") > 0)");
    }

    if !filter.sources.is_empty() || filter.include_unassigned {
        builder.push(" AND (");
        if !filter.sources.is_empty() {
            builder.push(
                "EXISTS (
                    SELECT 1
                    FROM skill_repository_members source_member
                    JOIN skill_repositories source_repository
                      ON source_repository.id = source_member.repository_id
                    WHERE source_member.skill_id = s.id
                      AND source_repository.id IN (",
            );
            {
                let mut separated = builder.separated(", ");
                for source in &filter.sources {
                    separated.push_bind(source);
                }
            }
            builder.push("))");
        }
        if filter.include_unassigned {
            if !filter.sources.is_empty() {
                builder.push(" OR ");
            }
            builder.push(
                "NOT EXISTS (
                    SELECT 1
                    FROM skill_repository_members assigned_member
                    JOIN skill_repositories assigned_repository
                      ON assigned_repository.id = assigned_member.repository_id
                    WHERE assigned_member.skill_id = s.id
                      AND assigned_repository.is_unknown = 0
                )",
            );
        }
        builder.push(")");
    }

    if !filter.tags.is_empty() || filter.include_uncategorized {
        builder.push(" AND (");
        if !filter.tags.is_empty() {
            builder.push(
                "EXISTS (
                    SELECT 1
                    FROM skill_tag_links selected_tag_link
                    JOIN skill_tags selected_tag ON selected_tag.id = selected_tag_link.tag_id
                    WHERE selected_tag_link.skill_id = s.id
                      AND selected_tag.id IN (",
            );
            {
                let mut separated = builder.separated(", ");
                for tag in &filter.tags {
                    separated.push_bind(tag);
                }
            }
            builder.push("))");
        }
        if filter.include_uncategorized {
            if !filter.tags.is_empty() {
                builder.push(" OR ");
            }
            builder
                .push(
                    "NOT EXISTS (
                        SELECT 1
                        FROM skill_tag_links category_link
                        JOIN skill_tags category_tag ON category_tag.id = category_link.tag_id
                        WHERE category_link.skill_id = s.id
                          AND category_tag.id <> ",
                )
                .push_bind(UNCATEGORIZED_TAG_ID)
                .push(")");
        }
        builder.push(")");
    }

    match (filter.install, filter.has_shared_root_agent) {
        (CentralSkillInstallFilter::All, _) | (CentralSkillInstallFilter::Linked, true) => {}
        (CentralSkillInstallFilter::Linked, false) => {
            builder.push(
                " AND EXISTS (
                    SELECT 1 FROM skill_installations installation
                    WHERE installation.skill_id = s.id
                )",
            );
        }
        (CentralSkillInstallFilter::Unlinked, true) => {
            builder.push(" AND 0 = 1");
        }
        (CentralSkillInstallFilter::Unlinked, false) => {
            builder.push(
                " AND NOT EXISTS (
                    SELECT 1 FROM skill_installations installation
                    WHERE installation.skill_id = s.id
                )",
            );
        }
    }
}

fn push_central_skill_page_order(
    builder: &mut QueryBuilder<'_, Sqlite>,
    sort: CentralSkillPageSort,
    descending: bool,
) {
    let direction = if descending { " DESC" } else { " ASC" };
    builder.push(" ORDER BY ");
    match sort {
        CentralSkillPageSort::Name => {
            builder.push("lower(s.name)").push(direction);
        }
        CentralSkillPageSort::CreatedAt => {
            builder
                .push("coalesce(s.fs_created_at, s.scanned_at)")
                .push(direction)
                .push(", lower(s.name)")
                .push(direction);
        }
        CentralSkillPageSort::UpdatedAt => {
            builder
                .push("coalesce(s.fs_updated_at, s.scanned_at)")
                .push(direction)
                .push(", lower(s.name)")
                .push(direction);
        }
    }
    builder
        .push(", s.name")
        .push(direction)
        .push(", s.id")
        .push(direction);
}

/// Filter, count, order, and page Central rows in SQLite before enrichment.
pub async fn get_central_skills_page(
    pool: &DbPool,
    filter: &CentralSkillPageQuery,
) -> Result<(Vec<Skill>, usize), sqlx::Error> {
    let mut transaction = pool.begin().await?;
    let mut count_builder = QueryBuilder::<Sqlite>::new("SELECT COUNT(*) FROM skills s");
    push_central_skill_page_predicates(&mut count_builder, filter);
    let total = count_builder
        .build_query_scalar::<i64>()
        .fetch_one(&mut *transaction)
        .await?;
    let total = usize::try_from(total).map_err(|_| {
        sqlx::Error::InvalidArgument(format!("Central skill count is out of range: {total}"))
    })?;

    let mut page_builder = QueryBuilder::<Sqlite>::new("SELECT s.* FROM skills s");
    push_central_skill_page_predicates(&mut page_builder, filter);
    push_central_skill_page_order(&mut page_builder, filter.sort, filter.descending);
    page_builder
        .push(" LIMIT ")
        .push_bind(filter.limit)
        .push(" OFFSET ")
        .push_bind(filter.offset);
    let rows = page_builder
        .build_query_as::<Skill>()
        .fetch_all(&mut *transaction)
        .await?;
    transaction.commit().await?;

    Ok((rows, total))
}

#[cfg(test)]
pub(crate) async fn explain_central_skills_page(
    pool: &DbPool,
    filter: &CentralSkillPageQuery,
) -> Result<Vec<String>, sqlx::Error> {
    let mut builder = QueryBuilder::<Sqlite>::new("EXPLAIN QUERY PLAN SELECT s.* FROM skills s");
    push_central_skill_page_predicates(&mut builder, filter);
    push_central_skill_page_order(&mut builder, filter.sort, filter.descending);
    builder
        .push(" LIMIT ")
        .push_bind(filter.limit)
        .push(" OFFSET ")
        .push_bind(filter.offset);

    builder
        .build()
        .fetch_all(pool)
        .await?
        .into_iter()
        .map(|row| row.try_get("detail"))
        .collect()
}
