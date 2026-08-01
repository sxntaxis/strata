from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    if old not in text:
        raise SystemExit(f"missing {label} anchor")
    return text.replace(old, new, 1)


path = Path("src/app.rs")
text = path.read_text()
text = replace_once(
    text,
    '''        if let Some(database_path) = self.sqlite_database_path.clone() {
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
        self.begin_active_session_at(started_at_utc);''',
    '''        if let Some(database_path) = self.sqlite_database_path.clone()
            && self
                .record_storage_result(sqlite::reset_tui_active_session(
                    &database_path,
                    started_at_utc,
                ))
                .is_none()
        {
            return;
        }
        self.begin_active_session_at(started_at_utc);''',
    "active reset control flow",
)
text = replace_once(
    text,
    '''            if self
                .record_storage_result(sqlite::finish_tui_active_session(
                    &database_path,
                    clamped_end,
                    &operational_day,
                    elapsed,
                ))
                .is_none()
            {
                return None;
            }''',
    '''            self.record_storage_result(sqlite::finish_tui_active_session(
                &database_path,
                clamped_end,
                &operational_day,
                elapsed,
            ))?;''',
    "active completion option flow",
)
path.write_text(text)
