//! Zed stub provider —— 占位，等真实日志格式确认后补充。
use async_trait::async_trait;

use crate::services::usage::{Scope, SkillCall, UsageError, UsageProvider};

pub struct ZedProvider;

#[async_trait]
impl UsageProvider for ZedProvider {
    fn id(&self) -> &'static str {
        "zed"
    }
    fn display_name(&self) -> &'static str {
        "Zed"
    }
    async fn available(&self, _scope: &Scope) -> bool {
        false
    }
    async fn collect(&self, _scope: &Scope) -> Result<Vec<SkillCall>, UsageError> {
        Ok(vec![])
    }
}
