//! Compile-time command logging policy types.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationCategory {
    Startup,
    Target,
    Sync,
    Logs,
    Scan,
    Agent,
    Install,
    Central,
    Catalog,
    Settings,
    Secret,
    Update,
    Obsidian,
    Import,
    Project,
    Marketplace,
    PortableState,
    Usage,
    SkillsCli,
}

impl OperationCategory {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Startup => "startup",
            Self::Target => "target",
            Self::Sync => "sync",
            Self::Logs => "logs",
            Self::Scan => "scan",
            Self::Agent => "agent",
            Self::Install => "install",
            Self::Central => "central",
            Self::Catalog => "catalog",
            Self::Settings => "settings",
            Self::Secret => "secret",
            Self::Update => "update",
            Self::Obsidian => "obsidian",
            Self::Import => "import",
            Self::Project => "project",
            Self::Marketplace => "marketplace",
            Self::PortableState => "portable_state",
            Self::Usage => "usage",
            Self::SkillsCli => "skills_cli",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationPhase {
    Command,
    Startup,
    Database,
    Filesystem,
    Network,
    Recovery,
    Job,
}

impl OperationPhase {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Command => "command",
            Self::Startup => "startup",
            Self::Database => "database",
            Self::Filesystem => "filesystem",
            Self::Network => "network",
            Self::Recovery => "recovery",
            Self::Job => "job",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationLifecycle {
    TerminalOnly,
    StartedThenTerminal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeOnlyReason {
    ReadOnly,
    Preview,
    InternalRefresh,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExclusionReason {
    SelfLogging,
    FrontendReadyBridge,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OperationAction(&'static str);

impl OperationAction {
    /// Registry-only constructor. Product commands retrieve definitions by
    /// command name instead of inventing action strings at call sites.
    #[doc(hidden)]
    pub const fn registered(command: &'static str) -> Self {
        Self(command)
    }

    pub const fn as_str(self) -> &'static str {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OperationDefinition {
    category: OperationCategory,
    action: OperationAction,
    default_phase: OperationPhase,
    lifecycle: OperationLifecycle,
}

impl OperationDefinition {
    #[doc(hidden)]
    pub const fn registered(
        command: &'static str,
        category: OperationCategory,
        default_phase: OperationPhase,
        lifecycle: OperationLifecycle,
    ) -> Self {
        Self {
            category,
            action: OperationAction::registered(command),
            default_phase,
            lifecycle,
        }
    }

    pub const fn category(self) -> OperationCategory {
        self.category
    }

    pub const fn action(self) -> OperationAction {
        self.action
    }

    pub const fn default_phase(self) -> OperationPhase {
        self.default_phase
    }

    pub const fn lifecycle(self) -> OperationLifecycle {
        self.lifecycle
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandLogPolicy {
    Operation(OperationDefinition),
    RuntimeOnly(RuntimeOnlyReason),
    Excluded(ExclusionReason),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandPolicyEntry {
    pub command: &'static str,
    pub policy: CommandLogPolicy,
}

impl CommandPolicyEntry {
    #[doc(hidden)]
    pub const fn operation(
        command: &'static str,
        category: OperationCategory,
        phase: OperationPhase,
        lifecycle: OperationLifecycle,
    ) -> Self {
        Self {
            command,
            policy: CommandLogPolicy::Operation(OperationDefinition::registered(
                command, category, phase, lifecycle,
            )),
        }
    }

    #[doc(hidden)]
    pub const fn runtime_only(command: &'static str, reason: RuntimeOnlyReason) -> Self {
        Self {
            command,
            policy: CommandLogPolicy::RuntimeOnly(reason),
        }
    }

    #[doc(hidden)]
    pub const fn excluded(command: &'static str, reason: ExclusionReason) -> Self {
        Self {
            command,
            policy: CommandLogPolicy::Excluded(reason),
        }
    }
}
