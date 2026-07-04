use super::*;
use std::collections::HashMap;
use tempfile::TempDir;

#[test]
fn hash_entries_is_stable_across_input_order() {
    let one = hex_digest(&Sha256::digest(b"one"));
    let two = hex_digest(&Sha256::digest(b"two"));
    let left = hash_entries(vec![
        ("b.txt".to_string(), two.clone()),
        ("a.txt".to_string(), one.clone()),
    ]);
    let right = hash_entries(vec![("a.txt".to_string(), one), ("b.txt".to_string(), two)]);

    assert_eq!(left, right);
    assert!(left.starts_with("sha256-manifest:"));
}

#[test]
fn local_hash_changes_when_file_content_changes() {
    let temp = TempDir::new().unwrap();
    let skill_dir = temp.path().join("demo");
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::write(skill_dir.join("SKILL.md"), b"one").unwrap();

    let first = hash_local_directory(&skill_dir).unwrap();
    std::fs::write(skill_dir.join("SKILL.md"), b"two").unwrap();
    let second = hash_local_directory(&skill_dir).unwrap();

    assert_ne!(first, second);
}

#[test]
fn build_skill_archive_rejects_unsafe_paths() {
    let files = vec![RemoteSkillFile {
        repo_path: "skills/demo/../evil".to_string(),
        relative_path: "../evil".to_string(),
        bytes: b"bad".to_vec(),
    }];

    assert!(build_skill_archive(&files).is_err());
}

#[test]
fn remote_update_command_quotes_paths_and_script() {
    let command = remote_update_command("/tmp/skill one", "/tmp", "/tmp/stage'one", "/tmp/backup");

    assert!(command.starts_with("sh -c '"));
    assert!(command.contains("'/tmp/skill one'"));
    assert!(command.contains("'/tmp/stage'\\''one'"));
}

#[test]
fn remote_hash_output_matches_local_manifest_rule() {
    let skill_md = hex_digest(&Sha256::digest(b"one"));
    let readme = hex_digest(&Sha256::digest(b"two"));
    let output = format!(
        "ROOT\t/skills/demo\n{skill_md}  ./SKILL.md\n{readme}  ./README.md\nEND\t/skills/demo\n"
    );

    let parsed = parse_remote_hash_output(&output).unwrap();
    let expected = hash_entries(vec![
        ("README.md".to_string(), readme),
        ("SKILL.md".to_string(), skill_md),
    ]);

    assert_eq!(parsed.get("/skills/demo"), Some(&expected));
}

#[test]
fn github_snapshot_hash_matches_local_directory_hash() {
    let snapshot = GitHubRepoSnapshot {
        files: HashMap::from([
            ("skills/demo/SKILL.md".to_string(), b"one".to_vec()),
            ("skills/demo/README.md".to_string(), b"two".to_vec()),
        ]),
    };
    let files = collect_remote_skill_files(&snapshot, "skills/demo").unwrap();
    let remote_hash = hash_remote_files(&snapshot, &files).unwrap();

    let temp = TempDir::new().unwrap();
    let skill_dir = temp.path().join("demo");
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::write(skill_dir.join("SKILL.md"), b"one").unwrap();
    std::fs::write(skill_dir.join("README.md"), b"two").unwrap();

    assert_eq!(remote_hash, hash_local_directory(&skill_dir).unwrap());
}

#[test]
fn collect_remote_skill_files_requires_source_path() {
    let snapshot = GitHubRepoSnapshot {
        files: HashMap::from([(
            "skills/demo/SKILL.md".to_string(),
            b"---\nname: Demo\n---".to_vec(),
        )]),
    };

    let files = collect_remote_skill_files(&snapshot, "skills/demo").unwrap();

    assert_eq!(files.len(), 1);
    assert_eq!(files[0].relative_path, "SKILL.md");
}
