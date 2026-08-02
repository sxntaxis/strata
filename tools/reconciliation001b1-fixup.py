from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    target = Path(path)
    text = target.read_text()
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"expected one match in {path}, found {count}: {old[:140]!r}")
    target.write_text(text.replace(old, new, 1))


replace_once(
    "src/app/category_state.rs",
    '''        }
        self.refresh_active_runtime_checkpoint();
    }

    pub(super) fn persist_sessions(&mut self) {
''',
    '''        }
    }

    pub(super) fn persist_sessions(&mut self) {
''',
)

replace_once(
    "src/app.rs",
    '''            self.reload_sqlite_sessions();
            self.persist_categories();
            self.sync_drift_idle_state();
            self.refresh_active_runtime_checkpoint();
            return !self.has_persistence_recovery();
''',
    '''            self.reload_sqlite_sessions();
            self.persist_categories();
            self.sync_drift_idle_state();
            self.refresh_active_runtime_checkpoint();
            return !self.has_persistence_recovery();
''',
)

# Refresh only when the persisted description belongs to the current active category.
path = Path("src/app/event_handlers.rs")
text = path.read_text()
old = '''                        if self.time_tracker.set_category_description_by_index(
                            self.selected_index,
                            self.modal_description.clone(),
                        ) {
                            self.persist_categories();
                            if self.has_persistence_recovery() {
                                self.render_needed = true;
                                return true;
                            }
                        }
'''
new = '''                        if self.time_tracker.set_category_description_by_index(
                            self.selected_index,
                            self.modal_description.clone(),
                        ) {
                            let description_is_active =
                                self.time_tracker.active_category_index() == Some(self.selected_index);
                            self.persist_categories();
                            if description_is_active {
                                self.refresh_active_runtime_checkpoint();
                            }
                            if self.has_persistence_recovery() {
                                self.render_needed = true;
                                return true;
                            }
                        }
'''
if text.count(old) != 1:
    raise SystemExit("modal description persistence block not found")
path.write_text(text.replace(old, new, 1))

for temporary in [
    ".github/workflows/reconciliation001b1-fixup.yml",
    "tools/reconciliation001b1-fixup.py",
]:
    Path(temporary).unlink(missing_ok=True)
