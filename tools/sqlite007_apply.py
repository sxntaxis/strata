from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    if old not in text:
        raise SystemExit(f"missing {label} anchor")
    return text.replace(old, new, 1)


# SQLite module wiring and schema v3 category ordering.
path = Path("src/sqlite.rs")
text = path.read_text()
text = replace_once(
    text,
    "mod repository;\n",
    "mod repository;\nmod tui_runtime;\n",
    "tui module",
)
text = replace_once(
    text,
    "pub(crate) use migration_command::{ControlledMigrationOptions, ControlledMigrationReport};\n",
    "pub(crate) use migration_command::{ControlledMigrationOptions, ControlledMigrationReport};\n"
    "pub(crate) use tui_runtime::{\n"
    "    SqliteTuiActiveSession, SqliteTuiState, clear_checkpoint as clear_tui_checkpoint,\n"
    "    delete_daily_snapshot as delete_tui_daily_snapshot,\n"
    "    ensure_active_session as ensure_tui_active_session,\n"
    "    finish_active_session as finish_tui_active_session, load_checkpoint as load_tui_checkpoint,\n"
    "    load_daily_snapshot as load_tui_daily_snapshot, load_sand_state as load_tui_sand_state,\n"
    "    load_state as load_tui_state, reset_active_session as reset_tui_active_session,\n"
    "    save_checkpoint as save_tui_checkpoint, save_daily_snapshot as save_tui_daily_snapshot,\n"
    "    save_sand_state as save_tui_sand_state, sync_categories as sync_tui_categories,\n"
    "    sync_category_tags as sync_tui_category_tags, sync_sessions as sync_tui_sessions,\n"
    "    switch_active_session as switch_tui_active_session,\n"
    "};\n",
    "tui exports",
)
text = text.replace("const CURRENT_SCHEMA_VERSION: i64 = 2;", "const CURRENT_SCHEMA_VERSION: i64 = 3;", 1)
migration_3 = r'''
const MIGRATION_3: &str = r#"
ALTER TABLE categories
    ADD COLUMN sort_order INTEGER NOT NULL DEFAULT 0 CHECK (sort_order >= 0);

UPDATE categories SET sort_order = id;

CREATE INDEX categories_active_order_index
    ON categories(archived_at_utc, sort_order, id);

INSERT INTO schema_migrations(version, applied_at_utc)
VALUES (3, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'));

PRAGMA user_version = 3;
"#;

'''
text = replace_once(
    text,
    "#[derive(Debug, Error)]\npub(crate) enum SqliteStoreError",
    migration_3 + "#[derive(Debug, Error)]\npub(crate) enum SqliteStoreError",
    "schema v3",
)
text = replace_once(
    text,
    "        if version < 2 {\n            let transaction =\n                connection.transaction_with_behavior(TransactionBehavior::Immediate)?;\n            transaction.execute_batch(MIGRATION_2)?;\n            transaction.commit()?;\n        }\n\n        Ok(Self { connection })",
    "        if version < 2 {\n            let transaction =\n                connection.transaction_with_behavior(TransactionBehavior::Immediate)?;\n            transaction.execute_batch(MIGRATION_2)?;\n            transaction.commit()?;\n            version = 2;\n        }\n\n        if version < 3 {\n            let transaction =\n                connection.transaction_with_behavior(TransactionBehavior::Immediate)?;\n            transaction.execute_batch(MIGRATION_3)?;\n            transaction.commit()?;\n        }\n\n        Ok(Self { connection })",
    "schema migration application",
)
text = text.replace("assert_eq!(repository.schema_version().unwrap(), 2);", "assert_eq!(repository.schema_version().unwrap(), 3);")
path.write_text(text)

# TUI is no longer blocked after SQLite activation.
path = Path("src/lib.rs")
text = path.read_text()
text = text.replace("\n    sqlite::ensure_tui_legacy_allowed().map_err(io::Error::other)?;", "", 1)
path.write_text(text)

# App structure, startup loading, session lifecycle, checkpoint routing, and fail-closed loop.
path = Path("src/app.rs")
text = path.read_text()
text = text.replace(
    "    io,\n    time::{Duration, Instant, SystemTime},",
    "    io,\n    path::PathBuf,\n    time::{Duration, Instant, SystemTime},",
    1,
)
text = text.replace("    storage,\n};", "    sqlite, storage,\n};", 1)
text = replace_once(
    text,
    "    render_needed: bool,\n}",
    "    render_needed: bool,\n    sqlite_database_path: Option<PathBuf>,\n    archived_categories: Vec<Category>,\n    storage_error: Option<String>,\n}",
    "app storage fields",
)
text = text.replace("    fn new(width: u16, height: u16) -> Self {", "    fn new(width: u16, height: u16) -> Result<Self, String> {", 1)
old_load = '''        let mut tracker = TimeTracker::new();
        let categories_path = storage::get_categories_path();
        let sessions_path = storage::get_time_log_path();

        let loaded_categories = storage::load_categories_from_csv(&categories_path);
        let loaded_sessions =
            storage::load_sessions_from_csv(&sessions_path, &loaded_categories.categories);
        tracker.apply_loaded_state(
            loaded_categories.categories,
            loaded_categories.next_category_id,
            loaded_sessions.sessions,
            loaded_sessions.next_session_id,
        );

        let mut category_tags = storage::load_category_tags(&storage::get_category_tags_path());
        let valid_category_ids: HashSet<u64> = tracker
            .categories_for_storage()
            .into_iter()
            .map(|category| category.id.0)
            .collect();
        category_tags
            .tags_by_category
            .retain(|category_id, _| valid_category_ids.contains(category_id));
'''
new_load = '''        let mut tracker = TimeTracker::new();
        let authority = sqlite::resolve_runtime_authority()?;
        let (
            sqlite_database_path,
            loaded_categories,
            loaded_sessions,
            mut category_tags,
            archived_categories,
            sqlite_active_session,
        ) = match authority {
            sqlite::RuntimeAuthority::LegacyFiles => {
                let categories_path = storage::get_categories_path();
                let sessions_path = storage::get_time_log_path();
                let loaded_categories = storage::load_categories_from_csv(&categories_path);
                let loaded_sessions = storage::load_sessions_from_csv(
                    &sessions_path,
                    &loaded_categories.categories,
                );
                let tags = storage::load_category_tags(&storage::get_category_tags_path());
                (
                    None,
                    loaded_categories,
                    loaded_sessions,
                    tags,
                    Vec::new(),
                    None,
                )
            }
            sqlite::RuntimeAuthority::SqliteCli { database_path } => {
                let state = sqlite::load_tui_state(&database_path)?;
                (
                    Some(database_path),
                    state.loaded_categories,
                    state.loaded_sessions,
                    state.category_tags,
                    state.archived_categories,
                    state.active_session,
                )
            }
        };
        tracker.apply_loaded_state(
            loaded_categories.categories,
            loaded_categories.next_category_id,
            loaded_sessions.sessions,
            loaded_sessions.next_session_id,
        );

        let valid_category_ids: HashSet<u64> = tracker
            .categories_for_storage()
            .into_iter()
            .map(|category| category.id.0)
            .collect();
        category_tags
            .tags_by_category
            .retain(|category_id, _| valid_category_ids.contains(category_id));
'''
text = replace_once(text, old_load, new_load, "startup storage load")
text = replace_once(
    text,
    "            render_needed: true,\n        };",
    "            render_needed: true,\n            sqlite_database_path,\n            archived_categories,\n            storage_error: None,\n        };",
    "app storage initialization",
)
old_startup = '''        app.persist_category_tags();

        if !app.restore_from_detached_checkpoint() {
            app.begin_active_session_now();
            app.restore_sand_state();
        }

        app.sync_drift_idle_state();

        app
'''
new_startup = '''        app.persist_category_tags();

        if !app.restore_from_detached_checkpoint() {
            if let Some(active) = sqlite_active_session {
                if !app
                    .time_tracker
                    .set_active_category_by_id(active.category_id)
                {
                    return Err(format!(
                        "SQLite active session references unavailable category {}",
                        active.category_id.0
                    ));
                }
                let _ = app.time_tracker.set_category_description_by_id(
                    active.category_id,
                    active.description,
                );
                app.begin_active_session_at(active.started_at_utc);
            } else {
                app.begin_active_session_now();
                app.persist_active_session_start();
            }
            app.restore_sand_state();
        }

        app.sync_drift_idle_state();
        if let Some(error) = app.storage_error.take() {
            return Err(error);
        }

        Ok(app)
'''
text = replace_once(text, old_startup, new_startup, "startup active session")

# Insert storage helpers before modal methods.
helpers = r'''    fn record_storage_result<T>(&mut self, result: Result<T, String>) -> Option<T> {
        match result {
            Ok(value) => Some(value),
            Err(error) => {
                if self.storage_error.is_none() {
                    self.storage_error = Some(error);
                }
                None
            }
        }
    }

    fn persist_active_session_start(&mut self) {
        let Some(database_path) = self.sqlite_database_path.clone() else {
            return;
        };
        let category_id = self.time_tracker.active_category_id();
        let description = self
            .time_tracker
            .category_description_by_id(category_id)
            .unwrap_or_default()
            .to_string();
        let started_at = self
            .session
            .active_session_started_at_utc
            .unwrap_or_else(Utc::now);
        let result = sqlite::ensure_tui_active_session(
            &database_path,
            category_id,
            &description,
            started_at,
        );
        self.record_storage_result(result);
    }

    fn reload_sqlite_sessions(&mut self) -> bool {
        let Some(database_path) = self.sqlite_database_path.clone() else {
            return true;
        };
        let Some(state) = self.record_storage_result(sqlite::load_tui_state(&database_path)) else {
            return false;
        };
        self.time_tracker.sessions = state.loaded_sessions.sessions;
        self.time_tracker.session_id_counter = state.loaded_sessions.next_session_id;
        self.archived_categories = state.archived_categories;
        true
    }

    fn reset_active_session_at(&mut self, started_at_utc: DateTime<Utc>) {
        if let Some(database_path) = self.sqlite_database_path.clone() {
            if self
                .record_storage_result(sqlite::reset_tui_active_session(
                    &database_path,
                    started_at_utc,
                ))
                .is_none()
            {
                return;
            }
        }
        self.begin_active_session_at(started_at_utc);
    }

'''
text = replace_once(text, "    fn open_modal(&mut self) {", helpers + "    fn open_modal(&mut self) {", "app storage helpers")

# Replace SQLite-aware session end and switch implementations.
old_end = '''        let elapsed = (clamped_end - start_utc)
            .to_std()
            .unwrap_or(Duration::ZERO)
            .as_secs() as usize;
        let ended_local = clamped_end.with_timezone(&Local);
        let result = self
            .time_tracker
            .end_session_with_elapsed_at_local(elapsed, ended_local);
        self.session.active_session_started_at_utc = None;
        result
'''
new_end = '''        let elapsed = (clamped_end - start_utc)
            .to_std()
            .unwrap_or(Duration::ZERO)
            .as_secs() as usize;
        let ended_local = clamped_end.with_timezone(&Local);

        if let Some(database_path) = self.sqlite_database_path.clone() {
            let operational_day = operational_day_key_for_local(&ended_local)
                .format("%Y-%m-%d")
                .to_string();
            if self
                .record_storage_result(sqlite::finish_tui_active_session(
                    &database_path,
                    clamped_end,
                    &operational_day,
                    elapsed,
                ))
                .is_none()
            {
                return None;
            }
            let active_category_id = self.time_tracker.active_category_id();
            let _ = self.time_tracker.set_category_description_by_id(
                active_category_id,
                String::new(),
            );
            self.time_tracker.current_session_start = None;
            self.session.active_session_started_at_utc = None;
            self.reload_sqlite_sessions();
            self.persist_categories();
            return Some(elapsed);
        }

        let result = self
            .time_tracker
            .end_session_with_elapsed_at_local(elapsed, ended_local);
        self.session.active_session_started_at_utc = None;
        result
'''
text = replace_once(text, old_end, new_end, "session completion")
old_switch = '''        self.end_active_session_at(switched_at_utc);
        self.persist_sessions();

        if !self.time_tracker.set_active_category_by_id(category_id) {
            return false;
        }

        self.begin_active_session_at(switched_at_utc);
        self.sync_drift_idle_state();

        true
'''
new_switch = '''        if let Some(database_path) = self.sqlite_database_path.clone() {
            let start_utc = self
                .session
                .active_session_started_at_utc
                .unwrap_or(switched_at_utc);
            let elapsed = (switched_at_utc - start_utc)
                .to_std()
                .unwrap_or(Duration::ZERO)
                .as_secs() as usize;
            let switched_local = switched_at_utc.with_timezone(&Local);
            let operational_day = operational_day_key_for_local(&switched_local)
                .format("%Y-%m-%d")
                .to_string();
            let next_description = self
                .time_tracker
                .category_description_by_id(category_id)
                .unwrap_or_default()
                .to_string();
            if self
                .record_storage_result(sqlite::switch_tui_active_session(
                    &database_path,
                    category_id,
                    &next_description,
                    switched_at_utc,
                    &operational_day,
                    elapsed,
                ))
                .is_none()
            {
                return false;
            }
            let previous_category_id = self.time_tracker.active_category_id();
            let _ = self.time_tracker.set_category_description_by_id(
                previous_category_id,
                String::new(),
            );
            if !self.time_tracker.set_active_category_by_id(category_id) {
                return false;
            }
            self.begin_active_session_at(switched_at_utc);
            self.reload_sqlite_sessions();
            self.persist_categories();
            self.sync_drift_idle_state();
            return true;
        }

        self.end_active_session_at(switched_at_utc);
        self.persist_sessions();

        if !self.time_tracker.set_active_category_by_id(category_id) {
            return false;
        }

        self.begin_active_session_at(switched_at_utc);
        self.sync_drift_idle_state();

        true
'''
text = replace_once(text, old_switch, new_switch, "session switch")
text = text.replace("                    self.begin_active_session_at(scheduled_at_utc);", "                    self.reset_active_session_at(scheduled_at_utc);", 1)

# Route detached checkpoint persistence.
text = text.replace("    fn persist_detached_checkpoint(&self) {", "    fn persist_detached_checkpoint(&mut self) {", 1)
old_checkpoint_write = '''        let path = storage::get_detached_runtime_path();
        let _ = storage::write_json_atomic(&path, &checkpoint);
    }

    fn clear_detached_checkpoint(&self) {
        let path = storage::get_detached_runtime_path();
        let _ = storage::delete_file_if_exists(&path);
    }

    fn restore_from_detached_checkpoint(&mut self) -> bool {
        let path = storage::get_detached_runtime_path();
        if !storage::file_exists(&path) {
            return false;
        }

        let checkpoint: DetachedRuntimeCheckpoint = match storage::read_json(&path) {
            Ok(value) => value,
            Err(_) => {
                let _ = storage::delete_file_if_exists(&path);
                return false;
            }
        };
'''
new_checkpoint_write = '''        if let Some(database_path) = self.sqlite_database_path.clone() {
            let result = sqlite::save_tui_checkpoint(
                &database_path,
                checkpoint.detached_at_utc,
                checkpoint.simulation_time_utc,
                &checkpoint,
            );
            self.record_storage_result(result);
        } else {
            let path = storage::get_detached_runtime_path();
            if let Err(error) = storage::write_json_atomic(&path, &checkpoint) {
                self.record_storage_result::<()>(Err(error));
            }
        }
    }

    fn clear_detached_checkpoint(&mut self) {
        if let Some(database_path) = self.sqlite_database_path.clone() {
            let result = sqlite::clear_tui_checkpoint(&database_path);
            self.record_storage_result(result);
        } else {
            let path = storage::get_detached_runtime_path();
            if let Err(error) = storage::delete_file_if_exists(&path) {
                self.record_storage_result::<()>(Err(error));
            }
        }
    }

    fn restore_from_detached_checkpoint(&mut self) -> bool {
        let checkpoint: DetachedRuntimeCheckpoint = if let Some(database_path) =
            self.sqlite_database_path.clone()
        {
            match sqlite::load_tui_checkpoint(&database_path) {
                Ok(Some(value)) => value,
                Ok(None) => return false,
                Err(error) => {
                    self.record_storage_result::<()>(Err(error));
                    return false;
                }
            }
        } else {
            let path = storage::get_detached_runtime_path();
            if !storage::file_exists(&path) {
                return false;
            }
            match storage::read_json(&path) {
                Ok(value) => value,
                Err(error) => {
                    self.record_storage_result::<()>(Err(error));
                    return false;
                }
            }
        };
'''
text = replace_once(text, old_checkpoint_write, new_checkpoint_write, "checkpoint routing")
text = text.replace(
    '''        if checkpoint.schema_version != 1 {
            let _ = storage::delete_file_if_exists(&path);
            return false;
        }
''',
    '''        if checkpoint.schema_version != 1 {
            self.record_storage_result::<()>(Err(format!(
                "unsupported detached checkpoint schema {}",
                checkpoint.schema_version
            )));
            return false;
        }
''',
    1,
)

# Replace run_ui with fail-closed setup/cleanup.
run_start = text.index("pub fn run_ui() -> Result<(), io::Error> {")
new_run = r'''pub fn run_ui() -> Result<(), io::Error> {
    let (width, height) = crossterm::terminal::size()?;
    let mut app = App::new(width, height).map_err(io::Error::other)?;

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let physics_rate = Duration::from_millis(TIME_SETTINGS.physics_ms);
    let tick_rate = Duration::from_millis(TIME_SETTINGS.tick_ms);
    let render_rate = Duration::from_millis(1000 / TIME_SETTINGS.target_fps);
    let save_rate = Duration::from_secs(RUNTIME_LOOP_SETTINGS.autosave_secs);
    let mut last_simulation_update = Instant::now();
    let mut last_render = Instant::now();
    let mut last_save = Instant::now();
    let mut runtime_error = None;

    loop {
        if let Some(error) = app.storage_error.take() {
            runtime_error = Some(error);
            break;
        }

        let now = Instant::now();
        let wall_delta = now.saturating_duration_since(last_simulation_update);
        last_simulation_update = now;
        app.advance_runtime(wall_delta, tick_rate, physics_rate);

        if last_save.elapsed() >= save_rate {
            app.persist_sessions();
            app.persist_sand_state();
            app.persist_daily_sand_snapshot();
            last_save = Instant::now();
        }

        app.refresh_keymap_if_changed();

        if last_render.elapsed() >= render_rate && app.render_needed {
            terminal.draw(|f| {
                app.draw_frame(f);
            })?;
            app.render_needed = false;
            last_render = Instant::now();
        }

        if event::poll(Duration::from_millis(RUNTIME_LOOP_SETTINGS.input_poll_ms))?
            && let Event::Key(key) = event::read()?
        {
            if app.handle_key(key) {
                break;
            }
            if app.detach_requested {
                break;
            }
        }
    }

    if runtime_error.is_none() {
        if app.detach_requested {
            app.persist_sessions();
            app.persist_sand_state();
            app.persist_daily_sand_snapshot();
            app.persist_detached_checkpoint();
        } else {
            app.end_active_session_now();
            app.persist_sessions();
            app.persist_sand_state();
            app.persist_daily_sand_snapshot();
            app.clear_detached_checkpoint();
        }
        runtime_error = app.storage_error.take();
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Some(error) = runtime_error {
        return Err(io::Error::other(error));
    }
    Ok(())
}
'''
text = text[:run_start] + new_run
path.write_text(text)

# Replace category persistence methods with backend-aware fail-closed implementations.
path = Path("src/app/category_state.rs")
text = path.read_text()
text = text.replace("    storage,\n};", "    sqlite, storage,\n};", 1)
text = text.replace("use super::App;", "use super::App;\nuse chrono::NaiveDate;", 1)
start = text.index("    pub(super) fn persist_categories")
end = text.index("    pub(super) fn sync_modal_description_from_selection")
new_persistence = r'''    pub(super) fn persist_categories(&mut self) {
        let categories = self.time_tracker.categories_for_storage();
        if let Some(database_path) = self.sqlite_database_path.clone() {
            let result = sqlite::sync_tui_categories(
                &database_path,
                &categories,
                self.time_tracker.active_category_id(),
            );
            if let Some(archived) = self.record_storage_result(result) {
                self.archived_categories = archived;
            }
        } else {
            let path = storage::get_categories_path();
            if let Err(error) = storage::save_categories_to_csv(&path, &categories) {
                self.record_storage_result::<()>(Err(error));
            }
        }
    }

    pub(super) fn persist_sessions(&mut self) {
        if let Some(database_path) = self.sqlite_database_path.clone() {
            let result = sqlite::sync_tui_sessions(&database_path, &self.time_tracker.sessions);
            self.record_storage_result(result);
        } else {
            let categories = self.time_tracker.categories_for_storage();
            let path = storage::get_time_log_path();
            if let Err(error) =
                storage::save_sessions_to_csv(&path, &self.time_tracker.sessions, &categories)
            {
                self.record_storage_result::<()>(Err(error));
            }
        }
    }

    pub(super) fn persist_sand_state(&mut self) {
        let state = self.sand_engine.snapshot_state();
        if let Some(database_path) = self.sqlite_database_path.clone() {
            let result = sqlite::save_tui_sand_state(&database_path, &state);
            self.record_storage_result(result);
        } else {
            let path = storage::get_sand_state_path();
            if let Err(error) = storage::save_sand_state(&path, &state) {
                self.record_storage_result::<()>(Err(error));
            }
        }
    }

    pub(super) fn persist_daily_sand_snapshot(&mut self) {
        let mut state = self.sand_engine.snapshot_state();
        if is_drift_category_id(self.time_tracker.active_category_id()) {
            state.grains.retain(|grain| grain.category_id != 0);
        }
        self.save_daily_sand_snapshot(operational_day_key_now(), &state);
    }

    pub(super) fn persist_category_tags(&mut self) {
        if let Some(database_path) = self.sqlite_database_path.clone() {
            let result = sqlite::sync_tui_category_tags(&database_path, &self.category_tags);
            self.record_storage_result(result);
        } else {
            let path = storage::get_category_tags_path();
            if let Err(error) = storage::save_category_tags(&path, &self.category_tags) {
                self.record_storage_result::<()>(Err(error));
            }
        }
    }

    pub(super) fn restore_sand_state(&mut self) {
        let state = if let Some(database_path) = self.sqlite_database_path.clone() {
            match sqlite::load_tui_sand_state(&database_path) {
                Ok(value) => value,
                Err(error) => {
                    self.record_storage_result::<()>(Err(error));
                    return;
                }
            }
        } else {
            storage::load_sand_state(&storage::get_sand_state_path())
        };
        let Some(state) = state else {
            return;
        };

        let valid_category_ids = self
            .time_tracker
            .categories_for_storage()
            .into_iter()
            .chain(self.archived_categories.iter().cloned())
            .map(|category| category.id)
            .collect::<std::collections::HashSet<_>>();
        self.sand_engine.restore_state(&state, &valid_category_ids);
    }

    pub(super) fn load_daily_sand_snapshot(&mut self, day: NaiveDate) -> Option<crate::sand::SandState> {
        if let Some(database_path) = self.sqlite_database_path.clone() {
            let day = day.format("%Y-%m-%d").to_string();
            match sqlite::load_tui_daily_snapshot(&database_path, &day) {
                Ok(value) => value,
                Err(error) => {
                    self.record_storage_result::<()>(Err(error));
                    None
                }
            }
        } else {
            storage::load_sand_state(&storage::get_sand_history_path_for_day(day))
        }
    }

    pub(super) fn save_daily_sand_snapshot(
        &mut self,
        day: NaiveDate,
        state: &crate::sand::SandState,
    ) {
        if let Some(database_path) = self.sqlite_database_path.clone() {
            let day = day.format("%Y-%m-%d").to_string();
            let result = sqlite::save_tui_daily_snapshot(&database_path, &day, state);
            self.record_storage_result(result);
        } else {
            let path = storage::get_sand_history_path_for_day(day);
            if let Err(error) = storage::save_sand_state(&path, state) {
                self.record_storage_result::<()>(Err(error));
            }
        }
    }

    pub(super) fn delete_daily_sand_snapshot(&mut self, day: NaiveDate) {
        if let Some(database_path) = self.sqlite_database_path.clone() {
            let day = day.format("%Y-%m-%d").to_string();
            let result = sqlite::delete_tui_daily_snapshot(&database_path, &day);
            self.record_storage_result(result);
        } else {
            let path = storage::get_sand_history_path_for_day(day);
            if let Err(error) = storage::delete_file_if_exists(&path) {
                self.record_storage_result::<()>(Err(error));
            }
        }
    }

'''
text = text[:start] + new_persistence + text[end:]
path.write_text(text)

# Reports use active plus archived category identity and SQLite snapshots.
path = Path("src/app/report_state.rs")
text = path.read_text()
text = text.replace(
    "use crate::{\n    sand::{SandEngine, SandState, SandStateGrain},\n    storage,\n};",
    "use crate::sand::{SandEngine, SandState, SandStateGrain};",
    1,
)
text = replace_once(
    text,
    "    pub(super) fn category_color_for_id(&self, category_id: CategoryId) -> Color {\n        self.time_tracker\n            .category_color_by_id(category_id)\n            .unwrap_or(Color::White)\n    }",
    "    fn report_categories(&self) -> Vec<Category> {\n"
    "        let mut categories = self.time_tracker.categories_for_storage();\n"
    "        categories.extend(self.archived_categories.iter().cloned());\n"
    "        categories\n"
    "    }\n\n"
    "    pub(super) fn category_color_for_id(&self, category_id: CategoryId) -> Color {\n"
    "        self.time_tracker\n"
    "            .category_color_by_id(category_id)\n"
    "            .or_else(|| {\n"
    "                self.archived_categories\n"
    "                    .iter()\n"
    "                    .find(|category| category.id == category_id)\n"
    "                    .map(|category| category.color)\n"
    "            })\n"
    "            .unwrap_or(Color::White)\n"
    "    }",
    "report categories",
)
text = text.replace("let categories = self.time_tracker.categories_for_storage();", "let categories = self.report_categories();", 2)
text = text.replace("        categories: &[Category],\n    )", "        _categories: &[Category],\n    )", 1)
text = replace_once(
    text,
    "        let valid_category_ids: HashSet<CategoryId> =\n            categories.iter().map(|category| category.id).collect();",
    "        let categories = self.report_categories();\n        let valid_category_ids: HashSet<CategoryId> =\n            categories.iter().map(|category| category.id).collect();",
    "report snapshot categories",
)
text = text.replace(
    '''        let path = storage::get_sand_history_path_for_day(end_day);
        self.report_snapshot_state = storage::load_sand_state(&path)
            .or_else(|| self.synthetic_snapshot_from_time_log(end_day));''',
    '''        self.report_snapshot_state = self
            .load_daily_sand_snapshot(end_day)
            .or_else(|| self.synthetic_snapshot_from_time_log(end_day));''',
    1,
)
text = text.replace(
    '''        let path = storage::get_sand_history_path_for_day(end_day);

        if let Some(state) = self.synthetic_snapshot_from_time_log(end_day) {
            let _ = storage::save_sand_state(&path, &state);
            self.report_snapshot_state = Some(state);
        } else {
            let _ = storage::delete_file_if_exists(&path);
            self.report_snapshot_state = None;
        }''',
    '''        if let Some(state) = self.synthetic_snapshot_from_time_log(end_day) {
            self.save_daily_sand_snapshot(end_day, &state);
            self.report_snapshot_state = Some(state);
        } else {
            self.delete_daily_sand_snapshot(end_day);
            self.report_snapshot_state = None;
        }''',
    1,
)
path.write_text(text)

# Existing SQLITE-006 integration test no longer expects a TUI lockout.
path = Path("tests/sqlite_cli_authority.rs")
text = path.read_text()
text = text.replace(
    "fn activated_cli_uses_sqlite_without_legacy_dual_writes_and_blocks_tui()",
    "fn activated_cli_uses_sqlite_without_legacy_dual_writes()",
    1,
)
text = text.replace(
    '''
    let tui = profile.run(&[]);
    assert!(!tui.status.success());
    assert!(stderr(&tui).contains("legacy-backed TUI is disabled"));
''',
    "\n",
    1,
)
path.write_text(text)
