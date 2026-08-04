#![forbid(unsafe_code)]

use std::io;

#[allow(clippy::unnecessary_sort_by, clippy::while_let_loop)]
mod app;
mod category_lifecycle;
mod cli;
mod constants;
#[allow(clippy::unnecessary_sort_by)]
mod domain;
mod keybindings;
#[allow(dead_code)]
mod legacy_category_lifecycle;
mod legacy_transition;
#[allow(clippy::manual_checked_ops)]
mod sand;
#[allow(dead_code)]
mod sqlite;
mod storage;
mod temporal;

fn load_startup_configuration(
    ignore_config: bool,
) -> Result<keybindings::LoadedKeybindings, io::Error> {
    if ignore_config {
        return Ok(keybindings::default_loaded_keybindings());
    }

    let path = storage::get_keymap_path();
    keybindings::load_keybindings(&path).map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "Configuration error: {error}. Fix {} or rerun with --ignore-config to deliberately use built-in defaults",
                path.display()
            ),
        )
    })
}

fn apply_startup_configuration(loaded: &keybindings::LoadedKeybindings) {
    domain::set_runtime_settings(loaded.runtime_settings);
    storage::set_runtime_storage_settings(storage::RuntimeStorageSettings {
        time_log_path: loaded.time_log_path.clone(),
    });
}

pub fn run() -> Result<(), io::Error> {
    let invocation = cli::parse_invocation();
    let loaded = load_startup_configuration(invocation.ignore_config)?;
    apply_startup_configuration(&loaded);

    match invocation.command {
        Some(command) => {
            cli::run_command(command);
            Ok(())
        }
        None => app::run_ui(loaded),
    }
}
