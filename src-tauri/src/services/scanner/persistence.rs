use std::collections::{HashMap, HashSet};

use crate::db::{self, AgentSkillObservation, DbPool, LinkType, Skill, SkillInstallation};

use super::ScannerError;

#[derive(Default)]
pub(super) struct ScanPersistenceBatch {
    pub(super) skills: Vec<Skill>,
    pub(super) installations: Vec<SkillInstallation>,
    pub(super) observations: Vec<AgentSkillObservation>,
    pub(super) agent_detected: Vec<(String, bool)>,
    pub(super) found_install_ids_by_agent: HashMap<String, HashSet<String>>,
    pub(super) found_observation_row_ids_by_agent: HashMap<String, HashSet<String>>,
    pub(super) global_found_skill_ids: HashSet<String>,
}

impl ScanPersistenceBatch {
    pub(super) fn set_agent_detected(&mut self, agent_id: &str, is_detected: bool) {
        self.agent_detected
            .push((agent_id.to_string(), is_detected));
    }

    pub(super) fn touch_install_agent(&mut self, agent_id: &str) {
        self.found_install_ids_by_agent
            .entry(agent_id.to_string())
            .or_default();
    }

    pub(super) fn touch_observation_agent(&mut self, agent_id: &str) {
        self.found_observation_row_ids_by_agent
            .entry(agent_id.to_string())
            .or_default();
    }

    pub(super) fn remember_installation(&mut self, agent_id: &str, skill_id: &str) {
        self.found_install_ids_by_agent
            .entry(agent_id.to_string())
            .or_default()
            .insert(skill_id.to_string());
    }

    pub(super) fn remember_observation(&mut self, agent_id: &str, row_id: &str) {
        self.found_observation_row_ids_by_agent
            .entry(agent_id.to_string())
            .or_default()
            .insert(row_id.to_string());
    }
}

async fn execute_scan_query<'q>(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    query: sqlx::query::Query<'q, sqlx::Sqlite, sqlx::sqlite::SqliteArguments<'q>>,
) -> Result<(), ScannerError> {
    query.execute(&mut **tx).await?;
    Ok(())
}

async fn upsert_scan_skill(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    skill: &Skill,
) -> Result<(), ScannerError> {
    execute_scan_query(
        tx,
        sqlx::query(
            "INSERT INTO skills
             (id, uid, name, description, file_path, canonical_path, is_central, source, content,
              scanned_at, fs_created_at, fs_updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(id) DO UPDATE SET
               name           = CASE
                                  WHEN skills.is_central = 1 AND excluded.is_central = 0 THEN skills.name
                                  ELSE excluded.name
                                END,
               description    = CASE
                                  WHEN skills.is_central = 1 AND excluded.is_central = 0 THEN skills.description
                                  ELSE excluded.description
                                END,
               file_path      = CASE
                                  WHEN skills.is_central = 1 AND excluded.is_central = 0 THEN skills.file_path
                                  ELSE excluded.file_path
                                END,
               canonical_path = CASE
                                  WHEN skills.is_central = 1 AND excluded.is_central = 0 THEN skills.canonical_path
                                  ELSE COALESCE(excluded.canonical_path, skills.canonical_path)
                                END,
               is_central     = MAX(skills.is_central, excluded.is_central),
               source         = CASE
                                  WHEN skills.is_central = 1 AND excluded.is_central = 0 THEN skills.source
                                  ELSE excluded.source
                                END,
               content        = CASE
                                  WHEN skills.is_central = 1 AND excluded.is_central = 0 THEN skills.content
                                  ELSE excluded.content
                                END,
               scanned_at     = excluded.scanned_at,
               fs_created_at  = CASE
                                  WHEN skills.is_central = 1 AND excluded.is_central = 0 THEN skills.fs_created_at
                                  ELSE COALESCE(excluded.fs_created_at, skills.fs_created_at)
                                END,
               fs_updated_at  = CASE
                                  WHEN skills.is_central = 1 AND excluded.is_central = 0 THEN skills.fs_updated_at
                                  ELSE COALESCE(excluded.fs_updated_at, skills.fs_updated_at)
                                END",
        )
        .bind(&skill.id)
        .bind(&skill.uid)
        .bind(&skill.name)
        .bind(&skill.description)
        .bind(&skill.file_path)
        .bind(&skill.canonical_path)
        .bind(skill.is_central)
        .bind(&skill.source)
        .bind(&skill.content)
        .bind(&skill.scanned_at)
        .bind(&skill.fs_created_at)
        .bind(&skill.fs_updated_at),
    )
    .await
}

async fn upsert_scan_installation(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    installation: &SkillInstallation,
) -> Result<(), ScannerError> {
    installation
        .link_type
        .parse::<LinkType>()
        .map_err(ScannerError::InvalidLinkType)?;
    execute_scan_query(
        tx,
        sqlx::query(
            "INSERT INTO skill_installations
             (skill_id, agent_id, installed_path, link_type, symlink_target, created_at)
             VALUES (?, ?, ?, ?, ?, ?)
             ON CONFLICT(skill_id, agent_id) DO UPDATE SET
               installed_path = excluded.installed_path,
               link_type      = excluded.link_type,
               symlink_target = excluded.symlink_target",
        )
        .bind(&installation.skill_id)
        .bind(&installation.agent_id)
        .bind(&installation.installed_path)
        .bind(&installation.link_type)
        .bind(&installation.symlink_target)
        .bind(&installation.created_at),
    )
    .await
}

async fn upsert_scan_observation(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    observation: &AgentSkillObservation,
) -> Result<(), ScannerError> {
    observation
        .link_type
        .parse::<LinkType>()
        .map_err(ScannerError::InvalidLinkType)?;
    execute_scan_query(
        tx,
        sqlx::query(
            "INSERT INTO agent_skill_observations
             (row_id, agent_id, skill_id, name, description, file_path, dir_path,
              source_kind, source_root, link_type, symlink_target, is_read_only, scanned_at,
              fs_created_at, fs_updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(row_id) DO UPDATE SET
               agent_id       = excluded.agent_id,
               skill_id       = excluded.skill_id,
               name           = excluded.name,
               description    = excluded.description,
               file_path      = excluded.file_path,
               dir_path       = excluded.dir_path,
               source_kind    = excluded.source_kind,
               source_root    = excluded.source_root,
               link_type      = excluded.link_type,
               symlink_target = excluded.symlink_target,
               is_read_only   = excluded.is_read_only,
               scanned_at     = excluded.scanned_at,
               fs_created_at  = COALESCE(excluded.fs_created_at, agent_skill_observations.fs_created_at),
               fs_updated_at  = COALESCE(excluded.fs_updated_at, agent_skill_observations.fs_updated_at)",
        )
        .bind(&observation.row_id)
        .bind(&observation.agent_id)
        .bind(&observation.skill_id)
        .bind(&observation.name)
        .bind(&observation.description)
        .bind(&observation.file_path)
        .bind(&observation.dir_path)
        .bind(&observation.source_kind)
        .bind(&observation.source_root)
        .bind(&observation.link_type)
        .bind(&observation.symlink_target)
        .bind(observation.is_read_only)
        .bind(&observation.scanned_at)
        .bind(&observation.fs_created_at)
        .bind(&observation.fs_updated_at),
    )
    .await
}

async fn reset_scan_temp_tables(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
) -> Result<(), ScannerError> {
    let statements = [
        "CREATE TEMP TABLE IF NOT EXISTS scan_keep_skills (skill_id TEXT PRIMARY KEY)",
        "CREATE TEMP TABLE IF NOT EXISTS scan_touched_install_agents (agent_id TEXT PRIMARY KEY)",
        "CREATE TEMP TABLE IF NOT EXISTS scan_keep_installations (
            agent_id TEXT NOT NULL,
            skill_id TEXT NOT NULL,
            PRIMARY KEY (agent_id, skill_id)
         )",
        "CREATE TEMP TABLE IF NOT EXISTS scan_touched_observation_agents (agent_id TEXT PRIMARY KEY)",
        "CREATE TEMP TABLE IF NOT EXISTS scan_keep_observations (
            agent_id TEXT NOT NULL,
            row_id TEXT NOT NULL,
            PRIMARY KEY (agent_id, row_id)
         )",
        "DELETE FROM scan_keep_skills",
        "DELETE FROM scan_touched_install_agents",
        "DELETE FROM scan_keep_installations",
        "DELETE FROM scan_touched_observation_agents",
        "DELETE FROM scan_keep_observations",
    ];
    for statement in statements {
        execute_scan_query(tx, sqlx::query(statement)).await?;
    }
    Ok(())
}

async fn persist_scan_keep_tables(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    batch: &ScanPersistenceBatch,
) -> Result<(), ScannerError> {
    for skill_id in &batch.global_found_skill_ids {
        execute_scan_query(
            tx,
            sqlx::query("INSERT OR IGNORE INTO scan_keep_skills (skill_id) VALUES (?)")
                .bind(skill_id),
        )
        .await?;
    }

    for (agent_id, skill_ids) in &batch.found_install_ids_by_agent {
        execute_scan_query(
            tx,
            sqlx::query("INSERT OR IGNORE INTO scan_touched_install_agents (agent_id) VALUES (?)")
                .bind(agent_id),
        )
        .await?;
        for skill_id in skill_ids {
            execute_scan_query(
                tx,
                sqlx::query(
                    "INSERT OR IGNORE INTO scan_keep_installations (agent_id, skill_id)
                     VALUES (?, ?)",
                )
                .bind(agent_id)
                .bind(skill_id),
            )
            .await?;
        }
    }

    for (agent_id, row_ids) in &batch.found_observation_row_ids_by_agent {
        execute_scan_query(
            tx,
            sqlx::query(
                "INSERT OR IGNORE INTO scan_touched_observation_agents (agent_id) VALUES (?)",
            )
            .bind(agent_id),
        )
        .await?;
        for row_id in row_ids {
            execute_scan_query(
                tx,
                sqlx::query(
                    "INSERT OR IGNORE INTO scan_keep_observations (agent_id, row_id)
                     VALUES (?, ?)",
                )
                .bind(agent_id)
                .bind(row_id),
            )
            .await?;
        }
    }

    Ok(())
}

async fn delete_scan_stale_rows(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
) -> Result<(), ScannerError> {
    let independently_scoped_statements = [
        "DELETE FROM skill_installations
         WHERE agent_id IN (SELECT agent_id FROM scan_touched_install_agents)
           AND NOT EXISTS (
             SELECT 1 FROM scan_keep_installations keep
             WHERE keep.agent_id = skill_installations.agent_id
               AND keep.skill_id = skill_installations.skill_id
           )",
        "DELETE FROM agent_skill_observations
         WHERE agent_id IN (SELECT agent_id FROM scan_touched_observation_agents)
           AND NOT EXISTS (
             SELECT 1 FROM scan_keep_observations keep
             WHERE keep.agent_id = agent_skill_observations.agent_id
               AND keep.row_id = agent_skill_observations.row_id
           )",
    ];

    for statement in independently_scoped_statements {
        execute_scan_query(tx, sqlx::query(statement)).await?;
    }

    db::delete_owned_skill_relations_missing_from_scan_keep(tx).await?;
    execute_scan_query(
        tx,
        sqlx::query(
            "DELETE FROM skills
             WHERE NOT EXISTS (
               SELECT 1 FROM scan_keep_skills keep
               WHERE keep.skill_id = skills.id
             )",
        ),
    )
    .await?;
    db::prune_empty_skill_repositories_in_transaction(tx).await?;
    Ok(())
}

pub(super) async fn persist_scan_batch(
    pool: &DbPool,
    batch: ScanPersistenceBatch,
) -> Result<(), ScannerError> {
    let mut tx = pool.begin().await?;
    reset_scan_temp_tables(&mut tx).await?;

    for (agent_id, is_detected) in &batch.agent_detected {
        execute_scan_query(
            &mut tx,
            sqlx::query("UPDATE agents SET is_detected = ? WHERE id = ?")
                .bind(*is_detected)
                .bind(agent_id),
        )
        .await?;
    }
    for skill in &batch.skills {
        upsert_scan_skill(&mut tx, skill).await?;
    }
    for observation in &batch.observations {
        upsert_scan_observation(&mut tx, observation).await?;
    }
    for installation in &batch.installations {
        upsert_scan_installation(&mut tx, installation).await?;
    }

    persist_scan_keep_tables(&mut tx, &batch).await?;
    delete_scan_stale_rows(&mut tx).await?;
    tx.commit().await?;
    Ok(())
}
