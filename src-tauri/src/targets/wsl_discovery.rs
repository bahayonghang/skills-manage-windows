use super::*;

pub async fn list_wsl_distributions_impl() -> Result<Vec<WslDistributionSummary>, TargetsError> {
    #[cfg(not(windows))]
    {
        Err(TargetsError::WslDiscoveryWindowsOnly)
    }

    #[cfg(windows)]
    {
        let output = ProcessRunner
            .run(ProcessRequest::new(
                wsl_distribution_list_command(),
                ProcessPolicy::probe(),
            ))
            .await
            .map_err(wsl_runner_error)?;
        if !output.status.success() {
            let detail = normalize_wsl_list_output(&output.stderr);
            let detail = detail.trim();
            return Err(if detail.is_empty() {
                TargetsError::WslListFailed
            } else {
                TargetsError::WslListFailedDetail(detail.to_string())
            });
        }

        Ok(parse_wsl_distribution_list(&normalize_wsl_list_output(
            &output.stdout,
        )))
    }
}

#[cfg(windows)]
pub(super) fn wsl_distribution_list_command() -> Command {
    let mut command = Command::new(super::wsl_program());
    hide_child_window(&mut command);
    command.arg("-l").arg("-v").stdin(Stdio::null());
    command
}

#[cfg(any(test, windows))]
pub(super) fn normalize_wsl_list_output(bytes: &[u8]) -> String {
    let nul_count = bytes.iter().filter(|byte| **byte == 0).count();
    if !bytes.is_empty() && nul_count > bytes.len() / 4 {
        let mut words = Vec::with_capacity(bytes.len() / 2);
        let (chunks, remainder) = bytes.as_chunks::<2>();
        for chunk in chunks {
            words.push(u16::from_le_bytes([chunk[0], chunk[1]]));
        }
        let mut decoded = String::from_utf16_lossy(&words);
        if let Some(remainder) = remainder.first() {
            if *remainder != 0 {
                decoded.push(char::from(*remainder));
            }
        }
        decoded.replace('\u{feff}', "")
    } else {
        String::from_utf8_lossy(bytes).replace('\0', "")
    }
}

#[cfg(any(test, windows))]
pub(super) fn parse_wsl_distribution_list(output: &str) -> Vec<WslDistributionSummary> {
    output
        .lines()
        .filter_map(parse_wsl_distribution_row)
        .collect()
}

#[cfg(any(test, windows))]
fn parse_wsl_distribution_row(line: &str) -> Option<WslDistributionSummary> {
    let trimmed = line.trim();
    let is_default = trimmed.starts_with('*');
    let row = if is_default {
        trimmed.trim_start_matches('*').trim()
    } else {
        trimmed
    };
    if row.is_empty() || row.to_ascii_uppercase().starts_with("NAME") {
        return None;
    }

    let columns = row.split_whitespace().collect::<Vec<_>>();
    match columns.as_slice() {
        [] => None,
        [name] => Some(WslDistributionSummary {
            name: (*name).to_string(),
            is_default,
            state: None,
            version: None,
        }),
        [name, state] => Some(WslDistributionSummary {
            name: (*name).to_string(),
            is_default,
            state: Some((*state).to_string()),
            version: None,
        }),
        _ => {
            let version = columns.last().map(|value| (*value).to_string());
            let state = columns
                .get(columns.len().saturating_sub(2))
                .map(|value| (*value).to_string());
            let name = columns[..columns.len().saturating_sub(2)].join(" ");
            if name.is_empty() {
                None
            } else {
                Some(WslDistributionSummary {
                    name,
                    is_default,
                    state,
                    version,
                })
            }
        }
    }
}
