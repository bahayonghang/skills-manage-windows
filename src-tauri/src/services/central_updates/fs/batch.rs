#[cfg(test)]
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

#[cfg(test)]
use flate2::{write::GzEncoder, Compression};
#[cfg(test)]
use tracing::Instrument;
#[cfg(test)]
use uuid::Uuid;

use crate::fs_util::run_blocking_fs_with;
#[cfg(test)]
use crate::targets::shell_quote;
use crate::targets::{remote_parent, ConnectedRemoteTarget};

use super::remote_scripts::REMOTE_BATCH_REFRESH_COPY_SCRIPT;
#[cfg(test)]
use super::remote_scripts::REMOTE_CENTRAL_BATCH_UPDATE_SCRIPT;
#[cfg(test)]
use super::write_skill_dir_atomic_local;
use super::{
    is_safe_relative_path, posix_path, refresh_copy_install_local, CentralFs, RemoteSkillFile,
};
use crate::services::central_updates::error::CentralUpdatesError;

#[cfg_attr(not(test), allow(dead_code))]
pub(crate) const REMOTE_WRITE_CHUNK_SIZE: usize = 16;
pub(crate) const REMOTE_COPY_CHUNK_SIZE: usize = 32;

#[derive(Debug, Clone)]
pub(crate) struct CentralSkillWrite {
    pub(crate) skill_id: String,
    pub(crate) target_dir: PathBuf,
    pub(crate) files: Vec<RemoteSkillFile>,
}

#[cfg(test)]
#[derive(Debug)]
pub(crate) struct CentralSkillWriteOutcome {
    #[allow(dead_code)]
    pub(crate) skill_id: String,
    pub(crate) result: Result<(), CentralUpdatesError>,
}

#[derive(Debug, Clone)]
pub(crate) struct CopyRefreshRequest {
    pub(crate) skill_id: String,
    pub(crate) source_dir: PathBuf,
    pub(crate) target: String,
}

#[derive(Debug)]
pub(crate) struct CopyRefreshOutcome {
    #[allow(dead_code)]
    pub(crate) skill_id: String,
    pub(crate) target: String,
    pub(crate) result: Result<(), CentralUpdatesError>,
}

type BatchRowOutcomes = Vec<(String, Result<(), CentralUpdatesError>)>;

impl CentralFs {
    #[tracing::instrument(
        skip_all,
        fields(
            phase = "central_write",
            target_kind = self.target_kind(),
            skills = writes.len(),
            write_chunks = writes.len().div_ceil(REMOTE_WRITE_CHUNK_SIZE)
        )
    )]
    #[cfg(test)]
    pub(crate) async fn write_skill_dirs_atomic_cancellable(
        &self,
        writes: Vec<CentralSkillWrite>,
        cancel: Option<&AtomicBool>,
    ) -> Vec<CentralSkillWriteOutcome> {
        match self {
            Self::Local => write_skill_dirs_atomic_local(writes).await,
            Self::Remote(conn) => write_skill_dirs_atomic_remote(conn, writes, cancel).await,
        }
    }

    #[tracing::instrument(
        skip_all,
        fields(
            phase = "copy_refresh",
            target_kind = self.target_kind(),
            copies = copies.len(),
            copy_chunks = copies.len().div_ceil(REMOTE_COPY_CHUNK_SIZE)
        )
    )]
    pub(crate) async fn refresh_copy_installs_cancellable(
        &self,
        copies: Vec<CopyRefreshRequest>,
        cancel: Option<&AtomicBool>,
    ) -> Vec<CopyRefreshOutcome> {
        match self {
            Self::Local => refresh_copy_installs_local(copies).await,
            Self::Remote(conn) => refresh_copy_installs_remote(conn, copies, cancel).await,
        }
    }

    pub(crate) fn target_kind(&self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Remote(connected) => match connected.as_ref() {
                ConnectedRemoteTarget::Ssh(_) => "ssh",
                ConnectedRemoteTarget::Wsl(_) => "wsl",
            },
        }
    }
}

#[cfg(test)]
async fn write_skill_dirs_atomic_local(
    writes: Vec<CentralSkillWrite>,
) -> Vec<CentralSkillWriteOutcome> {
    let skill_ids = writes
        .iter()
        .map(|write| write.skill_id.clone())
        .collect::<Vec<_>>();
    match run_blocking_fs_with(
        "central skill batch atomic write",
        move || {
            Ok(writes
                .into_iter()
                .map(|write| CentralSkillWriteOutcome {
                    skill_id: write.skill_id.clone(),
                    result: validate_central_skill_write(&write).and_then(|()| {
                        write_skill_dir_atomic_local(
                            &write.skill_id,
                            &write.target_dir,
                            &write.files,
                        )
                    }),
                })
                .collect())
        },
        CentralUpdatesError::task_join,
    )
    .await
    {
        Ok(outcomes) => outcomes,
        Err(error) => batch_error_outcomes(skill_ids, error.to_string()),
    }
}

#[cfg(test)]
async fn write_skill_dirs_atomic_remote(
    conn: &ConnectedRemoteTarget,
    writes: Vec<CentralSkillWrite>,
    cancel: Option<&AtomicBool>,
) -> Vec<CentralSkillWriteOutcome> {
    let mut outcomes = Vec::with_capacity(writes.len());
    let mut groups = BTreeMap::<String, Vec<CentralSkillWrite>>::new();
    for write in writes {
        if let Err(error) = validate_central_skill_write(&write) {
            outcomes.push(CentralSkillWriteOutcome {
                skill_id: write.skill_id,
                result: Err(error),
            });
            continue;
        }
        let target = posix_path(&write.target_dir);
        let Some(parent) = remote_parent(&target) else {
            outcomes.push(CentralSkillWriteOutcome {
                skill_id: write.skill_id.clone(),
                result: Err(CentralUpdatesError::RemoteTargetDirNoParent {
                    skill_id: write.skill_id,
                    target_dir: target,
                }),
            });
            continue;
        };
        groups.entry(parent).or_default().push(write);
    }

    for (parent, group) in groups {
        for chunk in group.chunks(REMOTE_WRITE_CHUNK_SIZE) {
            let chunk = chunk.to_vec();
            let skill_ids = chunk
                .iter()
                .map(|write| write.skill_id.clone())
                .collect::<Vec<_>>();
            if cancel.is_some_and(|flag| flag.load(Ordering::SeqCst)) {
                outcomes.extend(cancelled_write_outcomes(skill_ids));
                continue;
            }
            let batch_root = format!(
                "{}/.skillport-update-batch-{}",
                parent.trim_end_matches('/'),
                Uuid::new_v4()
            );
            let archive_span = tracing::info_span!(
                "central_update_phase",
                phase = "archive_build",
                skills = chunk.len(),
                payload_bytes = tracing::field::Empty
            );
            let archive = run_blocking_fs_with(
                "central skill batch archive",
                move || build_skill_batch_archive(&chunk),
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
                    outcomes.extend(batch_error_outcomes(skill_ids, error.to_string()));
                    continue;
                }
            };
            let command = remote_batch_update_command(&batch_root);
            match conn
                .run_command_with_stdin_bytes_cancellable(&command, &archive, cancel)
                .await
            {
                Ok(stdout) => match parse_batch_rows(&stdout, &skill_ids, "Central write") {
                    Ok(results) => {
                        outcomes.extend(results.into_iter().map(|(skill_id, result)| {
                            CentralSkillWriteOutcome { skill_id, result }
                        }))
                    }
                    Err(error) => {
                        outcomes.extend(batch_error_outcomes(skill_ids, error.to_string()));
                    }
                },
                Err(error) => outcomes.extend(batch_error_outcomes(
                    skill_ids,
                    format!("Remote batch update failed: {error}"),
                )),
            }
        }
    }
    outcomes
}

async fn refresh_copy_installs_local(copies: Vec<CopyRefreshRequest>) -> Vec<CopyRefreshOutcome> {
    let fallback = copies
        .iter()
        .map(|copy| (copy.skill_id.clone(), copy.target.clone()))
        .collect::<Vec<_>>();
    match run_blocking_fs_with(
        "copy install batch refresh",
        move || {
            Ok(copies
                .into_iter()
                .map(|copy| CopyRefreshOutcome {
                    skill_id: copy.skill_id.clone(),
                    target: copy.target.clone(),
                    result: refresh_copy_install_local(
                        &copy.skill_id,
                        &copy.source_dir,
                        &copy.target,
                    ),
                })
                .collect())
        },
        CentralUpdatesError::task_join,
    )
    .await
    {
        Ok(outcomes) => outcomes,
        Err(error) => fallback
            .into_iter()
            .map(|(skill_id, target)| CopyRefreshOutcome {
                skill_id,
                target,
                result: Err(CentralUpdatesError::Batch(error.to_string())),
            })
            .collect(),
    }
}

async fn refresh_copy_installs_remote(
    conn: &ConnectedRemoteTarget,
    copies: Vec<CopyRefreshRequest>,
    cancel: Option<&AtomicBool>,
) -> Vec<CopyRefreshOutcome> {
    let mut outcomes = Vec::with_capacity(copies.len());
    let mut valid = Vec::new();
    for copy in copies {
        if remote_basename(&copy.target) != Some(copy.skill_id.as_str()) {
            outcomes.push(CopyRefreshOutcome {
                skill_id: copy.skill_id,
                target: copy.target.clone(),
                result: Err(CentralUpdatesError::CopyInstallOutsideSkillDir(copy.target)),
            });
        } else {
            valid.push(copy);
        }
    }

    for chunk in valid.chunks(REMOTE_COPY_CHUNK_SIZE) {
        if cancel.is_some_and(|flag| flag.load(Ordering::SeqCst)) {
            outcomes.extend(chunk.iter().map(|copy| CopyRefreshOutcome {
                skill_id: copy.skill_id.clone(),
                target: copy.target.clone(),
                result: Err(CentralUpdatesError::BatchCancelled),
            }));
            continue;
        }
        let mut args = Vec::with_capacity(chunk.len() * 3);
        for copy in chunk {
            args.push(copy.skill_id.clone());
            args.push(posix_path(&copy.source_dir));
            args.push(copy.target.clone());
        }
        let arg_refs = args.iter().map(String::as_str).collect::<Vec<_>>();
        let skill_ids = chunk
            .iter()
            .map(|copy| copy.skill_id.clone())
            .collect::<Vec<_>>();
        match conn
            .run_script_cancellable(REMOTE_BATCH_REFRESH_COPY_SCRIPT, &arg_refs, cancel)
            .await
        {
            Ok(stdout) => match parse_batch_rows(stdout.as_bytes(), &skill_ids, "Copy refresh") {
                Ok(results) => {
                    outcomes.extend(chunk.iter().zip(results).map(|(copy, (_, result))| {
                        CopyRefreshOutcome {
                            skill_id: copy.skill_id.clone(),
                            target: copy.target.clone(),
                            result,
                        }
                    }))
                }
                Err(error) => outcomes.extend(chunk.iter().map(|copy| CopyRefreshOutcome {
                    skill_id: copy.skill_id.clone(),
                    target: copy.target.clone(),
                    result: Err(CentralUpdatesError::Batch(error.to_string())),
                })),
            },
            Err(error) => outcomes.extend(chunk.iter().map(|copy| CopyRefreshOutcome {
                skill_id: copy.skill_id.clone(),
                target: copy.target.clone(),
                result: Err(CentralUpdatesError::Remote(format!(
                    "Remote copy refresh batch failed: {error}"
                ))),
            })),
        }
    }
    outcomes
}

#[cfg(test)]
pub(super) fn build_skill_batch_archive(
    writes: &[CentralSkillWrite],
) -> Result<Vec<u8>, CentralUpdatesError> {
    let encoder = GzEncoder::new(Vec::new(), Compression::default());
    let mut builder = tar::Builder::new(encoder);
    let mut manifest = String::new();

    for (index, write) in writes.iter().enumerate() {
        validate_central_skill_write(write)?;
        let archive_key = format!("{index:04}");
        manifest.push_str(&archive_key);
        manifest.push('\t');
        manifest.push_str(&write.skill_id);
        manifest.push('\t');
        manifest.push_str(&posix_path(&write.target_dir));
        manifest.push('\n');

        for file in &write.files {
            let archive_path = format!("{archive_key}/{}", file.relative_path);
            append_archive_bytes(&mut builder, &archive_path, &file.bytes).map_err(|error| {
                CentralUpdatesError::io(
                    format!("Failed to build update archive entry '{}'", file.repo_path),
                    error,
                )
            })?;
        }
    }
    append_archive_bytes(&mut builder, ".skillport-manifest.tsv", manifest.as_bytes())
        .map_err(|error| CentralUpdatesError::io("Failed to build batch update manifest", error))?;

    let encoder = builder.into_inner().map_err(|error| {
        CentralUpdatesError::io("Failed to finalize batch update archive", error)
    })?;
    encoder
        .finish()
        .map_err(|error| CentralUpdatesError::io("Failed to compress batch update archive", error))
}

#[cfg(test)]
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

pub(super) fn validate_central_skill_write(
    write: &CentralSkillWrite,
) -> Result<(), CentralUpdatesError> {
    if write.skill_id.is_empty()
        || !write
            .skill_id
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
    {
        return Err(CentralUpdatesError::Batch(format!(
            "Skill id '{}' is unsafe for a batch update.",
            write.skill_id
        )));
    }
    let target = posix_path(&write.target_dir);
    if target.contains(['\t', '\n', '\r']) || remote_parent(&target).is_none() {
        return Err(CentralUpdatesError::RemoteTargetDirNoParent {
            skill_id: write.skill_id.clone(),
            target_dir: target,
        });
    }
    for file in &write.files {
        if !is_safe_relative_path(&file.relative_path)
            || file.relative_path.contains(['\t', '\n', '\r'])
        {
            return Err(CentralUpdatesError::UnsupportedRepoFilePath(
                file.repo_path.clone(),
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
fn remote_batch_update_command(batch_root: &str) -> String {
    format!(
        "sh -c {} -- {}",
        shell_quote(REMOTE_CENTRAL_BATCH_UPDATE_SCRIPT),
        shell_quote(batch_root)
    )
}

pub(super) fn parse_batch_rows(
    stdout: &[u8],
    expected_skill_ids: &[String],
    label: &str,
) -> Result<BatchRowOutcomes, CentralUpdatesError> {
    let output = std::str::from_utf8(stdout).map_err(|error| {
        CentralUpdatesError::Batch(format!("{label} returned non-UTF-8 output: {error}"))
    })?;
    let rows = output.lines().collect::<Vec<_>>();
    if rows.len() != expected_skill_ids.len() {
        return Err(CentralUpdatesError::Batch(format!(
            "{label} returned {} result rows for {} skills.",
            rows.len(),
            expected_skill_ids.len()
        )));
    }

    rows.into_iter()
        .zip(expected_skill_ids)
        .map(|(row, expected_skill_id)| {
            let mut fields = row.splitn(3, '\t');
            let status = fields.next().unwrap_or_default();
            let skill_id = fields.next().unwrap_or_default();
            if skill_id != expected_skill_id {
                return Err(CentralUpdatesError::Batch(format!(
                    "{label} returned an unexpected skill id."
                )));
            }
            let result = match status {
                "OK" => Ok(()),
                "ERR" => Err(CentralUpdatesError::Batch(format!(
                    "{label} failed for skill '{}': {}",
                    skill_id,
                    fields.next().unwrap_or("unknown_error")
                ))),
                _ => {
                    return Err(CentralUpdatesError::Batch(format!(
                        "{label} returned an invalid result row."
                    )))
                }
            };
            Ok((skill_id.to_string(), result))
        })
        .collect()
}

#[cfg(test)]
fn batch_error_outcomes(skill_ids: Vec<String>, message: String) -> Vec<CentralSkillWriteOutcome> {
    skill_ids
        .into_iter()
        .map(|skill_id| CentralSkillWriteOutcome {
            skill_id,
            result: Err(CentralUpdatesError::Batch(message.clone())),
        })
        .collect()
}

#[cfg(test)]
fn cancelled_write_outcomes(skill_ids: Vec<String>) -> Vec<CentralSkillWriteOutcome> {
    skill_ids
        .into_iter()
        .map(|skill_id| CentralSkillWriteOutcome {
            skill_id,
            result: Err(CentralUpdatesError::BatchCancelled),
        })
        .collect()
}

fn remote_basename(path: &str) -> Option<&str> {
    path.trim_end_matches('/').rsplit('/').next()
}
