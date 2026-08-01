#![forbid(unsafe_code)]

use std::io;

#[allow(clippy::unnecessary_sort_by, clippy::while_let_loop)]
mod app;
mod cli;
mod constants;
#[allow(clippy::unnecessary_sort_by)]
mod domain;
mod keybindings;
#[allow(clippy::manual_checked_ops)]
mod sand;
#[allow(dead_code)]
mod sqlite;
mod storage;

pub fn run() -> Result<(), io::Error> {
    if let Ok(loaded) = keybindings::load_keybindings(&storage::get_keymap_path()) {
        domain::set_runtime_settings(loaded.runtime_settings);
        storage::set_runtime_storage_settings(storage::RuntimeStorageSettings {
            time_log_path: loaded.time_log_path,
        });
    }

    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        cli::run_cli();
        return Ok(());
    }

    sqlite::ensure_tui_legacy_allowed().map_err(io::Error::other)?;
    app::run_ui()
}
