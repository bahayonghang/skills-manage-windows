use super::helpers::to_ipc_error;
use crate::services::skills_cli::SkillsCliError;

#[test]
fn dynamic_process_details_do_not_enter_the_ipc_envelope() {
    let secret = r"C:\Users\alice\private --force token=ghp_secret";
    let error = SkillsCliError::TaskJoin {
        label: "skills-cli",
        message: secret.to_string(),
    };
    let serialized = serde_json::to_string(&to_ipc_error(&error)).unwrap();
    assert!(serialized.contains("internal.unexpected"));
    assert!(!serialized.contains(secret));
    assert!(!serialized.contains("ghp_secret"));
}

#[test]
fn update_check_failed_is_retryable_at_the_ipc_boundary() {
    let error = SkillsCliError::UpdateCheckFailed;
    assert_eq!(error.ipc_code(), "skills_cli.update_check_failed");
    assert!(error.retryable());
    assert!(!SkillsCliError::UpdateBaselineRequired.retryable());
}
