//! 8 个 UsageProvider 实现。
//!
//! - 5 个真实 provider 直接对应 skilled 项目的 5 个数据源：
//!   - [`claude_code`] / [`codex`] / [`droid`] / [`opencode`] / [`grok`]
//! - 3 个 stub 是 SkillPort 独有但 skilled 没有覆盖的 agent，
//!   永远 `available()=false`，等真实日志格式确认后再补：
//!   - [`antigravity`] / [`kiro`] / [`zed`]
//!
//! [`all_providers`] 是编排器入口，按稳定顺序返回所有 8 个 provider，
//! UI 上 Provider Health 表格按这个顺序渲染。

pub mod antigravity;
pub mod claude_code;
pub mod codex;
pub mod droid;
pub mod grok;
pub mod kiro;
pub mod opencode;
pub mod zed;

use super::UsageProvider;

/// 返回所有 8 个 provider 的固定列表。顺序同 skilled 的 `all_providers`：
/// 5 个真实 provider 在前，3 个 stub 在后。
pub fn all_providers() -> Vec<Box<dyn UsageProvider>> {
    vec![
        Box::new(claude_code::ClaudeCodeProvider),
        Box::new(codex::CodexProvider),
        Box::new(droid::DroidProvider),
        Box::new(opencode::OpenCodeProvider),
        Box::new(grok::GrokProvider),
        Box::new(antigravity::AntigravityProvider),
        Box::new(kiro::KiroProvider),
        Box::new(zed::ZedProvider),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_providers_returns_eight_in_stable_order() {
        let providers = all_providers();
        assert_eq!(providers.len(), 8);

        let ids: Vec<&str> = providers.iter().map(|p| p.id()).collect();
        assert_eq!(
            ids,
            vec![
                "claude-code",
                "codex",
                "droid",
                "opencode",
                "grok",
                "antigravity",
                "kiro",
                "zed",
            ]
        );
    }

    #[tokio::test]
    async fn stubs_report_confirmed_unavailable() {
        for provider in all_providers()
            .into_iter()
            .filter(|provider| matches!(provider.id(), "antigravity" | "kiro" | "zed"))
        {
            assert!(!provider
                .available(&crate::services::usage::Scope::Local)
                .await
                .unwrap());
        }
    }
}
