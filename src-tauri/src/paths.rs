use std::ffi::OsString;
use std::path::{Path, PathBuf};

/*
 * ========================================================================
 * 步骤1：解析用户家目录
 * ========================================================================
 * 目标：
 * 1) 统一后端家目录解析规则
 * 2) 在 Windows 下优先兼容 USERPROFILE 与 HOMEDRIVE/HOMEPATH
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
    // 1.1 优先使用 HOME
    if let Some(home) = non_empty_os_env(home) {
        return PathBuf::from(home);
    }

    // 1.2 Windows 环境优先回退到 USERPROFILE
    if let Some(user_profile) = non_empty_os_env(userprofile) {
        return PathBuf::from(user_profile);
    }

    // 1.3 最后尝试 HOMEDRIVE + HOMEPATH
    if let (Some(home_drive), Some(home_path)) =
        (non_empty_os_env(homedrive), non_empty_os_env(homepath))
    {
        return PathBuf::from(format!(
            "{}{}",
            home_drive.to_string_lossy(),
            home_path.to_string_lossy()
        ));
    }

    // 1.4 全部缺失时保底回退到当前平台临时目录
    std::env::temp_dir()
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
    central_skills_dir_from_home(&resolve_home_dir())
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

pub fn platform_global_skills_dir(
    agent_id: &str,
    specs: &[PlatformPathSpec<'_>],
) -> Result<PathBuf, String> {
    let spec = find_platform_path_spec(agent_id, specs)?;
    Ok(expand_home_path(spec.global_skills_dir))
}

pub fn platform_project_skills_dir(
    agent_id: &str,
    specs: &[PlatformPathSpec<'_>],
) -> Result<Option<PathBuf>, String> {
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
) -> Result<String, String> {
    let spec = find_platform_path_spec(agent_id, specs)?;
    Ok(expand_remote_home_path(spec.global_skills_dir, remote_home))
}

pub fn platform_project_skills_dir_for_remote(
    agent_id: &str,
    specs: &[PlatformPathSpec<'_>],
    remote_home: &str,
) -> Result<Option<String>, String> {
    let spec = find_platform_path_spec(agent_id, specs)?;
    Ok(spec
        .project_skills_dir
        .and_then(non_empty_str)
        .map(|path| expand_remote_home_path(path, remote_home).replace('\\', "/")))
}

fn find_platform_path_spec<'a>(
    agent_id: &str,
    specs: &'a [PlatformPathSpec<'a>],
) -> Result<&'a PlatformPathSpec<'a>, String> {
    specs
        .iter()
        .find(|spec| spec.agent_id == agent_id)
        .ok_or_else(|| format!("Agent '{}' not found", agent_id))
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
    // 3.1 复用家目录规则构造应用数据目录
    resolve_home_dir().join(".skillsmanage")
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

pub fn paths_equivalent(left: &Path, right: &Path) -> bool {
    normalize_equivalence_path(left) == normalize_equivalence_path(right)
}

fn normalize_equivalence_path(path: &Path) -> String {
    let resolved = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let mut value = resolved.to_string_lossy().replace('\\', "/");
    while value.len() > 1 && value.ends_with('/') {
        value.pop();
    }

    #[cfg(windows)]
    value.make_ascii_lowercase();

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

fn non_empty_os_env(value: Option<OsString>) -> Option<OsString> {
    value.and_then(|value| {
        if value.to_string_lossy().trim().is_empty() {
            None
        } else {
            Some(value)
        }
    })
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;

    #[test]
    fn resolve_home_dir_prefers_home() {
        let home = resolve_home_dir_with(|key| match key {
            "HOME" => Some("/custom/home".to_string()),
            "USERPROFILE" => Some(r"C:\Users\fallback".to_string()),
            "HOMEDRIVE" => Some("D:".to_string()),
            "HOMEPATH" => Some(r"\Users\drive-path".to_string()),
            _ => None,
        });

        assert_eq!(home, PathBuf::from("/custom/home"));
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

    #[test]
    fn resolve_home_dir_falls_back_to_home_drive_and_path() {
        let home = resolve_home_dir_with(|key| match key {
            "HOME" => None,
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
        assert!(error.contains("missing"));
    }

    #[test]
    fn path_to_string_serializes_lossy_paths() {
        let path = Path::new(r"C:\Users\lyh\.agents\skills");
        assert_eq!(path_to_string(path), r"C:\Users\lyh\.agents\skills");
    }

    #[test]
    fn paths_equivalent_ignores_trailing_separator() {
        assert!(paths_equivalent(
            Path::new(r"C:\Users\lyh\.agents\skills\"),
            Path::new(r"C:\Users\lyh\.agents\skills")
        ));
    }
}
