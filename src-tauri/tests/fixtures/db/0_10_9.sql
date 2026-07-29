-- Frozen from v0.10.9 (2ad251e671982e9942161cecada37c8246e3f98c).
-- Generated from the tag schema CREATE statements; do not edit.

CREATE TABLE IF NOT EXISTS skills (
            id             TEXT PRIMARY KEY,
            name           TEXT NOT NULL,
            description    TEXT,
            file_path      TEXT NOT NULL,
            canonical_path TEXT,
            is_central     BOOLEAN NOT NULL DEFAULT 0,
            source         TEXT,
            content        TEXT,
            scanned_at     TEXT NOT NULL,
            fs_created_at  TEXT,
            fs_updated_at  TEXT
        );
CREATE TABLE IF NOT EXISTS skill_installations (
            skill_id       TEXT NOT NULL,
            agent_id       TEXT NOT NULL,
            installed_path TEXT NOT NULL,
            link_type      TEXT NOT NULL,
            symlink_target TEXT,
            created_at     TEXT NOT NULL DEFAULT (datetime('now')),
            PRIMARY KEY (skill_id, agent_id)
        );
CREATE INDEX IF NOT EXISTS idx_skill_installations_agent_skill_id
         ON skill_installations(agent_id, skill_id);
CREATE TABLE IF NOT EXISTS agent_skill_observations (
            row_id         TEXT PRIMARY KEY,
            agent_id       TEXT NOT NULL,
            skill_id       TEXT NOT NULL,
            name           TEXT NOT NULL,
            description    TEXT,
            file_path      TEXT NOT NULL,
            dir_path       TEXT NOT NULL,
            source_kind    TEXT NOT NULL,
            source_root    TEXT NOT NULL,
            link_type      TEXT NOT NULL,
            symlink_target TEXT,
            is_read_only   BOOLEAN NOT NULL DEFAULT 0,
            scanned_at     TEXT NOT NULL,
            fs_created_at  TEXT,
            fs_updated_at  TEXT
        );
CREATE INDEX IF NOT EXISTS idx_agent_skill_observations_agent_id
         ON agent_skill_observations(agent_id);
CREATE INDEX IF NOT EXISTS idx_agent_skill_observations_agent_name_dir
         ON agent_skill_observations(agent_id, name, dir_path);
CREATE TABLE IF NOT EXISTS agents (
            id                 TEXT PRIMARY KEY,
            display_name       TEXT NOT NULL,
            category           TEXT NOT NULL,
            global_skills_dir  TEXT NOT NULL,
            project_skills_dir TEXT,
            icon_name          TEXT,
            is_detected        BOOLEAN NOT NULL DEFAULT 0,
            is_builtin         BOOLEAN NOT NULL DEFAULT 1,
            is_enabled         BOOLEAN NOT NULL DEFAULT 1
        );
CREATE INDEX IF NOT EXISTS idx_skills_is_central
         ON skills(is_central);
CREATE INDEX IF NOT EXISTS idx_skills_is_central_name
         ON skills(is_central, name);
CREATE TABLE IF NOT EXISTS collections (
            id          TEXT PRIMARY KEY,
            name        TEXT NOT NULL,
            description TEXT,
            created_at  TEXT NOT NULL,
            updated_at  TEXT NOT NULL
        );
CREATE TABLE IF NOT EXISTS collection_skills (
            collection_id TEXT NOT NULL,
            skill_id      TEXT NOT NULL,
            added_at      TEXT NOT NULL,
            PRIMARY KEY (collection_id, skill_id)
        );
CREATE INDEX IF NOT EXISTS idx_collection_skills_skill_id
         ON collection_skills(skill_id);
CREATE TABLE IF NOT EXISTS skill_repositories (
            id          TEXT PRIMARY KEY,
            name        TEXT NOT NULL,
            source_type TEXT NOT NULL,
            owner       TEXT,
            repo        TEXT,
            branch      TEXT,
            url         TEXT,
            pinned      BOOLEAN NOT NULL DEFAULT 0,
            is_unknown  BOOLEAN NOT NULL DEFAULT 0,
            created_at  TEXT NOT NULL,
            updated_at  TEXT NOT NULL
        );
CREATE TABLE IF NOT EXISTS skill_repository_members (
            skill_id      TEXT PRIMARY KEY,
            repository_id TEXT NOT NULL,
            source_path   TEXT,
            added_at      TEXT NOT NULL,
            updated_at    TEXT NOT NULL
        );
CREATE INDEX IF NOT EXISTS idx_skill_repository_members_repository_skill_id
         ON skill_repository_members(repository_id, skill_id);
CREATE TABLE IF NOT EXISTS skill_repository_sync_skips (
            repository_id TEXT NOT NULL,
            source_path   TEXT NOT NULL,
            skill_id      TEXT NOT NULL,
            skill_name    TEXT NOT NULL,
            created_at    TEXT NOT NULL,
            updated_at    TEXT NOT NULL,
            last_seen_at  TEXT NOT NULL,
            PRIMARY KEY (repository_id, source_path)
        );
CREATE INDEX IF NOT EXISTS idx_skill_repository_sync_skips_repository_seen
         ON skill_repository_sync_skips(repository_id, last_seen_at DESC);
CREATE TABLE IF NOT EXISTS skill_update_states (
            skill_id           TEXT PRIMARY KEY,
            source_type        TEXT NOT NULL,
            source_url         TEXT,
            ref_name           TEXT,
            source_path        TEXT,
            last_remote_hash   TEXT,
            latest_remote_hash TEXT,
            last_checked_at    TEXT,
            last_updated_at    TEXT,
            status             TEXT NOT NULL,
            error              TEXT
        );
CREATE INDEX IF NOT EXISTS idx_skill_update_states_checked_skill
         ON skill_update_states(last_checked_at DESC, skill_id);
CREATE INDEX IF NOT EXISTS idx_skill_update_states_status_skill
         ON skill_update_states(status, skill_id);
CREATE TABLE IF NOT EXISTS skill_tag_groups (
            id          TEXT PRIMARY KEY,
            name        TEXT NOT NULL,
            color       TEXT,
            sort_order  INTEGER NOT NULL DEFAULT 0,
            is_builtin  BOOLEAN NOT NULL DEFAULT 0,
            created_at  TEXT NOT NULL,
            updated_at  TEXT NOT NULL
        );
CREATE INDEX IF NOT EXISTS idx_skill_tag_groups_sort_order
         ON skill_tag_groups(sort_order);
CREATE TABLE IF NOT EXISTS skill_tags (
            id          TEXT PRIMARY KEY,
            name        TEXT NOT NULL UNIQUE,
            description TEXT,
            color       TEXT,
            is_builtin  BOOLEAN NOT NULL DEFAULT 0,
            created_at  TEXT NOT NULL,
            updated_at  TEXT NOT NULL
        );
CREATE TABLE IF NOT EXISTS skill_tag_links (
            skill_id    TEXT NOT NULL,
            tag_id      TEXT NOT NULL,
            confidence  REAL,
            reason      TEXT,
            source      TEXT NOT NULL DEFAULT 'manual',
            added_at    TEXT NOT NULL,
            PRIMARY KEY (skill_id, tag_id)
        );
CREATE INDEX IF NOT EXISTS idx_skill_tag_links_tag_id
         ON skill_tag_links(tag_id);
CREATE TABLE IF NOT EXISTS skill_ai_tag_reviews (
            skill_id     TEXT NOT NULL,
            tag_id       TEXT NOT NULL,
            confidence   REAL NOT NULL,
            reason       TEXT,
            status       TEXT NOT NULL DEFAULT 'pending',
            suggested_at TEXT NOT NULL,
            updated_at   TEXT NOT NULL,
            PRIMARY KEY (skill_id, tag_id)
        );
CREATE INDEX IF NOT EXISTS idx_skill_ai_tag_reviews_status_updated_skill_tag
         ON skill_ai_tag_reviews(status, updated_at DESC, skill_id, tag_id);
CREATE TABLE IF NOT EXISTS skill_repository_pending_additions (
            repository_id              TEXT NOT NULL,
            source_path                TEXT NOT NULL,
            skill_id                   TEXT NOT NULL,
            skill_name                 TEXT NOT NULL,
            conflict_existing_skill_id TEXT,
            discovered_at              TEXT NOT NULL,
            PRIMARY KEY (repository_id, source_path)
        );
CREATE INDEX IF NOT EXISTS idx_skill_repository_pending_additions_repo
         ON skill_repository_pending_additions(repository_id, discovered_at DESC);
CREATE TABLE IF NOT EXISTS scan_directories (
            id         INTEGER PRIMARY KEY AUTOINCREMENT,
            path       TEXT NOT NULL UNIQUE,
            label      TEXT,
            is_active  BOOLEAN NOT NULL DEFAULT 1,
            is_builtin BOOLEAN NOT NULL DEFAULT 0,
            added_at   TEXT NOT NULL
        );
CREATE TABLE IF NOT EXISTS settings (
            key   TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );
CREATE TABLE IF NOT EXISTS operation_logs (
            id             TEXT PRIMARY KEY,
            created_at     TEXT NOT NULL,
            level          TEXT NOT NULL,
            target_kind    TEXT NOT NULL,
            target_id      TEXT NOT NULL,
            target_label   TEXT,
            category       TEXT NOT NULL,
            action         TEXT NOT NULL,
            status         TEXT NOT NULL,
            subject_type   TEXT,
            subject_id     TEXT,
            subject_label  TEXT,
            summary        TEXT NOT NULL,
            error_summary  TEXT,
            details_json   TEXT,
            duration_ms    INTEGER,
            batch_id       TEXT
        );
CREATE INDEX IF NOT EXISTS idx_operation_logs_created_at
         ON operation_logs(created_at);
CREATE INDEX IF NOT EXISTS idx_operation_logs_target
         ON operation_logs(target_kind, target_id);
CREATE INDEX IF NOT EXISTS idx_operation_logs_level_status
         ON operation_logs(level, status);
CREATE INDEX IF NOT EXISTS idx_operation_logs_action
         ON operation_logs(action);
CREATE INDEX IF NOT EXISTS idx_operation_logs_category
         ON operation_logs(category);
CREATE INDEX IF NOT EXISTS idx_operation_logs_batch_id
         ON operation_logs(batch_id);
CREATE TABLE IF NOT EXISTS skill_registries (
            id          TEXT PRIMARY KEY,
            name        TEXT NOT NULL,
            source_type TEXT NOT NULL,
            url         TEXT NOT NULL,
            is_builtin  BOOLEAN NOT NULL DEFAULT 0,
            is_enabled  BOOLEAN NOT NULL DEFAULT 1,
            last_synced TEXT,
            last_attempted_sync TEXT,
            last_sync_status TEXT NOT NULL DEFAULT 'never',
            last_sync_error TEXT,
            cache_updated_at TEXT,
            cache_expires_at TEXT,
            etag TEXT,
            last_modified TEXT,
            created_at  TEXT NOT NULL
        );
CREATE TABLE IF NOT EXISTS marketplace_skills (
            id           TEXT PRIMARY KEY,
            registry_id  TEXT NOT NULL,
            name         TEXT NOT NULL,
            description  TEXT,
            download_url TEXT NOT NULL,
            is_installed BOOLEAN NOT NULL DEFAULT 0,
            synced_at    TEXT NOT NULL,
            cache_updated_at TEXT,
            FOREIGN KEY (registry_id) REFERENCES skill_registries(id)
        );
CREATE TABLE IF NOT EXISTS skill_explanations (
            skill_id    TEXT NOT NULL,
            explanation TEXT NOT NULL,
            lang        TEXT NOT NULL DEFAULT 'zh',
            model       TEXT,
            created_at  TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at  TEXT NOT NULL DEFAULT (datetime('now')),
            PRIMARY KEY (skill_id, lang)
        );
CREATE TABLE IF NOT EXISTS skill_saved_views (
            id               TEXT PRIMARY KEY,
            name             TEXT NOT NULL,
            query            TEXT NOT NULL,
            sort_order       INTEGER NOT NULL DEFAULT 0,
            icon             TEXT,
            pinned           INTEGER NOT NULL DEFAULT 0,
            created_at       TEXT NOT NULL,
            updated_at       TEXT NOT NULL
        );
CREATE INDEX IF NOT EXISTS idx_skill_saved_views_order
         ON skill_saved_views(sort_order);
CREATE TABLE IF NOT EXISTS projects (
            id              TEXT PRIMARY KEY,
            path            TEXT NOT NULL UNIQUE,
            name            TEXT NOT NULL,
            pinned          BOOLEAN NOT NULL DEFAULT 0,
            added_at        TEXT NOT NULL,
            last_scanned_at TEXT
        );
CREATE TABLE IF NOT EXISTS project_skill_installations (
            project_id      TEXT NOT NULL,
            skill_id        TEXT NOT NULL,
            name            TEXT NOT NULL DEFAULT '',
            description     TEXT,
            file_path       TEXT NOT NULL DEFAULT '',
            source_origin   TEXT NOT NULL DEFAULT 'project',
            agent_id        TEXT NOT NULL,
            installed_path  TEXT NOT NULL,
            link_type       TEXT NOT NULL,
            symlink_target  TEXT,
            created_at      TEXT NOT NULL,
            PRIMARY KEY (project_id, skill_id, agent_id),
            FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
        );
CREATE INDEX IF NOT EXISTS idx_psi_project
         ON project_skill_installations(project_id);
CREATE TABLE IF NOT EXISTS skill_calls (
            id           INTEGER PRIMARY KEY AUTOINCREMENT,
            target_id    TEXT NOT NULL DEFAULT 'local',
            skill        TEXT NOT NULL,
            timestamp_ms INTEGER NOT NULL,
            project      TEXT NOT NULL,
            session_id   TEXT NOT NULL,
            source       TEXT NOT NULL
        );
CREATE INDEX IF NOT EXISTS idx_skill_calls_skill
         ON skill_calls(skill);
CREATE INDEX IF NOT EXISTS idx_skill_calls_source
         ON skill_calls(source);
CREATE INDEX IF NOT EXISTS idx_skill_calls_ts
         ON skill_calls(timestamp_ms);
CREATE INDEX IF NOT EXISTS idx_skill_calls_target
         ON skill_calls(target_id);
CREATE INDEX IF NOT EXISTS idx_skill_calls_target_ts
         ON skill_calls(target_id, timestamp_ms);
CREATE INDEX IF NOT EXISTS idx_skill_calls_target_skill_ts
         ON skill_calls(target_id, skill, timestamp_ms);
CREATE TABLE IF NOT EXISTS skill_call_providers (
            target_id    TEXT NOT NULL DEFAULT 'local',
            provider_id  TEXT NOT NULL,
            display_name TEXT NOT NULL,
            available    INTEGER NOT NULL,
            call_count   INTEGER NOT NULL DEFAULT 0,
            scanned_at   INTEGER NOT NULL,
            PRIMARY KEY (target_id, provider_id)
        );
CREATE TABLE IF NOT EXISTS skill_call_scan_state (
            target_id          TEXT NOT NULL PRIMARY KEY,
            last_full_scan_ms  INTEGER NOT NULL
        );


-- Migration sentinel data supported by every selected release schema.
INSERT INTO skills (id, name, file_path, is_central, scanned_at) VALUES ('fixture-skill', 'Fixture Skill', '/fixtures/fixture-skill/SKILL.md', 1, '2026-01-01T00:00:00Z');
INSERT INTO collections (id, name, created_at, updated_at) VALUES ('fixture-collection', 'Fixture Collection', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z');
INSERT INTO skill_repositories (id, name, source_type, is_unknown, created_at, updated_at) VALUES ('fixture-repository', 'Fixture Repository', 'github', 0, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z');
INSERT INTO skill_tags (id, name, created_at, updated_at) VALUES ('fixture-tag', 'Fixture Tag', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z');
INSERT INTO skill_update_states (skill_id, source_type, status) VALUES ('fixture-skill', 'github', 'up_to_date');
INSERT INTO skill_repository_members (skill_id, repository_id, source_path, added_at, updated_at) VALUES ('fixture-skill', 'fixture-repository', 'skills/fixture-skill', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z');
INSERT INTO collection_skills (collection_id, skill_id, added_at) VALUES ('fixture-collection', 'fixture-skill', '2026-01-01T00:00:00Z');
INSERT INTO skill_tag_links (skill_id, tag_id, source, added_at) VALUES ('fixture-skill', 'fixture-tag', 'manual', '2026-01-01T00:00:00Z');
INSERT INTO skill_ai_tag_reviews (skill_id, tag_id, confidence, status, suggested_at, updated_at) VALUES ('fixture-skill', 'fixture-tag', 0.9, 'pending', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z');
INSERT INTO skill_explanations (skill_id, explanation, lang) VALUES ('fixture-skill', 'Fixture explanation', 'en');
INSERT INTO skill_installations (skill_id, agent_id, installed_path, link_type) VALUES ('fixture-skill', 'codex', '/fixtures/codex/fixture-skill', 'symlink');
