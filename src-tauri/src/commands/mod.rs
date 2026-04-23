pub mod agents;
pub mod bootstrap;
pub mod collections;
pub mod discover;
pub mod github_import;
pub mod linker;
pub mod marketplace;
pub mod scanner;
pub mod settings;
pub mod skills;

pub const APP_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));
