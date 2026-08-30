use std::fmt::Write;

/// Encode raw digest bytes as stable lowercase hexadecimal text.
pub(crate) fn encode_lower_hex(bytes: &[u8]) -> String {
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(&mut encoded, "{byte:02x}").expect("writing to a String cannot fail");
    }
    encoded
}

#[cfg(test)]
mod tests {
    use sha2::{Digest, Sha256};

    use super::encode_lower_hex;

    #[test]
    fn sha256_vectors_remain_byte_exact_and_lowercase() {
        assert_eq!(
            encode_lower_hex(Sha256::digest(b"").as_ref()),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            encode_lower_hex(Sha256::digest(b"abc").as_ref()),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            encode_lower_hex(&[0x00, 0x01, 0x0f, 0x10, 0xff]),
            "00010f10ff"
        );
    }
}
