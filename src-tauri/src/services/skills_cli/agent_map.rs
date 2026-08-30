//! Complete mapping closure between SkillPort builtin platform ids and
//! official Skills CLI `--agent` ids.
//!
//! Every SkillPort builtin id must be either mapped or explicitly listed as
//! unsupported; a table-driven test locks this closure against the seed list.

/// SkillPort builtin id → Skills CLI `--agent` id.
pub const SKILLS_CLI_AGENT_MAP: &[(&str, &str)] = &[
    ("claude-code", "claude-code"),
    ("codex", "codex"),
    ("grok", "grok"),
    ("cursor", "cursor"),
    ("gemini-cli", "gemini-cli"),
    ("trae", "trae"),
    ("factory-droid", "droid"),
    ("junie", "junie"),
    ("qwen", "qwen-code"),
    ("trae-cn", "trae-cn"),
    ("windsurf", "windsurf"),
    ("qoder", "qoder"),
    ("augment", "augment"),
    ("opencode", "opencode"),
    ("kilocode", "kilo"),
    ("amp", "amp"),
    ("kiro", "kiro-cli"),
    ("codebuddy", "codebuddy"),
    ("hermes", "hermes-agent"),
    ("copilot", "github-copilot"),
    ("antigravity", "antigravity"),
    ("antigravity-cli", "antigravity-cli"),
    ("zed", "zed"),
    ("cline", "cline"),
    ("deep-agents", "deepagents"),
    ("firebender", "firebender"),
    ("kimi-code-cli", "kimi-code-cli"),
    ("warp", "warp"),
    ("aider", "aider-desk"),
    ("reasonix", "reasonix"),
    ("openclaw", "openclaw"),
];

/// Builtin ids with no Skills CLI target. Selector hides them; tests lock the
/// reason per id.
pub const SKILLS_CLI_UNSUPPORTED: &[(&str, &str)] = &[
    ("ob1", "official CLI has no matching --agent id"),
    ("qclaw", "lobster derivative without a dedicated CLI id"),
    ("easyclaw", "lobster derivative without a dedicated CLI id"),
    ("autoclaw", "lobster derivative without a dedicated CLI id"),
    ("workbuddy", "lobster derivative without a dedicated CLI id"),
    ("central", "not a platform target"),
];

/// The CLI `--agent` id for a SkillPort builtin platform, if one exists.
pub fn cli_agent_for_skillport_id(skillport_id: &str) -> Option<&'static str> {
    SKILLS_CLI_AGENT_MAP
        .iter()
        .find(|(id, _)| *id == skillport_id)
        .map(|(_, cli)| *cli)
}

/// Whether the builtin is explicitly unsupported (selector must hide it).
pub fn is_explicitly_unsupported(skillport_id: &str) -> bool {
    SKILLS_CLI_UNSUPPORTED
        .iter()
        .any(|(id, _)| *id == skillport_id)
}

/// Map selected SkillPort ids to deduplicated CLI agent ids.
///
/// Returns [`super::SkillsCliError::AgentUnmapped`] for the first id with no
/// reviewed mapping so callers never silently drop platforms.
pub fn map_skillport_ids_to_cli_agents(
    skillport_ids: &[String],
) -> Result<Vec<String>, super::SkillsCliError> {
    let mut mapped: Vec<String> = Vec::with_capacity(skillport_ids.len());
    for id in skillport_ids {
        let Some(cli) = cli_agent_for_skillport_id(id) else {
            return Err(super::SkillsCliError::AgentUnmapped(id.clone()));
        };
        if !mapped.iter().any(|existing| existing == cli) {
            mapped.push(cli.to_string());
        }
    }
    Ok(mapped)
}
