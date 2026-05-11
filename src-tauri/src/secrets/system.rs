use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use super::{SecretError, SecretStorageState, SecretStore, SKILLPORT_SECRET_SERVICE};

pub trait SecretStoreBackend: Send + Sync {
    fn set_secret(&self, key: &str, value: &str) -> Result<(), SecretError>;
    fn get_secret(&self, key: &str) -> Result<String, SecretError>;
    fn delete_secret(&self, key: &str) -> Result<(), SecretError>;
}

trait ProtectedSecretBackend: Send + Sync {
    fn set_protected_secret(&self, key: &str, value: &str) -> Result<(), SecretError>;
    fn get_protected_secret(&self, key: &str) -> Result<String, SecretError>;
    fn delete_protected_secret(&self, key: &str) -> Result<(), SecretError>;
}

trait SecretProtector: Send + Sync {
    fn protect(&self, value: &str) -> Result<String, SecretError>;
    fn unprotect(&self, value: &str) -> Result<String, SecretError>;
}

struct SystemKeyringBackend;

impl SystemKeyringBackend {
    fn entry(key: &str) -> Result<keyring::Entry, SecretError> {
        keyring::Entry::new(SKILLPORT_SECRET_SERVICE, key).map_err(|error| {
            SecretError::Other(format!("Failed to access system secret store: {}", error))
        })
    }
}

impl SecretStoreBackend for SystemKeyringBackend {
    fn set_secret(&self, key: &str, value: &str) -> Result<(), SecretError> {
        Self::entry(key)?.set_password(value).map_err(|error| {
            SecretError::Other(format!(
                "Failed to store secret in system secret store: {}",
                error
            ))
        })
    }

    fn get_secret(&self, key: &str) -> Result<String, SecretError> {
        match Self::entry(key)?.get_password() {
            Ok(value) => Ok(value),
            Err(keyring::Error::NoEntry) => Err(SecretError::NoEntry),
            Err(error) => Err(SecretError::Other(format!(
                "Failed to read secret from system secret store: {}",
                error
            ))),
        }
    }

    fn delete_secret(&self, key: &str) -> Result<(), SecretError> {
        match Self::entry(key)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(error) => Err(SecretError::Other(format!(
                "Failed to delete secret from system secret store: {}",
                error
            ))),
        }
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(HEX[(byte >> 4) as usize] as char);
        encoded.push(HEX[(byte & 0x0f) as usize] as char);
    }
    encoded
}

#[cfg(windows)]
fn hex_decode(value: &str) -> Result<Vec<u8>, SecretError> {
    if !value.len().is_multiple_of(2) {
        return Err(SecretError::Other(
            "Protected secret payload is not valid hex.".to_string(),
        ));
    }
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|chunk| {
            let high = (chunk[0] as char).to_digit(16).ok_or_else(|| {
                SecretError::Other("Protected secret payload is not valid hex.".to_string())
            })?;
            let low = (chunk[1] as char).to_digit(16).ok_or_else(|| {
                SecretError::Other("Protected secret payload is not valid hex.".to_string())
            })?;
            Ok(((high << 4) | low) as u8)
        })
        .collect()
}

struct FileProtectedSecretBackend {
    directory: PathBuf,
}

impl Default for FileProtectedSecretBackend {
    fn default() -> Self {
        Self {
            directory: crate::paths::app_data_dir().join("protected-secrets"),
        }
    }
}

impl FileProtectedSecretBackend {
    fn path_for_key(&self, key: &str) -> PathBuf {
        self.directory
            .join(format!("{}.secret", hex_encode(key.as_bytes())))
    }
}

impl ProtectedSecretBackend for FileProtectedSecretBackend {
    fn set_protected_secret(&self, key: &str, value: &str) -> Result<(), SecretError> {
        fs::create_dir_all(&self.directory).map_err(|error| {
            SecretError::Other(format!(
                "Failed to create app-local protected secret directory: {}",
                error
            ))
        })?;
        fs::write(self.path_for_key(key), value).map_err(|error| {
            SecretError::Other(format!(
                "Failed to write app-local protected secret: {}",
                error
            ))
        })
    }

    fn get_protected_secret(&self, key: &str) -> Result<String, SecretError> {
        match fs::read_to_string(self.path_for_key(key)) {
            Ok(value) => Ok(value),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Err(SecretError::NoEntry),
            Err(error) => Err(SecretError::Other(format!(
                "Failed to read app-local protected secret: {}",
                error
            ))),
        }
    }

    fn delete_protected_secret(&self, key: &str) -> Result<(), SecretError> {
        match fs::remove_file(self.path_for_key(key)) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(SecretError::Other(format!(
                "Failed to delete app-local protected secret: {}",
                error
            ))),
        }
    }
}

struct DpapiSecretProtector;

#[cfg(windows)]
mod platform_protector {
    use std::ffi::c_void;
    use std::io;
    use std::ptr::{null, null_mut};
    use std::slice;

    use super::{hex_decode, hex_encode, SecretError};

    const CRYPTPROTECT_UI_FORBIDDEN: u32 = 0x1;

    #[repr(C)]
    #[allow(non_snake_case)]
    struct DataBlob {
        cbData: u32,
        pbData: *mut u8,
    }

    #[link(name = "Crypt32")]
    unsafe extern "system" {
        fn CryptProtectData(
            pDataIn: *mut DataBlob,
            szDataDescr: *const u16,
            pOptionalEntropy: *mut DataBlob,
            pvReserved: *mut c_void,
            pPromptStruct: *mut c_void,
            dwFlags: u32,
            pDataOut: *mut DataBlob,
        ) -> i32;

        fn CryptUnprotectData(
            pDataIn: *mut DataBlob,
            ppszDataDescr: *mut *mut u16,
            pOptionalEntropy: *mut DataBlob,
            pvReserved: *mut c_void,
            pPromptStruct: *mut c_void,
            dwFlags: u32,
            pDataOut: *mut DataBlob,
        ) -> i32;
    }

    #[link(name = "Kernel32")]
    unsafe extern "system" {
        fn LocalFree(hmem: *mut c_void) -> *mut c_void;
    }

    fn last_error(action: &str) -> SecretError {
        SecretError::Other(format!(
            "Failed to {} app-local secret with Windows DPAPI: {}",
            action,
            io::Error::last_os_error()
        ))
    }

    pub fn protect(value: &str) -> Result<String, SecretError> {
        let mut input = value.as_bytes().to_vec();
        let mut input_blob = DataBlob {
            cbData: input
                .len()
                .try_into()
                .map_err(|_| SecretError::Other("Secret is too large to protect.".to_string()))?,
            pbData: input.as_mut_ptr(),
        };
        let mut output_blob = DataBlob {
            cbData: 0,
            pbData: null_mut(),
        };

        let ok = unsafe {
            CryptProtectData(
                &mut input_blob,
                null(),
                null_mut(),
                null_mut(),
                null_mut(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output_blob,
            )
        };
        if ok == 0 {
            return Err(last_error("protect"));
        }

        let protected = unsafe {
            slice::from_raw_parts(output_blob.pbData, output_blob.cbData as usize).to_vec()
        };
        unsafe {
            LocalFree(output_blob.pbData.cast::<c_void>());
        }
        Ok(hex_encode(&protected))
    }

    pub fn unprotect(value: &str) -> Result<String, SecretError> {
        let mut input = hex_decode(value)?;
        let mut input_blob = DataBlob {
            cbData: input.len().try_into().map_err(|_| {
                SecretError::Other("Protected secret payload is too large.".to_string())
            })?,
            pbData: input.as_mut_ptr(),
        };
        let mut output_blob = DataBlob {
            cbData: 0,
            pbData: null_mut(),
        };

        let ok = unsafe {
            CryptUnprotectData(
                &mut input_blob,
                null_mut(),
                null_mut(),
                null_mut(),
                null_mut(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output_blob,
            )
        };
        if ok == 0 {
            return Err(last_error("unprotect"));
        }

        let plaintext = unsafe {
            slice::from_raw_parts(output_blob.pbData, output_blob.cbData as usize).to_vec()
        };
        unsafe {
            LocalFree(output_blob.pbData.cast::<c_void>());
        }
        String::from_utf8(plaintext).map_err(|error| {
            SecretError::Other(format!("Protected secret is not valid UTF-8: {}", error))
        })
    }
}

#[cfg(not(windows))]
mod platform_protector {
    use super::SecretError;

    pub fn protect(_value: &str) -> Result<String, SecretError> {
        Err(SecretError::Other(
            "App-local protected secret fallback is only available on Windows.".to_string(),
        ))
    }

    pub fn unprotect(_value: &str) -> Result<String, SecretError> {
        Err(SecretError::Other(
            "App-local protected secret fallback is only available on Windows.".to_string(),
        ))
    }
}

impl SecretProtector for DpapiSecretProtector {
    fn protect(&self, value: &str) -> Result<String, SecretError> {
        platform_protector::protect(value)
    }

    fn unprotect(&self, value: &str) -> Result<String, SecretError> {
        platform_protector::unprotect(value)
    }
}

pub struct SystemSecretStore {
    backend: Arc<dyn SecretStoreBackend>,
    protected_backend: Arc<dyn ProtectedSecretBackend>,
    protector: Arc<dyn SecretProtector>,
    session_secrets: Mutex<HashMap<String, String>>,
}

impl Default for SystemSecretStore {
    fn default() -> Self {
        Self {
            backend: Arc::new(SystemKeyringBackend),
            protected_backend: Arc::new(FileProtectedSecretBackend::default()),
            protector: Arc::new(DpapiSecretProtector),
            session_secrets: Mutex::new(HashMap::new()),
        }
    }
}

impl SystemSecretStore {
    #[cfg(test)]
    fn with_parts(
        backend: Arc<dyn SecretStoreBackend>,
        protected_backend: Arc<dyn ProtectedSecretBackend>,
        protector: Arc<dyn SecretProtector>,
    ) -> Self {
        Self {
            backend,
            protected_backend,
            protector,
            session_secrets: Mutex::new(HashMap::new()),
        }
    }

    fn normalize_key<'a>(&self, key: &'a str) -> Result<&'a str, SecretError> {
        let key = key.trim();
        if key.is_empty() {
            return Err(SecretError::Other("Secret key is required.".to_string()));
        }
        Ok(key)
    }

    fn validate_value(&self, value: &str) -> Result<(), SecretError> {
        if value.is_empty() {
            return Err(SecretError::Other(
                "Secret value cannot be empty; delete the secret instead.".to_string(),
            ));
        }
        Ok(())
    }

    fn session_secret(&self, key: &str) -> Result<Option<String>, SecretError> {
        Ok(self
            .session_secrets
            .lock()
            .map_err(|_| SecretError::Other("Failed to read secret session cache.".to_string()))?
            .get(key)
            .cloned())
    }

    fn set_session_secret(&self, key: &str, value: &str) -> Result<(), SecretError> {
        self.session_secrets
            .lock()
            .map_err(|_| SecretError::Other("Failed to update secret session cache.".to_string()))?
            .insert(key.to_string(), value.to_string());
        Ok(())
    }

    fn clear_session_secret(&self, key: &str) -> Result<(), SecretError> {
        self.session_secrets
            .lock()
            .map_err(|_| SecretError::Other("Failed to clear secret session cache.".to_string()))?
            .remove(key);
        Ok(())
    }

    fn protected_secret(&self, key: &str) -> Result<Option<String>, SecretError> {
        match self.protected_backend.get_protected_secret(key) {
            Ok(value) => self.protector.unprotect(&value).map(Some),
            Err(SecretError::NoEntry) => Ok(None),
            Err(error) => Err(error),
        }
    }

    fn protected_state(&self, key: &str) -> Result<SecretStorageState, SecretError> {
        match self.protected_secret(key) {
            Ok(Some(value)) if !value.is_empty() => Ok(SecretStorageState::Stored),
            Ok(_) => match self.session_secret(key)? {
                Some(_) => Ok(SecretStorageState::Session),
                None => Ok(SecretStorageState::Missing),
            },
            Err(error) => match self.session_secret(key)? {
                Some(_) => Ok(SecretStorageState::Session),
                None => Err(error),
            },
        }
    }

    fn protected_or_session_secret(
        &self,
        key: &str,
        primary_error: Option<SecretError>,
    ) -> Result<Option<String>, SecretError> {
        match self.protected_secret(key) {
            Ok(Some(value)) if !value.is_empty() => Ok(Some(value)),
            Ok(_) => match self.session_secret(key)? {
                Some(value) => Ok(Some(value)),
                None => {
                    if let Some(error) = primary_error {
                        Err(error)
                    } else {
                        Ok(None)
                    }
                }
            },
            Err(protected_error) => match self.session_secret(key)? {
                Some(value) => Ok(Some(value)),
                None => Err(match primary_error {
                    Some(error) => SecretError::Other(format!(
                        "{}; {}",
                        error.message(),
                        protected_error.message()
                    )),
                    None => protected_error,
                }),
            },
        }
    }

    fn fallback_to_protected_or_session(
        &self,
        key: &str,
        value: &str,
    ) -> Result<SecretStorageState, SecretError> {
        let protected_result = (|| {
            let protected = self.protector.protect(value)?;
            self.protected_backend
                .set_protected_secret(key, &protected)?;
            let readback = self.protected_backend.get_protected_secret(key)?;
            let unprotected = self.protector.unprotect(&readback)?;
            if unprotected != value {
                return Err(SecretError::Other(
                    "App-local protected secret readback did not match after saving.".to_string(),
                ));
            }
            Ok(())
        })();

        self.set_session_secret(key, value)?;
        if protected_result.is_ok() {
            Ok(SecretStorageState::Stored)
        } else {
            Ok(SecretStorageState::Session)
        }
    }
}

impl SecretStore for SystemSecretStore {
    fn get(&self, key: &str) -> Result<Option<String>, SecretError> {
        let key = self.normalize_key(key)?;
        match self.backend.get_secret(key) {
            Ok(value) if !value.is_empty() => Ok(Some(value)),
            Ok(_) => self.protected_or_session_secret(key, None),
            Err(SecretError::NoEntry) => self.protected_or_session_secret(key, None),
            Err(error) => self.protected_or_session_secret(key, Some(error)),
        }
    }

    fn set(&self, key: &str, value: &str) -> Result<SecretStorageState, SecretError> {
        let key = self.normalize_key(key)?;
        self.validate_value(value)?;

        match self
            .backend
            .set_secret(key, value)
            .and_then(|_| self.backend.get_secret(key))
        {
            Ok(saved_value) if saved_value == value => {
                let _ = self.protected_backend.delete_protected_secret(key);
                self.clear_session_secret(key)?;
                Ok(SecretStorageState::Stored)
            }
            Ok(_) | Err(_) => self.fallback_to_protected_or_session(key, value),
        }
    }

    fn delete(&self, key: &str) -> Result<(), SecretError> {
        let key = self.normalize_key(key)?;
        let backend_result = self.backend.delete_secret(key);
        let protected_result = self.protected_backend.delete_protected_secret(key);
        let session_result = self.clear_session_secret(key);

        let errors: Vec<String> = [backend_result, protected_result, session_result]
            .into_iter()
            .filter_map(|result| result.err())
            .filter(|error| !matches!(error, SecretError::NoEntry))
            .map(|error| error.message())
            .collect();

        if errors.is_empty() {
            Ok(())
        } else {
            Err(SecretError::Other(errors.join("; ")))
        }
    }

    fn state(&self, key: &str) -> Result<SecretStorageState, SecretError> {
        let key = self.normalize_key(key)?;
        match self.backend.get_secret(key) {
            Ok(value) if !value.is_empty() => Ok(SecretStorageState::Stored),
            Ok(_) => self.protected_state(key),
            Err(SecretError::NoEntry) => self.protected_state(key),
            Err(error) => match self.protected_state(key) {
                Ok(SecretStorageState::Stored | SecretStorageState::Session) => {
                    self.protected_state(key)
                }
                Ok(_) | Err(_) => {
                    if self.session_secret(key)?.is_some() {
                        Ok(SecretStorageState::Session)
                    } else {
                        let _ = error;
                        Ok(SecretStorageState::Unreadable)
                    }
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct MemorySecretStoreBackend {
        values: Mutex<HashMap<String, String>>,
        set_error: Mutex<Option<SecretError>>,
        get_error: Mutex<Option<SecretError>>,
        delete_error: Mutex<Option<SecretError>>,
    }

    impl MemorySecretStoreBackend {
        fn with_get_error(error: SecretError) -> Arc<Self> {
            Arc::new(Self {
                get_error: Mutex::new(Some(error)),
                ..Self::default()
            })
        }

        fn with_set_error(error: SecretError) -> Arc<Self> {
            Arc::new(Self {
                set_error: Mutex::new(Some(error)),
                ..Self::default()
            })
        }

        fn stored_value(&self, key: &str) -> Option<String> {
            self.values.lock().expect("values").get(key).cloned()
        }
    }

    impl SecretStoreBackend for MemorySecretStoreBackend {
        fn set_secret(&self, key: &str, value: &str) -> Result<(), SecretError> {
            if let Some(error) = self.set_error.lock().expect("set error").clone() {
                return Err(error);
            }
            self.values
                .lock()
                .expect("values")
                .insert(key.to_string(), value.to_string());
            Ok(())
        }

        fn get_secret(&self, key: &str) -> Result<String, SecretError> {
            if let Some(error) = self.get_error.lock().expect("get error").clone() {
                return Err(error);
            }
            self.values
                .lock()
                .expect("values")
                .get(key)
                .cloned()
                .ok_or(SecretError::NoEntry)
        }

        fn delete_secret(&self, key: &str) -> Result<(), SecretError> {
            if let Some(error) = self.delete_error.lock().expect("delete error").clone() {
                return Err(error);
            }
            self.values.lock().expect("values").remove(key);
            Ok(())
        }
    }

    #[derive(Default)]
    struct MemoryProtectedSecretBackend {
        values: Mutex<HashMap<String, String>>,
    }

    impl MemoryProtectedSecretBackend {
        fn stored_value(&self, key: &str) -> Option<String> {
            self.values.lock().expect("values").get(key).cloned()
        }
    }

    impl ProtectedSecretBackend for MemoryProtectedSecretBackend {
        fn set_protected_secret(&self, key: &str, value: &str) -> Result<(), SecretError> {
            self.values
                .lock()
                .expect("values")
                .insert(key.to_string(), value.to_string());
            Ok(())
        }

        fn get_protected_secret(&self, key: &str) -> Result<String, SecretError> {
            self.values
                .lock()
                .expect("values")
                .get(key)
                .cloned()
                .ok_or(SecretError::NoEntry)
        }

        fn delete_protected_secret(&self, key: &str) -> Result<(), SecretError> {
            self.values.lock().expect("values").remove(key);
            Ok(())
        }
    }

    struct MockProtector {
        fail_protect: bool,
    }

    impl Default for MockProtector {
        fn default() -> Self {
            Self {
                fail_protect: false,
            }
        }
    }

    impl SecretProtector for MockProtector {
        fn protect(&self, value: &str) -> Result<String, SecretError> {
            if self.fail_protect {
                return Err(SecretError::Other("dpapi unavailable".to_string()));
            }
            Ok(format!("protected::{value}"))
        }

        fn unprotect(&self, value: &str) -> Result<String, SecretError> {
            value
                .strip_prefix("protected::")
                .map(ToString::to_string)
                .ok_or_else(|| SecretError::Other("invalid protected payload".to_string()))
        }
    }

    fn store_with(
        backend: Arc<dyn SecretStoreBackend>,
        protected_backend: Arc<dyn ProtectedSecretBackend>,
        protector: Arc<dyn SecretProtector>,
    ) -> SystemSecretStore {
        SystemSecretStore::with_parts(backend, protected_backend, protector)
    }

    #[test]
    fn set_uses_system_keyring_when_readback_matches() {
        let backend = Arc::new(MemorySecretStoreBackend::default());
        let protected_backend = Arc::new(MemoryProtectedSecretBackend::default());
        let store = store_with(
            backend.clone(),
            protected_backend.clone(),
            Arc::new(MockProtector::default()),
        );

        let state = store.set("github_pat", "token").unwrap();

        assert_eq!(state, SecretStorageState::Stored);
        assert_eq!(store.get("github_pat").unwrap().as_deref(), Some("token"));
        assert_eq!(
            store.state("github_pat").unwrap(),
            SecretStorageState::Stored
        );
        assert_eq!(backend.stored_value("github_pat").as_deref(), Some("token"));
        assert_eq!(protected_backend.stored_value("github_pat"), None);
    }

    #[test]
    fn set_falls_back_to_protected_secret_when_keyring_write_fails() {
        let backend = MemorySecretStoreBackend::with_set_error(SecretError::Other(
            "credential vault denied write".to_string(),
        ));
        let protected_backend = Arc::new(MemoryProtectedSecretBackend::default());
        let store = store_with(
            backend,
            protected_backend.clone(),
            Arc::new(MockProtector::default()),
        );

        let state = store.set("github_pat", "token").unwrap();

        assert_eq!(state, SecretStorageState::Stored);
        assert_eq!(store.get("github_pat").unwrap().as_deref(), Some("token"));
        assert_eq!(
            store.state("github_pat").unwrap(),
            SecretStorageState::Stored
        );
        assert_eq!(
            protected_backend.stored_value("github_pat").as_deref(),
            Some("protected::token")
        );
    }

    #[test]
    fn set_falls_back_to_session_when_keyring_and_protected_secret_fail() {
        let backend = MemorySecretStoreBackend::with_set_error(SecretError::Other(
            "credential vault denied write".to_string(),
        ));
        let protected_backend = Arc::new(MemoryProtectedSecretBackend::default());
        let store = store_with(
            backend,
            protected_backend.clone(),
            Arc::new(MockProtector { fail_protect: true }),
        );

        let state = store.set("ai_api_key", "sk-test").unwrap();

        assert_eq!(state, SecretStorageState::Session);
        assert_eq!(store.get("ai_api_key").unwrap().as_deref(), Some("sk-test"));
        assert_eq!(
            store.state("ai_api_key").unwrap(),
            SecretStorageState::Session
        );
        assert_eq!(protected_backend.stored_value("ai_api_key"), None);
    }

    #[test]
    fn get_reports_unreadable_when_no_fallback_is_available() {
        let backend = MemorySecretStoreBackend::with_get_error(SecretError::Other(
            "credential vault locked".to_string(),
        ));
        let store = store_with(
            backend,
            Arc::new(MemoryProtectedSecretBackend::default()),
            Arc::new(MockProtector::default()),
        );

        let error = store.get("github_pat").unwrap_err();

        assert!(error.message().contains("credential vault locked"));
        assert_eq!(
            store.state("github_pat").unwrap(),
            SecretStorageState::Unreadable
        );
    }

    #[test]
    fn delete_clears_all_secret_layers() {
        let backend = Arc::new(MemorySecretStoreBackend::default());
        let protected_backend = Arc::new(MemoryProtectedSecretBackend::default());
        let store = store_with(
            backend.clone(),
            protected_backend.clone(),
            Arc::new(MockProtector::default()),
        );

        store.set("github_pat", "token").unwrap();
        store.delete("github_pat").unwrap();

        assert_eq!(store.get("github_pat").unwrap(), None);
        assert_eq!(
            store.state("github_pat").unwrap(),
            SecretStorageState::Missing
        );
        assert_eq!(backend.stored_value("github_pat"), None);
        assert_eq!(protected_backend.stored_value("github_pat"), None);
    }
}
