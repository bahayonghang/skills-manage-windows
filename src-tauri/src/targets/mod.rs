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

mod askpass;
mod commands;
mod cred;
mod error;
mod exec;
mod model;
mod registry;
mod remote;
mod runner;
#[cfg(test)]
mod tests;
mod wsl_discovery;

#[cfg(test)]
use askpass::*;
use commands::*;
use cred::*;
use exec::*;
use model::*;
#[cfg(test)]
use wsl_discovery::*;

pub use askpass::{connect_ssh_target, ConnectedSshTarget};
pub use commands::*;
pub use error::TargetsError;
pub use exec::*;
pub use model::*;
pub use registry::TargetRegistry;
pub use remote::*;
pub(crate) use runner::{CommandRunner, ProcessRunner, RunnerError, RunnerPhase};
pub use wsl_discovery::list_wsl_distributions_impl;
