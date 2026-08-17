//! Named-skill usage stats (`count` + `last_used_ms`) for platform sort/rank.

use serde::{Deserialize, Serialize};
use sqlx::{FromRow, QueryBuilder, Sqlite};

use crate::db::types::DbPool;

/// Per-skill call count + last-used timestamp for a named skill set.
/// `cutoff_ms = None` means all recorded history (no `timestamp_ms >=` filter).
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct SkillUsageStatRow {
    pub skill: String,
    pub count: i64,
    pub last_used_ms: Option<i64>,
}

/// Count calls and last-used time for the requested skill names.
///
/// `cutoff_ms = None` omits the time filter (all recorded history).
/// An empty `skills` list returns an empty vec and does not query.
pub async fn list_skill_usage_stats(
    pool: &DbPool,
    target_id: &str,
    skills: &[String],
    cutoff_ms: Option<i64>,
) -> Result<Vec<SkillUsageStatRow>, sqlx::Error> {
    if skills.is_empty() {
        return Ok(vec![]);
    }

    let mut builder = QueryBuilder::<Sqlite>::new(
        "SELECT skill, COUNT(*) AS count, MAX(timestamp_ms) AS last_used_ms
         FROM skill_calls
         WHERE target_id = ",
    );
    builder.push_bind(target_id);
    if let Some(cutoff) = cutoff_ms {
        builder.push(" AND timestamp_ms >= ").push_bind(cutoff);
    }
    builder.push(" AND skill IN (");
    {
        let mut separated = builder.separated(", ");
        for skill in skills {
            separated.push_bind(skill);
        }
    }
    builder.push(
        ")
         GROUP BY skill",
    );

    builder
        .build_query_as::<SkillUsageStatRow>()
        .fetch_all(pool)
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::repos::usage_repo::{replace_calls_for_target, NewSkillCall};
    use crate::test_support::mem_pool;

    fn call_at(skill: &str, timestamp_ms: i64) -> NewSkillCall {
        NewSkillCall {
            skill: skill.to_string(),
            timestamp_ms,
            project: "/project".to_string(),
            session_id: "session".to_string(),
            source: "Codex CLI".to_string(),
        }
    }

    #[tokio::test]
    async fn list_skill_usage_stats_covers_empty_cutoff_and_targets() {
        let pool = mem_pool().await;
        replace_calls_for_target(
            &pool,
            "local",
            &[
                call_at("review", 1_000),
                call_at("review", 5_000),
                call_at("review", 9_000),
                call_at("other", 8_000),
            ],
            &[],
            &[],
            10,
        )
        .await
        .unwrap();
        replace_calls_for_target(
            &pool,
            "ssh-prod",
            &[call_at("review", 20_000)],
            &[],
            &[],
            20,
        )
        .await
        .unwrap();

        assert!(list_skill_usage_stats(&pool, "local", &[], None)
            .await
            .unwrap()
            .is_empty());

        let all = list_skill_usage_stats(
            &pool,
            "local",
            &["review".to_string(), "missing".to_string()],
            None,
        )
        .await
        .unwrap();
        assert_eq!(
            all,
            vec![SkillUsageStatRow {
                skill: "review".to_string(),
                count: 3,
                last_used_ms: Some(9_000),
            }]
        );

        let cutoff = list_skill_usage_stats(&pool, "local", &["review".to_string()], Some(4_000))
            .await
            .unwrap();
        assert_eq!(cutoff[0].count, 2);
        assert_eq!(cutoff[0].last_used_ms, Some(9_000));

        let remote = list_skill_usage_stats(&pool, "ssh-prod", &["review".to_string()], None)
            .await
            .unwrap();
        assert_eq!(remote[0].count, 1);
        assert_eq!(remote[0].last_used_ms, Some(20_000));
    }
}
