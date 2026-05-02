use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::ffi::OsString;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, ExitStatus, Stdio};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

use crate::db::{self, DbPool};

include!("model.rs");
include!("cred.rs");
include!("registry.rs");
include!("commands.rs");
include!("askpass.rs");
include!("exec.rs");
#[cfg(test)]
include!("tests.rs");
