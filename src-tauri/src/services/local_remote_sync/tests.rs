use super::*;

#[test]
fn repo_slug_prefers_directory_name_and_sanitizes() {
    let root = std::path::Path::new(r"D:\Documents\Code\Agents\skills-manage-windows");
    assert_eq!(repo_slug(root), "skills-manage-windows");
}

#[test]
fn safe_relative_path_rejects_parent_components() {
    assert!(is_safe_relative_archive_path("src/main.rs"));
    assert!(!is_safe_relative_archive_path("../secret.txt"));
    assert!(!is_safe_relative_archive_path("src/../../secret.txt"));
    assert!(!is_safe_relative_archive_path("/absolute/path"));
    assert!(!is_safe_relative_archive_path(r"C:\Users\secret"));
}

#[test]
fn repo_snapshot_excludes_heavy_and_git_dirs() {
    let tmp = tempfile::TempDir::new().unwrap();
    std::fs::create_dir_all(tmp.path().join("src")).unwrap();
    std::fs::create_dir_all(tmp.path().join(".git")).unwrap();
    std::fs::create_dir_all(tmp.path().join("node_modules/pkg")).unwrap();
    std::fs::write(tmp.path().join("src/lib.rs"), "pub fn demo() {}\n").unwrap();
    std::fs::write(tmp.path().join(".git/config"), "secret").unwrap();
    std::fs::write(tmp.path().join("node_modules/pkg/index.js"), "big").unwrap();

    let snapshot = collect_repo_snapshot(tmp.path()).unwrap();
    let paths = snapshot
        .files
        .iter()
        .map(|file| file.relative_path.as_str())
        .collect::<Vec<_>>();
    assert_eq!(paths, vec!["src/lib.rs"]);
}

#[test]
fn skills_snapshot_collects_only_valid_skill_dirs() {
    let tmp = tempfile::TempDir::new().unwrap();
    std::fs::create_dir_all(tmp.path().join("good")).unwrap();
    std::fs::create_dir_all(tmp.path().join("bad")).unwrap();
    std::fs::write(tmp.path().join("good/SKILL.md"), "---\nname: Good\n---\n").unwrap();
    std::fs::write(tmp.path().join("bad/README.md"), "no skill").unwrap();

    let skills = collect_skill_snapshots(tmp.path()).unwrap();
    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0].id, "good");
}

#[test]
fn archive_builder_rejects_unsafe_path() {
    let snapshot = LocalSnapshot {
        id: "bad".to_string(),
        label: "bad".to_string(),
        root: std::path::PathBuf::from("."),
        files: vec![SnapshotFile {
            relative_path: "../bad.txt".to_string(),
            bytes: b"bad".to_vec(),
        }],
        file_count: 1,
        byte_count: 3,
        hash: "hash".to_string(),
    };
    assert!(build_archive(&snapshot).is_err());
}

#[test]
fn remote_hash_output_missing_returns_none() {
    assert_eq!(parse_remote_hash_output("MISSING\n").unwrap(), None);
}

#[test]
fn remote_hash_output_ignores_extra_files_by_using_supplied_manifest() {
    let file = SnapshotFile {
        relative_path: "SKILL.md".to_string(),
        bytes: b"demo".to_vec(),
    };
    let digest = hex_digest(&<sha2::Sha256 as sha2::Digest>::digest(&file.bytes));
    let expected = hash_snapshot_files(&[file]);
    let parsed = parse_remote_hash_output(&format!("{digest}\tSKILL.md\n")).unwrap();
    assert_eq!(parsed, Some(expected));
}
