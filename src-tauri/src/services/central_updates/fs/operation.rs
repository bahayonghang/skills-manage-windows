use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use flate2::{write::GzEncoder, Compression};
use sha2::{Digest, Sha256};
use tracing::Instrument;
use uuid::Uuid;

use crate::fs_util::run_blocking_fs_with;
use crate::services::central_operation::{
    CentralOperationError, CopyProjection, OperationPhase, UpdateManifest, MANIFEST_VERSION,
};
use crate::targets::{remote_parent, shell_quote};

use super::batch::{parse_batch_rows, validate_central_skill_write, REMOTE_WRITE_CHUNK_SIZE};
use super::remote_scripts::{
    REMOTE_BATCH_STAGE_UPDATE, REMOTE_FINALIZE_UPDATE, REMOTE_ROLLBACK_UPDATE, REMOTE_STAGE_UPDATE,
    REMOTE_SWAP_UPDATE,
};
use super::{
    hash_entries, hash_local_directory, posix_path, remove_path, write_remote_skill_files,
    CentralFs, CentralSkillWrite,
};
use crate::services::central_updates::error::CentralUpdatesError;

#[derive(Debug, Clone)]
pub(crate) struct OperationUpdateStage {
    pub(crate) manifest: UpdateManifest,
    pub(crate) write: CentralSkillWrite,
}

#[derive(Debug)]
pub(crate) struct OperationUpdateStageOutcome {
    pub(crate) operation_id: String,
    pub(crate) result: Result<(), CentralUpdatesError>,
}

impl CentralFs {
    pub(crate) fn target_id(&self) -> &str {
        match self {
            Self::Local => "local",
            Self::Remote(connection) => connection.target_id(),
        }
    }

    pub(crate) fn target_kind_value(&self) -> crate::targets::TargetKind {
        match self {
            Self::Local => crate::targets::TargetKind::Local,
            Self::Remote(connection) => match connection.as_ref() {
                crate::targets::ConnectedRemoteTarget::Ssh(_) => crate::targets::TargetKind::Ssh,
                crate::targets::ConnectedRemoteTarget::Wsl(_) => crate::targets::TargetKind::Wsl,
            },
        }
    }

    pub(crate) fn connected_remote(&self) -> Option<&crate::targets::ConnectedRemoteTarget> {
        match self {
            Self::Local => None,
            Self::Remote(connection) => Some(connection),
        }
    }

    pub(crate) async fn build_operation_update_manifest(
        &self,
        operation_id: &str,
        write: &CentralSkillWrite,
        copy_targets: Vec<String>,
    ) -> Result<UpdateManifest, CentralUpdatesError> {
        validate_central_skill_write(write)?;
        let target = match self {
            Self::Local => write.target_dir.to_string_lossy().into_owned(),
            Self::Remote(_) => posix_path(&write.target_dir),
        };
        let parent = match self {
            Self::Local => write
                .target_dir
                .parent()
                .map(|path| path.to_string_lossy().into_owned()),
            Self::Remote(_) => target
                .rsplit_once('/')
                .map(|(parent, _)| parent.to_string()),
        }
        .filter(|parent| !parent.is_empty())
        .ok_or_else(|| CentralUpdatesError::TargetDirNoParent(write.skill_id.clone()))?;
        let token = short_digest(&target);
        let separator = if matches!(self, Self::Local) {
            std::path::MAIN_SEPARATOR
        } else {
            '/'
        };
        let sibling =
            |role: &str| format!("{parent}{separator}.skillport-{role}-{operation_id}-{token}");
        let had_target = match self {
            Self::Local => std::fs::symlink_metadata(&write.target_dir).is_ok(),
            Self::Remote(connection) => connection
                .exists(&target)
                .await
                .map_err(|error| CentralUpdatesError::Remote(error.to_string()))?,
        };
        let old_fingerprint = if had_target {
            self.hash_directories(std::slice::from_ref(&write.target_dir))
                .await?
                .remove(&write.target_dir)
        } else {
            None
        };
        let new_fingerprint = fingerprint_files(&write.files);
        Ok(UpdateManifest {
            version: MANIFEST_VERSION,
            operation_id: operation_id.to_string(),
            target,
            staging: sibling("update-staging"),
            backup: sibling("update-backup"),
            marker: sibling("operation-marker"),
            had_target,
            old_fingerprint,
            new_fingerprint,
            copies: copy_targets
                .into_iter()
                .map(|target| CopyProjection {
                    target,
                    completed: false,
                })
                .collect(),
        })
    }

    pub(crate) async fn stage_operation_update(
        &self,
        manifest: &UpdateManifest,
        write: &CentralSkillWrite,
    ) -> Result<(), CentralUpdatesError> {
        match self {
            Self::Local => {
                let manifest = manifest.clone();
                let files = write.files.clone();
                run_blocking_fs_with(
                    "Central update durable staging",
                    move || stage_local(&manifest, &files),
                    CentralUpdatesError::task_join,
                )
                .await
            }
            Self::Remote(connection) => {
                let archive = {
                    let stage = OperationUpdateStage {
                        manifest: manifest.clone(),
                        write: write.clone(),
                    };
                    run_blocking_fs_with(
                        "Central update durable archive",
                        move || build_operation_stage_archive(&[stage]),
                        CentralUpdatesError::task_join,
                    )
                    .await?
                };
                let extract_root = format!("{}.extract", manifest.staging);
                let command = format!(
                    "sh -c {} -- {} {} {} {}",
                    shell_quote(REMOTE_STAGE_UPDATE),
                    shell_quote(&manifest.operation_id),
                    shell_quote(&manifest.staging),
                    shell_quote(&manifest.marker),
                    shell_quote(&extract_root),
                );
                let output = connection
                    .run_command_with_stdin_bytes(&command, &archive)
                    .await
                    .map_err(|_| CentralOperationError::Remote {
                        code: "update_remote_stage",
                    })?;
                if String::from_utf8_lossy(&output).trim() != "STAGED" {
                    return Err(CentralOperationError::Remote {
                        code: "update_remote_stage_protocol",
                    }
                    .into());
                }
                let actual = self
                    .hash_directories(&[PathBuf::from(&manifest.staging)])
                    .await?
                    .remove(&PathBuf::from(&manifest.staging));
                if actual.as_deref() != Some(manifest.new_fingerprint.as_str()) {
                    return Err(CentralOperationError::RecoveryCollision {
                        code: "update_staging_fingerprint",
                    }
                    .into());
                }
                Ok(())
            }
        }
    }

    #[tracing::instrument(
        skip_all,
        fields(
            phase = "durable_stage",
            target_kind = self.target_kind(),
            skills = stages.len(),
            write_chunks = stages.len().div_ceil(REMOTE_WRITE_CHUNK_SIZE)
        )
    )]
    pub(crate) async fn stage_operation_updates(
        &self,
        stages: Vec<OperationUpdateStage>,
        cancel: Option<&AtomicBool>,
    ) -> Vec<OperationUpdateStageOutcome> {
        match self {
            Self::Local => {
                let mut outcomes = Vec::with_capacity(stages.len());
                for stage in stages {
                    let operation_id = stage.manifest.operation_id.clone();
                    let result = if cancel.is_some_and(|flag| flag.load(Ordering::SeqCst)) {
                        Err(CentralUpdatesError::BatchCancelled)
                    } else {
                        self.stage_operation_update(&stage.manifest, &stage.write)
                            .await
                    };
                    outcomes.push(OperationUpdateStageOutcome {
                        operation_id,
                        result,
                    });
                }
                outcomes
            }
            Self::Remote(connection) => {
                stage_operation_updates_remote(self, connection, stages, cancel).await
            }
        }
    }

    pub(crate) async fn swap_operation_update(
        &self,
        manifest: &UpdateManifest,
    ) -> Result<(), CentralUpdatesError> {
        match self {
            Self::Local => {
                let manifest = manifest.clone();
                run_blocking_fs_with(
                    "Central update durable swap",
                    move || swap_local(&manifest),
                    CentralUpdatesError::task_join,
                )
                .await
            }
            Self::Remote(connection) => {
                verify_remote_hash(self, &manifest.staging, &manifest.new_fingerprint).await?;
                if manifest.had_target {
                    let expected = manifest.old_fingerprint.as_deref().ok_or_else(|| {
                        CentralOperationError::InvalidManifest(
                            "existing update target has no old fingerprint".to_string(),
                        )
                    })?;
                    verify_remote_hash(self, &manifest.target, expected).await?;
                } else if connection
                    .exists(&manifest.target)
                    .await
                    .map_err(|error| CentralUpdatesError::Remote(error.to_string()))?
                {
                    return Err(CentralOperationError::RecoveryCollision {
                        code: "update_swap_collision",
                    }
                    .into());
                }
                let had_target = if manifest.had_target { "1" } else { "0" };
                let output = connection
                    .run_script(
                        REMOTE_SWAP_UPDATE,
                        &[
                            &manifest.operation_id,
                            &manifest.target,
                            &manifest.staging,
                            &manifest.backup,
                            &manifest.marker,
                            had_target,
                        ],
                    )
                    .await
                    .map_err(|_| CentralOperationError::Remote {
                        code: "update_remote_swap",
                    })?;
                if output.trim() != "SWAPPED" {
                    return Err(CentralOperationError::Remote {
                        code: "update_remote_swap_protocol",
                    }
                    .into());
                }
                Ok(())
            }
        }
    }

    pub(crate) async fn rollback_operation_update(
        &self,
        manifest: &UpdateManifest,
        phase: OperationPhase,
    ) -> Result<(), CentralUpdatesError> {
        match self {
            Self::Local => {
                let manifest = manifest.clone();
                run_blocking_fs_with(
                    "Central update durable rollback",
                    move || rollback_local(&manifest, phase),
                    CentralUpdatesError::task_join,
                )
                .await
            }
            Self::Remote(connection) => {
                let backup_exists = connection
                    .exists(&manifest.backup)
                    .await
                    .map_err(|error| CentralUpdatesError::Remote(error.to_string()))?;
                let target_exists = connection
                    .exists(&manifest.target)
                    .await
                    .map_err(|error| CentralUpdatesError::Remote(error.to_string()))?;
                let staging_exists = connection
                    .exists(&manifest.staging)
                    .await
                    .map_err(|error| CentralUpdatesError::Remote(error.to_string()))?;

                if backup_exists {
                    if target_exists {
                        verify_remote_hash(self, &manifest.target, &manifest.new_fingerprint)
                            .await?;
                    }
                    let expected = manifest.old_fingerprint.as_deref().ok_or_else(|| {
                        CentralOperationError::InvalidManifest(
                            "update backup has no old fingerprint".to_string(),
                        )
                    })?;
                    verify_remote_hash(self, &manifest.backup, expected).await?;
                } else if staging_exists {
                    verify_remote_hash(self, &manifest.staging, &manifest.new_fingerprint).await?;
                }
                if !backup_exists {
                    match (manifest.had_target, target_exists) {
                        (true, true) => {
                            let expected =
                                manifest.old_fingerprint.as_deref().ok_or_else(|| {
                                    CentralOperationError::InvalidManifest(
                                        "existing update target has no old fingerprint".to_string(),
                                    )
                                })?;
                            verify_remote_hash(self, &manifest.target, expected).await?;
                        }
                        (true, false) => {
                            return Err(CentralOperationError::RecoveryCollision {
                                code: "update_remote_rollback_target_missing",
                            }
                            .into());
                        }
                        (false, true) if phase == OperationPhase::Prepared => {
                            return Err(CentralOperationError::RecoveryCollision {
                                code: "update_remote_rollback_unexpected_target",
                            }
                            .into());
                        }
                        (false, true) => {
                            verify_remote_hash(self, &manifest.target, &manifest.new_fingerprint)
                                .await?;
                        }
                        (false, false) => {}
                    }
                }
                let had_target = if manifest.had_target { "1" } else { "0" };
                let output = connection
                    .run_script(
                        REMOTE_ROLLBACK_UPDATE,
                        &[
                            &manifest.operation_id,
                            &manifest.target,
                            &manifest.staging,
                            &manifest.backup,
                            &manifest.marker,
                            had_target,
                        ],
                    )
                    .await
                    .map_err(|_| CentralOperationError::Remote {
                        code: "update_remote_rollback",
                    })?;
                if output.trim() != "ROLLED_BACK" {
                    return Err(CentralOperationError::Remote {
                        code: "update_remote_rollback_protocol",
                    }
                    .into());
                }
                Ok(())
            }
        }
    }

    pub(crate) async fn finalize_operation_update(
        &self,
        manifest: &UpdateManifest,
    ) -> Result<(), CentralUpdatesError> {
        match self {
            Self::Local => {
                let manifest = manifest.clone();
                run_blocking_fs_with(
                    "Central update durable finalize",
                    move || finalize_local(&manifest),
                    CentralUpdatesError::task_join,
                )
                .await
            }
            Self::Remote(connection) => {
                verify_remote_hash(self, &manifest.target, &manifest.new_fingerprint).await?;
                if manifest.had_target
                    && connection
                        .exists(&manifest.backup)
                        .await
                        .map_err(|error| CentralUpdatesError::Remote(error.to_string()))?
                {
                    let expected = manifest.old_fingerprint.as_deref().ok_or_else(|| {
                        CentralOperationError::InvalidManifest(
                            "update backup has no old fingerprint".to_string(),
                        )
                    })?;
                    verify_remote_hash(self, &manifest.backup, expected).await?;
                }
                let output = connection
                    .run_script(
                        REMOTE_FINALIZE_UPDATE,
                        &[
                            &manifest.operation_id,
                            &manifest.target,
                            &manifest.staging,
                            &manifest.backup,
                            &manifest.marker,
                        ],
                    )
                    .await
                    .map_err(|_| CentralOperationError::Remote {
                        code: "update_remote_finalize",
                    })?;
                if output.trim() != "FINALIZED" {
                    return Err(CentralOperationError::Remote {
                        code: "update_remote_finalize_protocol",
                    }
                    .into());
                }
                Ok(())
            }
        }
    }
}

async fn stage_operation_updates_remote(
    fs: &CentralFs,
    connection: &crate::targets::ConnectedRemoteTarget,
    stages: Vec<OperationUpdateStage>,
    cancel: Option<&AtomicBool>,
) -> Vec<OperationUpdateStageOutcome> {
    let mut outcomes = Vec::with_capacity(stages.len());
    let mut groups = BTreeMap::<String, Vec<OperationUpdateStage>>::new();
    for stage in stages {
        let operation_id = stage.manifest.operation_id.clone();
        if let Err(error) = validate_operation_stage(&stage) {
            outcomes.push(OperationUpdateStageOutcome {
                operation_id,
                result: Err(error),
            });
            continue;
        }
        let parent = remote_parent(&stage.manifest.staging).expect("validated staging parent");
        groups.entry(parent).or_default().push(stage);
    }

    let mut staged = Vec::new();
    for (parent, group) in groups {
        for chunk in group.chunks(REMOTE_WRITE_CHUNK_SIZE) {
            let chunk = chunk.to_vec();
            if cancel.is_some_and(|flag| flag.load(Ordering::SeqCst)) {
                outcomes.extend(chunk.into_iter().map(|stage| OperationUpdateStageOutcome {
                    operation_id: stage.manifest.operation_id,
                    result: Err(CentralUpdatesError::BatchCancelled),
                }));
                continue;
            }
            let operation_ids = chunk
                .iter()
                .map(|stage| stage.manifest.operation_id.clone())
                .collect::<Vec<_>>();
            let skill_ids = chunk
                .iter()
                .map(|stage| stage.write.skill_id.clone())
                .collect::<Vec<_>>();
            let batch_root = format!(
                "{}/.skillport-operation-stage-batch-{}",
                parent.trim_end_matches('/'),
                Uuid::new_v4()
            );
            let archive_span = tracing::info_span!(
                "central_update_phase",
                phase = "durable_archive_build",
                skills = chunk.len(),
                payload_bytes = tracing::field::Empty
            );
            let archive_chunk = chunk.clone();
            let archive = run_blocking_fs_with(
                "Central update durable batch archive",
                move || build_operation_stage_archive(&archive_chunk),
                CentralUpdatesError::task_join,
            )
            .instrument(archive_span.clone())
            .await;
            let archive = match archive {
                Ok(archive) => {
                    archive_span.record("payload_bytes", archive.len());
                    archive
                }
                Err(error) => {
                    let message = error.to_string();
                    outcomes.extend(operation_ids.into_iter().map(|operation_id| {
                        OperationUpdateStageOutcome {
                            operation_id,
                            result: Err(CentralUpdatesError::Batch(message.clone())),
                        }
                    }));
                    continue;
                }
            };
            let command = format!(
                "sh -c {} -- {}",
                shell_quote(REMOTE_BATCH_STAGE_UPDATE),
                shell_quote(&batch_root)
            );
            match connection
                .run_command_with_stdin_bytes_cancellable(&command, &archive, cancel)
                .await
            {
                Ok(stdout) => {
                    match parse_batch_rows(&stdout, &skill_ids, "Central durable stage") {
                        Ok(rows) => {
                            for ((stage, (_, result)), operation_id) in
                                chunk.into_iter().zip(rows).zip(operation_ids)
                            {
                                if result.is_ok() {
                                    staged.push(stage);
                                } else {
                                    outcomes.push(OperationUpdateStageOutcome {
                                        operation_id,
                                        result,
                                    });
                                }
                            }
                        }
                        Err(error) => {
                            let message = error.to_string();
                            outcomes.extend(operation_ids.into_iter().map(|operation_id| {
                                OperationUpdateStageOutcome {
                                    operation_id,
                                    result: Err(CentralUpdatesError::Batch(message.clone())),
                                }
                            }));
                        }
                    }
                }
                Err(error) => {
                    let message = format!("Remote durable stage batch failed: {error}");
                    outcomes.extend(operation_ids.into_iter().map(|operation_id| {
                        OperationUpdateStageOutcome {
                            operation_id,
                            result: Err(CentralUpdatesError::Batch(message.clone())),
                        }
                    }));
                }
            }
        }
    }

    if staged.is_empty() {
        return outcomes;
    }
    let roots = staged
        .iter()
        .map(|stage| PathBuf::from(&stage.manifest.staging))
        .collect::<Vec<_>>();
    match fs.hash_directories(&roots).await {
        Ok(hashes) => outcomes.extend(staged.into_iter().map(|stage| {
            let operation_id = stage.manifest.operation_id.clone();
            let root = PathBuf::from(&stage.manifest.staging);
            let result = if hashes.get(&root) == Some(&stage.manifest.new_fingerprint) {
                Ok(())
            } else {
                Err(CentralOperationError::RecoveryCollision {
                    code: "update_staging_fingerprint",
                }
                .into())
            };
            OperationUpdateStageOutcome {
                operation_id,
                result,
            }
        })),
        Err(error) => {
            let message = error.to_string();
            outcomes.extend(staged.into_iter().map(|stage| OperationUpdateStageOutcome {
                operation_id: stage.manifest.operation_id,
                result: Err(CentralUpdatesError::Batch(message.clone())),
            }));
        }
    }
    outcomes
}

fn validate_operation_stage(stage: &OperationUpdateStage) -> Result<(), CentralUpdatesError> {
    validate_central_skill_write(&stage.write)?;
    crate::services::central_operation::OperationManifest::Update(stage.manifest.clone())
        .validate(&stage.manifest.operation_id)
        .map_err(CentralOperationError::InvalidManifest)?;
    if stage.manifest.operation_id.is_empty()
        || !stage
            .manifest
            .operation_id
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
        || [&stage.manifest.staging, &stage.manifest.marker]
            .iter()
            .any(|path| path.contains(['\t', '\n', '\r']))
        || remote_parent(&stage.manifest.staging) != remote_parent(&stage.manifest.marker)
    {
        return Err(CentralOperationError::InvalidManifest(
            "operation stage manifest is unsafe".to_string(),
        )
        .into());
    }
    Ok(())
}

pub(super) fn build_operation_stage_archive(
    stages: &[OperationUpdateStage],
) -> Result<Vec<u8>, CentralUpdatesError> {
    let encoder = GzEncoder::new(Vec::new(), Compression::default());
    let mut builder = tar::Builder::new(encoder);
    let mut manifest = String::new();
    for (index, stage) in stages.iter().enumerate() {
        validate_operation_stage(stage)?;
        let archive_key = format!("{index:04}");
        manifest.push_str(&format!(
            "{archive_key}\t{}\t{}\t{}\t{}\n",
            stage.write.skill_id,
            stage.manifest.operation_id,
            stage.manifest.staging,
            stage.manifest.marker
        ));
        for file in &stage.write.files {
            let archive_path = format!("{archive_key}/{}", file.relative_path);
            append_archive_bytes(&mut builder, &archive_path, &file.bytes).map_err(|error| {
                CentralUpdatesError::io(
                    format!("Failed to build durable update entry '{}'", file.repo_path),
                    error,
                )
            })?;
        }
    }
    append_archive_bytes(
        &mut builder,
        ".skillport-operation-manifest.tsv",
        manifest.as_bytes(),
    )
    .map_err(|error| CentralUpdatesError::io("Failed to build durable update manifest", error))?;
    let encoder = builder.into_inner().map_err(|error| {
        CentralUpdatesError::io("Failed to finalize durable update archive", error)
    })?;
    encoder.finish().map_err(|error| {
        CentralUpdatesError::io("Failed to compress durable update archive", error)
    })
}

fn append_archive_bytes<W: std::io::Write>(
    builder: &mut tar::Builder<W>,
    path: &str,
    bytes: &[u8],
) -> std::io::Result<()> {
    let mut header = tar::Header::new_gnu();
    header.set_size(bytes.len() as u64);
    header.set_mode(0o644);
    header.set_cksum();
    builder.append_data(&mut header, path, bytes)
}

mod helpers;
use helpers::{
    finalize_local, fingerprint_files, rollback_local, short_digest, stage_local, swap_local,
    verify_remote_hash,
};

#[cfg(test)]
mod tests;
