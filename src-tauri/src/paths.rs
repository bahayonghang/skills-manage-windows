use std::ffi::OsString;
use std::path::{Path, PathBuf};

pub const APP_DATA_DIR_NAME: &str = ".skillsmanage";
pub const CENTRAL_SKILLS_REL_FROM_HOME: &str = ".skillsmanage/skills";
pub const REMOTE_REPOS_REL_FROM_HOME: &str = ".skillsmanage/repos";
pub const TARGETS_CACHE_DIR_NAME: &str = "targets";
pub const UNIVERSAL_AGENTS_DIR_NAME: &str = ".agents";
pub const UNIVERSAL_SKILLS_REL: &str = ".agents/skills";
#[cfg(windows)]
const APP_DATABASE_FILE_NAME: &str = "db.sqlite";

/*
 * ========================================================================
 * 步骤1：解析用户家目录
 * ========================================================================
 * 目标：
 * 1) 统一后端家目录解析规则
 * 2) 在 Windows 下优先兼容 USERPROFILE 与 HOMEDRIVE/HOMEPATH，
 *    非 Windows 平台保持 HOME 优先
 */
pub fn resolve_home_dir() -> PathBuf {
    resolve_home_dir_from_env_vars(
        std::env::var_os("HOME"),
        std::env::var_os("USERPROFILE"),
        std::env::var_os("HOMEDRIVE"),
        std::env::var_os("HOMEPATH"),
    )
}

fn resolve_home_dir_from_env_vars(
    home: Option<OsString>,
    userprofile: Option<OsString>,
    homedrive: Option<OsString>,
    homepath: Option<OsString>,
) -> PathBuf {
    #[cfg(windows)]
    {
        // 1.1 Windows 优先使用 USERPROFILE
        if let Some(user_profile) = non_empty_os_env(userprofile) {
            return PathBuf::from(user_profile);
        }

        // 1.2 再回退到 HOMEDRIVE + HOMEPATH
        if let (Some(home_drive), Some(home_path)) =
            (non_empty_os_env(homedrive), non_empty_os_env(homepath))
        {
            return PathBuf::from(format!(
                "{}{}",
                home_drive.to_string_lossy(),
                home_path.to_string_lossy()
            ));
        }

        // 1.3 Windows 兼容 Git Bash / MSYS HOME，但只作为最后的 shell fallback
        if let Some(home) = non_empty_os_env(home) {
            return PathBuf::from(home);
        }

        // 1.4 全部缺失时保底回退到当前平台临时目录
        std::env::temp_dir()
    }

    #[cfg(not(windows))]
    {
        // 1.1 非 Windows 平台优先使用 HOME
        if let Some(home) = non_empty_os_env(home) {
            return PathBuf::from(home);
        }

        // 1.2 兼容 USERPROFILE / HOMEDRIVE+HOMEPATH 的跨平台测试夹具
        if let Some(user_profile) = non_empty_os_env(userprofile) {
            return PathBuf::from(user_profile);
        }

        if let (Some(home_drive), Some(home_path)) =
            (non_empty_os_env(homedrive), non_empty_os_env(homepath))
        {
            return PathBuf::from(format!(
                "{}{}",
                home_drive.to_string_lossy(),
                home_path.to_string_lossy()
            ));
        }

        // 1.3 全部缺失时保底回退到当前平台临时目录
        std::env::temp_dir()
    }
}

pub fn resolve_home_dir_with<F>(mut get_var: F) -> PathBuf
where
    F: FnMut(&str) -> Option<String>,
{
    resolve_home_dir_from_env_vars(
        get_var("HOME").map(OsString::from),
        get_var("USERPROFILE").map(OsString::from),
        get_var("HOMEDRIVE").map(OsString::from),
        get_var("HOMEPATH").map(OsString::from),
    )
}

/*
 * ========================================================================
 * 步骤2：构造技能存储目录
 * ========================================================================
 * 目标：
 * 1) 将 SkillPort 私有中央仓库隔离到 `~/.skillsmanage/skills`
 * 2) 保留 `~/.agents/skills` 作为 Universal Agents 安装目标
 */
pub fn central_skills_dir() -> PathBuf {
    // Keep Central store writes next to the selected app data DB. On Windows
    // this preserves an existing Git Bash HOME-backed DB/store until the user
    // explicitly moves it through Central store location preview/apply.
    app_data_dir().join("skills")
}

pub fn central_skills_dir_from_home(home_dir: &Path) -> PathBuf {
    // 2.1 在家目录下拼出 SkillPort 私有中央技能目录
    home_dir.join(".skillsmanage").join("skills")
}

pub fn universal_skills_dir() -> PathBuf {
    universal_skills_dir_from_home(&resolve_home_dir())
}

pub fn universal_skills_dir_from_home(home_dir: &Path) -> PathBuf {
    // 2.2 在家目录下拼出 Universal Agents 技能目录
    home_dir.join(".agents").join("skills")
}

#[derive(Debug, Clone, Copy)]
pub struct PlatformPathSpec<'a> {
    pub agent_id: &'a str,
    pub global_skills_dir: &'a str,
    pub project_skills_dir: Option<&'a str>,
}

/// A platform path lookup referenced an agent id with no registered
/// [`PlatformPathSpec`]. Display text preserves the historical string-error
/// wording verbatim.
#[derive(Debug, thiserror::Error)]
#[error("Agent '{0}' not found")]
pub struct AgentPathSpecNotFound(pub String);

pub fn platform_global_skills_dir(
    agent_id: &str,
    specs: &[PlatformPathSpec<'_>],
) -> Result<PathBuf, AgentPathSpecNotFound> {
    let spec = find_platform_path_spec(agent_id, specs)?;
    Ok(expand_home_path(spec.global_skills_dir))
}

pub fn platform_project_skills_dir(
    agent_id: &str,
    specs: &[PlatformPathSpec<'_>],
) -> Result<Option<PathBuf>, AgentPathSpecNotFound> {
    let spec = find_platform_path_spec(agent_id, specs)?;
    Ok(spec
        .project_skills_dir
        .and_then(non_empty_str)
        .map(|path| PathBuf::from(path.replace('\\', "/"))))
}

pub fn platform_global_skills_dir_for_remote(
    agent_id: &str,
    specs: &[PlatformPathSpec<'_>],
    remote_home: &str,
) -> Result<String, AgentPathSpecNotFound> {
    let spec = find_platform_path_spec(agent_id, specs)?;
    Ok(expand_remote_home_path(spec.global_skills_dir, remote_home))
}

pub fn platform_project_skills_dir_for_remote(
    agent_id: &str,
    specs: &[PlatformPathSpec<'_>],
    remote_home: &str,
) -> Result<Option<String>, AgentPathSpecNotFound> {
    let spec = find_platform_path_spec(agent_id, specs)?;
    Ok(spec
        .project_skills_dir
        .and_then(non_empty_str)
        .map(|path| expand_remote_home_path(path, remote_home).replace('\\', "/")))
}

fn find_platform_path_spec<'a>(
    agent_id: &str,
    specs: &'a [PlatformPathSpec<'a>],
) -> Result<&'a PlatformPathSpec<'a>, AgentPathSpecNotFound> {
    specs
        .iter()
        .find(|spec| spec.agent_id == agent_id)
        .ok_or_else(|| AgentPathSpecNotFound(agent_id.to_string()))
}

fn non_empty_str(value: &str) -> Option<&str> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

/*
 * ========================================================================
 * 步骤3：补充应用数据与展示路径工具
 * ========================================================================
 * 目标：
 * 1) 统一 `.skillsmanage` 数据目录解析
 * 2) 统一 `~` 展开与 Path -> String 转换
 */
pub fn app_data_dir() -> PathBuf {
    // 3.1 复用家目录规则构造应用数据目录；Windows 兼容旧 Git Bash HOME 数据目录
    app_data_dir_from_env_vars(
        std::env::var_os("HOME"),
        std::env::var_os("USERPROFILE"),
        std::env::var_os("HOMEDRIVE"),
        std::env::var_os("HOMEPATH"),
    )
}
pub fn central_mutation_lock_path() -> PathBuf {
    app_data_dir().join("locks").join("central-mutation.lock")
}

pub fn expand_home_path(path: &str) -> PathBuf {
    expand_home_path_with_home(path, &resolve_home_dir())
}

pub fn expand_remote_home_path(path: &str, remote_home: &str) -> String {
    let trimmed = path.trim();
    if trimmed == "~" {
        let home = remote_home.trim_end_matches('/');
        if home.is_empty() && remote_home.starts_with('/') {
            return "/".to_string();
        }
        return home.to_string();
    }

    if let Some(rest) = trimmed
        .strip_prefix("~/")
        .or_else(|| trimmed.strip_prefix("~\\"))
    {
        return remote_join_home(remote_home, rest);
    }

    trimmed.to_string()
}

pub fn path_to_string(path: &Path) -> String {
    // 3.2 用 lossy 规则统一路径序列化
    path.to_string_lossy().into_owned()
}

pub fn strip_windows_extended_path_prefix(path: &str) -> String {
    let normalized = path.replace('\\', "/");
    if normalized.len() >= "//?/UNC/".len()
        && normalized[.."//?/UNC/".len()].eq_ignore_ascii_case("//?/UNC/")
    {
        return format!("//{}", &normalized["//?/UNC/".len()..]);
    }

    if normalized.len() >= "//?/".len() && normalized[.."//?/".len()].eq_ignore_ascii_case("//?/") {
        return normalized["//?/".len()..].to_string();
    }

    normalized
}

pub fn normalize_stored_path(path: &str) -> String {
    let mut value = strip_windows_extended_path_prefix(path.trim());
    #[cfg(target_os = "macos")]
    {
        value = strip_macos_private_aliases(&value);
    }
    while value.len() > 1 && value.ends_with('/') {
        value.pop();
    }
    value
}

#[cfg(target_os = "macos")]
fn strip_macos_private_aliases(path: &str) -> String {
    const ALIASES: [(&str, &str); 3] = [
        ("/private/var", "/var"),
        ("/private/tmp", "/tmp"),
        ("/private/etc", "/etc"),
    ];

    for (private_root, public_root) in ALIASES {
        if let Some(rest) = path.strip_prefix(private_root) {
            if rest.is_empty() {
                return public_root.to_string();
            }
            if rest.starts_with('/') {
                return format!("{public_root}{rest}");
            }
        }
    }

    path.to_string()
}

pub fn paths_equivalent(left: &Path, right: &Path) -> bool {
    normalize_equivalence_path(left) == normalize_equivalence_path(right)
}

pub fn canonicalize_path_with_missing(path: &Path) -> PathBuf {
    if let Ok(resolved) = path.canonicalize() {
        return resolved;
    }
    match (path.parent(), path.file_name()) {
        (Some(parent), Some(file_name)) => canonicalize_path_with_missing(parent).join(file_name),
        _ => path.to_path_buf(),
    }
}

fn normalize_equivalence_path(path: &Path) -> String {
    let resolved = canonicalize_path_with_missing(path);
    let value = normalize_stored_path(&resolved.to_string_lossy());

    #[cfg(windows)]
    let value = value.to_ascii_lowercase();

    value
}

fn expand_home_path_with_home(path: &str, home_dir: &Path) -> PathBuf {
    let trimmed = path.trim();
    if trimmed == "~" {
        return home_dir.to_path_buf();
    }

    if let Some(rest) = trimmed
        .strip_prefix("~/")
        .or_else(|| trimmed.strip_prefix("~\\"))
    {
        return home_dir.join(rest);
    }

    PathBuf::from(trimmed)
}

fn app_data_dir_from_env_vars(
    home: Option<OsString>,
    userprofile: Option<OsString>,
    homedrive: Option<OsString>,
    homepath: Option<OsString>,
) -> PathBuf {
    let preferred_home = resolve_home_dir_from_env_vars(
        home.clone(),
        userprofile.clone(),
        homedrive.clone(),
        homepath.clone(),
    );
    let preferred_app_dir = app_data_dir_from_home(&preferred_home);

    #[cfg(windows)]
    {
        if preferred_app_dir.join(APP_DATABASE_FILE_NAME).exists() {
            return preferred_app_dir;
        }

        let legacy_home =
            resolve_windows_legacy_home_dir_from_env_vars(home, userprofile, homedrive, homepath);
        if !paths_equivalent(&preferred_home, &legacy_home) {
            let legacy_app_dir = app_data_dir_from_home(&legacy_home);
            if legacy_app_dir.join(APP_DATABASE_FILE_NAME).exists() {
                return legacy_app_dir;
            }
        }
    }

    preferred_app_dir
}

fn app_data_dir_from_home(home_dir: &Path) -> PathBuf {
    home_dir.join(APP_DATA_DIR_NAME)
}

#[cfg(windows)]
fn resolve_windows_legacy_home_dir_from_env_vars(
    home: Option<OsString>,
    userprofile: Option<OsString>,
    homedrive: Option<OsString>,
    homepath: Option<OsString>,
) -> PathBuf {
    if let Some(home) = non_empty_os_env(home) {
        return PathBuf::from(home);
    }

    if let Some(user_profile) = non_empty_os_env(userprofile) {
        return PathBuf::from(user_profile);
    }

    if let (Some(home_drive), Some(home_path)) =
        (non_empty_os_env(homedrive), non_empty_os_env(homepath))
    {
        return PathBuf::from(format!(
            "{}{}",
            home_drive.to_string_lossy(),
            home_path.to_string_lossy()
        ));
    }

    std::env::temp_dir()
}

fn non_empty_os_env(value: Option<OsString>) -> Option<OsString> {
    value.filter(|value| !value.to_string_lossy().trim().is_empty())
}

fn remote_join_home(remote_home: &str, child: &str) -> String {
    let home = remote_home.trim_end_matches('/');
    let child = child.trim_start_matches(['/', '\\']).replace('\\', "/");

    if child.is_empty() {
        return home.to_string();
    }

    if home.is_empty() || home == "/" {
        format!("/{}", child)
    } else {
        format!("{}/{}", home, child)
    }
}

pub fn remote_join(parent: &str, child: &str) -> String {
    let parent = parent.trim_end_matches('/');
    let child = child.trim_start_matches('/');
    if parent.is_empty() || parent == "/" {
        format!("/{}", child)
    } else if child.is_empty() {
        parent.to_string()
    } else {
        format!("{}/{}", parent, child)
    }
}

pub fn remote_central_skills_root(remote_home: &str) -> String {
    remote_join(remote_home, CENTRAL_SKILLS_REL_FROM_HOME)
}

pub fn remote_repos_root(remote_home: &str) -> String {
    remote_join(remote_home, REMOTE_REPOS_REL_FROM_HOME)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;
    #[cfg(any(windows, target_os = "macos"))]
    use tempfile::TempDir;

    #[cfg(windows)]
    #[test]
    fn resolve_home_dir_prefers_userprofile_on_windows_even_when_home_is_set() {
        let home = resolve_home_dir_with(|key| match key {
            "HOME" => Some("/custom/home".to_string()),
            "USERPROFILE" => Some(r"C:\Users\fallback".to_string()),
            "HOMEDRIVE" => Some("D:".to_string()),
            "HOMEPATH" => Some(r"\Users\drive-path".to_string()),
            _ => None,
        });

        assert_eq!(home, PathBuf::from(r"C:\Users\fallback"));
    }

    #[cfg(not(windows))]
    #[test]
    fn resolve_home_dir_prefers_home_on_non_windows() {
        let home = resolve_home_dir_with(|key| match key {
            "HOME" => Some("/custom/home".to_string()),
            "USERPROFILE" => Some(r"C:\Users\fallback".to_string()),
            "HOMEDRIVE" => Some("D:".to_string()),
            "HOMEPATH" => Some(r"\Users\drive-path".to_string()),
            _ => None,
        });

        assert_eq!(home, PathBuf::from("/custom/home"));
    }

    #[cfg(windows)]
    #[test]
    fn resolve_home_dir_uses_home_drive_and_path_before_home_on_windows() {
        let home = resolve_home_dir_with(|key| match key {
            "HOME" => Some("/custom/home".to_string()),
            "USERPROFILE" => None,
            "HOMEDRIVE" => Some("D:".to_string()),
            "HOMEPATH" => Some(r"\Users\drive-path".to_string()),
            _ => None,
        });

        assert_eq!(home, PathBuf::from(r"D:\Users\drive-path"));
    }

    #[test]
    fn resolve_home_dir_falls_back_to_userprofile() {
        let home = resolve_home_dir_with(|key| match key {
            "HOME" => None,
            "USERPROFILE" => Some(r"C:\Users\lyh".to_string()),
            "HOMEDRIVE" => Some("D:".to_string()),
            "HOMEPATH" => Some(r"\Users\drive-path".to_string()),
            _ => None,
        });

        assert_eq!(home, PathBuf::from(r"C:\Users\lyh"));
    }

    #[cfg(windows)]
    #[test]
    fn resolve_home_dir_falls_back_to_home_drive_and_path() {
        let home = resolve_home_dir_with(|key| match key {
            "HOME" => Some("/custom/home".to_string()),
            "USERPROFILE" => None,
            "HOMEDRIVE" => Some("D:".to_string()),
            "HOMEPATH" => Some(r"\Users\lyh".to_string()),
            _ => None,
        });

        assert_eq!(home, PathBuf::from(r"D:\Users\lyh"));
    }

    #[test]
    fn resolve_home_dir_uses_platform_temp_dir_as_last_resort() {
        let home = resolve_home_dir_with(|_| None);
        assert_eq!(home, std::env::temp_dir());
    }

    #[test]
    fn resolve_home_dir_accepts_os_string_env_values() {
        let home = resolve_home_dir_from_env_vars(
            None,
            Some(OsString::from(r"C:\Users\alice")),
            None,
            None,
        );

        assert_eq!(home, PathBuf::from(r"C:\Users\alice"));
    }

    #[test]
    fn central_skills_dir_is_built_under_home() {
        let central = central_skills_dir_from_home(Path::new(r"C:\Users\lyh"));
        assert_eq!(
            central,
            PathBuf::from(r"C:\Users\lyh")
                .join(".skillsmanage")
                .join("skills")
        );
    }

    #[test]
    fn universal_skills_dir_is_built_under_home() {
        let universal = universal_skills_dir_from_home(Path::new(r"C:\Users\lyh"));
        assert_eq!(
            universal,
            PathBuf::from(r"C:\Users\lyh")
                .join(".agents")
                .join("skills")
        );
    }

    #[test]
    fn app_data_dir_is_built_under_home() {
        let app_dir = app_data_dir();
        assert!(app_dir.ends_with(".skillsmanage"));
    }

    #[cfg(windows)]
    #[test]
    fn app_data_dir_uses_legacy_home_db_when_preferred_windows_profile_is_empty() {
        let temp = TempDir::new().unwrap();
        let preferred_home = temp.path().join("userprofile-home");
        let legacy_home = temp.path().join("git-bash-home");
        std::fs::create_dir_all(&preferred_home).unwrap();
        let legacy_app_dir = app_data_dir_from_home(&legacy_home);
        std::fs::create_dir_all(&legacy_app_dir).unwrap();
        std::fs::write(legacy_app_dir.join(APP_DATABASE_FILE_NAME), b"legacy-db").unwrap();

        let app_dir = app_data_dir_from_env_vars(
            Some(OsString::from(legacy_home.as_os_str())),
            Some(OsString::from(preferred_home.as_os_str())),
            None,
            None,
        );

        assert!(paths_equivalent(&app_dir, &legacy_app_dir));
    }

    #[cfg(windows)]
    #[test]
    fn app_data_dir_prefers_windows_profile_when_it_already_has_a_db() {
        let temp = TempDir::new().unwrap();
        let preferred_home = temp.path().join("userprofile-home");
        let legacy_home = temp.path().join("git-bash-home");
        let preferred_app_dir = app_data_dir_from_home(&preferred_home);
        let legacy_app_dir = app_data_dir_from_home(&legacy_home);
        std::fs::create_dir_all(&preferred_app_dir).unwrap();
        std::fs::create_dir_all(&legacy_app_dir).unwrap();
        std::fs::write(
            preferred_app_dir.join(APP_DATABASE_FILE_NAME),
            b"preferred-db",
        )
        .unwrap();
        std::fs::write(legacy_app_dir.join(APP_DATABASE_FILE_NAME), b"legacy-db").unwrap();

        let app_dir = app_data_dir_from_env_vars(
            Some(OsString::from(legacy_home.as_os_str())),
            Some(OsString::from(preferred_home.as_os_str())),
            None,
            None,
        );

        assert!(paths_equivalent(&app_dir, &preferred_app_dir));
    }

    #[test]
    fn expand_home_path_expands_unix_style_tilde() {
        let expanded = expand_home_path_with_home("~/.claude/skills", Path::new("/tmp/home"));
        assert_eq!(expanded, PathBuf::from("/tmp/home/.claude/skills"));
    }

    #[test]
    fn expand_home_path_expands_windows_style_tilde() {
        let expanded =
            expand_home_path_with_home("~\\.claude\\skills", Path::new("C:\\Users\\alice"));
        assert_eq!(expanded, PathBuf::from("C:\\Users\\alice/.claude\\skills"));
    }

    #[test]
    fn expand_home_path_leaves_absolute_paths_unchanged() {
        let expanded =
            expand_home_path_with_home("/opt/skills/custom", Path::new("/tmp/ignored-home"));
        assert_eq!(expanded, PathBuf::from("/opt/skills/custom"));
    }

    #[test]
    fn expand_remote_home_path_uses_posix_separators() {
        let expanded = expand_remote_home_path("~/.agents/skills", "/home/alice");
        assert_eq!(expanded, "/home/alice/.agents/skills");
    }

    #[test]
    fn remote_join_uses_posix_separators() {
        assert_eq!(
            remote_join("/home/alice", CENTRAL_SKILLS_REL_FROM_HOME),
            "/home/alice/.skillsmanage/skills"
        );
    }

    #[test]
    fn remote_join_handles_root_parent_and_empty_child() {
        assert_eq!(remote_join("/", "skills"), "/skills");
        assert_eq!(remote_join("", "skills"), "/skills");
        assert_eq!(remote_join("/home/alice", ""), "/home/alice");
    }

    #[test]
    fn remote_central_skills_root_matches_readme_semantics() {
        assert_eq!(
            remote_central_skills_root("/home/alice"),
            "/home/alice/.skillsmanage/skills"
        );
        assert_eq!(
            remote_central_skills_root("/home/alice/"),
            "/home/alice/.skillsmanage/skills"
        );
        assert_eq!(remote_central_skills_root("/"), "/.skillsmanage/skills");
    }

    #[test]
    fn remote_repos_root_matches_readme_semantics() {
        assert_eq!(
            remote_repos_root("/home/alice"),
            "/home/alice/.skillsmanage/repos"
        );
    }

    #[test]
    fn path_policy_dir_name_constants_are_pinned() {
        assert_eq!(APP_DATA_DIR_NAME, ".skillsmanage");
        assert_eq!(CENTRAL_SKILLS_REL_FROM_HOME, ".skillsmanage/skills");
        assert_eq!(REMOTE_REPOS_REL_FROM_HOME, ".skillsmanage/repos");
        assert_eq!(TARGETS_CACHE_DIR_NAME, "targets");
        assert_eq!(UNIVERSAL_AGENTS_DIR_NAME, ".agents");
        assert_eq!(UNIVERSAL_SKILLS_REL, ".agents/skills");
    }

    #[test]
    fn expand_remote_home_path_preserves_root_home() {
        let expanded = expand_remote_home_path("~", "/");
        assert_eq!(expanded, "/");
    }

    #[test]
    fn expand_remote_home_path_leaves_absolute_paths_unchanged() {
        let expanded = expand_remote_home_path("/opt/skills/custom", "/home/alice");
        assert_eq!(expanded, "/opt/skills/custom");
    }

    #[test]
    fn platform_global_skills_dir_resolves_agent_home_path() {
        let specs = [PlatformPathSpec {
            agent_id: "claude-code",
            global_skills_dir: "~/.claude/skills",
            project_skills_dir: Some(".claude/skills"),
        }];

        let resolved = platform_global_skills_dir("claude-code", &specs).unwrap();
        assert_eq!(resolved, expand_home_path("~/.claude/skills"));
    }

    #[test]
    fn platform_project_skills_dir_keeps_relative_pattern() {
        let specs = [PlatformPathSpec {
            agent_id: "claude-code",
            global_skills_dir: "~/.claude/skills",
            project_skills_dir: Some(".claude/skills"),
        }];

        let resolved = platform_project_skills_dir("claude-code", &specs)
            .unwrap()
            .unwrap();
        assert_eq!(resolved, PathBuf::from(".claude/skills"));
    }

    #[test]
    fn platform_paths_expand_remote_home_with_posix_separators() {
        let specs = [PlatformPathSpec {
            agent_id: "codex",
            global_skills_dir: "~/.codex/skills",
            project_skills_dir: Some("~\\.codex\\skills"),
        }];

        let global = platform_global_skills_dir_for_remote("codex", &specs, "/home/alice").unwrap();
        let project = platform_project_skills_dir_for_remote("codex", &specs, "/home/alice")
            .unwrap()
            .unwrap();
        assert_eq!(global, "/home/alice/.codex/skills");
        assert_eq!(project, "/home/alice/.codex/skills");
    }

    #[test]
    fn platform_path_lookup_errors_for_unknown_agent() {
        let specs = [PlatformPathSpec {
            agent_id: "known",
            global_skills_dir: "~/.known/skills",
            project_skills_dir: None,
        }];

        let error = platform_global_skills_dir("missing", &specs).unwrap_err();
        assert!(error.to_string().contains("missing"));
    }

    #[test]
    fn path_to_string_serializes_lossy_paths() {
        let path = Path::new(r"C:\Users\lyh\.agents\skills");
        assert_eq!(path_to_string(path), r"C:\Users\lyh\.agents\skills");
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn normalize_stored_path_collapses_private_system_aliases() {
        assert_eq!(
            normalize_stored_path("/private/var/folders/demo"),
            "/var/folders/demo"
        );
        assert_eq!(normalize_stored_path("/private/tmp/demo"), "/tmp/demo");
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn paths_equivalent_treats_private_system_aliases_as_same_location() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("demo");
        std::fs::create_dir_all(&path).unwrap();
        let canonical = path.canonicalize().unwrap();

        assert!(paths_equivalent(&path, &canonical));
    }

    #[test]
    fn paths_equivalent_canonicalizes_existing_ancestor_for_missing_descendants() {
        let temp = TempDir::new().unwrap();
        let canonical_root = temp.path().canonicalize().unwrap();
        assert!(paths_equivalent(
            &temp.path().join("missing/child"),
            &canonical_root.join("missing/child"),
        ));
    }

    #[test]
    fn paths_equivalent_ignores_trailing_separator() {
        assert!(paths_equivalent(
            Path::new(r"C:\Users\lyh\.agents\skills\"),
            Path::new(r"C:\Users\lyh\.agents\skills")
        ));
    }
}
