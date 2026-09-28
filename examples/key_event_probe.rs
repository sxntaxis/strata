use std::io::{self, Write};

use crossterm::{
    event::{self, Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode},
};

struct RawModeGuard;

impl RawModeGuard {
    fn enter() -> io::Result<Self> {
        enable_raw_mode()?;
        Ok(Self)
    }
}

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
    }
}

fn main() -> io::Result<()> {
    println!("Strata BALANCE-RANGE-UX-001 raw key-event probe");
    println!("Run this inside the same terminal + tmux path used for Strata.");
    println!();
    println!("Suggested sequence:");
    println!("  1. Shift+Left, Shift+Right");
    println!("  2. Ctrl+Left,  Ctrl+Right");
    println!("  3. Alt+Left,   Alt+Right");
    println!("  4. type lowercase a");
    println!("  5. enable Caps Lock, type a, disable Caps Lock");
    println!("  6. Shift+a");
    println!();
    println!("Press Esc to exit. Every crossterm event is printed verbatim.");
    io::stdout().flush()?;

    let _raw = RawModeGuard::enter()?;
    loop {
        let event = event::read()?;
        match event {
            Event::Key(key) => {
                disable_raw_mode()?;
                println!(
                    "KEY code={:?} modifiers={:?} kind={:?} state={:?}",
                    key.code, key.modifiers, key.kind, key.state
                );
                io::stdout().flush()?;
                if key.code == KeyCode::Esc {
                    break;
                }
                enable_raw_mode()?;
            }
            other => {
                disable_raw_mode()?;
                println!("EVENT {other:?}");
                io::stdout().flush()?;
                enable_raw_mode()?;
            }
        }
    }

    Ok(())
}
