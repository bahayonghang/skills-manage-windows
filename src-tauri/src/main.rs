// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if skillport_lib::targets::maybe_run_ssh_askpass_helper() {
        return;
    }

    skillport_lib::run()
}
