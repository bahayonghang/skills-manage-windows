pub struct TargetRegistry {
    pools: Mutex<HashMap<String, DbPool>>,
    session_passwords: Mutex<HashMap<String, String>>,
    credential_backend: Arc<dyn CredentialBackend>,
}

impl Default for TargetRegistry {
    fn default() -> Self {
        Self {
            pools: Mutex::new(HashMap::new()),
            session_passwords: Mutex::new(HashMap::new()),
            credential_backend: Arc::new(SystemCredentialBackend),
        }
    }
}

impl TargetRegistry {
    #[cfg(test)]
    fn with_credential_backend(credential_backend: Arc<dyn CredentialBackend>) -> Self {
        Self {
            pools: Mutex::new(HashMap::new()),
            session_passwords: Mutex::new(HashMap::new()),
            credential_backend,
        }
    }

    fn get_session_password(&self, credential_key: &str) -> Option<String> {
        self.session_passwords
            .lock()
            .ok()
            .and_then(|passwords| passwords.get(credential_key).cloned())
    }

    fn set_session_password(&self, credential_key: &str, password: &str) -> Result<(), String> {
        self.session_passwords
            .lock()
            .map_err(|_| "Failed to update SSH password session cache.".to_string())?
            .insert(credential_key.to_string(), password.to_string());
        Ok(())
    }

    fn clear_session_password(&self, credential_key: &str) {
        if let Ok(mut passwords) = self.session_passwords.lock() {
            passwords.remove(credential_key);
        }
    }

    fn target_credential_state(
        &self,
        target: &RemoteTargetConfig,
    ) -> Option<TargetCredentialState> {
        let credential_key = credential_key_for_password_target(target)?;
        match self.credential_backend.get_password(&credential_key) {
            Ok(password) if !password.is_empty() => Some(TargetCredentialState {
                status: TargetCredentialStatus::Stored,
                error: None,
            }),
            Ok(_) => {
                if let Ok(Some(_)) = protected_password_for_target(target) {
                    return Some(TargetCredentialState {
                        status: TargetCredentialStatus::Stored,
                        error: Some(
                            "System credential store returned an empty SSH password; using the Windows protected local credential."
                                .to_string(),
                        ),
                    });
                }
                if self.get_session_password(&credential_key).is_some() {
                    Some(TargetCredentialState {
                        status: TargetCredentialStatus::Session,
                        error: Some(
                            "Stored SSH password is empty; using the current session password."
                                .to_string(),
                        ),
                    })
                } else {
                    Some(TargetCredentialState {
                        status: TargetCredentialStatus::Missing,
                        error: None,
                    })
                }
            }
            Err(CredentialStoreError::NoEntry) => {
                match protected_password_for_target(target) {
                    Ok(Some(_)) => {
                        return Some(TargetCredentialState {
                            status: TargetCredentialStatus::Stored,
                            error: None,
                        });
                    }
                    Err(error) => {
                        if self.get_session_password(&credential_key).is_none() {
                            return Some(TargetCredentialState {
                                status: TargetCredentialStatus::Unreadable,
                                error: Some(error),
                            });
                        }
                    }
                    Ok(None) => {}
                }
                if self.get_session_password(&credential_key).is_some() {
                    Some(TargetCredentialState {
                        status: TargetCredentialStatus::Session,
                        error: None,
                    })
                } else {
                    Some(TargetCredentialState {
                        status: TargetCredentialStatus::Missing,
                        error: None,
                    })
                }
            }
            Err(error) => {
                match protected_password_for_target(target) {
                    Ok(Some(_)) => {
                        return Some(TargetCredentialState {
                            status: TargetCredentialStatus::Stored,
                            error: Some(error.message()),
                        });
                    }
                    Err(protected_error) => {
                        if self.get_session_password(&credential_key).is_none() {
                            return Some(TargetCredentialState {
                                status: TargetCredentialStatus::Unreadable,
                                error: Some(format!("{}; {}", error.message(), protected_error)),
                            });
                        }
                    }
                    Ok(None) => {}
                }
                if self.get_session_password(&credential_key).is_some() {
                    Some(TargetCredentialState {
                        status: TargetCredentialStatus::Session,
                        error: Some(error.message()),
                    })
                } else {
                    Some(TargetCredentialState {
                        status: TargetCredentialStatus::Unreadable,
                        error: Some(error.message()),
                    })
                }
            }
        }
    }

    fn attach_available_password(&self, target: &mut RemoteTargetConfig) {
        if target.auth_method != SshAuthMethod::Password
            || target
                .password
                .as_deref()
                .is_some_and(|password| !password.is_empty())
        {
            return;
        }
        let Some(credential_key) = credential_key_for_password_target(target) else {
            return;
        };
        match self.credential_backend.get_password(&credential_key) {
            Ok(password) if !password.is_empty() => {
                target.password = Some(password);
            }
            _ => {
                if let Ok(Some(password)) = protected_password_for_target(target) {
                    target.password = Some(password);
                    return;
                }
                if let Some(password) = self.get_session_password(&credential_key) {
                    target.password = Some(password);
                }
            }
        }
    }

    fn save_target_password(
        &self,
        target: &mut RemoteTargetConfig,
    ) -> Result<TargetCredentialState, String> {
        let credential_key = credential_key_for_password_target(target)
            .ok_or_else(|| "Password target is missing its credential key.".to_string())?;
        let password = target
            .password
            .as_deref()
            .filter(|value| !value.is_empty())
            .ok_or_else(|| "Password is required for password authentication.".to_string())?
            .to_string();

        let fallback_to_protected =
            |registry: &TargetRegistry,
             target: &mut RemoteTargetConfig,
             error: CredentialStoreError| {
                match protected_credentials::protect(&password) {
                    Ok(protected_password) => {
                        target.protected_password = Some(protected_password);
                        registry.set_session_password(&credential_key, &password)?;
                        Ok(TargetCredentialState {
                            status: TargetCredentialStatus::Stored,
                            error: Some(error.message()),
                        })
                    }
                    Err(protected_error) => {
                        registry.set_session_password(&credential_key, &password)?;
                        Ok(TargetCredentialState {
                            status: TargetCredentialStatus::Session,
                            error: Some(format!("{}; {}", error.message(), protected_error)),
                        })
                    }
                }
            };

        match self
            .credential_backend
            .set_password(&credential_key, &password)
            .and_then(|_| self.credential_backend.get_password(&credential_key))
        {
            Ok(saved_password) if saved_password == password => {
                target.protected_password = None;
                self.clear_session_password(&credential_key);
                Ok(TargetCredentialState {
                    status: TargetCredentialStatus::Stored,
                    error: None,
                })
            }
            Ok(_) => fallback_to_protected(
                self,
                target,
                CredentialStoreError::Other(
                    "System credential store returned a different SSH password after saving."
                        .to_string(),
                ),
            ),
            Err(error) => fallback_to_protected(self, target, error),
        }
    }

    fn delete_target_password(&self, target: &mut RemoteTargetConfig) -> Result<(), String> {
        let Some(credential_key) = credential_key_for_password_target(target) else {
            return Ok(());
        };
        self.clear_session_password(&credential_key);
        target.protected_password = None;
        self.credential_backend
            .delete_credential(&credential_key)
            .map_err(|error| error.message())
    }

    fn target_summary(&self, target: &RemoteTargetConfig, active_id: &str) -> TargetSummary {
        let credential_state = self.target_credential_state(target);
        TargetSummary {
            id: target.id.clone(),
            kind: TargetKind::Ssh,
            label: target.label.clone(),
            host: Some(target.host.clone()),
            username: Some(target.username.clone()),
            port: Some(target.port),
            auth_method: Some(target.auth_method),
            key_path: match target.auth_method {
                SshAuthMethod::Key => Some(target.key_path.clone()),
                SshAuthMethod::Password => None,
            },
            remote_home: Some(target.remote_home.clone()),
            remote_os: Some(target.remote_os.clone()),
            cache_db_path: remote_cache_db_path(&target.id)
                .ok()
                .map(|path| path.to_string_lossy().into_owned()),
            has_stored_password: credential_state
                .as_ref()
                .map(|state| state.status.is_available()),
            credential_status: credential_state.as_ref().map(|state| state.status),
            credential_error: credential_state.and_then(|state| state.error),
            symlink_enabled: Some(remote_symlink_allowed(target)),
            is_active: target.id == active_id,
        }
    }

    pub async fn list_targets(&self, local_db: &DbPool) -> Result<Vec<TargetSummary>, String> {
        let active_id = active_target_id(local_db).await?;
        let mut targets = vec![TargetSummary {
            id: LOCAL_TARGET_ID.to_string(),
            kind: TargetKind::Local,
            label: "Local".to_string(),
            host: None,
            username: None,
            port: None,
            auth_method: None,
            key_path: None,
            remote_home: None,
            remote_os: None,
            cache_db_path: None,
            has_stored_password: None,
            credential_status: None,
            credential_error: None,
            symlink_enabled: None,
            is_active: active_id == LOCAL_TARGET_ID,
        }];

        for target in load_remote_targets(local_db).await? {
            targets.push(self.target_summary(&target, active_id.as_str()));
        }

        Ok(targets)
    }

    pub async fn active_target(&self, local_db: &DbPool) -> Result<ActiveTarget, String> {
        let active_id = active_target_id(local_db).await?;
        if active_id == LOCAL_TARGET_ID {
            return Ok(ActiveTarget::Local);
        }

        let mut target = load_remote_targets(local_db)
            .await?
            .into_iter()
            .find(|target| target.id == active_id)
            .ok_or_else(|| {
                format!(
                    "Active target '{}' no longer exists. Switch back to Local.",
                    active_id
                )
            })?;
        self.attach_available_password(&mut target);
        Ok(ActiveTarget::Ssh(Box::new(target)))
    }

    pub async fn active_db(&self, local_db: &DbPool) -> Result<DbPool, String> {
        match self.active_target(local_db).await? {
            ActiveTarget::Local => Ok(local_db.clone()),
            ActiveTarget::Ssh(target) => self.remote_db(&target).await,
        }
    }

    pub async fn remote_db(&self, target: &RemoteTargetConfig) -> Result<DbPool, String> {
        if let Ok(pools) = self.pools.lock() {
            if let Some(pool) = pools.get(&target.id) {
                return Ok(pool.clone());
            }
        }

        let db_path = remote_cache_db_path(&target.id)?;
        if let Some(parent) = db_path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                format!(
                    "Failed to create target cache directory '{}': {}",
                    parent.display(),
                    e
                )
            })?;
        }

        let db_path = db_path.to_string_lossy().into_owned();
        let pool = db::create_pool(&db_path).await?;
        db::init_database_for_remote_home(&pool, &target.remote_home).await?;

        if let Ok(mut pools) = self.pools.lock() {
            pools.insert(target.id.clone(), pool.clone());
        }

        Ok(pool)
    }

    fn drop_remote_pool(&self, target_id: &str) {
        if let Ok(mut pools) = self.pools.lock() {
            pools.remove(target_id);
        }
    }
}

