use std::{
    fs::OpenOptions,
    io::{self, Stdout, Write},
    panic,
    path::Path,
    sync::{Arc, Mutex, Once, Weak},
};

use crossterm::{
    cursor::Show,
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};

pub(super) type ManagedTerminal = Terminal<CrosstermBackend<Stdout>>;

#[derive(Debug, Default)]
struct CleanupState {
    raw_mode_may_be_enabled: bool,
    alternate_screen_may_be_active: bool,
    restored: bool,
}

#[derive(Debug, Default)]
struct TerminalCleanup {
    state: Mutex<CleanupState>,
}

impl TerminalCleanup {
    fn mark_raw_mode_attempted(&self) {
        if let Ok(mut state) = self.state.lock() {
            state.raw_mode_may_be_enabled = true;
        }
    }

    fn mark_alternate_screen_attempted(&self) {
        if let Ok(mut state) = self.state.lock() {
            state.alternate_screen_may_be_active = true;
        }
    }

    fn restore_once(&self) -> io::Result<()> {
        let (restore_raw_mode, leave_alternate_screen) = {
            let mut state = self
                .state
                .lock()
                .map_err(|_| io::Error::other("terminal cleanup state is poisoned"))?;
            if state.restored {
                return Ok(());
            }
            state.restored = true;
            (
                state.raw_mode_may_be_enabled,
                state.alternate_screen_may_be_active,
            )
        };

        record_restore_execution();

        let mut failures = Vec::new();
        if restore_raw_mode
            && let Err(error) = disable_raw_mode()
        {
            failures.push(format!("disable raw mode: {error}"));
        }

        let mut stdout = io::stdout();
        if leave_alternate_screen
            && let Err(error) = execute!(stdout, LeaveAlternateScreen)
        {
            failures.push(format!("leave alternate screen: {error}"));
        }
        if let Err(error) = execute!(stdout, Show) {
            failures.push(format!("show cursor: {error}"));
        }
        if let Err(error) = stdout.flush() {
            failures.push(format!("flush terminal restoration: {error}"));
        }

        if failures.is_empty() {
            Ok(())
        } else {
            Err(io::Error::other(failures.join("; ")))
        }
    }
}

fn active_terminal_slot() -> &'static Mutex<Option<Weak<TerminalCleanup>>> {
    static ACTIVE_TERMINAL: Mutex<Option<Weak<TerminalCleanup>>> = Mutex::new(None);
    &ACTIVE_TERMINAL
}

fn register_active_terminal(cleanup: &Arc<TerminalCleanup>) {
    if let Ok(mut slot) = active_terminal_slot().lock() {
        *slot = Some(Arc::downgrade(cleanup));
    }
}

fn unregister_active_terminal(cleanup: &Arc<TerminalCleanup>) {
    if let Ok(mut slot) = active_terminal_slot().lock() {
        let owns_slot = slot
            .as_ref()
            .and_then(Weak::upgrade)
            .is_some_and(|active| Arc::ptr_eq(&active, cleanup));
        if owns_slot {
            *slot = None;
        }
    }
}

fn install_process_panic_hook() {
    static INSTALL_HOOK: Once = Once::new();
    INSTALL_HOOK.call_once(|| {
        let previous = panic::take_hook();
        panic::set_hook(Box::new(move |info| {
            let cleanup = active_terminal_slot()
                .lock()
                .ok()
                .and_then(|slot| slot.as_ref().and_then(Weak::upgrade));
            if let Some(cleanup) = cleanup
                && let Err(error) = cleanup.restore_once()
            {
                eprintln!("Strata panic terminal cleanup failed: {error}");
            }
            previous(info);
        }));
    });
}

pub(super) struct TerminalSession {
    terminal: ManagedTerminal,
    cleanup: Arc<TerminalCleanup>,
}

impl TerminalSession {
    pub(super) fn enter() -> io::Result<Self> {
        install_process_panic_hook();
        let cleanup = Arc::new(TerminalCleanup::default());
        register_active_terminal(&cleanup);

        cleanup.mark_raw_mode_attempted();
        if let Err(primary) = enable_raw_mode() {
            return Err(startup_failure(primary, &cleanup));
        }

        cleanup.mark_alternate_screen_attempted();
        let mut stdout = io::stdout();
        if let Err(primary) = execute!(stdout, EnterAlternateScreen) {
            return Err(startup_failure(primary, &cleanup));
        }

        let backend = CrosstermBackend::new(stdout);
        let terminal = match Terminal::new(backend) {
            Ok(terminal) => terminal,
            Err(primary) => return Err(startup_failure(primary, &cleanup)),
        };

        Ok(Self { terminal, cleanup })
    }

    pub(super) fn terminal_mut(&mut self) -> &mut ManagedTerminal {
        &mut self.terminal
    }

    pub(super) fn restore(&mut self) -> io::Result<()> {
        let result = self.cleanup.restore_once();
        unregister_active_terminal(&self.cleanup);
        result
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        if let Err(error) = self.restore() {
            eprintln!("Strata terminal cleanup failed: {error}");
        }
    }
}

fn startup_failure(primary: io::Error, cleanup: &Arc<TerminalCleanup>) -> io::Error {
    let cleanup_result = cleanup.restore_once();
    unregister_active_terminal(cleanup);
    compose_primary_error(primary, None, cleanup_result)
}

pub(super) fn compose_runtime_failure(
    primary: io::Error,
    checkpoint_result: Result<(), String>,
    cleanup_result: io::Result<()>,
) -> io::Error {
    compose_primary_error(primary, Some(checkpoint_result), cleanup_result)
}

pub(super) fn finish_normal_run(
    application_error: Option<String>,
    cleanup_result: io::Result<()>,
) -> io::Result<()> {
    match (application_error, cleanup_result) {
        (None, Ok(())) => Ok(()),
        (None, Err(cleanup)) => Err(cleanup),
        (Some(application), Ok(())) => Err(io::Error::other(application)),
        (Some(application), Err(cleanup)) => Err(io::Error::other(format!(
            "{application}; terminal cleanup failed: {cleanup}"
        ))),
    }
}

fn compose_primary_error(
    primary: io::Error,
    checkpoint_result: Option<Result<(), String>>,
    cleanup_result: io::Result<()>,
) -> io::Error {
    let kind = primary.kind();
    let mut message = primary.to_string();
    if let Some(checkpoint_result) = checkpoint_result {
        match checkpoint_result {
            Ok(()) => message.push_str("; emergency checkpoint: committed"),
            Err(error) => {
                message.push_str("; emergency checkpoint failed: ");
                message.push_str(&error);
            }
        }
    }
    if let Err(error) = cleanup_result {
        message.push_str("; terminal cleanup failed: ");
        message.push_str(&error.to_string());
    }
    io::Error::new(kind, message)
}

pub(super) fn maybe_inject_runtime_io_fault(stage: &str) -> io::Result<()> {
    if cfg!(debug_assertions)
        && std::env::var("STRATA_TEST_TUI_FAULT").as_deref() == Ok(stage)
    {
        return Err(io::Error::other(format!(
            "injected TUI {stage} failure"
        )));
    }
    Ok(())
}

pub(super) fn maybe_inject_runtime_panic() {
    if cfg!(debug_assertions)
        && std::env::var("STRATA_TEST_TUI_FAULT").as_deref() == Ok("panic")
    {
        panic!("injected TUI panic");
    }
}

fn record_restore_execution() {
    if !cfg!(debug_assertions) {
        return;
    }
    let Ok(path) = std::env::var("STRATA_TEST_TERMINAL_RESTORE_MARKER") else {
        return;
    };
    if path.trim().is_empty() {
        return;
    }
    let result = OpenOptions::new()
        .create(true)
        .append(true)
        .open(Path::new(&path))
        .and_then(|mut file| writeln!(file, "restored"));
    if let Err(error) = result {
        eprintln!("Strata terminal restore marker failed: {error}");
    }
}

#[cfg(test)]
mod tests {
    use super::{CleanupState, TerminalCleanup, compose_runtime_failure, finish_normal_run};
    use std::{io, sync::Mutex};

    #[test]
    fn cleanup_state_executes_only_once() {
        let cleanup = TerminalCleanup {
            state: Mutex::new(CleanupState {
                raw_mode_may_be_enabled: false,
                alternate_screen_may_be_active: false,
                restored: false,
            }),
        };
        cleanup.restore_once().unwrap();
        cleanup.restore_once().unwrap();
        assert!(cleanup.state.lock().unwrap().restored);
    }

    #[test]
    fn original_runtime_error_remains_primary_with_context() {
        let error = compose_runtime_failure(
            io::Error::new(io::ErrorKind::BrokenPipe, "original draw failure"),
            Err("checkpoint unavailable".to_string()),
            Err(io::Error::other("cleanup unavailable")),
        );
        assert_eq!(error.kind(), io::ErrorKind::BrokenPipe);
        let message = error.to_string();
        assert!(message.contains("original draw failure"));
        assert!(message.contains("checkpoint unavailable"));
        assert!(message.contains("cleanup unavailable"));
    }

    #[test]
    fn normal_application_error_is_not_erased_by_cleanup_failure() {
        let error = finish_normal_run(
            Some("application finalization failed".to_string()),
            Err(io::Error::other("cleanup failed")),
        )
        .unwrap_err();
        assert!(error.to_string().contains("application finalization failed"));
        assert!(error.to_string().contains("cleanup failed"));
    }
}
