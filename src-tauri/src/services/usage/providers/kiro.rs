//! Kiro stub provider —— 占位，等真实日志格式确认后补充。
use async_trait::async_trait;

use crate::services::usage::{Scope, SkillCall, UsageError, UsageProvider};

pub struct KiroProvider;

#[async_trait]
impl UsageProvider for KiroProvider {
    fn id(&self) -> &'static str {
        "kiro"
    }
    fn display_name(&self) -> &'static str {
        "Kiro"
    }
    async fn available(&self, _scope: &Scope) -> Result<bool, UsageError> {
        Ok(false)
    }
    async fn collect(&self, _scope: &Scope) -> Result<Vec<SkillCall>, UsageError> {
        Ok(vec![])
    }
}
