use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AppPlatform {
    Windows,
    Macos,
    Linux,
    Other,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppRuntimeInfo {
    pub platform: AppPlatform,
    pub windows_updater_supported: bool,
}

pub fn current_platform() -> AppPlatform {
    if cfg!(target_os = "windows") {
        AppPlatform::Windows
    } else if cfg!(target_os = "macos") {
        AppPlatform::Macos
    } else if cfg!(target_os = "linux") {
        AppPlatform::Linux
    } else {
        AppPlatform::Other
    }
}

pub fn app_runtime_info() -> AppRuntimeInfo {
    let platform = current_platform();
    AppRuntimeInfo {
        platform,
        windows_updater_supported: platform == AppPlatform::Windows && cfg!(target_arch = "x86_64"),
    }
}

#[tauri::command]
pub fn get_app_runtime_info() -> AppRuntimeInfo {
    app_runtime_info()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_info_marks_windows_updater_support_only_on_windows() {
        let info = app_runtime_info();

        assert_eq!(info.platform, current_platform());
        assert_eq!(
            info.windows_updater_supported,
            info.platform == AppPlatform::Windows && cfg!(target_arch = "x86_64")
        );
    }
}
