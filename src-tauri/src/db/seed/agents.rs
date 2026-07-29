use std::path::Path;

use super::super::types::*;

const DEFAULT_ENABLED_PLATFORM_IDS: [&str; 7] = [
    "claude-code",
    "codex",
    "grok",
    "antigravity",
    "antigravity-cli",
    "opencode",
    "kiro",
];

/// Returns the list of built-in agents using the current user's home directory.
pub fn builtin_agents() -> Vec<Agent> {
    builtin_agents_for_home(&crate::paths::resolve_home_dir())
}

pub fn builtin_agents_for_posix_home(home: &str) -> Vec<Agent> {
    let home = normalize_posix_home(home);
    let central_skills_dir = posix_join(&home, &[crate::paths::APP_DATA_DIR_NAME, "skills"]);
    let universal_skills_dir =
        posix_join(&home, &[crate::paths::UNIVERSAL_AGENTS_DIR_NAME, "skills"]);

    let rewrite_path = |local_path: &str| -> String {
        let normalized = local_path.replace('\\', "/");
        if normalized.ends_with(&format!("/{}", crate::paths::CENTRAL_SKILLS_REL_FROM_HOME)) {
            return central_skills_dir.clone();
        }
        if normalized.ends_with(&format!("/{}", crate::paths::UNIVERSAL_SKILLS_REL)) {
            return universal_skills_dir.clone();
        }

        let Some(relative) = normalized
            .rsplit("/.")
            .next()
            .map(|suffix| format!(".{suffix}"))
        else {
            return normalized;
        };

        posix_join(&home, &[relative.as_str()])
    };

    builtin_agents_for_home(Path::new("/__skillport_remote_home__"))
        .into_iter()
        .map(|mut agent| {
            agent.global_skills_dir = rewrite_path(&agent.global_skills_dir);
            if let Some(project_skills_dir) = agent.project_skills_dir.as_deref() {
                agent.project_skills_dir = Some(project_skills_dir.replace('\\', "/"));
            }
            agent
        })
        .collect()
}

fn normalize_posix_home(home: &str) -> String {
    let mut value = home.trim().replace('\\', "/");
    while value.len() > 1 && value.ends_with('/') {
        value.pop();
    }
    if value.is_empty() {
        "/".to_string()
    } else {
        value
    }
}

fn posix_join(home: &str, segments: &[&str]) -> String {
    let mut value = normalize_posix_home(home);
    for segment in segments {
        let segment = segment.trim_matches('/');
        if segment.is_empty() {
            continue;
        }
        if value == "/" {
            value.push_str(segment);
        } else {
            value.push('/');
            value.push_str(segment);
        }
    }
    value
}

fn is_builtin_agent_enabled_by_default(agent_id: &str, category: &str) -> bool {
    if category == "central" {
        return true;
    }

    if category == "lobster" {
        return false;
    }

    DEFAULT_ENABLED_PLATFORM_IDS.contains(&agent_id)
}

pub fn is_universal_agent(agent_id: &str) -> bool {
    UNIVERSAL_AGENT_IDS.contains(&agent_id)
}

pub fn is_universal_project_agent(agent_id: &str) -> bool {
    UNIVERSAL_PROJECT_AGENT_IDS.contains(&agent_id)
}

fn builtin_coding_agent(
    id: &str,
    display_name: &str,
    global_skills_dir: String,
    project_skills_dir: Option<&str>,
    icon_name: &str,
) -> Agent {
    Agent {
        id: id.to_string(),
        display_name: display_name.to_string(),
        category: "coding".to_string(),
        global_skills_dir,
        project_skills_dir: project_skills_dir.map(str::to_string),
        icon_name: Some(icon_name.to_string()),
        is_detected: false,
        is_builtin: true,
        is_enabled: is_builtin_agent_enabled_by_default(id, "coding"),
    }
}

fn builtin_agents_for_home(home: &Path) -> Vec<Agent> {
    let central_skills_dir = crate::paths::central_skills_dir_from_home(home)
        .to_string_lossy()
        .into_owned();
    let universal_skills_dir = crate::paths::universal_skills_dir_from_home(home)
        .to_string_lossy()
        .into_owned();

    let skill_dir = |segments: &[&str]| -> String {
        segments
            .iter()
            .fold(home.to_path_buf(), |path, segment| path.join(segment))
            .to_string_lossy()
            .into_owned()
    };

    let agent_skill_dir = |agent_id: &str, segments: &[&str]| -> String {
        if is_universal_agent(agent_id) {
            return universal_skills_dir.clone();
        }

        skill_dir(segments)
    };

    vec![
        // ── Coding platforms ─────────────────────────────────────────────────
        builtin_coding_agent(
            "claude-code",
            "Claude Code",
            skill_dir(&[".claude", "skills"]),
            Some(".claude/skills"),
            "claude",
        ),
        builtin_coding_agent(
            "codex",
            "Codex CLI",
            agent_skill_dir("codex", &[".codex", "skills"]),
            Some(UNIVERSAL_PROJECT_SKILLS_DIR),
            "codex",
        ),
        builtin_coding_agent(
            "grok",
            "Grok",
            skill_dir(&[".grok", "skills"]),
            Some(".grok/skills"),
            "grok",
        ),
        builtin_coding_agent(
            "cursor",
            "Cursor",
            agent_skill_dir("cursor", &[".cursor", "skills"]),
            Some(UNIVERSAL_PROJECT_SKILLS_DIR),
            "cursor",
        ),
        builtin_coding_agent(
            "gemini-cli",
            "Gemini CLI (legacy)",
            skill_dir(&[".gemini", "skills"]),
            Some(UNIVERSAL_PROJECT_SKILLS_DIR),
            "gemini",
        ),
        builtin_coding_agent(
            "trae",
            "Trae",
            skill_dir(&[".trae", "skills"]),
            None,
            "trae",
        ),
        builtin_coding_agent(
            "factory-droid",
            "Factory Droid",
            skill_dir(&[".factory", "skills"]),
            None,
            "factory",
        ),
        builtin_coding_agent(
            "junie",
            "Junie",
            skill_dir(&[".junie", "skills"]),
            None,
            "junie",
        ),
        builtin_coding_agent(
            "qwen",
            "Qwen",
            skill_dir(&[".qwen", "skills"]),
            None,
            "qwen",
        ),
        Agent {
            id: "trae-cn".to_string(),
            display_name: "Trae CN".to_string(),
            category: "coding".to_string(),
            global_skills_dir: skill_dir(&[".trae-cn", "skills"]),
            project_skills_dir: None,
            icon_name: Some("trae-cn".to_string()),
            is_detected: false,
            is_builtin: true,
            is_enabled: is_builtin_agent_enabled_by_default("trae-cn", "coding"),
        },
        Agent {
            id: "windsurf".to_string(),
            display_name: "Windsurf".to_string(),
            category: "coding".to_string(),
            global_skills_dir: skill_dir(&[".windsurf", "skills"]),
            project_skills_dir: None,
            icon_name: Some("windsurf".to_string()),
            is_detected: false,
            is_builtin: true,
            is_enabled: is_builtin_agent_enabled_by_default("windsurf", "coding"),
        },
        Agent {
            id: "qoder".to_string(),
            display_name: "Qoder".to_string(),
            category: "coding".to_string(),
            global_skills_dir: skill_dir(&[".qoder", "skills"]),
            project_skills_dir: None,
            icon_name: Some("qoder".to_string()),
            is_detected: false,
            is_builtin: true,
            is_enabled: is_builtin_agent_enabled_by_default("qoder", "coding"),
        },
        Agent {
            id: "augment".to_string(),
            display_name: "Augment".to_string(),
            category: "coding".to_string(),
            global_skills_dir: skill_dir(&[".augment", "skills"]),
            project_skills_dir: None,
            icon_name: Some("augment".to_string()),
            is_detected: false,
            is_builtin: true,
            is_enabled: is_builtin_agent_enabled_by_default("augment", "coding"),
        },
        Agent {
            id: "opencode".to_string(),
            display_name: "OpenCode".to_string(),
            category: "coding".to_string(),
            global_skills_dir: agent_skill_dir("opencode", &[".opencode", "skills"]),
            project_skills_dir: Some(UNIVERSAL_PROJECT_SKILLS_DIR.to_string()),
            icon_name: Some("opencode".to_string()),
            is_detected: false,
            is_builtin: true,
            is_enabled: is_builtin_agent_enabled_by_default("opencode", "coding"),
        },
        Agent {
            id: "kilocode".to_string(),
            display_name: "KiloCode".to_string(),
            category: "coding".to_string(),
            global_skills_dir: skill_dir(&[".kilocode", "skills"]),
            project_skills_dir: None,
            icon_name: Some("kilocode".to_string()),
            is_detected: false,
            is_builtin: true,
            is_enabled: is_builtin_agent_enabled_by_default("kilocode", "coding"),
        },
        Agent {
            id: "ob1".to_string(),
            display_name: "OB1".to_string(),
            category: "coding".to_string(),
            global_skills_dir: skill_dir(&[".ob1", "skills"]),
            project_skills_dir: None,
            icon_name: Some("ob1".to_string()),
            is_detected: false,
            is_builtin: true,
            is_enabled: is_builtin_agent_enabled_by_default("ob1", "coding"),
        },
        Agent {
            id: "amp".to_string(),
            display_name: "Amp".to_string(),
            category: "coding".to_string(),
            global_skills_dir: agent_skill_dir("amp", &[".amp", "skills"]),
            project_skills_dir: Some(UNIVERSAL_PROJECT_SKILLS_DIR.to_string()),
            icon_name: Some("amp".to_string()),
            is_detected: false,
            is_builtin: true,
            is_enabled: is_builtin_agent_enabled_by_default("amp", "coding"),
        },
        Agent {
            id: "kiro".to_string(),
            display_name: "Kiro".to_string(),
            category: "coding".to_string(),
            global_skills_dir: skill_dir(&[".kiro", "skills"]),
            project_skills_dir: Some(".kiro/skills".to_string()),
            icon_name: Some("kiro".to_string()),
            is_detected: false,
            is_builtin: true,
            is_enabled: is_builtin_agent_enabled_by_default("kiro", "coding"),
        },
        Agent {
            id: "codebuddy".to_string(),
            display_name: "CodeBuddy".to_string(),
            category: "coding".to_string(),
            global_skills_dir: skill_dir(&[".codebuddy", "skills"]),
            project_skills_dir: None,
            icon_name: Some("codebuddy".to_string()),
            is_detected: false,
            is_builtin: true,
            is_enabled: is_builtin_agent_enabled_by_default("codebuddy", "coding"),
        },
        Agent {
            id: "hermes".to_string(),
            display_name: "Hermes".to_string(),
            category: "lobster".to_string(),
            global_skills_dir: skill_dir(&[".hermes", "skills"]),
            project_skills_dir: None,
            icon_name: Some("hermes".to_string()),
            is_detected: false,
            is_builtin: true,
            is_enabled: is_builtin_agent_enabled_by_default("hermes", "lobster"),
        },
        Agent {
            id: "copilot".to_string(),
            display_name: "GitHub Copilot".to_string(),
            category: "coding".to_string(),
            global_skills_dir: agent_skill_dir("copilot", &[".copilot", "skills"]),
            project_skills_dir: None,
            icon_name: Some("copilot".to_string()),
            is_detected: false,
            is_builtin: true,
            is_enabled: is_builtin_agent_enabled_by_default("copilot", "coding"),
        },
        Agent {
            id: "antigravity".to_string(),
            display_name: "Antigravity".to_string(),
            category: "coding".to_string(),
            global_skills_dir: skill_dir(&[".gemini", "antigravity", "skills"]),
            project_skills_dir: Some(UNIVERSAL_PROJECT_SKILLS_DIR.to_string()),
            icon_name: Some("antigravity".to_string()),
            is_detected: false,
            is_builtin: true,
            is_enabled: is_builtin_agent_enabled_by_default("antigravity", "coding"),
        },
        Agent {
            id: "antigravity-cli".to_string(),
            display_name: "Antigravity CLI".to_string(),
            category: "coding".to_string(),
            global_skills_dir: skill_dir(&[".gemini", "antigravity-cli", "skills"]),
            project_skills_dir: Some(UNIVERSAL_PROJECT_SKILLS_DIR.to_string()),
            icon_name: Some("antigravity".to_string()),
            is_detected: false,
            is_builtin: true,
            is_enabled: is_builtin_agent_enabled_by_default("antigravity-cli", "coding"),
        },
        Agent {
            id: "zed".to_string(),
            display_name: "Zed".to_string(),
            category: "coding".to_string(),
            global_skills_dir: skill_dir(&[".config", "zed", "skills"]),
            project_skills_dir: None,
            icon_name: Some("zed".to_string()),
            is_detected: false,
            is_builtin: true,
            is_enabled: is_builtin_agent_enabled_by_default("zed", "coding"),
        },
        Agent {
            id: "cline".to_string(),
            display_name: "Cline".to_string(),
            category: "coding".to_string(),
            global_skills_dir: agent_skill_dir("cline", &[".cline", "skills"]),
            project_skills_dir: None,
            icon_name: Some("cline".to_string()),
            is_detected: false,
            is_builtin: true,
            is_enabled: is_builtin_agent_enabled_by_default("cline", "coding"),
        },
        Agent {
            id: "deep-agents".to_string(),
            display_name: "Deep Agents".to_string(),
            category: "coding".to_string(),
            global_skills_dir: agent_skill_dir("deep-agents", &[".deep-agents", "skills"]),
            project_skills_dir: None,
            icon_name: Some("deep-agents".to_string()),
            is_detected: false,
            is_builtin: true,
            is_enabled: is_builtin_agent_enabled_by_default("deep-agents", "coding"),
        },
        Agent {
            id: "firebender".to_string(),
            display_name: "Firebender".to_string(),
            category: "coding".to_string(),
            global_skills_dir: agent_skill_dir("firebender", &[".firebender", "skills"]),
            project_skills_dir: None,
            icon_name: Some("firebender".to_string()),
            is_detected: false,
            is_builtin: true,
            is_enabled: is_builtin_agent_enabled_by_default("firebender", "coding"),
        },
        Agent {
            id: "kimi-code-cli".to_string(),
            display_name: "Kimi Code CLI".to_string(),
            category: "coding".to_string(),
            global_skills_dir: agent_skill_dir("kimi-code-cli", &[".kimi-code", "skills"]),
            project_skills_dir: None,
            icon_name: Some("kimi-code".to_string()),
            is_detected: false,
            is_builtin: true,
            is_enabled: is_builtin_agent_enabled_by_default("kimi-code-cli", "coding"),
        },
        Agent {
            id: "warp".to_string(),
            display_name: "Warp".to_string(),
            category: "coding".to_string(),
            global_skills_dir: agent_skill_dir("warp", &[".warp", "skills"]),
            project_skills_dir: None,
            icon_name: Some("warp".to_string()),
            is_detected: false,
            is_builtin: true,
            is_enabled: is_builtin_agent_enabled_by_default("warp", "coding"),
        },
        Agent {
            id: "aider".to_string(),
            display_name: "Aider".to_string(),
            category: "coding".to_string(),
            global_skills_dir: skill_dir(&[".aider", "skills"]),
            project_skills_dir: None,
            icon_name: Some("aider".to_string()),
            is_detected: false,
            is_builtin: true,
            is_enabled: is_builtin_agent_enabled_by_default("aider", "coding"),
        },
        Agent {
            id: "reasonix".to_string(),
            display_name: "Reasonix".to_string(),
            category: "coding".to_string(),
            global_skills_dir: skill_dir(&[".reasonix", "skills"]),
            project_skills_dir: Some(".reasonix/skills".to_string()),
            icon_name: Some("reasonix".to_string()),
            is_detected: false,
            is_builtin: true,
            is_enabled: is_builtin_agent_enabled_by_default("reasonix", "coding"),
        },
        // ── Lobster platforms ────────────────────────────────────────────────
        Agent {
            id: "openclaw".to_string(),
            display_name: "OpenClaw".to_string(),
            category: "lobster".to_string(),
            global_skills_dir: skill_dir(&[".openclaw", "skills"]),
            project_skills_dir: None,
            icon_name: Some("openclaw".to_string()),
            is_detected: false,
            is_builtin: true,
            is_enabled: is_builtin_agent_enabled_by_default("openclaw", "lobster"),
        },
        Agent {
            id: "qclaw".to_string(),
            display_name: "QClaw".to_string(),
            category: "lobster".to_string(),
            global_skills_dir: skill_dir(&[".qclaw", "skills"]),
            project_skills_dir: None,
            icon_name: Some("qclaw".to_string()),
            is_detected: false,
            is_builtin: true,
            is_enabled: is_builtin_agent_enabled_by_default("qclaw", "lobster"),
        },
        Agent {
            id: "easyclaw".to_string(),
            display_name: "EasyClaw".to_string(),
            category: "lobster".to_string(),
            global_skills_dir: skill_dir(&[".easyclaw", "skills"]),
            project_skills_dir: None,
            icon_name: Some("easyclaw".to_string()),
            is_detected: false,
            is_builtin: true,
            is_enabled: is_builtin_agent_enabled_by_default("easyclaw", "lobster"),
        },
        Agent {
            id: "autoclaw".to_string(),
            display_name: "AutoClaw".to_string(),
            category: "lobster".to_string(),
            global_skills_dir: skill_dir(&[".openclaw-autoclaw", "skills"]),
            project_skills_dir: None,
            icon_name: Some("autoclaw".to_string()),
            is_detected: false,
            is_builtin: true,
            is_enabled: is_builtin_agent_enabled_by_default("autoclaw", "lobster"),
        },
        Agent {
            id: "workbuddy".to_string(),
            display_name: "WorkBuddy".to_string(),
            category: "lobster".to_string(),
            global_skills_dir: skill_dir(&[".workbuddy", "skills-marketplace", "skills"]),
            project_skills_dir: None,
            icon_name: Some("workbuddy".to_string()),
            is_detected: false,
            is_builtin: true,
            is_enabled: is_builtin_agent_enabled_by_default("workbuddy", "lobster"),
        },
        // ── Central Skills ────────────────────────────────────────────────────
        Agent {
            id: "central".to_string(),
            display_name: "Central Skills".to_string(),
            category: "central".to_string(),
            global_skills_dir: central_skills_dir.clone(),
            project_skills_dir: None,
            icon_name: Some("central".to_string()),
            is_detected: false,
            is_builtin: true,
            is_enabled: is_builtin_agent_enabled_by_default("central", "central"),
        },
    ]
}
