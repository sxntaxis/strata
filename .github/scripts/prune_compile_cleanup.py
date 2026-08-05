from pathlib import Path
import re


def sub_once(text: str, pattern: str, replacement: str, label: str) -> str:
    result, count = re.subn(pattern, replacement, text, count=1, flags=re.S)
    if count != 1:
        raise SystemExit(f"{label}: expected one match, found {count}")
    return result


path = Path("src/domain.rs")
text = path.read_text()
if "pub fn end_session_with_elapsed_at_local" not in text:
    marker = "    pub fn get_todays_time(&self) -> usize {"
    helpers = '''    #[cfg(test)]
    pub fn end_session_with_elapsed_at_local<Tz>(
        &mut self,
        elapsed: usize,
        end_local: DateTime<Tz>,
    ) -> Option<usize>
    where
        Tz: chrono::TimeZone,
        Tz::Offset: std::fmt::Display,
    {
        self.current_session_start?;
        let category_id = self.active_category_id;
        let description = self.active_description.clone();
        if elapsed > 0 {
            self.record_session_at(category_id, &description, elapsed, end_local);
        }
        self.active_description.clear();
        self.current_session_start = None;
        Some(elapsed)
    }

    #[cfg(test)]
    pub fn record_session_at<Tz>(
        &mut self,
        category_id: CategoryId,
        description: &str,
        elapsed: usize,
        end_local: DateTime<Tz>,
    ) where
        Tz: chrono::TimeZone,
        Tz::Offset: std::fmt::Display,
    {
        if elapsed == 0 {
            return;
        }
        let end_utc = end_local.with_timezone(&Utc);
        let start_utc = end_utc - ChronoDuration::seconds(elapsed as i64);
        let start_local = end_local.clone() - ChronoDuration::seconds(elapsed as i64);
        let operational_day = operational_day_key_for_utc(end_utc)
            .format("%Y-%m-%d")
            .to_string();
        self.sessions.push(Session {
            id: self.session_id_counter,
            date: operational_day,
            category_id,
            project: String::new(),
            description: description.to_string(),
            start_time: start_local.format("%H:%M:%S").to_string(),
            end_time: end_local.format("%H:%M:%S").to_string(),
            elapsed_seconds: elapsed,
            started_at_utc: Some(start_utc),
            ended_at_utc: Some(end_utc),
            operational_day_policy: Some(OperationalDayPolicy::from_config(day_boundary_config())),
        });
        self.session_id_counter += 1;
    }

'''
    if marker not in text:
        raise SystemExit("domain test helper insertion marker missing")
    text = text.replace(marker, helpers + marker, 1)
path.write_text(text)

path = Path("src/app.rs")
text = path.read_text()
text = sub_once(
    text,
    r"\nfn stage_clear_all_active_state\(.*?\n\}\n\nfn sand_state_is_empty",
    "\nfn sand_state_is_empty",
    "obsolete clear-all active staging",
)
path.write_text(text)

path = Path("src/storage.rs")
text = path.read_text()
text = sub_once(
    text,
    r"\npub fn file_exists\(.*?\nfn unique_publication_sibling",
    "\nfn unique_publication_sibling",
    "unused JSON file helpers",
)
text = text.replace(
    "use serde::{Deserialize, Serialize, de::DeserializeOwned};",
    "use serde::{Deserialize, Serialize};",
)
path.write_text(text)

print("final SQLite-only compiler cleanup applied")
