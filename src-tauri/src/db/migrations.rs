//! Versioned SQLite migration runner and database-open boundary.

mod backup;
pub(super) mod versions;

#[cfg(test)]
mod tests;

use std::path::Path;

use chrono::Utc;
use sqlx::Row;

use super::types::{Agent, DbPool};

#[derive(Debug, Clone, Copy)]
struct MigrationState {
    applied_version: i64,
    has_pending: bool,
}

fn descriptor_checksum(version: i64) -> Result<String, sqlx::Error> {
    let descriptor = versions::MIGRATIONS
        .iter()
        .find(|descriptor| descriptor.version == version)
        .ok_or_else(|| {
            sqlx::Error::InvalidArgument(format!("Unknown migration version {version}"))
        })?;
    Ok(versions::checksum(descriptor.source))
}

fn validate_descriptors() -> Result<(), sqlx::Error> {
    for (index, descriptor) in versions::MIGRATIONS.iter().enumerate() {
        let expected = i64::try_from(index + 1).map_err(|_| {
            sqlx::Error::InvalidArgument("Migration descriptor count overflow".to_string())
        })?;
        if descriptor.version != expected {
            return Err(sqlx::Error::InvalidArgument(format!(
                "Migration descriptors are not contiguous: expected {expected}, found {}",
                descriptor.version
            )));
        }
    }
    Ok(())
}

async fn preflight(pool: &DbPool) -> Result<MigrationState, sqlx::Error> {
    validate_descriptors()?;
    let latest = i64::try_from(versions::MIGRATIONS.len()).map_err(|_| {
        sqlx::Error::InvalidArgument("Migration descriptor count overflow".to_string())
    })?;
    let table_exists = sqlx::query(
        "SELECT EXISTS(
            SELECT 1 FROM sqlite_master
            WHERE type = 'table' AND name = 'schema_migrations'
        ) AS present",
    )
    .fetch_one(pool)
    .await?
    .try_get::<i64, _>("present")?
        == 1;

    if !table_exists {
        return Ok(MigrationState {
            applied_version: 0,
            has_pending: latest > 0,
        });
    }

    let rows = sqlx::query("SELECT version, checksum FROM schema_migrations ORDER BY version")
        .fetch_all(pool)
        .await?;
    if rows.is_empty() {
        return Err(sqlx::Error::InvalidArgument(
            "schema_migrations exists without an applied version".to_string(),
        ));
    }
    for (index, row) in rows.iter().enumerate() {
        let version = row.try_get::<i64, _>("version")?;
        let expected_version = i64::try_from(index + 1).map_err(|_| {
            sqlx::Error::InvalidArgument("Applied migration count overflow".to_string())
        })?;
        if version != expected_version {
            return Err(sqlx::Error::InvalidArgument(format!(
                "Applied migration versions are not contiguous: expected {expected_version}, found {version}"
            )));
        }
        if version > latest {
            return Err(sqlx::Error::InvalidArgument(format!(
                "Database schema version {version} is newer than supported version {latest}"
            )));
        }
        let stored_checksum = row.try_get::<String, _>("checksum")?;
        let expected_checksum = descriptor_checksum(version)?;
        if stored_checksum != expected_checksum {
            return Err(sqlx::Error::InvalidArgument(format!(
                "Migration checksum mismatch for version {version}"
            )));
        }
    }

    let applied_version = rows
        .last()
        .map(|row| row.try_get::<i64, _>("version"))
        .transpose()?
        .unwrap_or(0);
    Ok(MigrationState {
        applied_version,
        has_pending: applied_version < latest,
    })
}

async fn apply_migration(pool: &DbPool, version: i64) -> Result<(), sqlx::Error> {
    let checksum = descriptor_checksum(version)?;
    let mut transaction = pool.begin().await?;
    let apply_result = match version {
        1 => versions::v1::apply(&mut transaction).await,
        2 => versions::v2::apply(&mut transaction).await,
        _ => {
            return Err(sqlx::Error::InvalidArgument(format!(
                "Unknown migration version {version}"
            )))
        }
    };
    if let Err(error) = apply_result {
        transaction.rollback().await.map_err(|rollback_error| {
            sqlx::Error::InvalidArgument(format!(
                "Migration {version} failed: {error}; rollback also failed: {rollback_error}"
            ))
        })?;
        return Err(error);
    }

    let metadata_result = sqlx::query(
        "INSERT INTO schema_migrations (version, checksum, applied_at)
         VALUES (?, ?, ?)",
    )
    .bind(version)
    .bind(checksum)
    .bind(Utc::now().to_rfc3339())
    .execute(&mut *transaction)
    .await;
    if let Err(error) = metadata_result {
        transaction.rollback().await.map_err(|rollback_error| {
            sqlx::Error::InvalidArgument(format!(
                "Migration {version} metadata write failed: {error}; rollback also failed: {rollback_error}"
            ))
        })?;
        return Err(error);
    }
    transaction.commit().await
}

async fn validate_foreign_keys(pool: &DbPool) -> Result<(), sqlx::Error> {
    if sqlx::query("PRAGMA foreign_key_check")
        .fetch_optional(pool)
        .await?
        .is_some()
    {
        return Err(sqlx::Error::InvalidArgument(
            "SQLite foreign key validation failed".to_string(),
        ));
    }
    Ok(())
}

async fn migrate_and_seed(pool: &DbPool, agents: &[Agent]) -> Result<(), sqlx::Error> {
    let state = preflight(pool).await?;
    if state.applied_version < 1 {
        apply_migration(pool, 1).await?;
    }
    if state.applied_version < 2 {
        super::repair_orphan_skill_relations(pool).await?;
        apply_migration(pool, 2).await?;
    }
    validate_foreign_keys(pool).await?;
    super::seed::seed_database_with_agents(pool, agents).await
}

async fn open_database_with_agents(path: &Path, agents: Vec<Agent>) -> Result<DbPool, sqlx::Error> {
    let path = crate::paths::canonicalize_path_with_missing(path);
    let source_had_data = std::fs::metadata(&path)
        .map(|metadata| metadata.is_file() && metadata.len() > 0)
        .unwrap_or(false);
    let pool = super::pool::create_pool(&path).await?;
    let state = match preflight(&pool).await {
        Ok(state) => state,
        Err(error) => {
            pool.close().await;
            drop(pool);
            return Err(error);
        }
    };

    let backup_path = if source_had_data && state.has_pending {
        match backup::create_backup(&pool, &path, state.applied_version).await {
            Ok(path) => Some(path),
            Err(error) => {
                pool.close().await;
                drop(pool);
                return Err(error);
            }
        }
    } else {
        None
    };

    match migrate_and_seed(&pool, &agents).await {
        Ok(()) => Ok(pool),
        Err(migration_error) => {
            pool.close().await;
            drop(pool);
            if let Some(backup_path) = backup_path {
                if let Err(restore_error) = backup::restore_backup(&path, &backup_path).await {
                    return Err(sqlx::Error::InvalidArgument(format!(
                        "Database migration failed: {migration_error}; backup restore also failed: {restore_error}"
                    )));
                }
            }
            Err(migration_error)
        }
    }
}

pub async fn open_database(path: &Path) -> Result<DbPool, sqlx::Error> {
    open_database_with_agents(path, super::seed::builtin_agents()).await
}

pub async fn open_database_for_remote_home(
    path: &Path,
    remote_home: &str,
) -> Result<DbPool, sqlx::Error> {
    open_database_with_agents(
        path,
        super::seed::builtin_agents_for_posix_home(remote_home),
    )
    .await
}

pub(crate) async fn initialize_pool(pool: &DbPool, agents: Vec<Agent>) -> Result<(), sqlx::Error> {
    migrate_and_seed(pool, &agents).await
}
