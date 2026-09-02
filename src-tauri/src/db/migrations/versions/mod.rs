use sha2::{Digest, Sha256};

pub(crate) mod v1;
pub(super) mod v2;
pub(super) mod v3;
pub(super) mod v4;
pub(super) mod v5;
pub(super) mod v6;
pub(super) mod v7;

#[derive(Debug, Clone, Copy)]
pub(super) struct MigrationDescriptor {
    pub(super) version: i64,
    pub(super) source: &'static str,
    pub(super) legacy_checksums: &'static [&'static str],
}

pub(super) const PUBLISHED_WINDOWS_V1_CHECKSUM: &str =
    "aabde4fd51822355cbe2a7982ac895073f6e49e9f34882a50086d145462a736d";
/// Checksum written by released binaries before base CREATE TABLE included
/// the ensure_column upgrade columns. Preflight accepts this exact digest.
pub(super) const PUBLISHED_PRE_FINAL_SCHEMA_V1_CHECKSUM: &str =
    "173296a19419edf197e3baa3b22de1f33184a1d8631141549751fbf1cfc24f7f";

pub(super) const MIGRATIONS: [MigrationDescriptor; 7] = [
    MigrationDescriptor {
        version: 1,
        source: v1::SOURCE,
        legacy_checksums: &[
            PUBLISHED_WINDOWS_V1_CHECKSUM,
            PUBLISHED_PRE_FINAL_SCHEMA_V1_CHECKSUM,
        ],
    },
    MigrationDescriptor {
        version: 2,
        source: v2::SOURCE,
        legacy_checksums: &[],
    },
    MigrationDescriptor {
        version: 3,
        source: v3::SOURCE,
        legacy_checksums: &[],
    },
    MigrationDescriptor {
        version: 4,
        source: v4::SOURCE,
        legacy_checksums: &[],
    },
    MigrationDescriptor {
        version: 5,
        source: v5::SOURCE,
        legacy_checksums: &[],
    },
    MigrationDescriptor {
        version: 6,
        source: v6::SOURCE,
        legacy_checksums: &[],
    },
    MigrationDescriptor {
        version: 7,
        source: v7::SOURCE,
        legacy_checksums: &[],
    },
];

pub(super) fn checksum(source: &str) -> String {
    // Normalize newlines so Windows CRLF working trees and Unix LF checkouts
    // produce the same migration lock hash from identical logical sources.
    let normalized = source.replace("\r\n", "\n").replace('\r', "\n");
    crate::hashing::encode_lower_hex(Sha256::digest(normalized.as_bytes()).as_ref())
}
