import re
import os

os.chdir(
    r"D:\Documents\Code\Agents\skills-manage-windows\src-tauri\src\services\central_updates\inventory"
)


def rw(path, fn):
    s = open(path, encoding="utf-8").read()
    s2 = fn(s)
    open(path, "w", encoding="utf-8", newline="\n").write(s2)


# ---------- repositories.rs ----------
def repositories(s):
    s = s.replace(
        "use crate::services::central_updates;\nuse crate::services::github_import::{self, GitHubRepoRef};\nuse crate::db::{self, DbPool, SkillRepository, SkillUpdateState};",
        "use crate::db::{self, DbPool, SkillRepository, SkillUpdateState};\nuse crate::services::central_updates::{\n    CentralRemoteAddedSkill, CentralUpdatesError, PreparedSkillUpdate,\n};\nuse crate::services::github_import::{self, GitHubRepoRef};",
    )
    s = s.replace(
        "prepared: &central_updates::PreparedSkillUpdate,",
        "prepared: &PreparedSkillUpdate,",
    )
    s = s.replace(
        "    item: central_updates::CentralRemoteAddedSkill,",
        "    item: CentralRemoteAddedSkill,",
    )
    s = s.replace(
        ") -> Result<Vec<(SkillRepository, GitHubRepoRef)>, String> {",
        ") -> Result<Vec<(SkillRepository, GitHubRepoRef)>, CentralUpdatesError> {",
    )
    s = s.replace(
        "        let Some(repository) = db::get_skill_repository_by_id(pool, repository_id)\n            .await\n            .map_err(|e| e.to_string())?\n        else {\n            continue;\n        };",
        "        let Some(repository) = db::get_skill_repository_by_id(pool, repository_id).await? else {\n            continue;\n        };",
    )
    s = s.replace(
        "            github_import::resolve_repo_source(&url, auth_token)\n                .await\n                .map_err(|e| e.to_string())?\n                .repo",
        "            github_import::resolve_repo_source(&url, auth_token).await?.repo",
    )
    return s


rw("repositories.rs", repositories)


# ---------- scan.rs ----------
def scan(s):
    s = s.replace(
        "use crate::db::{self, Agent, AgentSkillObservation, DbPool, SkillInstallation};",
        "use crate::db::{self, Agent, AgentSkillObservation, DbPool, SkillInstallation};\nuse crate::services::central_updates::CentralUpdatesError;",
    )
    s = s.replace(
        ") -> Result<Vec<PlatformDuplicateGroup>, String> {",
        ") -> Result<Vec<PlatformDuplicateGroup>, CentralUpdatesError> {",
    )
    s = s.replace(
        ") -> Result<Vec<DeletedPlatformCopyGroup>, String> {",
        ") -> Result<Vec<DeletedPlatformCopyGroup>, CentralUpdatesError> {",
    )
    s = s.replace(
        "    let agents = db::get_all_agents(pool).await.map_err(|e| e.to_string())?;",
        "    let agents = db::get_all_agents(pool).await?;",
    )
    s = s.replace(
        "        let observations = db::get_agent_skill_observations(pool, &agent.id)\n            .await\n            .map_err(|e| e.to_string())?;",
        "        let observations = db::get_agent_skill_observations(pool, &agent.id).await?;",
    )
    s = s.replace(
        "    let central_skill_ids = db::get_central_skills(pool)\n        .await\n        .map_err(|e| e.to_string())?\n        .into_iter()",
        "    let central_skill_ids = db::get_central_skills(pool)\n        .await?\n        .into_iter()",
    )
    s = s.replace(
        "        .bind(&agent.id)\n        .fetch_all(pool)\n        .await\n        .map_err(|e| e.to_string())?;",
        "        .bind(&agent.id)\n        .fetch_all(pool)\n        .await?;",
    )
    s = s.replace(
        "async fn deleted_installation_skill_name(pool: &DbPool, skill_id: &str) -> Result<String, String> {\n    Ok(db::get_skill_by_id(pool, skill_id)\n        .await\n        .map_err(|e| e.to_string())?\n        .map(|skill| skill.name)\n        .unwrap_or_else(|| skill_id.to_string()))\n}",
        "async fn deleted_installation_skill_name(\n    pool: &DbPool,\n    skill_id: &str,\n) -> Result<String, CentralUpdatesError> {\n    Ok(db::get_skill_by_id(pool, skill_id)\n        .await?\n        .map(|skill| skill.name)\n        .unwrap_or_else(|| skill_id.to_string()))\n}",
    )
    return s


rw("scan.rs", scan)


# ---------- scope.rs ----------
def scope(s):
    s = s.replace(
        "use crate::db::{self, DbPool};",
        "use crate::db::{self, DbPool};\nuse crate::services::central_updates::CentralUpdatesError;",
    )
    s = s.replace(
        "    ) -> Result<Self, String> {",
        "    ) -> Result<Self, CentralUpdatesError> {",
    )
    s = s.replace(
        ") -> Result<Vec<String>, String> {",
        ") -> Result<Vec<String>, CentralUpdatesError> {",
    )
    s = s.replace(
        "    let central_skill_ids = db::get_central_skills(pool)\n        .await\n        .map_err(|e| e.to_string())?\n        .into_iter()",
        "    let central_skill_ids = db::get_central_skills(pool)\n        .await?\n        .into_iter()",
    )
    s = s.replace(
        "        for observation in db::get_agent_skill_observations(pool, agent_id)\n            .await\n            .map_err(|e| e.to_string())?\n        {",
        "        for observation in db::get_agent_skill_observations(pool, agent_id).await? {",
    )
    s = s.replace(
        "        .bind(agent_id)\n        .fetch_all(pool)\n        .await\n        .map_err(|e| e.to_string())?;",
        "        .bind(agent_id)\n        .fetch_all(pool)\n        .await?;",
    )
    return s


rw("scope.rs", scope)


# ---------- persistence.rs ----------
def persistence(s):
    s = s.replace(
        "use crate::db::{self, DbPool, SkillUpdateInventoryEntry};",
        "use crate::db::{self, DbPool, SkillUpdateInventoryEntry};\nuse crate::services::central_updates::CentralUpdatesError;",
    )
    s = s.replace(") -> Result<(), String> {", ") -> Result<(), CentralUpdatesError> {")
    s = s.replace(
        ") -> Result<SkillUpdateInventoryEntry, String> {",
        ") -> Result<SkillUpdateInventoryEntry, CentralUpdatesError> {",
    )
    s = s.replace(
        ") -> Result<EntryInventory, String> {",
        ") -> Result<EntryInventory, CentralUpdatesError> {",
    )
    s = s.replace(
        ") -> Result<Option<String>, String> {",
        ") -> Result<Option<String>, CentralUpdatesError> {",
    )
    s = s.replace(
        "    db::replace_skill_update_inventory(pool, &run, &entries)\n        .await\n        .map_err(|e| e.to_string())\n}",
        "    db::replace_skill_update_inventory(pool, &run, &entries).await?;\n    Ok(())\n}",
    )
    s = s.replace(
        "payload_json: serde_json::to_string(payload).map_err(|e| e.to_string())?,",
        "payload_json: serde_json::to_string(payload)\n            .map_err(|e| CentralUpdatesError::Json(e.to_string()))?,",
    )
    s = s.replace(
        ".map_err(|e| e.to_string())?,\n            ),",
        ".map_err(|e| CentralUpdatesError::Json(e.to_string()))?,\n            ),",
    )
    s = s.replace(
        "    serde_json::to_string(&normalize_ids(ids))\n        .map(Some)\n        .map_err(|e| e.to_string())\n}",
        "    serde_json::to_string(&normalize_ids(ids))\n        .map(Some)\n        .map_err(|e| CentralUpdatesError::Json(e.to_string()))\n}",
    )
    return s


rw("persistence.rs", persistence)


# ---------- view.rs ----------
def view(s):
    s = s.replace(
        ") -> Result<SkillUpdateInventory, String> {",
        ") -> Result<SkillUpdateInventory, CentralUpdatesError> {",
    )
    s = s.replace(") -> Result<(), String> {", ") -> Result<(), CentralUpdatesError> {")
    s = re.sub(r"\.await\n\s*\.map_err\(\|e\| e\.to_string\(\)\)\?", ".await?", s)
    return s


rw("view.rs", view)

print("all done")
