//! skills.sh Marketplace integration: search, GitHub-backed preview,
//! remote file browsing, and full-directory installation.

use futures_util::stream::{self, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Component, Path};

use crate::secrets::SecretStore;
use crate::services::github_import;
use crate::targets::ActiveTarget;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct SkillsShSkill {
    pub id: String,
    pub skill_id: String,
    pub name: String,
    pub source: String,
    pub installs: u64,
    pub stars: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct SkillsShFileEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
}

// ─── skills.sh Integration ───────────────────────────────────────────────────

const SKILLS_SH_SEARCH_URL: &str = "https://skills.sh/api/search";
const SKILLS_SH_DEFAULT_LIMIT: u32 = 20;
const SKILLS_SH_MAX_LIMIT: u32 = 50;
const SKILLS_SH_STARS_CONCURRENCY: usize = 4;

#[derive(Debug, Deserialize)]
struct SkillsShSearchResponse {
    #[serde(default)]
    skills: Vec<SkillsShSearchItem>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SkillsShSearchItem {
    id: Option<String>,
    skill_id: Option<String>,
    name: Option<String>,
    source: Option<String>,
    #[serde(default)]
    installs: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct GitHubRepoMetadata {
    stargazers_count: Option<u64>,
}

pub async fn search_skills_sh_impl(
    pool: &crate::db::DbPool,
    secrets: &dyn SecretStore,
    query: String,
    limit: Option<u32>,
) -> Result<Vec<SkillsShSkill>, String> {
    let trimmed_query = query.trim();
    if trimmed_query.is_empty() {
        return Ok(Vec::new());
    }

    let bounded_limit = limit
        .unwrap_or(SKILLS_SH_DEFAULT_LIMIT)
        .clamp(1, SKILLS_SH_MAX_LIMIT);
    let client = github_import::github_client()?;
    let response = client
        .get(SKILLS_SH_SEARCH_URL)
        .query(&[
            ("q", trimmed_query.to_string()),
            ("limit", bounded_limit.to_string()),
        ])
        .send()
        .await
        .map_err(|e| format!("skills.sh search failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("skills.sh returned HTTP {}", response.status()));
    }

    let payload = response
        .json::<SkillsShSearchResponse>()
        .await
        .map_err(|e| format!("Failed to parse skills.sh search results: {}", e))?;
    let mut skills = payload
        .skills
        .into_iter()
        .filter_map(skills_sh_skill_from_search_item)
        .collect::<Vec<_>>();

    let auth = github_import::github_direct_auth_from_secret_store(pool, secrets).await?;
    enrich_skills_sh_stars(&client, &mut skills, auth.as_deref()).await;

    Ok(skills)
}

fn skills_sh_skill_from_search_item(item: SkillsShSearchItem) -> Option<SkillsShSkill> {
    let id = item.id?;
    let skill_id = item.skill_id?;
    let name = item.name?;
    let source = item.source?;
    is_valid_github_owner_repo(&source).then_some(SkillsShSkill {
        id,
        skill_id,
        name,
        source,
        installs: item.installs.unwrap_or(0),
        stars: None,
    })
}

async fn enrich_skills_sh_stars(
    client: &reqwest::Client,
    skills: &mut [SkillsShSkill],
    auth: Option<&str>,
) {
    let sources = skills
        .iter()
        .filter(|skill| is_valid_github_owner_repo(&skill.source))
        .map(|skill| skill.source.clone())
        .collect::<HashSet<_>>();
    if sources.is_empty() {
        return;
    }

    let star_map = stream::iter(sources)
        .map(|source| async move {
            let stars = fetch_github_star_count(client, &source, auth).await?;
            Some((source, stars))
        })
        .buffer_unordered(SKILLS_SH_STARS_CONCURRENCY)
        .filter_map(|entry| async move { entry })
        .collect::<HashMap<_, _>>()
        .await;

    for skill in skills {
        skill.stars = star_map.get(&skill.source).copied();
    }
}

async fn fetch_github_star_count(
    client: &reqwest::Client,
    source: &str,
    auth: Option<&str>,
) -> Option<u64> {
    if !is_valid_github_owner_repo(source) {
        return None;
    }
    let url = format!("https://api.github.com/repos/{source}");
    let mut request = client.get(&url);
    if let Some(token) = auth.filter(|token| !token.trim().is_empty()) {
        request = request.bearer_auth(token);
    }
    let response = request.send().await.ok()?;
    let response = if auth.is_some() && response.status() == reqwest::StatusCode::UNAUTHORIZED {
        client.get(&url).send().await.ok()?
    } else {
        response
    };
    if !response.status().is_success() {
        return None;
    }
    response
        .json::<GitHubRepoMetadata>()
        .await
        .ok()?
        .stargazers_count
}

pub async fn resolve_skills_sh_url_impl(
    pool: &crate::db::DbPool,
    secrets: &dyn SecretStore,
    source: String,
    skill_id: String,
) -> Result<String, String> {
    let auth = github_import::github_direct_auth_from_secret_store(pool, secrets).await?;
    let (resolved, auth_used) = resolved_skills_sh_source_with_auth(&source, &auth).await?;
    let candidate =
        resolve_skills_sh_candidate(&resolved.repo, &skill_id, auth_used.as_deref()).await?;
    Ok(candidate.download_url)
}

pub async fn browse_skills_sh_directory_impl(
    pool: &crate::db::DbPool,
    secrets: &dyn SecretStore,
    source: String,
    skill_id: String,
) -> Result<Vec<SkillsShFileEntry>, String> {
    let auth = github_import::github_direct_auth_from_secret_store(pool, secrets).await?;
    let (resolved, snapshot, _) = skills_sh_snapshot_with_auth(&source, &auth).await?;
    let candidate =
        resolve_skills_sh_candidate_from_snapshot(&resolved.repo, &snapshot, &skill_id)?;
    Ok(skills_sh_file_entries_from_snapshot(
        &snapshot,
        &candidate.source_path,
    ))
}

pub async fn read_skills_sh_file_impl(
    pool: &crate::db::DbPool,
    secrets: &dyn SecretStore,
    source: String,
    file_path: String,
) -> Result<String, String> {
    let auth = github_import::github_direct_auth_from_secret_store(pool, secrets).await?;
    let (resolved, auth_used) = resolved_skills_sh_source_with_auth(&source, &auth).await?;
    let normalized_path = normalize_skills_sh_file_path(&file_path)?;
    let client = github_import::github_client()?;
    let url = format!(
        "https://raw.githubusercontent.com/{}/{}/{}/{}",
        resolved.repo.owner, resolved.repo.repo, resolved.repo.branch, normalized_path
    );
    github_import::fetch_raw_text(&client, &url, auth_used.as_deref()).await
}

pub async fn install_from_skills_sh_impl(
    pool: &crate::db::DbPool,
    secrets: &dyn SecretStore,
    active_target: ActiveTarget,
    source: String,
    skill_id: String,
) -> Result<String, String> {
    let auth = github_import::github_direct_auth_from_secret_store(pool, secrets).await?;
    let (resolved, snapshot, auth_used) = skills_sh_snapshot_with_auth(&source, &auth).await?;
    let candidate =
        resolve_skills_sh_candidate_from_snapshot(&resolved.repo, &snapshot, &skill_id)?;
    let selection = github_import::GitHubSkillImportSelection {
        source_path: candidate.source_path.clone(),
        resolution: github_import::DuplicateResolution::Overwrite,
        renamed_skill_id: None,
    };

    match &active_target {
        ActiveTarget::Local => {
            let inspected = github_import::InspectedGitHubRepoSkills {
                repo: resolved.repo.clone(),
                valid_candidates: vec![candidate.clone()],
                invalid_candidates: Vec::new(),
            };
            let central_root = github_import::central_skills_root(pool).await?;
            std::fs::create_dir_all(&central_root)
                .map_err(|e| format!("Failed to create central skills directory: {}", e))?;
            let result = github_import::import_github_repo_skills_from_snapshot_partially(
                pool,
                &resolved.repo,
                &snapshot,
                inspected,
                vec![selection],
                &central_root,
                None,
            )
            .await?;
            let imported = result
                .imported_skills
                .first()
                .ok_or_else(|| format!("Skill '{}' was not imported.", skill_id))?;
            Ok(imported.imported_skill_id.clone())
        }
        ActiveTarget::Ssh(_) | ActiveTarget::Wsl(_) => {
            let repo_url = source_to_github_url(&source)?;
            let result = github_import::import_github_repo_skills_remote_with_auth(
                pool,
                &active_target,
                &repo_url,
                vec![selection],
                None,
                None,
                auth_used.as_deref(),
            )
            .await?;
            let imported = result
                .imported_skills
                .first()
                .ok_or_else(|| format!("Skill '{}' was not imported.", skill_id))?;
            Ok(imported.imported_skill_id.clone())
        }
    }
}

async fn skills_sh_snapshot_with_auth(
    source: &str,
    auth: &Option<String>,
) -> Result<
    (
        github_import::ResolvedGitHubRepoSource,
        github_import::GitHubRepoSnapshot,
        Option<String>,
    ),
    String,
> {
    let (resolved, auth_used) = resolved_skills_sh_source_with_auth(source, auth).await?;
    let client = github_import::github_client()?;
    let snapshot =
        github_import::download_repo_snapshot(&client, &resolved.repo, auth_used.as_deref())
            .await?;
    Ok((resolved, snapshot, auth_used))
}

async fn resolved_skills_sh_source_with_auth(
    source: &str,
    auth: &Option<String>,
) -> Result<(github_import::ResolvedGitHubRepoSource, Option<String>), String> {
    let repo_url = source_to_github_url(source)?;
    match github_import::resolve_repo_source(&repo_url, auth.as_deref()).await {
        Ok(resolved) => Ok((resolved, auth.clone())),
        Err(auth_error) if auth.is_some() => {
            let resolved = github_import::resolve_repo_source(&repo_url, None)
                .await
                .map_err(|_| auth_error)?;
            Ok((resolved, None))
        }
        Err(error) => Err(error),
    }
}

async fn resolve_skills_sh_candidate(
    repo: &github_import::GitHubRepoRef,
    skill_id: &str,
    auth: Option<&str>,
) -> Result<github_import::RemoteSkillCandidate, String> {
    let client = github_import::github_client()?;
    let snapshot = github_import::download_repo_snapshot(&client, repo, auth).await?;
    resolve_skills_sh_candidate_from_snapshot(repo, &snapshot, skill_id)
}

pub(crate) fn resolve_skills_sh_candidate_from_snapshot(
    repo: &github_import::GitHubRepoRef,
    snapshot: &github_import::GitHubRepoSnapshot,
    skill_id: &str,
) -> Result<github_import::RemoteSkillCandidate, String> {
    let normalized_skill_id = normalize_skills_sh_skill_id(skill_id)?;
    let candidates =
        github_import::build_repo_skill_candidates_from_snapshot_at_path(repo, snapshot, None)?;

    let mut fallback_match = None;
    for candidate in candidates {
        if candidate
            .skill_id
            .eq_ignore_ascii_case(&normalized_skill_id)
            || candidate
                .skill_directory_name
                .eq_ignore_ascii_case(&normalized_skill_id)
        {
            return Ok(candidate);
        }
        if fallback_match.is_none()
            && candidate
                .source_path
                .rsplit('/')
                .next()
                .is_some_and(|segment| segment.eq_ignore_ascii_case(&normalized_skill_id))
        {
            fallback_match = Some(candidate);
        }
    }

    fallback_match.ok_or_else(|| {
        format!(
            "Could not find SKILL.md for '{}' in {}/{}",
            skill_id, repo.owner, repo.repo
        )
    })
}

pub(crate) fn skills_sh_file_entries_from_snapshot(
    snapshot: &github_import::GitHubRepoSnapshot,
    source_path: &str,
) -> Vec<SkillsShFileEntry> {
    let base = normalize_repo_path_for_marketplace(source_path).unwrap_or_default();
    let prefix = if base.is_empty() {
        String::new()
    } else {
        format!("{base}/")
    };
    let mut paths = BTreeMap::<String, bool>::new();

    for file_path in snapshot.files.keys() {
        let Ok(path) = normalize_repo_path_for_marketplace(file_path) else {
            continue;
        };
        let relative = if base.is_empty() {
            Some(path.as_str())
        } else {
            path.strip_prefix(&prefix)
        };
        let Some(relative) = relative else {
            continue;
        };
        if relative.is_empty() {
            continue;
        }

        let mut current = base.clone();
        let segments = relative
            .split('/')
            .filter(|segment| !segment.is_empty())
            .collect::<Vec<_>>();
        for (index, segment) in segments.iter().enumerate() {
            current = if current.is_empty() {
                (*segment).to_string()
            } else {
                format!("{current}/{segment}")
            };
            let is_dir = index + 1 < segments.len();
            paths.entry(current.clone()).or_insert(is_dir);
        }
    }

    paths
        .into_iter()
        .map(|(path, is_dir)| SkillsShFileEntry {
            name: path
                .rsplit('/')
                .next()
                .filter(|value| !value.is_empty())
                .unwrap_or(&path)
                .to_string(),
            path,
            is_dir,
        })
        .collect()
}

pub(crate) fn source_to_github_url(source: &str) -> Result<String, String> {
    let trimmed = source.trim().trim_matches('/');
    if !is_valid_github_owner_repo(trimmed) {
        return Err("skills.sh source must be a GitHub owner/repo value.".to_string());
    }
    Ok(format!("https://github.com/{trimmed}"))
}

fn is_valid_github_owner_repo(value: &str) -> bool {
    let segments = value.split('/').collect::<Vec<_>>();
    if segments.len() != 2 {
        return false;
    }
    segments.iter().all(|segment| {
        !segment.is_empty()
            && !segment.starts_with('.')
            && segment
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
            && !segment.contains("..")
    })
}

fn normalize_skills_sh_skill_id(skill_id: &str) -> Result<String, String> {
    let trimmed = skill_id.trim();
    if trimmed.is_empty() || trimmed.contains('/') || trimmed.contains('\\') || trimmed == "." {
        return Err("skills.sh skill id is not supported.".to_string());
    }
    Ok(trimmed.to_string())
}

fn normalize_skills_sh_file_path(path: &str) -> Result<String, String> {
    let normalized = normalize_repo_path_for_marketplace(path)?;
    if normalized.is_empty() {
        return Err("skills.sh file path is required.".to_string());
    }
    Ok(normalized)
}

fn normalize_repo_path_for_marketplace(path: &str) -> Result<String, String> {
    let normalized = path.trim().trim_matches('/').replace('\\', "/");
    if normalized.is_empty() || normalized == "." {
        return Ok(String::new());
    }
    let relative = Path::new(&normalized);
    if relative.is_absolute()
        || !relative
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
    {
        return Err(format!("Repository path '{}' is not supported.", path));
    }
    Ok(normalized)
}
