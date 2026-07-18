//! Module-level integration tests for local archive import.
//!
//! These tests exercise the full preview → import pipeline end-to-end using
//! in-memory databases and temp directories, verifying:
//! - Preview is read-only (no filesystem/DB mutation).
//! - Fingerprint verification catches byte-level changes.
//! - Overwrite backs up and replaces the existing skill.
//! - Rename writes to a new skill id without touching the original.
//! - Skip produces no write.
//! - Archive skills have no GitHub repository assignment.

#![cfg(test)]
