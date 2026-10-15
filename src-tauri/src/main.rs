// No console window for the GUI in release builds (`randommac-cli` is the console program).
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() -> std::process::ExitCode {
    randommac_lib::main_entry()
}
