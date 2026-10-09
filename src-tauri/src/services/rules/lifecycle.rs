use super::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Receipt {
    pub operation_id: String,
    pub name: String,
    pub kind: RuleOperationKind,
    pub phase: String,
    pub tools: Vec<RuleTool>,
    pub central_fingerprint: String,
    pub original_fingerprint: Option<String>,
}

impl Receipt {
    fn validate(&self) -> Result<(), RulesError> {
        validate_name(&self.name).map_err(|_| RulesError::RecoveryBlocked)?;
        uuid::Uuid::parse_str(&self.operation_id).map_err(|_| RulesError::RecoveryBlocked)?;
        if !matches!(self.phase.as_str(), "prepared" | "completed" | "recovered")
            || self.tools.len() > 2
            || self
                .tools
                .iter()
                .enumerate()
                .any(|(index, tool)| self.tools[..index].contains(tool))
            || (matches!(
                self.kind,
                RuleOperationKind::Enable | RuleOperationKind::Disable
            ) && self.tools.len() != 1)
            || (!matches!(self.kind, RuleOperationKind::Enable)
                && self.original_fingerprint.is_some())
        {
            return Err(RulesError::RecoveryBlocked);
        }
        Ok(())
    }
}

impl RulesService {
    fn operation_path(&self, id: &str) -> Result<PathBuf, RulesError> {
        let id = uuid::Uuid::parse_str(id)
            .map_err(|_| RulesError::RecoveryBlocked)?
            .to_string();
        Ok(self
            .root()
            .join(".state")
            .join("operations")
            .join(format!("{id}.json")))
    }
    fn backup_path(&self, id: &str, owner: &str, name: &str) -> Result<PathBuf, RulesError> {
        validate_name(name)?;
        let id = uuid::Uuid::parse_str(id)
            .map_err(|_| RulesError::RecoveryBlocked)?
            .to_string();
        if !["central", "claude-code", "omp"].contains(&owner) {
            return Err(RulesError::RecoveryBlocked);
        }
        Ok(self.root().join(".backups").join(id).join(owner).join(name))
    }
    pub(super) fn backup(
        &self,
        id: &str,
        owner: &str,
        name: &str,
        bytes: &[u8],
    ) -> Result<(), RulesError> {
        self.phase("backup")?;
        let path = self.backup_path(id, owner, name)?;
        atomic_write(&self.home, &path, bytes, true)?;
        if regular_bytes(&self.home, &path)? != bytes {
            return Err(RulesError::Io);
        }
        Ok(())
    }
    fn write_receipt(&self, receipt: &Receipt) -> Result<(), RulesError> {
        self.phase("receipt")?;
        let bytes = serde_json::to_vec(receipt).map_err(|_| RulesError::Io)?;
        atomic_write(
            &self.home,
            &self.operation_path(&receipt.operation_id)?,
            &bytes,
            false,
        )
    }
    pub(super) fn receipts(&self) -> Result<Vec<Receipt>, RulesError> {
        let root = self.root().join(".state").join("operations");
        safe_dir(&self.home, &root, false)?;
        let entries = match fs::read_dir(&root) {
            Ok(v) => v,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(e.into()),
        };
        let mut receipts = Vec::new();
        for (index, entry) in entries.enumerate() {
            if index >= ResourceBudget::default_skill().tree_entries {
                return Err(RulesError::BudgetExceeded);
            }
            let entry = entry?;
            if entry.path().extension().is_none_or(|e| e != "json") {
                continue;
            }
            let bytes = regular_bytes(&self.home, &entry.path())?;
            let receipt: Receipt =
                serde_json::from_slice(&bytes).map_err(|_| RulesError::RecoveryBlocked)?;
            receipt.validate()?;
            if !crate::paths::paths_equivalent(
                &entry.path(),
                &self.operation_path(&receipt.operation_id)?,
            ) {
                return Err(RulesError::RecoveryBlocked);
            }
            receipts.push(receipt);
        }
        Ok(receipts)
    }
    pub(super) fn recovery_summary(&self, receipt: &Receipt) -> RuleRecoveryOperation {
        let tool = receipt.tools.first().copied();
        let owner = if matches!(receipt.kind, RuleOperationKind::Delete) {
            Some("central")
        } else {
            tool.filter(|_| receipt.original_fingerprint.is_some())
                .map(RuleTool::id)
        };
        RuleRecoveryOperation {
            operation_id: receipt.operation_id.clone(),
            name: receipt.name.clone(),
            kind: receipt.kind.clone(),
            phase: receipt.phase.clone(),
            tool,
            backup_path: owner
                .and_then(|owner| {
                    self.backup_path(&receipt.operation_id, owner, &receipt.name)
                        .ok()
                })
                .map(|p| crate::paths::path_to_string(&p)),
        }
    }
    fn new_receipt(
        &self,
        name: &str,
        kind: RuleOperationKind,
        tools: Vec<RuleTool>,
        central_bytes: &[u8],
        original_fingerprint: Option<String>,
    ) -> Receipt {
        Receipt {
            operation_id: uuid::Uuid::new_v4().to_string(),
            name: name.into(),
            kind,
            phase: "prepared".into(),
            tools,
            central_fingerprint: fingerprint(central_bytes),
            original_fingerprint,
        }
    }
    fn verify_link(
        &self,
        name: &str,
        tool: RuleTool,
        expected_bytes: &[u8],
    ) -> Result<(), RulesError> {
        let destination = self.tool_root(tool).join(name);
        if !points_to(&destination, &self.root().join(name))
            || read_bounded(&destination)? != expected_bytes
        {
            return Err(RulesError::TargetConflict);
        }
        Ok(())
    }
    fn stage_link(&self, name: &str, tool: RuleTool) -> Result<PathBuf, RulesError> {
        self.require_tool(tool)?;
        let root = self.tool_root(tool);
        safe_dir(&self.home, &root, true)?;
        let stage = root.join(format!(".skillport-{}.tmp", uuid::Uuid::new_v4()));
        let target = crate::services::installation::fs_util::symlink_target_path(
            &root,
            &self.root().join(name),
        );
        create_file_link(&target, &stage)?;
        let bytes = regular_bytes(&self.home, &self.central(name)?)?;
        if !points_to(&stage, &self.root().join(name)) || read_bounded(&stage)? != bytes {
            let _ = fs::remove_file(&stage);
            return Err(RulesError::TargetConflict);
        }
        Ok(stage)
    }
    fn remove_owned_link(&self, name: &str, tool: RuleTool) -> Result<(), RulesError> {
        self.require_tool(tool)?;
        let destination = self.tool_root(tool).join(name);
        safe_dir(&self.home, &self.tool_root(tool), false)?;
        if slot_metadata(&destination)?.is_none() {
            return Ok(());
        }
        if !points_to(&destination, &self.root().join(name)) {
            return Err(RulesError::TargetConflict);
        }
        if !destination.exists()
            && !self.receipts()?.iter().any(|r| {
                r.name.to_lowercase() == name.to_lowercase()
                    && r.tools.contains(&tool)
                    && matches!(r.kind, RuleOperationKind::Enable)
                    && r.phase != "recovered"
            })
        {
            return Err(RulesError::TargetConflict);
        }
        fs::remove_file(destination)?;
        Ok(())
    }
    pub fn set_enabled(
        &self,
        name: &str,
        tool: RuleTool,
        enabled: bool,
        expected_destination_fingerprint: Option<&str>,
    ) -> Result<RuleDetail, RulesError> {
        self.require_tool(tool)?;
        let central = self.central(name)?;
        self.reject_pending(name)?;
        let central_bytes = regular_bytes(&self.home, &central)?;
        let parsed = format::parse(&central_bytes)?;
        if !parsed.supported || !parsed.always_apply {
            return Err(RulesError::Unsupported);
        }
        let destination = self.tool_root(tool).join(name);
        let status = self.target_status(name, tool, &central_bytes, false);
        if enabled {
            if status.state == RuleTargetState::Linked {
                return self.read(name);
            }
            if !matches!(
                status.state,
                RuleTargetState::Absent | RuleTargetState::NativeEquivalent
            ) {
                return Err(RulesError::TargetConflict);
            }
            let original = if status.state == RuleTargetState::NativeEquivalent {
                let bytes = regular_bytes(&self.home, &destination)?;
                if Some(fingerprint(&bytes).as_str()) != expected_destination_fingerprint {
                    return Err(RulesError::RevisionConflict);
                }
                Some(bytes)
            } else {
                if expected_destination_fingerprint.is_some() {
                    return Err(RulesError::RevisionConflict);
                }
                None
            };
            // Prove symlink privileges before moving any native file.
            let stage = self.stage_link(name, tool)?;
            let outcome = (|| -> Result<(), RulesError> {
                let mut receipt = self.new_receipt(
                    name,
                    RuleOperationKind::Enable,
                    vec![tool],
                    &central_bytes,
                    original.as_ref().map(|b| fingerprint(b)),
                );
                if let Some(bytes) = &original {
                    self.backup(&receipt.operation_id, tool.id(), name, bytes)?;
                }
                self.write_receipt(&receipt)?;
                self.phase("link_commit")?;
                // Recheck the exact slot immediately before the atomic replacement.
                match &original {
                    Some(bytes) if regular_bytes(&self.home, &destination)? != *bytes => {
                        return Err(RulesError::RevisionConflict)
                    }
                    None if slot_metadata(&destination)?.is_some() => {
                        return Err(RulesError::TargetConflict)
                    }
                    _ => {}
                }
                fs::rename(&stage, &destination)?;
                self.phase("link_verify")?;
                self.verify_link(name, tool, &central_bytes)?;
                receipt.phase = "completed".into();
                self.write_receipt(&receipt)
            })();
            let _ = fs::remove_file(stage);
            outcome?;
        } else {
            if status.state == RuleTargetState::Absent {
                return self.read(name);
            }
            if !matches!(
                status.state,
                RuleTargetState::Linked | RuleTargetState::Broken
            ) {
                return Err(RulesError::TargetConflict);
            }
            let mut receipt = self.new_receipt(
                name,
                RuleOperationKind::Disable,
                vec![tool],
                &central_bytes,
                None,
            );
            self.write_receipt(&receipt)?;
            self.phase("disable")?;
            self.remove_owned_link(name, tool)?;
            self.phase("disable_complete")?;
            receipt.phase = "completed".into();
            self.write_receipt(&receipt)?;
        }
        self.read(name)
    }
    pub fn delete(
        &self,
        name: &str,
        expected_revision: &str,
    ) -> Result<RuleMutationResult, RulesError> {
        let central = self.central(name)?;
        self.reject_pending(name)?;
        let bytes = regular_bytes(&self.home, &central)?;
        if fingerprint(&bytes) != expected_revision {
            return Err(RulesError::RevisionConflict);
        }
        let mut tools = Vec::new();
        for tool in RuleTool::ALL {
            // A configuration override makes the former destination unverifiable.
            // Do not delete a shared file while an unsupported tool may still link to it.
            self.require_tool(tool)?;
            let status = self.target_status(name, tool, &bytes, false);
            match status.state {
                RuleTargetState::Linked => tools.push(tool),
                RuleTargetState::Absent
                | RuleTargetState::NativeEquivalent
                | RuleTargetState::Unsupported => {}
                // An unrelated regular file is preserved; an unexpected link blocks deletion.
                RuleTargetState::Conflict
                    if fs::symlink_metadata(self.tool_root(tool).join(name))
                        .is_ok_and(|m| m.is_file() && !m.file_type().is_symlink()) => {}
                _ => return Err(RulesError::TargetConflict),
            }
        }
        let mut receipt = self.new_receipt(name, RuleOperationKind::Delete, tools, &bytes, None);
        self.backup(&receipt.operation_id, "central", name, &bytes)?;
        self.write_receipt(&receipt)?;
        for (index, tool) in receipt.tools.iter().copied().enumerate() {
            self.remove_owned_link(name, tool)?;
            if index == 0 {
                self.phase("delete_first_link")?;
            }
        }
        self.phase("delete_central")?;
        if regular_bytes(&self.home, &central)? != bytes {
            return Err(RulesError::RevisionConflict);
        }
        fs::remove_file(&central)?;
        self.phase("delete_complete")?;
        receipt.phase = "completed".into();
        self.write_receipt(&receipt)?;
        Ok(RuleMutationResult {
            operation_id: receipt.operation_id,
            name: name.into(),
            recovery_required: false,
        })
    }
    pub fn recover(&self, operation_id: &str) -> Result<RuleMutationResult, RulesError> {
        let path = self.operation_path(operation_id)?;
        let bytes = regular_bytes(&self.home, &path)?;
        let mut receipt: Receipt =
            serde_json::from_slice(&bytes).map_err(|_| RulesError::RecoveryBlocked)?;
        receipt.validate()?;
        if receipt.operation_id != operation_id {
            return Err(RulesError::RecoveryBlocked);
        }
        if receipt.phase == "recovered" {
            return Ok(RuleMutationResult {
                operation_id: operation_id.into(),
                name: receipt.name,
                recovery_required: false,
            });
        }
        let central = self.central(&receipt.name)?;
        // External edits stop recovery before any destination is changed.
        let central_bytes = match regular_bytes(&self.home, &central) {
            Ok(bytes) if fingerprint(&bytes) == receipt.central_fingerprint => bytes,
            Err(RulesError::NotFound) if matches!(receipt.kind, RuleOperationKind::Delete) => {
                let backup = regular_bytes(
                    &self.home,
                    &self.backup_path(operation_id, "central", &receipt.name)?,
                )?;
                if fingerprint(&backup) != receipt.central_fingerprint {
                    return Err(RulesError::RecoveryBlocked);
                }
                backup
            }
            _ => return Err(RulesError::RecoveryBlocked),
        };
        // Preflight all affected slots before rollback. Ordinary external content is never overwritten.
        for tool in &receipt.tools {
            self.require_tool(*tool)?;
            safe_dir(&self.home, &self.tool_root(*tool), false)?;
            let destination = self.tool_root(*tool).join(&receipt.name);
            if let Some(meta) =
                slot_metadata(&destination).map_err(|_| RulesError::RecoveryBlocked)?
            {
                if points_to(&destination, &central) {
                    continue;
                }
                if matches!(receipt.kind, RuleOperationKind::Enable)
                    && meta.is_file()
                    && !meta.file_type().is_symlink()
                    && receipt.original_fingerprint.as_ref().is_some_and(|fp| {
                        regular_bytes(&self.home, &destination)
                            .is_ok_and(|b| fingerprint(&b) == *fp)
                    })
                {
                    continue;
                }
                return Err(RulesError::RecoveryBlocked);
            }
        }
        match receipt.kind {
            RuleOperationKind::Enable => {
                if receipt.tools.len() != 1 {
                    return Err(RulesError::RecoveryBlocked);
                }
                let tool = receipt.tools[0];
                let destination = self.tool_root(tool).join(&receipt.name);
                if let Some(original_fingerprint) = &receipt.original_fingerprint {
                    let backup = regular_bytes(
                        &self.home,
                        &self.backup_path(operation_id, tool.id(), &receipt.name)?,
                    )?;
                    if fingerprint(&backup) != *original_fingerprint {
                        return Err(RulesError::RecoveryBlocked);
                    }
                    if points_to(&destination, &central) {
                        fs::remove_file(&destination)?;
                    }
                    if slot_metadata(&destination)?.is_none() {
                        atomic_write(&self.home, &destination, &backup, true)?;
                    }
                } else if points_to(&destination, &central) {
                    fs::remove_file(destination)?;
                }
            }
            RuleOperationKind::Disable | RuleOperationKind::Delete => {
                if !central.exists() {
                    atomic_write(&self.home, &central, &central_bytes, true)?;
                }
                for tool in &receipt.tools {
                    let destination = self.tool_root(*tool).join(&receipt.name);
                    if slot_metadata(&destination)?.is_none() {
                        let stage = self.stage_link(&receipt.name, *tool)?;
                        let outcome = fs::rename(&stage, &destination);
                        let _ = fs::remove_file(stage);
                        outcome?;
                    }
                    self.verify_link(&receipt.name, *tool, &central_bytes)?;
                }
            }
        }
        receipt.phase = "recovered".into();
        self.write_receipt(&receipt)?;
        Ok(RuleMutationResult {
            operation_id: operation_id.into(),
            name: receipt.name,
            recovery_required: false,
        })
    }
}
