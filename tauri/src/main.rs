// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if orbit_lib::services::mcp_config::is_mcp_stdio_mode() {
        orbit_lib::mcp_proxy::run();
        return;
    }
    orbit_lib::run()
}
