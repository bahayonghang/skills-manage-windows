#[derive(Debug, Clone, PartialEq, Eq)]
enum CredentialStoreError {
    NoEntry,
    Other(String),
}

impl CredentialStoreError {
    fn message(&self) -> String {
        match self {
            CredentialStoreError::NoEntry => "No saved SSH password is available.".to_string(),
            CredentialStoreError::Other(error) => error.clone(),
        }
    }
}

trait CredentialBackend: Send + Sync {
    fn set_password(
        &self,
        credential_key: &str,
        password: &str,
    ) -> Result<(), CredentialStoreError>;
    fn get_password(&self, credential_key: &str) -> Result<String, CredentialStoreError>;
    fn delete_credential(&self, credential_key: &str) -> Result<(), CredentialStoreError>;
}

struct SystemCredentialBackend;

#[cfg(windows)]
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
fn hex_decode(value: &str) -> Result<Vec<u8>, String> {
    if !value.len().is_multiple_of(2) {
        return Err("Protected password payload is not valid hex.".to_string());
    }
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|chunk| {
            let high = (chunk[0] as char)
                .to_digit(16)
                .ok_or_else(|| "Protected password payload is not valid hex.".to_string())?;
            let low = (chunk[1] as char)
                .to_digit(16)
                .ok_or_else(|| "Protected password payload is not valid hex.".to_string())?;
            Ok(((high << 4) | low) as u8)
        })
        .collect()
}

#[cfg(windows)]
mod protected_credentials {
    use std::ffi::c_void;
    use std::io;
    use std::ptr::{null, null_mut};
    use std::slice;

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

    fn last_error(action: &str) -> String {
        format!(
            "Failed to {} SSH password with Windows DPAPI: {}",
            action,
            io::Error::last_os_error()
        )
    }

    pub fn protect(password: &str) -> Result<String, String> {
        let mut input = password.as_bytes().to_vec();
        let mut input_blob = DataBlob {
            cbData: input
                .len()
                .try_into()
                .map_err(|_| "SSH password is too large to protect.".to_string())?,
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
        Ok(super::hex_encode(&protected))
    }

    pub fn unprotect(protected: &str) -> Result<String, String> {
        let mut input = super::hex_decode(protected)?;
        let mut input_blob = DataBlob {
            cbData: input
                .len()
                .try_into()
                .map_err(|_| "Protected SSH password payload is too large.".to_string())?,
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
        String::from_utf8(plaintext)
            .map_err(|e| format!("Protected SSH password is not valid UTF-8: {}", e))
    }
}

#[cfg(not(windows))]
mod protected_credentials {
    pub fn protect(_password: &str) -> Result<String, String> {
        Err("App-local protected SSH password fallback is only available on Windows.".to_string())
    }

    pub fn unprotect(_protected: &str) -> Result<String, String> {
        Err("App-local protected SSH password fallback is only available on Windows.".to_string())
    }
}

impl SystemCredentialBackend {
    fn entry(credential_key: &str) -> Result<keyring::Entry, CredentialStoreError> {
        keyring::Entry::new(SSH_PASSWORD_SERVICE, credential_key).map_err(|e| {
            CredentialStoreError::Other(format!("Failed to access system credential store: {}", e))
        })
    }
}

impl CredentialBackend for SystemCredentialBackend {
    fn set_password(
        &self,
        credential_key: &str,
        password: &str,
    ) -> Result<(), CredentialStoreError> {
        Self::entry(credential_key)?
            .set_password(password)
            .map_err(|e| {
                CredentialStoreError::Other(format!(
                    "Failed to store SSH password in system credential store: {}",
                    e
                ))
            })
    }

    fn get_password(&self, credential_key: &str) -> Result<String, CredentialStoreError> {
        match Self::entry(credential_key)?.get_password() {
            Ok(password) => Ok(password),
            Err(keyring::Error::NoEntry) => Err(CredentialStoreError::NoEntry),
            Err(e) => Err(CredentialStoreError::Other(format!(
                "Failed to read SSH password from system credential store: {}",
                e
            ))),
        }
    }

    fn delete_credential(&self, credential_key: &str) -> Result<(), CredentialStoreError> {
        match Self::entry(credential_key)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(CredentialStoreError::Other(format!(
                "Failed to delete SSH password from system credential store: {}",
                e
            ))),
        }
    }
}

fn protected_password_for_target(target: &RemoteTargetConfig) -> Result<Option<String>, String> {
    target
        .protected_password
        .as_deref()
        .map(protected_credentials::unprotect)
        .transpose()
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TargetCredentialState {
    status: TargetCredentialStatus,
    error: Option<String>,
}

