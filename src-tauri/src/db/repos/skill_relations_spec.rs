//! Immutable owned-skill relation definitions consumed by migration 2.

#[derive(Debug, Clone, Copy)]
pub(crate) struct SkillRelationSpec {
    pub(crate) table: &'static str,
    pub(crate) skill_column: &'static str,
    pub(crate) copy_columns: &'static str,
    pub(crate) rebuild_sql: &'static str,
    pub(crate) index_sql: &'static [&'static str],
}

// Agent observations, project snapshots, usage history, and update inventory
// have separate lifecycles and intentionally stay outside this ownership list.
const OWNED_SKILL_RELATIONS: [SkillRelationSpec; 7] = [
    SkillRelationSpec {
        table: "skill_update_states",
        skill_column: "skill_id",
        copy_columns: "skill_id, source_type, source_url, ref_name, source_path, last_remote_hash, latest_remote_hash, last_checked_at, last_updated_at, status, error",
        rebuild_sql: "CREATE TABLE __skill_relation_rebuild (
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
            error              TEXT,
            FOREIGN KEY (skill_id) REFERENCES skills(id) ON DELETE CASCADE
        )",
        index_sql: &[
            "CREATE INDEX idx_skill_update_states_checked_skill ON skill_update_states(last_checked_at DESC, skill_id)",
            "CREATE INDEX idx_skill_update_states_status_skill ON skill_update_states(status, skill_id)",
        ],
    },
    SkillRelationSpec {
        table: "skill_repository_members",
        skill_column: "skill_id",
        copy_columns: "skill_id, repository_id, source_path, added_at, updated_at",
        rebuild_sql: "CREATE TABLE __skill_relation_rebuild (
            skill_id      TEXT PRIMARY KEY,
            repository_id TEXT NOT NULL,
            source_path   TEXT,
            added_at      TEXT NOT NULL,
            updated_at    TEXT NOT NULL,
            FOREIGN KEY (skill_id) REFERENCES skills(id) ON DELETE CASCADE
        )",
        index_sql: &[
            "CREATE INDEX idx_skill_repository_members_repository_skill_id ON skill_repository_members(repository_id, skill_id)",
        ],
    },
    SkillRelationSpec {
        table: "collection_skills",
        skill_column: "skill_id",
        copy_columns: "collection_id, skill_id, added_at",
        rebuild_sql: "CREATE TABLE __skill_relation_rebuild (
            collection_id TEXT NOT NULL,
            skill_id      TEXT NOT NULL,
            added_at      TEXT NOT NULL,
            PRIMARY KEY (collection_id, skill_id),
            FOREIGN KEY (skill_id) REFERENCES skills(id) ON DELETE CASCADE
        )",
        index_sql: &[
            "CREATE INDEX idx_collection_skills_skill_id ON collection_skills(skill_id)",
        ],
    },
    SkillRelationSpec {
        table: "skill_tag_links",
        skill_column: "skill_id",
        copy_columns: "skill_id, tag_id, confidence, reason, source, added_at",
        rebuild_sql: "CREATE TABLE __skill_relation_rebuild (
            skill_id   TEXT NOT NULL,
            tag_id     TEXT NOT NULL,
            confidence REAL,
            reason     TEXT,
            source     TEXT NOT NULL DEFAULT 'manual',
            added_at   TEXT NOT NULL,
            PRIMARY KEY (skill_id, tag_id),
            FOREIGN KEY (skill_id) REFERENCES skills(id) ON DELETE CASCADE
        )",
        index_sql: &["CREATE INDEX idx_skill_tag_links_tag_id ON skill_tag_links(tag_id)"],
    },
    SkillRelationSpec {
        table: "skill_ai_tag_reviews",
        skill_column: "skill_id",
        copy_columns: "skill_id, tag_id, confidence, reason, status, suggested_at, updated_at, proposed_name, proposed_description",
        rebuild_sql: "CREATE TABLE __skill_relation_rebuild (
            skill_id             TEXT NOT NULL,
            tag_id               TEXT NOT NULL,
            confidence           REAL NOT NULL,
            reason               TEXT,
            status               TEXT NOT NULL DEFAULT 'pending',
            suggested_at         TEXT NOT NULL,
            updated_at           TEXT NOT NULL,
            proposed_name        TEXT,
            proposed_description TEXT,
            PRIMARY KEY (skill_id, tag_id),
            FOREIGN KEY (skill_id) REFERENCES skills(id) ON DELETE CASCADE
        )",
        index_sql: &[
            "CREATE INDEX idx_skill_ai_tag_reviews_status_updated_skill_tag ON skill_ai_tag_reviews(status, updated_at DESC, skill_id, tag_id)",
        ],
    },
    SkillRelationSpec {
        table: "skill_explanations",
        skill_column: "skill_id",
        copy_columns: "skill_id, explanation, lang, model, created_at, updated_at",
        rebuild_sql: "CREATE TABLE __skill_relation_rebuild (
            skill_id    TEXT NOT NULL,
            explanation TEXT NOT NULL,
            lang        TEXT NOT NULL DEFAULT 'zh',
            model       TEXT,
            created_at  TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at  TEXT NOT NULL DEFAULT (datetime('now')),
            PRIMARY KEY (skill_id, lang),
            FOREIGN KEY (skill_id) REFERENCES skills(id) ON DELETE CASCADE
        )",
        index_sql: &[],
    },
    SkillRelationSpec {
        table: "skill_installations",
        skill_column: "skill_id",
        copy_columns: "skill_id, agent_id, installed_path, link_type, symlink_target, created_at",
        rebuild_sql: "CREATE TABLE __skill_relation_rebuild (
            skill_id       TEXT NOT NULL,
            agent_id       TEXT NOT NULL,
            installed_path TEXT NOT NULL,
            link_type      TEXT NOT NULL CHECK (link_type IN ('native', 'symlink', 'copy', 'writable')),
            symlink_target TEXT,
            created_at     TEXT NOT NULL DEFAULT (datetime('now')),
            PRIMARY KEY (skill_id, agent_id),
            FOREIGN KEY (skill_id) REFERENCES skills(id) ON DELETE CASCADE
        )",
        index_sql: &[
            "CREATE INDEX idx_skill_installations_agent_skill_id ON skill_installations(agent_id, skill_id)",
        ],
    },
];

pub(crate) fn owned_skill_relations() -> &'static [SkillRelationSpec] {
    &OWNED_SKILL_RELATIONS
}
