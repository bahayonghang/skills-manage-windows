use std::process::ExitCode;

use clap::{Args, Parser, Subcommand, ValueEnum};
use serde::Serialize;
use serde_json::{json, Value};
use skillport_lib::cli_api::{CliApiError, CliContext};

#[derive(Debug, Clone, Copy, ValueEnum)]
enum Language {
    En,
    Zh,
}

#[derive(Debug, Parser)]
#[command(name = "skillport-cli", version, about = "Manage Local SkillPort skills")]
struct Cli {
    #[arg(long, global = true)]
    json: bool,
    #[arg(long, global = true, value_enum, default_value_t = Language::En)]
    lang: Language,
    #[command(subcommand)]
    command: TopCommand,
}

#[derive(Debug, Subcommand)]
enum TopCommand {
    Skills {
        #[command(subcommand)]
        command: SkillsCommand,
    },
}

#[derive(Debug, Subcommand)]
enum SkillsCommand {
    List,
    Show { reference: String },
    Search {
        query: String,
        #[arg(long)]
        limit: Option<u32>,
    },
    Install(InstallArgs),
    Sync(SyncArgs),
}

#[derive(Debug, Args)]
struct InstallArgs {
    source: String,
    #[arg(long)]
    replace: bool,
    #[arg(long)]
    yes: bool,
    #[arg(long)]
    sync: bool,
    #[arg(long = "agent")]
    agents: Vec<String>,
    #[arg(long, default_value = "auto")]
    method: String,
}

#[derive(Debug, Args)]
struct SyncArgs {
    references: Vec<String>,
    #[arg(long)]
    all: bool,
    #[arg(long = "agent")]
    agents: Vec<String>,
    #[arg(long, default_value = "auto")]
    method: String,
    #[arg(long)]
    dry_run: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SuccessEnvelope {
    schema_version: u8,
    ok: bool,
    data: Value,
    warnings: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ErrorEnvelope {
    schema_version: u8,
    ok: bool,
    error: ErrorPayload,
}

#[derive(Debug, Serialize)]
struct ErrorPayload {
    code: &'static str,
    message: String,
    details: Value,
}

struct CommandOutput {
    data: Value,
    partial: bool,
    action: &'static str,
}

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();
    match execute(&cli).await {
        Ok(output) => {
            render_success(&cli, &output);
            ExitCode::from(if output.partial { 5 } else { 0 })
        }
        Err(error) => {
            render_error(&cli, &error);
            ExitCode::from(error.exit_code())
        }
    }
}

async fn execute(cli: &Cli) -> Result<CommandOutput, CliApiError> {
    let context = CliContext::open_default().await?;
    match &cli.command {
        TopCommand::Skills { command } => match command {
            SkillsCommand::List => Ok(CommandOutput {
                data: to_value(context.list_skills().await?)?,
                partial: false,
                action: "list",
            }),
            SkillsCommand::Show { reference } => Ok(CommandOutput {
                data: to_value(context.show_skill(reference).await?)?,
                partial: false,
                action: "show",
            }),
            SkillsCommand::Search { query, limit } => Ok(CommandOutput {
                data: to_value(context.search_skills(query.clone(), *limit).await?)?,
                partial: false,
                action: "search",
            }),
            SkillsCommand::Install(args) => {
                let result = context
                    .install_skill(
                        &args.source,
                        args.replace,
                        args.yes,
                        args.sync,
                        args.agents.clone(),
                        &args.method,
                    )
                    .await?;
                let partial = result.is_partial_failure();
                Ok(CommandOutput {
                    data: to_value(result)?,
                    partial,
                    action: "install",
                })
            }
            SkillsCommand::Sync(args) => {
                let result = context
                    .sync_skills(
                        args.references.clone(),
                        args.all,
                        args.agents.clone(),
                        &args.method,
                        args.dry_run,
                    )
                    .await?;
                let partial = result.is_partial_failure();
                Ok(CommandOutput {
                    data: to_value(result)?,
                    partial,
                    action: if args.dry_run { "sync_preview" } else { "sync" },
                })
            }
        },
    }
}

fn to_value(value: impl Serialize) -> Result<Value, CliApiError> {
    serde_json::to_value(value).map_err(|error| CliApiError::Internal(error.to_string()))
}

fn render_success(cli: &Cli, output: &CommandOutput) {
    if cli.json {
        let envelope = SuccessEnvelope {
            schema_version: 1,
            ok: true,
            data: output.data.clone(),
            warnings: Vec::new(),
        };
        println!("{}", serde_json::to_string(&envelope).unwrap());
        return;
    }

    let label = match (cli.lang, output.action) {
        (Language::Zh, "list") => "中央技能",
        (Language::Zh, "show") => "技能详情",
        (Language::Zh, "search") => "搜索结果",
        (Language::Zh, "install") => "安装结果",
        (Language::Zh, "sync_preview") => "同步预览",
        (Language::Zh, _) => "同步结果",
        (Language::En, "list") => "Central skills",
        (Language::En, "show") => "Skill details",
        (Language::En, "search") => "Search results",
        (Language::En, "install") => "Install result",
        (Language::En, "sync_preview") => "Sync preview",
        (Language::En, _) => "Sync result",
    };
    println!("{label}");
    println!("{}", serde_json::to_string_pretty(&output.data).unwrap());
    if output.partial {
        eprintln!(
            "{}",
            match cli.lang {
                Language::En => "Some items failed.",
                Language::Zh => "部分项目失败。",
            }
        );
    }
}

fn render_error(cli: &Cli, error: &CliApiError) {
    if cli.json {
        let envelope = ErrorEnvelope {
            schema_version: 1,
            ok: false,
            error: ErrorPayload {
                code: error.code(),
                message: error.to_string(),
                details: json!({}),
            },
        };
        eprintln!("{}", serde_json::to_string(&envelope).unwrap());
        return;
    }
    eprintln!(
        "{}: {error}",
        match cli.lang {
            Language::En => "Error",
            Language::Zh => "错误",
        }
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn success_envelope_schema_is_versioned() {
        let encoded = serde_json::to_value(SuccessEnvelope {
            schema_version: 1,
            ok: true,
            data: json!({ "uid": "stable" }),
            warnings: Vec::new(),
        })
        .unwrap();
        assert_eq!(encoded["schemaVersion"], 1);
        assert_eq!(encoded["ok"], true);
        assert_eq!(encoded["data"]["uid"], "stable");
    }

    #[test]
    fn error_envelope_uses_locale_neutral_code() {
        let error = CliApiError::Ambiguous("ambiguous".to_string());
        let encoded = serde_json::to_value(ErrorEnvelope {
            schema_version: 1,
            ok: false,
            error: ErrorPayload {
                code: error.code(),
                message: error.to_string(),
                details: json!({}),
            },
        })
        .unwrap();
        assert_eq!(encoded["error"]["code"], "skill.ambiguous");
    }

    #[test]
    fn parser_requires_explicit_sync_scope() {
        let parsed = Cli::try_parse_from(["skillport-cli", "skills", "sync", "--all"]);
        assert!(parsed.is_ok());
        let parsed = Cli::try_parse_from([
            "skillport-cli",
            "--json",
            "skills",
            "install",
            "owner/repo@skill",
            "--sync",
            "--agent",
            "codex",
        ]);
        assert!(parsed.is_ok());
    }
}
