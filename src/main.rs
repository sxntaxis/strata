#![forbid(unsafe_code)]

use std::io;

use strata::{app, cli, domain, keybindings, storage};

fn main() -> Result<(), io::Error> {
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

    app::run_ui()
}
