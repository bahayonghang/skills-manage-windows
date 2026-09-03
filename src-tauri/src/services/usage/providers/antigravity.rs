//! Antigravity stub provider —— SkillPort 独有但 skilled 没覆盖的 agent。
//!
//! 占位实现：永远 `available()=false`，UI 上显示「未检测到」。
//! 等 Antigravity 的会话日志格式确认后再补真实解析。

use async_trait::async_trait;

use crate::services::usage::{Scope, SkillCall, UsageError, UsageProvider};

pub struct AntigravityProvider;

#[async_trait]
impl UsageProvider for AntigravityProvider {
    fn id(&self) -> &'static str {
        "antigravity"
    }
    fn display_name(&self) -> &'static str {
        "Antigravity"
    }
    async fn available(&self, _scope: &Scope) -> Result<bool, UsageError> {
        Ok(false)
    }
    async fn collect(&self, _scope: &Scope) -> Result<Vec<SkillCall>, UsageError> {
        Ok(vec![])
    }
}
