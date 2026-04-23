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
    resolve_home_dir_with(|key| std::env::var(key).ok())
}

pub fn resolve_home_dir_with<F>(mut get_var: F) -> PathBuf
where
    F: FnMut(&str) -> Option<String>,
{
    // 1.1 优先使用 HOME
    if let Some(home) = non_empty_env(&mut get_var, "HOME") {
        return PathBuf::from(home);
    }

    // 1.2 Windows 环境优先回退到 USERPROFILE
    if let Some(user_profile) = non_empty_env(&mut get_var, "USERPROFILE") {
        return PathBuf::from(user_profile);
    }

    // 1.3 最后尝试 HOMEDRIVE + HOMEPATH
    if let (Some(home_drive), Some(home_path)) = (
        non_empty_env(&mut get_var, "HOMEDRIVE"),
        non_empty_env(&mut get_var, "HOMEPATH"),
    ) {
        return PathBuf::from(format!("{}{}", home_drive, home_path));
    }

    // 1.4 全部缺失时保底回退到 /tmp
    PathBuf::from("/tmp")
}

/*
 * ========================================================================
 * 步骤2：构造中央技能目录
 * ========================================================================
 * 目标：
 * 1) 统一 `~/.agents/skills` 路径生成
 * 2) 让 db / discover / marketplace 共用同一规则
 */
pub fn central_skills_dir() -> PathBuf {
    central_skills_dir_from_home(&resolve_home_dir())
}

pub fn central_skills_dir_from_home(home_dir: &Path) -> PathBuf {
    // 2.1 在家目录下拼出中央技能目录
    home_dir.join(".agents").join("skills")
}

fn non_empty_env<F>(get_var: &mut F, key: &str) -> Option<String>
where
    F: FnMut(&str) -> Option<String>,
{
    get_var(key).and_then(|value| {
        if value.trim().is_empty() {
            None
        } else {
            Some(value)
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn resolve_home_dir_uses_tmp_as_last_resort() {
        let home = resolve_home_dir_with(|_| None);
        assert_eq!(home, PathBuf::from("/tmp"));
    }

    #[test]
    fn central_skills_dir_is_built_under_home() {
        let central = central_skills_dir_from_home(Path::new(r"C:\Users\lyh"));
        assert_eq!(
            central,
            PathBuf::from(r"C:\Users\lyh")
                .join(".agents")
                .join("skills")
        );
    }
}
