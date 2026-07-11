use std::process::Command;

fn main() {
    let script = r#"printf 'target=%s\nparent=%s\n' "$1" "$2""#;
    let output = Command::new("wsl.exe")
        .args([
            "-d",
            "Ubuntu-24.04",
            "--exec",
            "sh",
            "-c",
            script,
            "--",
            "/tmp/skillport-probe/skill",
            "/tmp/skillport-probe",
        ])
        .output()
        .expect("failed to start wsl.exe");

    print!("{}", String::from_utf8_lossy(&output.stdout));
    eprint!("{}", String::from_utf8_lossy(&output.stderr));
    std::process::exit(output.status.code().unwrap_or(1));
}
