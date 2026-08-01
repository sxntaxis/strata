from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    if old not in text:
        raise SystemExit(f"missing {label} anchor")
    return text.replace(old, new, 1)


# Domain support for restoring an archived category by stable identity.
path = Path("src/domain.rs")
text = path.read_text()
if "pub fn restore_category(&mut self, mut category: Category)" not in text:
    anchor = '''    pub fn delete_by_index(&mut self, index: usize) -> Option<CategoryId> {'''
    method = '''    pub fn restore_category(&mut self, mut category: Category) -> bool {
        let trimmed = category.name.trim();
        if category.id == DRIFT_CATEGORY_ID || trimmed.is_empty() {
            return false;
        }
        if self.by_id.contains_key(&category.id)
            || self
                .order
                .iter()
                .filter_map(|id| self.by_id.get(id))
                .any(|existing| existing.name.eq_ignore_ascii_case(trimmed))
        {
            return false;
        }

        category.name = trimmed.to_string();
        self.next_id = self.next_id.max(category.id.0.saturating_add(1));
        self.order.push(category.id);
        self.by_id.insert(category.id, category);
        true
    }

'''
    text = replace_once(text, anchor, method + anchor, "category restore store")
if "pub fn restore_category(&mut self, category: Category) -> bool" not in text:
    anchor = '''    pub fn delete_category(&mut self, index: usize) -> bool {'''
    method = '''    pub fn restore_category(&mut self, category: Category) -> bool {
        self.category_store.restore_category(category)
    }

'''
    text = replace_once(text, anchor, method + anchor, "category restore tracker")
if "test_restore_category_reuses_stable_identity" not in text:
    anchor = '''    #[test]
    fn test_category_id_stability_on_reorder() {'''
    test = '''    #[test]
    fn test_restore_category_reuses_stable_identity() {
        let mut tracker = TimeTracker::new();
        let id = tracker
            .add_category("Work".to_string(), "focus".to_string(), Some(0))
            .expect("category should be added");
        let archived = tracker
            .category_by_id(id)
            .cloned()
            .expect("category should exist");
        assert!(tracker.delete_category(1));
        assert!(tracker.restore_category(archived));
        assert_eq!(tracker.category_id_by_name("Work"), Some(id));
    }

'''
    text = replace_once(text, anchor, test + anchor, "category restore test")
path.write_text(text)

# SQLite adapter: snapshot syncs become update-only; destructive changes become explicit commands.
path = Path("src/sqlite/tui_runtime.rs")
text = path.read_text()
if "pub(crate) fn archive_category(" not in text:
    anchor = '''pub(crate) fn sync_category_tags('''
    methods = '''pub(crate) fn archive_category(
    database_path: &Path,
    category_id: CategoryId,
) -> Result<(), String> {
    let mut repository = open_cli_repository(database_path)?;
    let category_id = as_i64(category_id.0, "category ID")?;
    let active_category_id = repository
        .active_session()
        .map_err(|error| error.to_string())?
        .map(|active| active.category_id);
    if active_category_id == Some(category_id) {
        return Err("the active category cannot be archived".to_string());
    }
    repository
        .archive_category(category_id, &timestamp(Utc::now()))
        .map_err(|error| error.to_string())?;
    Ok(())
}

'''
    text = replace_once(text, anchor, methods + anchor, "explicit category archive")
# Remove implicit archive loop from category synchronization.
implicit_archive = '''    let now = timestamp(Utc::now());
    let mut statement = transaction
        .prepare("SELECT id FROM categories WHERE id <> 0 AND archived_at_utc IS NULL")
        .map_err(|error| error.to_string())?;
    let existing_ids = statement
        .query_map([], |row| row.get::<_, i64>(0))
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    drop(statement);
    for id in existing_ids {
        if !current_ids.contains(&id) {
            transaction
                .execute(
                    "UPDATE categories SET archived_at_utc = ?1 WHERE id = ?2",
                    params![now, id],
                )
                .map_err(|error| error.to_string())?;
            transaction
                .execute(
                    "DELETE FROM category_tags WHERE category_id = ?1",
                    params![id],
                )
                .map_err(|error| error.to_string())?;
        }
    }

'''
text = text.replace(implicit_archive, "", 1)
# Narrow tag synchronization to categories known by the current TUI state.
old_tag_signature = '''pub(crate) fn sync_category_tags(
    database_path: &Path,
    tags: &CategoryTagsState,
) -> Result<(), String> {
    let mut repository = open_cli_repository(database_path)?;
    let active_ids = repository
        .list_categories(false)
        .map_err(|error| error.to_string())?
        .into_iter()
        .map(|category| category.id)
        .collect::<BTreeSet<_>>();
    for category_id in active_ids {'''
new_tag_signature = '''pub(crate) fn sync_category_tags(
    database_path: &Path,
    tags: &CategoryTagsState,
    category_ids: &[CategoryId],
) -> Result<(), String> {
    let mut repository = open_cli_repository(database_path)?;
    for category_id in category_ids {
        let category_id = as_i64(category_id.0, "category ID")?;'''
text = text.replace(old_tag_signature, new_tag_signature, 1)
# Make session autosave verification/update-only.
old_desired = '''    let desired_ids = sessions
        .iter()
        .map(|session| i64::try_from(session.id).map_err(|_| "session ID is too large".to_string()))
        .collect::<Result<BTreeSet<_>, _>>()?;
'''
text = text.replace(old_desired, "", 1)
old_delete_loop = '''    for id in stored.keys() {
        if !desired_ids.contains(id) {
            transaction
                .execute("DELETE FROM sessions WHERE id = ?1", params![id])
                .map_err(|error| error.to_string())?;
        }
    }
'''
text = text.replace(old_delete_loop, "", 1)
if "pub(crate) fn update_session_description(" not in text:
    anchor = '''pub(crate) fn save_sand_state('''
    methods = '''pub(crate) fn update_session_description(
    database_path: &Path,
    session_id: usize,
    description: &str,
) -> Result<(), String> {
    let repository = open_cli_repository(database_path)?;
    let session_id = i64::try_from(session_id)
        .map_err(|_| "session ID is too large".to_string())?;
    let changed = repository
        .connection
        .execute(
            "UPDATE sessions SET description = ?1 WHERE id = ?2",
            params![description, session_id],
        )
        .map_err(|error| error.to_string())?;
    if changed != 1 {
        return Err(format!("SQLite session {session_id} does not exist"));
    }
    Ok(())
}

pub(crate) fn delete_session(database_path: &Path, session_id: usize) -> Result<(), String> {
    let repository = open_cli_repository(database_path)?;
    let session_id = i64::try_from(session_id)
        .map_err(|_| "session ID is too large".to_string())?;
    let changed = repository
        .connection
        .execute("DELETE FROM sessions WHERE id = ?1", params![session_id])
        .map_err(|error| error.to_string())?;
    if changed != 1 {
        return Err(format!("SQLite session {session_id} does not exist"));
    }
    Ok(())
}

pub(crate) fn delete_drift_sessions_for_day(
    database_path: &Path,
    operational_day: &str,
) -> Result<(), String> {
    let repository = open_cli_repository(database_path)?;
    repository
        .connection
        .execute(
            "DELETE FROM sessions WHERE category_id = 0 AND operational_day = ?1",
            params![operational_day],
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

'''
    text = replace_once(text, anchor, methods + anchor, "explicit session mutations")
# Strengthen category test to prove unknown active rows survive sync and archived identity restores.
old_category_test_tail = '''        assert_eq!(archived.len(), 1);
        let reloaded = load_state(&path).unwrap();
        assert_eq!(reloaded.loaded_categories.categories[1].name, "Rest");
        assert_eq!(reloaded.archived_categories[0].name, "Work");
        std::fs::remove_file(path).ok();'''
new_category_test_tail = '''        assert_eq!(archived.len(), 0, "sync alone must not archive absent rows");
        archive_category(&path, CategoryId::new(1)).unwrap();
        let mut reloaded = load_state(&path).unwrap();
        assert_eq!(reloaded.loaded_categories.categories[1].name, "Rest");
        assert_eq!(reloaded.archived_categories[0].name, "Work");
        let restored = reloaded.archived_categories.remove(0);
        reloaded.loaded_categories.categories.push(restored);
        sync_categories(
            &path,
            &reloaded.loaded_categories.categories,
            CategoryId::new(0),
        )
        .unwrap();
        let restored = load_state(&path).unwrap();
        assert_eq!(restored.loaded_categories.categories[2].id, CategoryId::new(1));
        assert!(restored.archived_categories.is_empty());
        std::fs::remove_file(path).ok();'''
text = text.replace(old_category_test_tail, new_category_test_tail, 1)
# Strengthen session test with concurrent-row survival and explicit mutations.
old_session_test_tail = '''        assert_eq!(row.0, "preserved-project");
        assert_eq!(row.1, "edited");
        assert_eq!(row.2, "2026-08-01T12:00:00Z");
        std::fs::remove_file(path).ok();'''
new_session_test_tail = '''        assert_eq!(row.0, "preserved-project");
        assert_eq!(row.1, "edited");
        assert_eq!(row.2, "2026-08-01T12:00:00Z");

        repository
            .connection
            .execute(
                "INSERT INTO sessions (
                    id, stable_id, project, category_id, description, started_at_utc,
                    ended_at_utc, operational_day, elapsed_seconds, source
                 ) VALUES (8, 'concurrent', 'external-project', 1, 'external',
                    '2026-08-01T14:00:00Z', '2026-08-01T15:00:00Z',
                    '2026-08-01', 3600, 'cli-runtime')",
                [],
            )
            .unwrap();
        drop(repository);
        sync_sessions(&path, &state.loaded_sessions.sessions).unwrap();
        update_session_description(&path, 7, "explicit-edit").unwrap();
        let repository = open_cli_repository(&path).unwrap();
        let preserved: (String, String, String, String) = repository
            .connection
            .query_row(
                "SELECT project, description, started_at_utc, source FROM sessions WHERE id = 7",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .unwrap();
        assert_eq!(preserved.0, "preserved-project");
        assert_eq!(preserved.1, "explicit-edit");
        assert_eq!(preserved.2, "2026-08-01T12:00:00Z");
        assert_eq!(preserved.3, "cli-runtime");
        let concurrent_count: i64 = repository
            .connection
            .query_row("SELECT count(*) FROM sessions WHERE id = 8", [], |row| row.get(0))
            .unwrap();
        assert_eq!(concurrent_count, 1);
        drop(repository);
        delete_session(&path, 7).unwrap();
        let repository = open_cli_repository(&path).unwrap();
        let remaining: i64 = repository
            .connection
            .query_row("SELECT count(*) FROM sessions", [], |row| row.get(0))
            .unwrap();
        assert_eq!(remaining, 1, "explicit deletion must not remove concurrent rows");
        std::fs::remove_file(path).ok();'''
text = text.replace(old_session_test_tail, new_session_test_tail, 1)
path.write_text(text)

# Re-export explicit mutation commands.
path = Path("src/sqlite.rs")
text = path.read_text()
if "archive_category as archive_tui_category" not in text:
    text = replace_once(
        text,
        '''pub(crate) use tui_runtime::{
    clear_checkpoint as clear_tui_checkpoint,''',
        '''pub(crate) use tui_runtime::{
    archive_category as archive_tui_category, clear_checkpoint as clear_tui_checkpoint,
    delete_drift_sessions_for_day as delete_tui_drift_sessions_for_day,
    delete_session as delete_tui_session,''',
        "explicit TUI mutation exports",
    )
    text = text.replace(
        "    sync_sessions as sync_tui_sessions,\n};",
        "    sync_sessions as sync_tui_sessions,\n"
        "    update_session_description as update_tui_session_description,\n};",
        1,
    )
path.write_text(text)

# App startup uses strict legacy parsers now that initialization can fail closed.
path = Path("src/app.rs")
text = path.read_text()
text = text.replace(
    '''                let loaded_categories = storage::load_categories_from_csv(&categories_path);
                let loaded_sessions =
                    storage::load_sessions_from_csv(&sessions_path, &loaded_categories.categories);''',
    '''                let loaded_categories = storage::try_load_categories_from_csv(&categories_path)
                    .map_err(|error| error.to_string())?;
                let loaded_sessions = storage::try_load_sessions_from_csv(
                    &sessions_path,
                    &loaded_categories.categories,
                )
                .map_err(|error| error.to_string())?;''',
    1,
)
# Clear-drift is an explicit destructive repository operation in SQLite mode.
old_clear = '''                let scheduled_local = scheduled_at_utc.with_timezone(&Local);
                let scheduled_day = operational_day_key_for_local(&scheduled_local);
                self.time_tracker
                    .clear_drift_sessions_for_day(scheduled_day);

                if is_drift_category_id(self.time_tracker.active_category_id()) {'''
new_clear = '''                let scheduled_local = scheduled_at_utc.with_timezone(&Local);
                let scheduled_day = operational_day_key_for_local(&scheduled_local);
                if let Some(database_path) = self.sqlite_database_path.clone() {
                    let day = scheduled_day.format("%Y-%m-%d").to_string();
                    let result = sqlite::delete_tui_drift_sessions_for_day(&database_path, &day);
                    if self.record_storage_result(result).is_none() {
                        return;
                    }
                }
                self.time_tracker.clear_drift_sessions_for_day(scheduled_day);

                if is_drift_category_id(self.time_tracker.active_category_id()) {'''
text = text.replace(old_clear, new_clear, 1)
path.write_text(text)

# Category persistence only touches known identities; deletion and restoration are explicit.
path = Path("src/app/category_state.rs")
text = path.read_text()
text = text.replace(
    '''            let result = sqlite::sync_tui_category_tags(&database_path, &self.category_tags);''',
    '''            let category_ids = self
                .time_tracker
                .categories_for_storage()
                .into_iter()
                .map(|category| category.id)
                .collect::<Vec<_>>();
            let result = sqlite::sync_tui_category_tags(
                &database_path,
                &self.category_tags,
                &category_ids,
            );''',
    1,
)
old_add = '''    pub(super) fn add_category(&mut self) {
        if !self.new_category_name.is_empty() {
            let added = self.time_tracker.add_category(
                self.new_category_name.clone(),
                String::new(),
                Some(self.color_index),
            );
            if let Some(added_id) = added {
                self.persist_categories();
                self.switch_active_category_at(added_id, chrono::Utc::now());
                self.sync_modal_description_from_selection();
            }
        }
    }'''
new_add = '''    pub(super) fn add_category(&mut self) {
        let requested_name = self.new_category_name.trim();
        if requested_name.is_empty() {
            return;
        }

        let restored = self
            .archived_categories
            .iter()
            .position(|category| category.name.eq_ignore_ascii_case(requested_name))
            .and_then(|index| {
                let mut category = self.archived_categories[index].clone();
                category.color = COLORS[self.color_index % COLORS.len()];
                category.description.clear();
                self.time_tracker
                    .restore_category(category)
                    .then_some((index, self.archived_categories[index].id))
            });

        let added_id = if let Some((archived_index, category_id)) = restored {
            self.archived_categories.remove(archived_index);
            Some(category_id)
        } else {
            self.time_tracker.add_category(
                requested_name.to_string(),
                String::new(),
                Some(self.color_index),
            )
        };

        if let Some(added_id) = added_id {
            self.persist_categories();
            self.switch_active_category_at(added_id, chrono::Utc::now());
            self.sync_modal_description_from_selection();
        }
    }'''
text = text.replace(old_add, new_add, 1)
old_delete = '''            if self.time_tracker.delete_category(self.selected_index) {
                if let Some(category_id) = removed_id {
                    self.category_tags.tags_by_category.remove(&category_id.0);
                    self.persist_category_tags();
                }

                if self.selected_index > 0
                    && self.selected_index >= self.time_tracker.category_count()
                {
                    self.selected_index = self.time_tracker.category_count();
                }
                self.persist_categories();
                self.sync_modal_description_from_selection();
            }'''
new_delete = '''            if let Some(category_id) = removed_id
                && let Some(database_path) = self.sqlite_database_path.clone()
            {
                let result = sqlite::archive_tui_category(&database_path, category_id);
                if self.record_storage_result(result).is_none() {
                    return;
                }
            }

            if self.time_tracker.delete_category(self.selected_index) {
                if let Some(category_id) = removed_id {
                    self.category_tags.tags_by_category.remove(&category_id.0);
                    self.persist_category_tags();
                }

                if self.selected_index > 0
                    && self.selected_index >= self.time_tracker.category_count()
                {
                    self.selected_index = self.time_tracker.category_count();
                }
                self.persist_categories();
                self.sync_modal_description_from_selection();
            }'''
text = text.replace(old_delete, new_delete, 1)
path.write_text(text)

# Report mutations become explicit and database-first under SQLite authority.
path = Path("src/app/report_state.rs")
text = path.read_text()
old_report_delete = '''        if !self.time_tracker.delete_session_by_id(session_id) {
            return false;
        }

        if self.report_interval_end_day() == operational_day_key_now() {
            self.sand_engine
                .remove_category_grains(category_id, removed_seconds);
        }

        self.persist_sessions();'''
new_report_delete = '''        if let Some(database_path) = self.sqlite_database_path.clone() {
            let result = crate::sqlite::delete_tui_session(&database_path, session_id);
            if self.record_storage_result(result).is_none() {
                return false;
            }
        }
        if !self.time_tracker.delete_session_by_id(session_id) {
            return false;
        }

        if self.report_interval_end_day() == operational_day_key_now() {
            self.sand_engine
                .remove_category_grains(category_id, removed_seconds);
        }

        if self.sqlite_database_path.is_none() {
            self.persist_sessions();
        }'''
text = text.replace(old_report_delete, new_report_delete, 1)
old_report_edit = '''        if !self
            .time_tracker
            .set_session_description_by_id(session_id, description)
        {
            return false;
        }

        self.persist_sessions();
        true'''
new_report_edit = '''        if let Some(database_path) = self.sqlite_database_path.clone() {
            let result = crate::sqlite::update_tui_session_description(
                &database_path,
                session_id,
                &description,
            );
            if self.record_storage_result(result).is_none() {
                return false;
            }
        }
        if !self
            .time_tracker
            .set_session_description_by_id(session_id, description)
        {
            return false;
        }

        if self.sqlite_database_path.is_none() {
            self.persist_sessions();
        }
        true'''
text = text.replace(old_report_edit, new_report_edit, 1)
path.write_text(text)
