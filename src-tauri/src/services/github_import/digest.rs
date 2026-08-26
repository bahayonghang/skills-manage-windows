//! Stable content digest v1 for GitHub preview snapshots.
//!
//! Preview registers an immutable snapshot; import must be able to prove the
//! bytes it is about to write are the bytes the user confirmed. Digests are
//! therefore domain-separated and unambiguously framed, so the value never
//! depends on `HashMap` iteration order or on string concatenation:
//!
//! ```text
//! domain_len:u64be | domain_bytes |
//! record_count:u64be |
//! repeat(path_len:u64be | path_bytes | byte_len:u64be | sha256_raw[32])
//! ```
//!
//! Paths are already normalized safe repository-relative paths (see
//! `source::normalize_repo_path`) and are sorted by UTF-8 byte order before
//! framing. The serialized form is `sha256-v1:<lowercase hex>`.

use sha2::{Digest, Sha256};

/// Domain separator for the whole retained repository snapshot.
pub(super) const REPOSITORY_SNAPSHOT_DIGEST_DOMAIN: &str =
    "skillport.github.repository-snapshot.v1";
/// Domain separator for one candidate skill's file set.
pub(super) const SKILL_CONTENT_DIGEST_DOMAIN: &str = "skillport.github.skill-content.v1";

const DIGEST_ALGORITHM_PREFIX: &str = "sha256-v1";

/// One framed digest record: repository- or skill-relative path, byte length,
/// and the raw SHA-256 of the file's bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DigestFileEntry {
    pub(super) path: String,
    pub(super) byte_len: u64,
    pub(super) sha256: [u8; 32],
}

pub(super) fn file_sha256(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

pub(super) fn hex_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push_str(&format!("{:02x}", byte));
    }
    out
}

/// Compute the framed aggregate digest for `entries` under `domain`.
///
/// Ordering is enforced inside this function so callers cannot leak an
/// unstable iteration order into the digest.
pub(super) fn aggregate_digest(domain: &str, entries: &[DigestFileEntry]) -> String {
    let mut ordered = entries.to_vec();
    ordered.sort_by(|left, right| left.path.as_bytes().cmp(right.path.as_bytes()));

    let mut hasher = Sha256::new();
    hasher.update((domain.len() as u64).to_be_bytes());
    hasher.update(domain.as_bytes());
    hasher.update((ordered.len() as u64).to_be_bytes());
    for entry in &ordered {
        hasher.update((entry.path.len() as u64).to_be_bytes());
        hasher.update(entry.path.as_bytes());
        hasher.update(entry.byte_len.to_be_bytes());
        hasher.update(entry.sha256);
    }

    format!(
        "{}:{}",
        DIGEST_ALGORITHM_PREFIX,
        hex_encode(hasher.finalize().as_slice())
    )
}

/// Domain-separated skill content digest from already-loaded file bytes.
/// Paths must be skill-relative POSIX paths. Ordering is enforced here.
pub(crate) fn skill_content_digest_from_file_bytes(files: &[(String, Vec<u8>)]) -> String {
    let entries = files
        .iter()
        .map(|(path, bytes)| DigestFileEntry {
            path: path.clone(),
            byte_len: bytes.len() as u64,
            sha256: file_sha256(bytes),
        })
        .collect::<Vec<_>>();
    aggregate_digest(SKILL_CONTENT_DIGEST_DOMAIN, &entries)
}
