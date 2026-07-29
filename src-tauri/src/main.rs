// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    #[cfg(feature = "ipc-codegen")]
    if std::env::args().nth(1).as_deref() == Some("--ipc-codegen") {
        let check = std::env::args()
            .skip(2)
            .any(|argument| argument == "--check");
        if let Err(error) = skillport_lib::ipc_codegen::run_tool(check) {
            eprintln!("[ipc-codegen] {error}");
            std::process::exit(1);
        }
        return;
    }

    if skillport_lib::targets::maybe_run_ssh_askpass_helper() {
        return;
    }

    skillport_lib::run()
}
