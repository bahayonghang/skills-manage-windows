use super::*;
#[cfg(test)]
pub(super) mod suite {
    use super::*;
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

    use crate::test_support::mem_pool as memory_db;

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
            runner: Arc::new(ProcessRunner),
        }
    }

    #[tokio::test]
    async fn active_target_defaults_to_local() {
        let pool = memory_db().await;
        assert_eq!(active_target_id(&pool).await.unwrap(), LOCAL_TARGET_ID);
    }

    #[tokio::test]
    async fn local_target_context_owns_matching_target_and_db() {
        let registry = TargetRegistry::default();
        let pool = memory_db().await;
        db::set_setting(&pool, "context_marker", "local")
            .await
            .unwrap();

        let context = registry.resolve_active_context(&pool).await.unwrap();

        assert_eq!(context.id(), LOCAL_TARGET_ID);
        assert_eq!(context.label(), "Local");
        assert_eq!(context.kind(), TargetKind::Local);
        assert_eq!(
            db::get_setting(context.db(), "context_marker")
                .await
                .unwrap()
                .as_deref(),
            Some("local")
        );
    }

    #[tokio::test]
    async fn resolved_context_keeps_owned_target_while_missing_active_falls_back_to_local() {
        let registry = TargetRegistry::default();
        let pool = memory_db().await;
        let context = registry.resolve_active_context(&pool).await.unwrap();

        db::set_setting(&pool, ACTIVE_TARGET_SETTING_KEY, "missing-target")
            .await
            .unwrap();

        assert_eq!(context.id(), LOCAL_TARGET_ID);
        assert_eq!(context.kind(), TargetKind::Local);
        assert!(matches!(context.target(), ActiveTarget::Local));
        let recovered = registry.resolve_active_context(&pool).await.unwrap();
        assert_eq!(recovered.id(), LOCAL_TARGET_ID);
        assert!(matches!(recovered.target(), ActiveTarget::Local));
        assert_eq!(
            db::get_setting(&pool, ACTIVE_TARGET_SETTING_KEY)
                .await
                .unwrap()
                .as_deref(),
            Some(LOCAL_TARGET_ID)
        );
    }

    #[tokio::test]
    async fn explicit_target_context_preserves_ssh_and_wsl_identity() {
        let pool = memory_db().await;
        let ssh = password_target();
        let wsl = wsl_target();
        let ssh_context =
            TargetContext::new(ActiveTarget::Ssh(Box::new(ssh.clone())), pool.clone());
        let wsl_context = TargetContext::new(ActiveTarget::Wsl(Box::new(wsl.clone())), pool);

        assert_eq!(ssh_context.id(), ssh.id);
        assert_eq!(ssh_context.label(), ssh.label);
        assert_eq!(ssh_context.kind(), TargetKind::Ssh);
        assert_eq!(wsl_context.id(), wsl.id);
        assert_eq!(wsl_context.label(), wsl.label);
        assert_eq!(wsl_context.kind(), TargetKind::Wsl);
    }

    #[tokio::test]
    async fn target_context_snapshot_survives_cross_target_switch_matrix() {
        use crate::operation_log::target_context_from_active_target;

        let registry = TargetRegistry::default();
        let local_db = memory_db().await;
        db::set_setting(&local_db, "context_marker", "local")
            .await
            .unwrap();

        let mut ssh_a = password_target();
        ssh_a.auth_method = SshAuthMethod::Key;
        ssh_a.key_path = "~/.ssh/id_ed25519".to_string();
        ssh_a.credential_key = None;
        ssh_a.password = None;
        let mut ssh_b = ssh_a.clone();
        ssh_b.id = "ssh-other".to_string();
        ssh_b.label = "Other Lab".to_string();
        let wsl = wsl_target();
        save_remote_targets(&local_db, &[ssh_a.clone(), ssh_b.clone()])
            .await
            .unwrap();
        save_wsl_targets(&local_db, std::slice::from_ref(&wsl))
            .await
            .unwrap();

        for (target_id, marker) in [
            (ssh_a.id.as_str(), "ssh-a"),
            (ssh_b.id.as_str(), "ssh-b"),
            (wsl.id.as_str(), "wsl"),
        ] {
            let pool = memory_db().await;
            db::set_setting(&pool, "context_marker", marker)
                .await
                .unwrap();
            registry.insert_test_pool(target_id, pool);
        }

        for (from_id, to_id, expected_marker) in [
            (LOCAL_TARGET_ID, ssh_a.id.as_str(), "local"),
            (ssh_a.id.as_str(), ssh_b.id.as_str(), "ssh-a"),
            (ssh_b.id.as_str(), wsl.id.as_str(), "ssh-b"),
            (wsl.id.as_str(), LOCAL_TARGET_ID, "wsl"),
        ] {
            db::set_setting(&local_db, ACTIVE_TARGET_SETTING_KEY, from_id)
                .await
                .unwrap();
            let context = registry.resolve_active_context(&local_db).await.unwrap();
            assert_eq!(context.id(), from_id);

            let ready = Arc::new(tokio::sync::Barrier::new(2));
            let resume = Arc::new(tokio::sync::Barrier::new(2));
            let operation_ready = ready.clone();
            let operation_resume = resume.clone();
            let operation = tokio::spawn(async move {
                operation_ready.wait().await;
                operation_resume.wait().await;

                let marker = db::get_setting(context.db(), "context_marker")
                    .await
                    .unwrap()
                    .unwrap();
                let log_context = target_context_from_active_target(context.target());
                let event_target_id = context.id().to_string();
                (
                    context.id().to_string(),
                    context.label().to_string(),
                    marker,
                    log_context,
                    event_target_id,
                )
            });

            ready.wait().await;
            db::set_setting(&local_db, ACTIVE_TARGET_SETTING_KEY, to_id)
                .await
                .unwrap();
            let next_context = registry.resolve_active_context(&local_db).await.unwrap();
            assert_eq!(next_context.id(), to_id);
            resume.wait().await;

            let (context_id, context_label, marker, log_context, event_target_id) =
                operation.await.unwrap();
            assert_eq!(context_id, from_id);
            assert_eq!(marker, expected_marker);
            assert_eq!(log_context.id, from_id);
            assert_eq!(log_context.label.as_deref(), Some(context_label.as_str()));
            assert_eq!(event_target_id, from_id);
        }
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
            runner: Arc::new(ProcessRunner),
        };
        let args = command_arg_strings(connection.base_command());

        assert_eq!(args, vec!["-d", "Ubuntu-24.04", "--exec"]);
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn remote_target_open_does_not_probe_wsl_distribution() {
        let mut target = wsl_target();
        target.distribution = "missing-skillport-test-distribution".to_string();

        let connection = connect_remote_target(&ActiveTarget::Wsl(Box::new(target)))
            .await
            .expect("opening a WSL transport must not start wsl.exe");

        assert_eq!(connection.target_id(), "wsl-ubuntu");
    }

    #[tokio::test]
    async fn remote_target_open_does_not_probe_ssh_host() {
        let mut target = password_target();
        target.auth_method = SshAuthMethod::Key;
        target.host.clear();
        target.password = None;

        let connection = connect_remote_target(&ActiveTarget::Ssh(Box::new(target)))
            .await
            .expect("opening an SSH transport must not start ssh");

        assert_eq!(connection.target_id(), "ssh-demo");
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
    fn remote_probe_script_matches_historical_literal_byte_for_byte() {
        assert_eq!(
            remote_probe_script(),
            r#"printf 'HOME\t%s\n' "$HOME"
printf 'OS\t%s\n' "$(uname -s 2>/dev/null || printf '%s' unknown)"
mkdir -p -- "$HOME/.skillsmanage/skills" && printf 'MKDIR_OK\n'"#
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
            runner: Arc::new(ProcessRunner),
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
    fn windows_hidden_window_flag_matches_create_no_window() {
        assert_eq!(CREATE_NO_WINDOW, 0x08000000);
        assert_eq!(hidden_child_creation_flags(), CREATE_NO_WINDOW);
    }

    #[cfg(windows)]
    #[test]
    fn wsl_discovery_command_uses_hidden_window_flag() {
        let command = wsl_distribution_list_command();
        let args = command_arg_strings(command);

        assert_eq!(args, vec!["-l", "-v"]);
        assert_eq!(hidden_child_creation_flags(), CREATE_NO_WINDOW);
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

        assert!(result.unwrap_err().to_string().contains("keyPath"));
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

    // ─── CommandRunner 注入：执行半边（base_command 之后）的单元测试 ─────────

    use crate::test_support::FakeRunner;

    fn key_target() -> RemoteTargetConfig {
        let mut target = password_target();
        target.auth_method = SshAuthMethod::Key;
        target.key_path = "~/.ssh/id_ed25519".to_string();
        target.credential_key = None;
        target.password = None;
        target
    }

    fn fake_ssh_connection() -> (Arc<FakeRunner>, ConnectedSshTarget) {
        let runner = Arc::new(FakeRunner::new());
        let connection = ConnectedSshTarget::for_tests_with_runner(key_target(), runner.clone());
        (runner, connection)
    }

    fn fake_wsl_connection() -> (Arc<FakeRunner>, ConnectedWslTarget) {
        let runner = Arc::new(FakeRunner::new());
        let connection = ConnectedWslTarget {
            target: wsl_target(),
            runner: runner.clone(),
        };
        (runner, connection)
    }

    #[tokio::test]
    async fn ssh_exists_maps_exit_codes_to_bool_and_error() {
        let (runner, connection) = fake_ssh_connection();
        runner.push_output(0, "", "");
        runner.push_output(1, "", "");
        runner.push_output(255, "", "ssh: connect refused");

        assert!(connection.exists("/tmp/a").await.unwrap());
        assert!(!connection.exists("/tmp/a").await.unwrap());
        let error = connection.exists("/tmp/a").await.unwrap_err();
        match error {
            TargetsError::RemoteInspectFailed { path, detail } => {
                assert_eq!(path, "/tmp/a");
                assert_eq!(detail, "ssh: connect refused");
            }
            other => panic!("unexpected error: {other:?}"),
        }

        let calls = runner.calls();
        assert_eq!(calls.len(), 3);
        assert_eq!(
            calls[0].args.last().map(String::as_str),
            Some("test -e '/tmp/a'")
        );
        assert!(calls[0].stdin.is_none());
        assert_eq!(calls[0].policy.class.label(), "probe");
    }

    #[tokio::test]
    async fn ssh_inspect_path_parses_symlink_output() {
        let (runner, connection) = fake_ssh_connection();
        runner.push_success("symlink\t/home/alice/.skillsmanage/skills/demo\n");

        let info = connection
            .inspect_path("/home/alice/.claude/skills/demo")
            .await;
        assert_eq!(
            info.unwrap(),
            Some(RemotePathInfo {
                file_type: "symlink".to_string(),
                symlink_target: Some("/home/alice/.skillsmanage/skills/demo".to_string()),
            })
        );
    }

    #[tokio::test]
    async fn ssh_run_script_pipes_script_via_stdin_with_quoted_args() {
        let (runner, connection) = fake_ssh_connection();
        runner.push_success("done\n");

        let output = connection
            .run_script("printf '%s' \"$1\"", &["a b", "c"])
            .await
            .unwrap();
        assert_eq!(output, "done\n");

        let calls = runner.calls();
        assert_eq!(
            calls[0].args.last().map(String::as_str),
            Some("sh -s -- 'a b' 'c'")
        );
        assert_eq!(
            calls[0].stdin.as_deref(),
            Some("printf '%s' \"$1\"".as_bytes())
        );
        assert_eq!(calls[0].policy.class.label(), "standard");
    }

    #[tokio::test]
    async fn ssh_run_command_failure_carries_stderr_detail() {
        let (runner, connection) = fake_ssh_connection();
        runner.push_output(2, "", "rm: cannot remove '/x': Read-only file system");

        let error = connection.run_command("rm -rf -- '/x'").await.unwrap_err();
        match error {
            TargetsError::RemoteCommandFailed { detail, .. } => {
                assert_eq!(detail, "rm: cannot remove '/x': Read-only file system");
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[tokio::test]
    async fn ssh_bounded_read_uses_size_probe_max_plus_one_policy_and_typed_overflow() {
        let (runner, connection) = fake_ssh_connection();
        runner.push_success("small");
        runner.push_success("too-big");
        runner.push_output(REMOTE_FILE_TOO_LARGE_EXIT, "", "must-not-leak");
        runner.push_output(1, "", "secret remote stderr");

        assert_eq!(
            connection.read_file_bounded("/tmp/a", 5).await.unwrap(),
            b"small"
        );
        assert!(matches!(
            connection.read_file_bounded("/tmp/a", 5).await.unwrap_err(),
            TargetsError::RemoteFileTooLarge { limit: 5 }
        ));
        let preflight = connection.read_file_bounded("/tmp/a", 5).await.unwrap_err();
        assert!(matches!(
            preflight,
            TargetsError::RemoteFileTooLarge { limit: 5 }
        ));
        assert!(!preflight.to_string().contains("must-not-leak"));
        let transport = connection.read_file_bounded("/tmp/a", 5).await.unwrap_err();
        assert!(matches!(
            transport,
            TargetsError::RemoteFileReadFailed { transport: "ssh" }
        ));
        assert!(!transport.to_string().contains("secret remote stderr"));

        let calls = runner.calls();
        assert_eq!(calls.len(), 4);
        assert!(calls[0].args.last().unwrap().contains("wc -c"));
        assert!(calls[0].args.last().unwrap().contains("bs=6 count=1"));
        assert_eq!(calls[0].policy.stdout_limit, 6);
        assert_eq!(calls[0].policy.class.label(), "standard");
    }

    #[tokio::test]
    async fn ssh_runner_start_failure_maps_to_start_ssh_error() {
        let (runner, connection) = fake_ssh_connection();
        runner.push_error(RunnerPhase::Start, "program not found");

        let error = connection.run_command("true").await.unwrap_err();
        assert!(error.to_string().contains("Failed to start ssh"));
    }

    #[test]
    fn ssh_supervision_errors_map_to_semantic_target_variants() {
        let timeout = ssh_runner_error(RunnerError::TimedOut {
            class: ProcessPolicy::probe().class,
            deadline: std::time::Duration::from_secs(30),
        });
        assert!(matches!(
            timeout,
            TargetsError::ProcessTimedOut {
                transport: "SSH",
                class: "probe",
                timeout_ms: 30_000,
            }
        ));

        let overflow = wsl_runner_error(RunnerError::OutputLimitExceeded {
            stream: super::runner::RunnerStream::Stderr,
            limit: 512,
        });
        assert!(matches!(
            overflow,
            TargetsError::ProcessOutputLimitExceeded {
                transport: "WSL",
                stream: "stderr",
                limit: 512,
            }
        ));
    }

    #[tokio::test]
    async fn wsl_exists_runs_through_login_shell_and_maps_exit_codes() {
        let (runner, connection) = fake_wsl_connection();
        runner.push_output(0, "", "");
        runner.push_output(1, "", "");

        assert!(connection.exists("/tmp/a").await.unwrap());
        assert!(!connection.exists("/tmp/a").await.unwrap());

        let calls = runner.calls();
        assert_eq!(
            calls[0].args,
            vec![
                "-d",
                "Ubuntu-24.04",
                "--exec",
                "sh",
                "-lc",
                "test -e '/tmp/a'"
            ]
        );
    }

    #[tokio::test]
    async fn wsl_bounded_read_matches_ssh_script_and_policy() {
        let (runner, connection) = fake_wsl_connection();
        runner.push_success("small");

        assert_eq!(
            connection.read_file_bounded("/tmp/a", 5).await.unwrap(),
            b"small"
        );
        let calls = runner.calls();
        assert_eq!(calls[0].args[3..5], ["sh", "-lc"]);
        assert!(calls[0].args.last().unwrap().contains("wc -c"));
        assert!(calls[0].args.last().unwrap().contains("bs=6 count=1"));
        assert_eq!(calls[0].policy.stdout_limit, 6);
        assert_eq!(calls[0].policy.class.label(), "standard");
    }

    #[tokio::test]
    async fn wsl_run_script_passes_args_after_stdin_shell() {
        let (runner, connection) = fake_wsl_connection();
        runner.push_success("ok");

        let output = connection.run_script("echo hi", &["x y"]).await.unwrap();
        assert_eq!(output, "ok");

        let calls = runner.calls();
        assert_eq!(
            calls[0].args,
            vec!["-d", "Ubuntu-24.04", "--exec", "sh", "-s", "--", "x y"]
        );
        assert_eq!(calls[0].stdin.as_deref(), Some("echo hi".as_bytes()));
    }

    #[tokio::test]
    async fn wsl_command_failure_carries_stderr_detail() {
        let (runner, connection) = fake_wsl_connection();
        runner.push_output(3, "", "cp: no space left");

        let error = connection.run_command("cp -R a b").await.unwrap_err();
        match error {
            TargetsError::WslCommandFailed { detail, .. } => {
                assert_eq!(detail, "cp: no space left");
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }
}
