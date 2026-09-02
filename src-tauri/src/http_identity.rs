//! Compile-time HTTP identity shared by commands and services.
//!
//! This module is intentionally free of Tauri, command, database, and service
//! dependencies so both layers can send the same `<package>/<version>` user-agent.

pub(crate) const APP_USER_AGENT: &str =
    concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));
