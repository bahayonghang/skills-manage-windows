//! Local rule library and recoverable file-link lifecycle.
mod error;
mod files;
mod format;
mod lifecycle;
#[cfg(test)]
mod tests;
mod types;

use crate::services::resource_budget::ResourceBudget;
pub use error::RulesError;
pub use files::validate_name;
use files::*;
use std::fs;
use std::path::PathBuf;
pub use types::*;

pub struct RulesService {
    home: PathBuf,
    supported: [bool; 2],
    #[cfg(test)]
    fault: Option<&'static str>,
}

impl RulesService {
    pub fn local() -> Self {
        Self::with_home(
            crate::paths::resolve_home_dir(),
            RuleTool::ALL.map(|tool| {
                crate::paths::rule_tool_uses_default_root_with(tool == RuleTool::Omp, |key| {
                    std::env::var_os(key)
                })
            }),
        )
    }
    pub fn ensure_local(target_id: &str) -> Result<(), RulesError> {
        if target_id != "local" {
            return Err(RulesError::LocalOnly);
        }
        Ok(())
    }
    fn with_home(home: PathBuf, supported: [bool; 2]) -> Self {
        Self {
            home,
            supported,
            #[cfg(test)]
            fault: None,
        }
    }
    fn root(&self) -> PathBuf {
        crate::paths::rules_dir_from_home(&self.home)
    }
    fn tool_root(&self, tool: RuleTool) -> PathBuf {
        crate::paths::rule_tool_dir_from_home(&self.home, tool == RuleTool::Omp)
    }
    fn tool_supported(&self, tool: RuleTool) -> bool {
        self.supported[usize::from(tool == RuleTool::Omp)]
    }
    fn require_tool(&self, tool: RuleTool) -> Result<(), RulesError> {
        if self.tool_supported(tool) {
            Ok(())
        } else {
            Err(RulesError::Unsupported)
        }
    }
    fn central(&self, name: &str) -> Result<PathBuf, RulesError> {
        validate_name(name)?;
        safe_dir(&self.home, &self.root(), false)?;
        Ok(self.root().join(name))
    }
    fn phase(&self, _name: &'static str) -> Result<(), RulesError> {
        #[cfg(test)]
        if self.fault == Some(_name) {
            return Err(RulesError::Io);
        }
        Ok(())
    }
    fn reject_pending(&self, name: &str) -> Result<(), RulesError> {
        if self.receipts()?.iter().any(|r| {
            r.name.to_lowercase() == name.to_lowercase()
                && r.phase != "completed"
                && r.phase != "recovered"
        }) {
            return Err(RulesError::RecoveryBlocked);
        }
        Ok(())
    }
    fn reject_duplicate(&self, name: &str) -> Result<(), RulesError> {
        if names(&self.home, &self.root())?
            .iter()
            .any(|n| n.to_lowercase() == name.to_lowercase())
        {
            return Err(RulesError::TargetConflict);
        }
        Ok(())
    }
    pub fn list(&self) -> Result<RulesSnapshot, RulesError> {
        let receipts = self.receipts()?;
        let recovery_operations = receipts
            .iter()
            .filter(|r| r.phase != "completed" && r.phase != "recovered")
            .map(|r| self.recovery_summary(r))
            .collect();
        let mut rules = Vec::new();
        let mut total = 0;
        for name in names(&self.home, &self.root())? {
            let detail = self.read_with_receipts(&name, &receipts)?;
            total += u64::from(detail.bytes);
            if total > ResourceBudget::default_skill().copy_bytes {
                return Err(RulesError::BudgetExceeded);
            }
            rules.push(RuleSummary {
                name: detail.name,
                title: detail.title,
                description: detail.description,
                bytes: detail.bytes,
                revision: detail.revision,
                compatibility: detail.compatibility,
                targets: detail.targets,
            });
        }
        Ok(RulesSnapshot {
            root_path: crate::paths::path_to_string(&self.root()),
            targets: RuleTool::ALL
                .into_iter()
                .map(|tool| RuleTargetInfo {
                    tool,
                    path: crate::paths::path_to_string(&self.tool_root(tool)),
                    supported: self.tool_supported(tool),
                    error_code: (!self.tool_supported(tool)).then(|| "rules.unsupported".into()),
                })
                .collect(),
            rules,
            recovery_operations,
        })
    }
    pub fn read(&self, name: &str) -> Result<RuleDetail, RulesError> {
        self.read_with_receipts(name, &self.receipts()?)
    }
    fn read_with_receipts(
        &self,
        name: &str,
        receipts: &[lifecycle::Receipt],
    ) -> Result<RuleDetail, RulesError> {
        let path = self.central(name)?;
        let bytes = regular_bytes(&self.home, &path)?;
        let parsed = format::parse(&bytes)?;
        let targets = RuleTool::ALL
            .into_iter()
            .map(|tool| {
                self.target_status(
                    name,
                    tool,
                    &bytes,
                    receipts.iter().any(|receipt| {
                        receipt.name.to_lowercase() == name.to_lowercase()
                            && receipt.tools.contains(&tool)
                            && receipt.phase != "completed"
                            && receipt.phase != "recovered"
                    }),
                )
            })
            .collect();
        Ok(RuleDetail {
            name: name.into(),
            title: format::title(parsed.body, name),
            description: parsed.description,
            bytes: bytes.len() as u32,
            revision: fingerprint(&bytes),
            compatibility: if parsed.supported && parsed.always_apply {
                RuleCompatibility::Supported
            } else {
                RuleCompatibility::Unsupported
            },
            body: parsed.body.into(),
            source: String::from_utf8(bytes.clone()).map_err(|_| RulesError::Unsupported)?,
            targets,
        })
    }
    fn target_status(
        &self,
        name: &str,
        tool: RuleTool,
        central_bytes: &[u8],
        pending: bool,
    ) -> RuleTargetStatus {
        let path = self.tool_root(tool).join(name);
        let mut status = RuleTargetStatus {
            tool,
            path: crate::paths::path_to_string(&path),
            state: RuleTargetState::Absent,
            fingerprint: None,
            error_code: None,
        };
        let result = (|| -> Result<(), RulesError> {
            self.require_tool(tool)?;
            safe_dir(&self.home, &self.tool_root(tool), false)?;
            let meta = match fs::symlink_metadata(&path) {
                Ok(m) => m,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
                Err(e) => return Err(e.into()),
            };
            if meta.file_type().is_symlink() {
                if points_to(&path, &self.root().join(name)) {
                    status.state = if path.exists() {
                        RuleTargetState::Linked
                    } else {
                        RuleTargetState::Broken
                    };
                    if path.exists() && read_bounded(&path)? != central_bytes {
                        return Err(RulesError::TargetConflict);
                    }
                } else {
                    status.state = if path.exists() {
                        RuleTargetState::Conflict
                    } else {
                        RuleTargetState::Broken
                    };
                }
            } else if meta.is_file() {
                let bytes = regular_bytes(&self.home, &path)?;
                status.fingerprint = Some(fingerprint(&bytes));
                let source = format::parse(&bytes)?;
                let central = format::parse(central_bytes)?;
                status.state = if !source.supported {
                    RuleTargetState::Unsupported
                } else if source.body.trim() == central.body.trim() {
                    RuleTargetState::NativeEquivalent
                } else {
                    RuleTargetState::Conflict
                };
            } else {
                status.state = RuleTargetState::Conflict;
            }
            Ok(())
        })();
        if let Err(error) = result {
            status.state = if matches!(error, RulesError::Unsupported) {
                RuleTargetState::Unsupported
            } else {
                RuleTargetState::Unreadable
            };
            status.error_code = Some(error.code().into());
        } else if pending {
            status.state = RuleTargetState::RecoveryRequired;
        }
        status
    }
    pub fn create(
        &self,
        name: &str,
        description: &str,
        body: &str,
    ) -> Result<RuleDetail, RulesError> {
        let path = self.central(name)?;
        self.reject_pending(name)?;
        self.reject_duplicate(name)?;
        let description = if description.trim().is_empty() {
            format::title(body, name)
        } else {
            description.into()
        };
        atomic_write(&self.home, &path, &format::shared(body, &description), true)?;
        self.read(name)
    }
    pub fn save(
        &self,
        name: &str,
        body: &str,
        expected_revision: &str,
    ) -> Result<RuleDetail, RulesError> {
        let path = self.central(name)?;
        self.reject_pending(name)?;
        let bytes = regular_bytes(&self.home, &path)?;
        if fingerprint(&bytes) != expected_revision {
            return Err(RulesError::RevisionConflict);
        }
        let parsed = format::parse(&bytes)?;
        if !parsed.supported || !parsed.always_apply {
            return Err(RulesError::Unsupported);
        }
        let updated = format!("{}{body}", parsed.prefix);
        self.phase("save")?;
        atomic_write_checked(
            &self.home,
            &path,
            updated.as_bytes(),
            false,
            Some(expected_revision),
        )?;
        self.read(name)
    }
    fn source_bytes(&self, name: &str, tool: RuleTool) -> Result<Vec<u8>, RulesError> {
        validate_name(name)?;
        self.require_tool(tool)?;
        let path = self.tool_root(tool).join(name);
        // Import only ordinary native files. Existing linked entries are already managed.
        regular_bytes(&self.home, &path)
    }
    fn preview_entry(&self, name: &str) -> RuleImportPreviewEntry {
        let mut entry = RuleImportPreviewEntry {
            name: name.into(),
            status: RuleImportStatus::New,
            source_fingerprints: Vec::new(),
            description: String::new(),
            error_code: None,
        };
        let outcome = (|| -> Result<(), RulesError> {
            validate_name(name)?;
            let mut bodies = Vec::new();
            for tool in RuleTool::ALL {
                if !self.tool_supported(tool) {
                    continue;
                }
                let source = self.tool_root(tool).join(name);
                if slot_metadata(&source)?.is_none() {
                    continue;
                }
                if points_to(&source, &self.root().join(name)) {
                    continue;
                }
                let bytes = self.source_bytes(name, tool)?;
                let parsed = format::parse(&bytes)?;
                if !parsed.supported {
                    return Err(RulesError::Unsupported);
                }
                entry.source_fingerprints.push(RuleSourceFingerprint {
                    tool,
                    fingerprint: fingerprint(&bytes),
                });
                if !parsed.description.is_empty() {
                    entry.description = parsed.description;
                }
                bodies.push(parsed.body.trim().to_string());
            }
            if bodies.windows(2).any(|b| b[0] != b[1]) {
                return Err(RulesError::TargetConflict);
            }
            let central_path = self.central(name)?;
            if slot_metadata(&central_path)?.is_some() {
                let bytes = regular_bytes(&self.home, &central_path)?;
                let parsed = format::parse(&bytes)?;
                if !parsed.supported || !parsed.always_apply {
                    return Err(RulesError::Unsupported);
                }
                if bodies.iter().any(|b| b != parsed.body.trim()) {
                    return Err(RulesError::TargetConflict);
                }
                entry.status = RuleImportStatus::Same;
            }
            Ok(())
        })();
        if let Err(error) = outcome {
            entry.status = match error {
                RulesError::Unsupported | RulesError::InvalidName => RuleImportStatus::Unsupported,
                RulesError::TargetConflict => RuleImportStatus::Conflict,
                _ => RuleImportStatus::Failed,
            };
            entry.error_code = Some(error.code().into());
        }
        entry
    }
    pub fn preview_import(&self) -> Result<RulesImportPreview, RulesError> {
        let mut all = std::collections::BTreeMap::new();
        let mut total = 0u64;
        for tool in RuleTool::ALL {
            if !self.tool_supported(tool) {
                continue;
            }
            for name in names(&self.home, &self.tool_root(tool))? {
                let meta = fs::symlink_metadata(self.tool_root(tool).join(&name))?;
                if meta.is_file() && !meta.file_type().is_symlink() {
                    total = total.saturating_add(meta.len());
                    if total > ResourceBudget::default_skill().copy_bytes {
                        return Err(RulesError::BudgetExceeded);
                    }
                }
                all.entry(name.to_lowercase()).or_insert(name);
                if all.len() > ResourceBudget::default_skill().tree_entries {
                    return Err(RulesError::BudgetExceeded);
                }
            }
        }
        let entries = all.values().map(|name| self.preview_entry(name)).collect();
        Ok(RulesImportPreview { entries })
    }
    pub fn import(&self, entries: &[RuleImportEntry]) -> Result<RulesImportResult, RulesError> {
        if entries.len() > ResourceBudget::default_skill().tree_entries {
            return Err(RulesError::BudgetExceeded);
        }
        let operation_id = uuid::Uuid::new_v4().to_string();
        let mut result = RulesImportResult {
            entries: Vec::new(),
            operation_id: operation_id.clone(),
        };
        let mut total = 0;
        for requested in entries {
            let preview = self.preview_entry(&requested.name);
            let imported = (|| -> Result<RuleImportStatus, RulesError> {
                self.reject_pending(&requested.name)?;
                if !matches!(
                    preview.status,
                    RuleImportStatus::New | RuleImportStatus::Same
                ) {
                    return Err(if preview.status == RuleImportStatus::Unsupported {
                        RulesError::Unsupported
                    } else {
                        RulesError::TargetConflict
                    });
                }
                if preview.source_fingerprints.len() != requested.source_fingerprints.len()
                    || preview.source_fingerprints.iter().any(|f| {
                        !requested
                            .source_fingerprints
                            .iter()
                            .any(|r| r.tool == f.tool && r.fingerprint == f.fingerprint)
                    })
                {
                    return Err(RulesError::RevisionConflict);
                }
                if preview.status == RuleImportStatus::Same {
                    return Ok(RuleImportStatus::Same);
                }
                self.reject_duplicate(&requested.name)?;
                let mut body = None;
                for source in &preview.source_fingerprints {
                    let bytes = self.source_bytes(&requested.name, source.tool)?;
                    if fingerprint(&bytes) != source.fingerprint {
                        return Err(RulesError::RevisionConflict);
                    }
                    total += bytes.len() as u64;
                    if total > ResourceBudget::default_skill().copy_bytes {
                        return Err(RulesError::BudgetExceeded);
                    }
                    let parsed = format::parse(&bytes)?;
                    if body.is_none() || source.tool == RuleTool::ClaudeCode {
                        body = Some(parsed.body.to_string());
                    }
                    self.backup(&operation_id, source.tool.id(), &requested.name, &bytes)?;
                }
                let body = body.ok_or(RulesError::NotFound)?;
                let description = if preview.description.is_empty() {
                    format::title(&body, &requested.name)
                } else {
                    preview.description.clone()
                };
                self.phase("import_commit")?;
                atomic_write(
                    &self.home,
                    &self.central(&requested.name)?,
                    &format::shared(&body, &description),
                    true,
                )?;
                Ok(RuleImportStatus::Imported)
            })();
            result.entries.push(match imported {
                Ok(status) => RuleImportResultEntry {
                    name: requested.name.clone(),
                    status,
                    error_code: None,
                },
                Err(error) => RuleImportResultEntry {
                    name: requested.name.clone(),
                    status: match error {
                        RulesError::Unsupported | RulesError::InvalidName => {
                            RuleImportStatus::Unsupported
                        }
                        RulesError::TargetConflict | RulesError::RevisionConflict => {
                            RuleImportStatus::Conflict
                        }
                        _ => RuleImportStatus::Failed,
                    },
                    error_code: Some(error.code().into()),
                },
            });
        }
        let bytes = serde_json::to_vec(&serde_json::json!({
            "operationId": operation_id,
            "sources": entries,
            "results": result.entries,
        }))
        .map_err(|_| RulesError::Io)?;
        // Marker contains source fingerprints/results only. Raw contents live exclusively in backups.
        atomic_write(
            &self.home,
            &self.root().join(".state").join("import.json"),
            &bytes,
            false,
        )?;
        Ok(result)
    }
}
