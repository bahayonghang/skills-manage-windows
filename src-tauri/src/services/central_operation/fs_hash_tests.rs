use super::{fingerprint_path_blocking, path_token};

#[test]
fn local_file_fingerprint_and_path_token_remain_byte_exact() {
    let temp = tempfile::tempdir().unwrap();
    let file = temp.path().join("fixture.txt");
    std::fs::write(&file, b"central-operation-fixture").unwrap();

    assert_eq!(
        fingerprint_path_blocking(&file).unwrap().as_deref(),
        Some("15722a6eef7bd65bd91506c76f1a746c18b2f9c66afb580b45b4ddecf7bb1e0a")
    );
    assert_eq!(path_token("C:/SkillPort/hash-fixture"), "8303f9fb6a2c052c");
}

#[test]
fn directory_fingerprint_preserves_the_previous_global_path_order() {
    use sha2::{Digest, Sha256};
    use walkdir::WalkDir;

    let root = tempfile::tempdir().unwrap();
    let directory = root.path().join("tree");
    std::fs::create_dir_all(directory.join("a/empty")).unwrap();
    std::fs::create_dir_all(directory.join("图/empty")).unwrap();
    for (name, bytes) in [
        ("a.ext", b"prefix".as_slice()),
        ("a/z", b"nested"),
        ("图/文本", b"unicode"),
        ("empty", b""),
    ] {
        std::fs::write(directory.join(name), bytes).unwrap();
    }
    let symlink_target = root.path().join("outside");
    std::fs::create_dir(&symlink_target).unwrap();
    std::fs::write(symlink_target.join("retained"), b"target").unwrap();
    crate::test_support::symlink_dir(&symlink_target, &directory.join("link"));
    let mut entries = WalkDir::new(&directory)
        .follow_links(false)
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    entries.sort_by_key(|entry| entry.path().to_path_buf());
    let mut hasher = Sha256::new();
    hasher.update(b"dir\0");
    for entry in entries.into_iter().skip(1) {
        hasher.update(
            entry
                .path()
                .strip_prefix(&directory)
                .unwrap()
                .to_string_lossy()
                .as_bytes(),
        );
        hasher.update([0]);
        let metadata = entry.metadata().unwrap();
        if metadata.file_type().is_symlink() {
            hasher.update(b"symlink\0");
            hasher.update(
                std::fs::read_link(entry.path())
                    .unwrap()
                    .to_string_lossy()
                    .as_bytes(),
            );
        } else if metadata.is_file() {
            hasher.update(b"file\0");
            super::hash_file(entry.path(), &mut hasher).unwrap();
        } else {
            hasher.update(b"dir\0");
        }
    }
    let expected = crate::hashing::encode_lower_hex(hasher.finalize().as_ref());
    assert_eq!(
        fingerprint_path_blocking(&directory).unwrap().as_deref(),
        Some(expected.as_str())
    );
    std::fs::write(symlink_target.join("retained"), b"changed external target").unwrap();
    assert_eq!(
        fingerprint_path_blocking(&directory).unwrap().as_deref(),
        Some(expected.as_str())
    );
    assert!(symlink_target.join("retained").is_file());
}
