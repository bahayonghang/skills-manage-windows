use super::*;
#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use sqlx::SqlitePool;
    use std::path::Path;

    #[derive(Default)]
    struct MemoryCredentialBackend {
        passwords: Mutex<HashMap<String, String>>,
        set_error: Mutex<Option<CredentialStoreError>>,
        get_error: Mutex<Option<CredentialStoreError>>,
        delete_error: Mutex<Option<CredentialStoreError>>,
    }

    impl MemoryCredentialBackend {
        fn with_get_error(error: CredentialStoreError) -> Arc<Self> {
            Arc::new(Self {
                get_error: Mutex::new(Some(error)),
                ..Self::default()
            })
        }

        fn with_set_error(error: CredentialStoreError) -> Arc<Self> {
            Arc::new(Self {
                set_error: Mutex::new(Some(error)),
                ..Self::default()
            })
        }
    }

    impl CredentialBackend for MemoryCredentialBackend {
        fn set_password(
            &self,
            credential_key: &str,
            password: &str,
        ) -> Result<(), CredentialStoreError> {
            if let Some(error) = self.set_error.lock().unwrap().clone() {
                return Err(error);
            }
            self.passwords
                .lock()
                .unwrap()
                .insert(credential_key.to_string(), password.to_string());
            Ok(())
        }

        fn get_password(&self, credential_key: &str) -> Result<String, CredentialStoreError> {
            if let Some(error) = self.get_error.lock().unwrap().clone() {
                return Err(error);
            }
            self.passwords
                .lock()
                .unwrap()
                .get(credential_key)
                .cloned()
                .ok_or(CredentialStoreError::NoEntry)
        }

        fn delete_credential(&self, credential_key: &str) -> Result<(), CredentialStoreError> {
            if let Some(error) = self.delete_error.lock().unwrap().clone() {
                return Err(error);
            }
            self.passwords.lock().unwrap().remove(credential_key);
            Ok(())
        }
    }

    async fn memory_db() -> DbPool {
        let pool = SqlitePool::connect(":memory:").await.unwrap();
        db::init_database(&pool).await.unwrap();
        pool
    }

    fn password_target() -> RemoteTargetConfig {
        RemoteTargetConfig {
            id: "ssh-demo".to_string(),
            label: "Lab".to_string(),
            host: "lab.local".to_string(),
            username: "alice".to_string(),
            port: 22,
            auth_method: SshAuthMethod::Password,
            key_path: String::new(),
            credential_key: Some("ssh-demo:ssh-password".to_string()),
            protected_password: None,
            password: Some("secret".to_string()),
            remote_home: "/home/alice".to_string(),
            remote_os: "Linux".to_string(),
            symlink_enabled: false,
        }
    }

    fn wsl_target() -> WslTargetConfig {
        WslTargetConfig {
            id: "wsl-ubuntu".to_string(),
            label: "Ubuntu".to_string(),
            distribution: "Ubuntu-24.04".to_string(),
            remote_home: "/home/alice".to_string(),
            remote_os: "Linux".to_string(),
            symlink_enabled: true,
        }
    }

    fn command_arg_strings(command: Command) -> Vec<String> {
        command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect()
    }

    #[cfg(windows)]
    fn command_env_strings(command: &Command) -> HashMap<String, Option<String>> {
        command
            .get_envs()
            .map(|(key, value)| {
                (
                    key.to_string_lossy().into_owned(),
                    value.map(|value| value.to_string_lossy().into_owned()),
                )
            })
            .collect()
    }

    fn ssh_connection_for(target: RemoteTargetConfig) -> ConnectedSshTarget {
        ConnectedSshTarget {
            password: target.password.clone(),
            target,
            askpass_helper: None,
        }
    }

    #[tokio::test]
    async fn active_target_defaults_to_local() {
        let pool = memory_db().await;
        assert_eq!(active_target_id(&pool).await.unwrap(), LOCAL_TARGET_ID);
    }

    #[test]
    fn remote_cache_path_is_scoped_by_target_id() {
        let path = remote_cache_db_path("ssh-demo_1").unwrap();
        assert!(path.ends_with(Path::new("targets").join("ssh-demo_1").join("db.sqlite")));
    }

    #[test]
    fn wsl_request_to_config_trims_required_fields() {
        let config = request_to_wsl_config(
            CreateWslTargetRequest {
                label: "  Ubuntu  ".to_string(),
                distribution: "  Ubuntu-24.04  ".to_string(),
            },
            "wsl-demo".to_string(),
        )
        .unwrap();

        assert_eq!(config.id, "wsl-demo");
        assert_eq!(config.label, "Ubuntu");
        assert_eq!(config.distribution, "Ubuntu-24.04");
        assert!(config.remote_home.is_empty());
        assert!(config.remote_os.is_empty());
    }

    #[test]
    fn update_wsl_request_preserves_probe_state() {
        let existing = wsl_target();
        let config = update_wsl_request_to_config(
            UpdateWslTargetRequest {
                id: existing.id.clone(),
                label: "  Work WSL  ".to_string(),
                distribution: "  Ubuntu  ".to_string(),
            },
            &existing,
        )
        .unwrap();

        assert_eq!(config.label, "Work WSL");
        assert_eq!(config.distribution, "Ubuntu");
        assert_eq!(config.remote_home, "/home/alice");
        assert_eq!(config.remote_os, "Linux");
        assert!(config.symlink_enabled);
    }

    #[test]
    fn wsl_base_command_passes_distribution_before_shell() {
        let connection = ConnectedWslTarget {
            target: wsl_target(),
        };
        let args = command_arg_strings(connection.base_command());

        assert_eq!(args, vec!["-d", "Ubuntu-24.04", "--"]);
    }

    #[test]
    fn parse_wsl_distribution_list_reads_default_state_and_version() {
        let distributions = parse_wsl_distribution_list(
            "  NAME            STATE           VERSION\n* Ubuntu-24.04    Stopped         2\n  Debian          Running         2\n",
        );

        assert_eq!(
            distributions,
            vec![
                WslDistributionSummary {
                    name: "Ubuntu-24.04".to_string(),
                    is_default: true,
                    state: Some("Stopped".to_string()),
                    version: Some("2".to_string()),
                },
                WslDistributionSummary {
                    name: "Debian".to_string(),
                    is_default: false,
                    state: Some("Running".to_string()),
                    version: Some("2".to_string()),
                },
            ]
        );
    }

    #[test]
    fn normalize_wsl_distribution_list_decodes_utf16le_output() {
        let raw = "  NAME      STATE    VERSION\r\n* Ubuntu    Stopped  2\r\n"
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect::<Vec<_>>();

        let distributions = parse_wsl_distribution_list(&normalize_wsl_list_output(&raw));

        assert_eq!(distributions.len(), 1);
        assert_eq!(distributions[0].name, "Ubuntu");
        assert!(distributions[0].is_default);
        assert_eq!(distributions[0].state.as_deref(), Some("Stopped"));
        assert_eq!(distributions[0].version.as_deref(), Some("2"));
    }

    #[test]
    fn normalize_wsl_distribution_list_strips_nul_bytes_from_utf8ish_output() {
        let mut raw = b"Ubuntu-24.04\r\n".to_vec();
        raw.insert(1, 0);
        let normalized = normalize_wsl_list_output(&raw);

        assert!(normalized.contains("Ubuntu-24.04"));
    }

    #[test]
    fn remote_join_uses_posix_separators() {
        assert_eq!(
            remote_join("/home/alice", ".skillsmanage/skills"),
            "/home/alice/.skillsmanage/skills"
        );
    }

    #[test]
    fn remote_script_command_runs_script_via_stdin_with_quoted_args() {
        let command = remote_script_command(&["/tmp/demo path", "plain"]);

        assert_eq!(command, "sh -s -- '/tmp/demo path' 'plain'");
    }

    #[test]
    fn parse_ssh_probe_output_reads_home_os_and_mkdir_status() {
        let probe = parse_ssh_probe_output("HOME\t/home/alice\nOS\tLinux\nMKDIR_OK\n").unwrap();

        assert_eq!(probe.remote_home, "/home/alice");
        assert_eq!(probe.remote_os, "Linux");
    }

    #[test]
    fn parse_ssh_probe_output_rejects_missing_home_or_mkdir_confirmation() {
        let missing_home = parse_ssh_probe_output("OS\tLinux\nMKDIR_OK\n");
        let missing_mkdir = parse_ssh_probe_output("HOME\t/home/alice\nOS\tLinux\n");

        assert!(missing_home.is_err());
        assert!(missing_mkdir.is_err());
    }

    #[test]
    fn askpass_helper_mode_requires_marker_env() {
        assert_eq!(
            askpass_password_from_env(None, Some(OsString::from("secret"))),
            None
        );
        assert_eq!(
            askpass_password_from_env(Some(OsString::from("1")), Some(OsString::from("secret"))),
            Some("secret".to_string())
        );
        assert_eq!(
            askpass_password_from_env(Some(OsString::from("1")), None),
            Some(String::new())
        );
    }

    #[cfg(windows)]
    #[test]
    fn windows_askpass_helper_uses_current_exe_without_temp_script() {
        let helper = create_askpass_helper().expect("askpass");

        assert_eq!(helper.path, env::current_exe().expect("current exe"));
        assert!(!helper.remove_on_drop);
        assert!(helper.use_app_helper_env);
        assert_ne!(
            helper.path.extension().and_then(|value| value.to_str()),
            Some("cmd")
        );
    }

    #[cfg(not(windows))]
    #[test]
    fn unix_askpass_helper_writes_shell_script() {
        let helper = create_askpass_helper().expect("askpass");
        let content = fs::read_to_string(&helper.path).expect("read askpass");
        let _ = fs::remove_file(&helper.path);

        assert!(helper.remove_on_drop);
        assert!(!helper.use_app_helper_env);
        assert!(content.contains("$SKILLPORT_SSH_PASSWORD"));
    }

    #[cfg(not(windows))]
    #[test]
    fn unix_askpass_helper_drop_removes_temp_file() {
        let helper = create_askpass_helper().expect("askpass");
        let helper_path = helper.path.clone();
        assert!(helper_path.exists());

        drop(helper);

        assert!(!helper_path.exists());
    }

    #[test]
    fn ssh_base_command_places_key_options_before_destination() {
        let mut target = password_target();
        target.auth_method = SshAuthMethod::Key;
        target.key_path = "C:\\Users\\alice\\.ssh\\id_ed25519".to_string();
        target.credential_key = None;
        target.password = None;
        let args = command_arg_strings(ssh_connection_for(target).base_command());
        let destination = args
            .iter()
            .position(|arg| arg == "alice@lab.local")
            .expect("destination");
        let key_option = args.iter().position(|arg| arg == "-i").expect("key option");
        let key_path = args
            .iter()
            .position(|arg| arg == "C:\\Users\\alice\\.ssh\\id_ed25519")
            .expect("key path");
        let batch_mode = args
            .iter()
            .position(|arg| arg == "BatchMode=yes")
            .expect("batch mode");
        let preferred = args
            .iter()
            .position(|arg| arg == "PreferredAuthentications=publickey")
            .expect("preferred auth");
        let connect_timeout = args
            .iter()
            .position(|arg| arg == "ConnectTimeout=10")
            .expect("connect timeout");
        let keepalive_interval = args
            .iter()
            .position(|arg| arg == "ServerAliveInterval=15")
            .expect("keepalive interval");
        let keepalive_count = args
            .iter()
            .position(|arg| arg == "ServerAliveCountMax=3")
            .expect("keepalive count");

        assert!(key_option < destination);
        assert!(key_path < destination);
        assert!(batch_mode < destination);
        assert!(preferred < destination);
        assert!(connect_timeout < destination);
        assert!(keepalive_interval < destination);
        assert!(keepalive_count < destination);
        assert_eq!(args.last().map(String::as_str), Some("alice@lab.local"));
    }

    #[test]
    fn ssh_base_command_places_password_options_before_destination() {
        let args = command_arg_strings(ssh_connection_for(password_target()).base_command());
        let destination = args
            .iter()
            .position(|arg| arg == "alice@lab.local")
            .expect("destination");
        let batch_mode = args
            .iter()
            .position(|arg| arg == "BatchMode=no")
            .expect("batch mode");
        let preferred = args
            .iter()
            .position(|arg| arg == "PreferredAuthentications=password,keyboard-interactive")
            .expect("preferred auth");
        let pubkey_disabled = args
            .iter()
            .position(|arg| arg == "PubkeyAuthentication=no")
            .expect("pubkey disabled");
        let prompt_count = args
            .iter()
            .position(|arg| arg == "NumberOfPasswordPrompts=1")
            .expect("prompt count");
        let connect_timeout = args
            .iter()
            .position(|arg| arg == "ConnectTimeout=10")
            .expect("connect timeout");
        let keepalive_interval = args
            .iter()
            .position(|arg| arg == "ServerAliveInterval=15")
            .expect("keepalive interval");
        let keepalive_count = args
            .iter()
            .position(|arg| arg == "ServerAliveCountMax=3")
            .expect("keepalive count");

        assert!(batch_mode < destination);
        assert!(preferred < destination);
        assert!(pubkey_disabled < destination);
        assert!(prompt_count < destination);
        assert!(connect_timeout < destination);
        assert!(keepalive_interval < destination);
        assert!(keepalive_count < destination);
        assert_eq!(args.last().map(String::as_str), Some("alice@lab.local"));
    }

    #[cfg(windows)]
    #[test]
    fn ssh_base_command_uses_hidden_app_askpass_on_windows() {
        let target = password_target();
        let password = target.password.clone();
        let helper = create_askpass_helper().expect("askpass");
        let helper_path = helper.path.to_string_lossy().into_owned();
        let connection = ConnectedSshTarget {
            password,
            target,
            askpass_helper: Some(helper),
        };

        let command = connection.base_command();
        let envs = command_env_strings(&command);

        assert_eq!(
            envs.get("SSH_ASKPASS").and_then(|value| value.as_deref()),
            Some(helper_path.as_str())
        );
        assert_eq!(
            envs.get(SSH_ASKPASS_HELPER_ENV)
                .and_then(|value| value.as_deref()),
            Some("1")
        );
        assert_eq!(
            envs.get(SSH_PASSWORD_ENV)
                .and_then(|value| value.as_deref()),
            Some("secret")
        );
    }

    #[test]
    fn ssh_auth_failure_detail_explains_password_denied() {
        let connection = ssh_connection_for(password_target());
        let detail = connection
            .ssh_failure_detail(b"alice@lab.local: Permission denied (publickey,password).\n");

        assert!(detail.contains("using password authentication"));
        assert!(detail.contains("remote sshd allows password"));
        assert!(detail.contains("Raw ssh error"));
    }

    #[cfg(windows)]
    #[test]
    fn windows_ssh_hidden_window_flag_matches_create_no_window() {
        assert_eq!(CREATE_NO_WINDOW, 0x08000000);
    }

    #[cfg(windows)]
    #[test]
    fn protected_password_roundtrips_with_windows_dpapi() {
        let protected = protected_credentials::protect("secret").unwrap();

        assert_ne!(protected, "secret");
        assert_eq!(
            protected_credentials::unprotect(&protected).unwrap(),
            "secret"
        );
    }

    #[test]
    fn key_target_requires_key_path() {
        let result = request_to_config(
            CreateSshTargetRequest {
                label: "Lab".to_string(),
                host: "lab.local".to_string(),
                username: "alice".to_string(),
                port: Some(22),
                auth_method: Some(SshAuthMethod::Key),
                key_path: None,
                password: None,
                passphrase: None,
            },
            "ssh-test".to_string(),
        );

        assert!(result.unwrap_err().contains("keyPath"));
    }

    #[test]
    fn password_target_uses_credential_key_without_key_path() {
        let target = request_to_config(
            CreateSshTargetRequest {
                label: "Lab".to_string(),
                host: "lab.local".to_string(),
                username: "alice".to_string(),
                port: Some(22),
                auth_method: Some(SshAuthMethod::Password),
                key_path: None,
                password: Some("secret".to_string()),
                passphrase: None,
            },
            "ssh-test".to_string(),
        )
        .unwrap();

        assert_eq!(target.auth_method, SshAuthMethod::Password);
        assert!(target.key_path.is_empty());
        assert_eq!(
            target.credential_key.as_deref(),
            Some("ssh-test:ssh-password")
        );
        assert_eq!(target.password.as_deref(), Some("secret"));
    }

    #[test]
    fn supported_remote_os_allows_symlink_for_legacy_targets() {
        let mut target = password_target();
        target.remote_os = "Linux".to_string();
        target.symlink_enabled = false;

        assert!(remote_symlink_allowed(&target));
    }

    #[test]
    fn unsupported_remote_os_keeps_symlink_disabled_without_override() {
        let mut target = password_target();
        target.remote_os = "unknown".to_string();
        target.symlink_enabled = false;

        assert!(!remote_symlink_allowed(&target));
    }

    #[test]
    fn password_override_repairs_existing_password_target() {
        let mut target = RemoteTargetConfig {
            id: "ssh-demo".to_string(),
            label: "Lab".to_string(),
            host: "lab.local".to_string(),
            username: "alice".to_string(),
            port: 22,
            auth_method: SshAuthMethod::Password,
            key_path: String::new(),
            credential_key: None,
            protected_password: None,
            password: None,
            remote_home: "/home/alice".to_string(),
            remote_os: "Linux".to_string(),
            symlink_enabled: false,
        };

        let changed =
            apply_supplied_password_to_existing_target(&mut target, Some("secret")).unwrap();

        assert!(changed);
        assert_eq!(target.password.as_deref(), Some("secret"));
        assert_eq!(
            target.credential_key.as_deref(),
            Some("ssh-demo:ssh-password")
        );
    }

    #[test]
    fn password_override_ignores_key_targets() {
        let mut target = RemoteTargetConfig {
            id: "ssh-demo".to_string(),
            label: "Lab".to_string(),
            host: "lab.local".to_string(),
            username: "alice".to_string(),
            port: 22,
            auth_method: SshAuthMethod::Key,
            key_path: "~/.ssh/id_ed25519".to_string(),
            credential_key: None,
            protected_password: None,
            password: None,
            remote_home: "/home/alice".to_string(),
            remote_os: "Linux".to_string(),
            symlink_enabled: false,
        };

        let changed =
            apply_supplied_password_to_existing_target(&mut target, Some("secret")).unwrap();

        assert!(!changed);
        assert!(target.password.is_none());
    }

    #[test]
    fn update_request_preserves_target_id_and_reuses_saved_password_when_blank() {
        let mut existing = password_target();
        existing.protected_password = Some("protected".to_string());
        let request = UpdateSshTargetRequest {
            id: "ssh-demo".to_string(),
            label: "Prod".to_string(),
            host: "prod.local".to_string(),
            username: "bob".to_string(),
            port: Some(2222),
            auth_method: Some(SshAuthMethod::Password),
            key_path: None,
            password: Some("  ".to_string()),
            passphrase: None,
        };

        let updated = update_request_to_config(request, &existing).unwrap();

        assert_eq!(updated.id, "ssh-demo");
        assert_eq!(updated.label, "Prod");
        assert_eq!(updated.host, "prod.local");
        assert_eq!(updated.username, "bob");
        assert_eq!(updated.port, 2222);
        assert_eq!(updated.auth_method, SshAuthMethod::Password);
        assert_eq!(
            updated.credential_key.as_deref(),
            Some("ssh-demo:ssh-password")
        );
        assert_eq!(updated.protected_password.as_deref(), Some("protected"));
        assert!(updated.password.is_none());
    }

    #[test]
    fn update_request_switching_to_key_clears_password_metadata() {
        let existing = password_target();
        let request = UpdateSshTargetRequest {
            id: "ssh-demo".to_string(),
            label: "Lab".to_string(),
            host: "lab.local".to_string(),
            username: "alice".to_string(),
            port: Some(22),
            auth_method: Some(SshAuthMethod::Key),
            key_path: Some("C:\\Users\\alice\\.ssh\\id_ed25519".to_string()),
            password: Some("secret".to_string()),
            passphrase: None,
        };

        let updated = update_request_to_config(request, &existing).unwrap();

        assert_eq!(updated.auth_method, SshAuthMethod::Key);
        assert_eq!(updated.key_path, "C:\\Users\\alice\\.ssh\\id_ed25519");
        assert!(updated.credential_key.is_none());
        assert!(updated.protected_password.is_none());
        assert!(updated.password.is_none());
    }

    #[test]
    fn credential_save_reports_stored_after_readback() {
        let backend = Arc::new(MemoryCredentialBackend::default());
        let registry = TargetRegistry::with_credential_backend(backend);
        let mut target = password_target();

        let state = registry.save_target_password(&mut target).unwrap();
        let summary = registry.target_summary(&target, LOCAL_TARGET_ID);

        assert_eq!(state.status, TargetCredentialStatus::Stored);
        assert_eq!(
            summary.credential_status,
            Some(TargetCredentialStatus::Stored)
        );
        assert_eq!(summary.has_stored_password, Some(true));
    }

    #[test]
    fn credential_save_remains_available_when_readback_fails() {
        let backend = MemoryCredentialBackend::with_get_error(CredentialStoreError::Other(
            "credential vault locked".to_string(),
        ));
        let registry = TargetRegistry::with_credential_backend(backend);
        let mut target = password_target();

        let state = registry.save_target_password(&mut target).unwrap();
        let mut runtime_target = RemoteTargetConfig {
            password: None,
            ..target.clone()
        };
        registry.attach_available_password(&mut runtime_target);
        let summary = registry.target_summary(&target, LOCAL_TARGET_ID);

        #[cfg(windows)]
        assert_eq!(state.status, TargetCredentialStatus::Stored);
        #[cfg(not(windows))]
        assert_eq!(state.status, TargetCredentialStatus::Session);
        assert_eq!(runtime_target.password.as_deref(), Some("secret"));
        #[cfg(windows)]
        assert_eq!(
            summary.credential_status,
            Some(TargetCredentialStatus::Stored)
        );
        #[cfg(not(windows))]
        assert_eq!(
            summary.credential_status,
            Some(TargetCredentialStatus::Session)
        );
        assert_eq!(summary.has_stored_password, Some(true));
    }

    #[test]
    fn credential_save_remains_available_when_write_fails() {
        let backend = MemoryCredentialBackend::with_set_error(CredentialStoreError::Other(
            "credential vault denied write".to_string(),
        ));
        let registry = TargetRegistry::with_credential_backend(backend);
        let mut target = password_target();

        let state = registry.save_target_password(&mut target).unwrap();
        let mut runtime_target = RemoteTargetConfig {
            password: None,
            ..target.clone()
        };
        registry.attach_available_password(&mut runtime_target);

        #[cfg(windows)]
        assert_eq!(state.status, TargetCredentialStatus::Stored);
        #[cfg(not(windows))]
        assert_eq!(state.status, TargetCredentialStatus::Session);
        assert_eq!(runtime_target.password.as_deref(), Some("secret"));
    }

    #[tokio::test]
    async fn active_target_attaches_available_password() {
        let backend = MemoryCredentialBackend::with_get_error(CredentialStoreError::NoEntry);
        let registry = TargetRegistry::with_credential_backend(backend);
        let pool = memory_db().await;
        let mut target = password_target();
        registry.save_target_password(&mut target).unwrap();
        target.password = None;
        save_remote_targets(&pool, &[target]).await.unwrap();
        db::set_setting(&pool, ACTIVE_TARGET_SETTING_KEY, "ssh-demo")
            .await
            .unwrap();

        let active = registry.active_target(&pool).await.unwrap();

        match active {
            ActiveTarget::Ssh(target) => {
                assert_eq!(target.password.as_deref(), Some("secret"));
            }
            ActiveTarget::Local => panic!("expected ssh target"),
            ActiveTarget::Wsl(_) => panic!("expected ssh target"),
        }
    }

    #[test]
    fn credential_summary_reports_missing_and_unreadable() {
        let missing =
            TargetRegistry::with_credential_backend(Arc::new(MemoryCredentialBackend::default()));
        let unreadable =
            TargetRegistry::with_credential_backend(MemoryCredentialBackend::with_get_error(
                CredentialStoreError::Other("credential vault unavailable".to_string()),
            ));
        let mut target = password_target();
        target.password = None;

        let missing_summary = missing.target_summary(&target, LOCAL_TARGET_ID);
        let unreadable_summary = unreadable.target_summary(&target, LOCAL_TARGET_ID);

        assert_eq!(
            missing_summary.credential_status,
            Some(TargetCredentialStatus::Missing)
        );
        assert_eq!(missing_summary.has_stored_password, Some(false));
        assert_eq!(
            unreadable_summary.credential_status,
            Some(TargetCredentialStatus::Unreadable)
        );
        assert_eq!(unreadable_summary.has_stored_password, Some(false));
        assert!(unreadable_summary.credential_error.is_some());
    }

    #[test]
    fn deleting_target_password_clears_session_cache() {
        let backend = MemoryCredentialBackend::with_get_error(CredentialStoreError::NoEntry);
        let registry = TargetRegistry::with_credential_backend(backend);
        let mut target = password_target();

        registry.save_target_password(&mut target).unwrap();
        #[cfg(windows)]
        assert_eq!(
            registry
                .target_summary(&target, LOCAL_TARGET_ID)
                .credential_status,
            Some(TargetCredentialStatus::Stored)
        );
        #[cfg(not(windows))]
        assert_eq!(
            registry
                .target_summary(&target, LOCAL_TARGET_ID)
                .credential_status,
            Some(TargetCredentialStatus::Session)
        );

        registry.delete_target_password(&mut target).unwrap();

        assert_eq!(
            registry
                .target_summary(&target, LOCAL_TARGET_ID)
                .credential_status,
            Some(TargetCredentialStatus::Missing)
        );
    }

    #[test]
    fn key_auth_targets_do_not_report_password_credential_state() {
        let registry =
            TargetRegistry::with_credential_backend(Arc::new(MemoryCredentialBackend::default()));
        let mut target = password_target();
        target.auth_method = SshAuthMethod::Key;
        target.key_path = "~/.ssh/id_ed25519".to_string();
        target.credential_key = None;
        target.password = None;

        let summary = registry.target_summary(&target, LOCAL_TARGET_ID);

        assert_eq!(summary.credential_status, None);
        assert_eq!(summary.has_stored_password, None);
        assert_eq!(summary.credential_error, None);
    }
}
