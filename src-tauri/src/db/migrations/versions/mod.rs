use sha2::{Digest, Sha256};

pub(crate) mod v1;
pub(super) mod v2;

#[derive(Debug, Clone, Copy)]
pub(super) struct MigrationDescriptor {
    pub(super) version: i64,
    pub(super) source: &'static str,
}

pub(super) const MIGRATIONS: [MigrationDescriptor; 2] = [
    MigrationDescriptor {
        version: 1,
        source: v1::SOURCE,
    },
    MigrationDescriptor {
        version: 2,
        source: v2::SOURCE,
    },
];

pub(super) fn checksum(source: &str) -> String {
    format!("{:x}", Sha256::digest(source.as_bytes()))
}
