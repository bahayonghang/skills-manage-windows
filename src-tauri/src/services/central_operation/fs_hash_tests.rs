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
