//! GitHub repository import service layer.
//!
//! Owns GitHub/PAT access, repository source discovery, preview/import staging,
//! SSH preview workspace lifecycle, archive extraction, and shared helper types.
//! Tauri IPC shells live in `crate::commands::github_import`.

use chrono::{DateTime, Duration, Utc};
use flate2::read::GzDecoder;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::io::{Cursor, Read};
use std::path::{Component, Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use tauri::{AppHandle, Emitter, State};

use crate::{
    db::{self, DbPool, Skill},
    targets::{connect_ssh_target, remote_join, ActiveTarget, RemoteTargetConfig},
    AppState,
};

include!("types.rs");
include!("preview_workspace.rs");
include!("preview.rs");
include!("remote.rs");
include!("import.rs");
include!("source.rs");
include!("archive.rs");
include!("raw_http.rs");
include!("pat.rs");

#[cfg(test)]
include!("tests.rs");
