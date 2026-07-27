use std::{fs, path::PathBuf};

use base64::{engine::general_purpose::STANDARD, Engine as _};
use clap::Parser;
use minisign_verify::{PublicKey, Signature};

#[derive(Parser)]
#[command(about = "Verify a Tauri updater signature for a release artifact")]
struct Args {
    #[arg(long)]
    installer: PathBuf,
    #[arg(long)]
    signature: PathBuf,
    #[arg(long)]
    config: PathBuf,
}

fn decode_tauri_text(value: &str, label: &str) -> Result<String, String> {
    let bytes = STANDARD
        .decode(value.trim())
        .map_err(|error| format!("invalid base64 {label}: {error}"))?;
    String::from_utf8(bytes).map_err(|_| format!("decoded {label} is not UTF-8"))
}

fn verify_signature(
    data: &[u8],
    wrapped_signature: &str,
    wrapped_public_key: &str,
) -> Result<(), String> {
    let public_key_text = decode_tauri_text(wrapped_public_key, "public key")?;
    let signature_text = decode_tauri_text(wrapped_signature, "signature")?;
    let public_key = PublicKey::decode(&public_key_text)
        .map_err(|error| format!("invalid minisign public key: {error}"))?;
    let signature = Signature::decode(&signature_text)
        .map_err(|error| format!("invalid minisign signature: {error}"))?;
    public_key
        .verify(data, &signature, true)
        .map_err(|error| format!("signature verification failed: {error}"))
}

fn run(args: Args) -> Result<(), String> {
    let installer =
        fs::read(&args.installer).map_err(|error| format!("failed to read installer: {error}"))?;
    let wrapped_signature = fs::read_to_string(&args.signature)
        .map_err(|error| format!("failed to read updater signature: {error}"))?;
    let config: serde_json::Value = serde_json::from_slice(
        &fs::read(&args.config)
            .map_err(|error| format!("failed to read release config: {error}"))?,
    )
    .map_err(|error| format!("invalid release config JSON: {error}"))?;
    let wrapped_public_key = config
        .pointer("/plugins/updater/pubkey")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "release config is missing plugins.updater.pubkey".to_string())?;
    verify_signature(&installer, &wrapped_signature, wrapped_public_key)
}

fn main() {
    if let Err(error) = run(Args::parse()) {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wrapped_signature() -> String {
        STANDARD.encode(
            "untrusted comment: signature from minisign secret key\nRWQf6LRCGA9i59SLOFxz6NxvASXDJeRtuZykwQepbDEGt87ig1BNpWaVWuNrm73YiIiJbq71Wi+dP9eKL8OC351vwIasSSbXxwA=\ntrusted comment: timestamp:1555779966\tfile:test\nQtKMXWyYcwdpZAlPF7tE2ENJkRd1ujvKjlj1m9RtHTBnZPa5WKU5uWRs5GoP5M/VqE81QFuMKI5k/SfNQUaOAA==",
        )
    }

    fn wrapped_public_key() -> String {
        STANDARD.encode(
            "untrusted comment: minisign public key E7620F1842B4E81F\nRWQf6LRCGA9i53mlYecO4IzT51TGPpvWucNSCh1CBM0QTaLn73Y7GFO3",
        )
    }

    #[test]
    fn accepts_runtime_compatible_wrapped_signature() {
        verify_signature(b"test", &wrapped_signature(), &wrapped_public_key()).unwrap();
    }

    #[test]
    fn rejects_tampered_installer_signature_and_key() {
        assert!(verify_signature(b"Test", &wrapped_signature(), &wrapped_public_key()).is_err());

        let signature_text = decode_tauri_text(&wrapped_signature(), "signature").unwrap();
        let tampered_signature = STANDARD.encode(signature_text.replacen("RWQf", "RWQg", 1));
        assert!(verify_signature(b"test", &tampered_signature, &wrapped_public_key()).is_err());

        let key_text = decode_tauri_text(&wrapped_public_key(), "public key").unwrap();
        let tampered_key = STANDARD.encode(key_text.replacen("RWQf", "RWQg", 1));
        assert!(verify_signature(b"test", &wrapped_signature(), &tampered_key).is_err());
    }
}
