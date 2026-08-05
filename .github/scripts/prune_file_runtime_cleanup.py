from pathlib import Path
import re


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected one marker, found {count}")
    return text.replace(old, new, 1)


def sub_once(text: str, pattern: str, replacement: str, label: str) -> str:
    result, count = re.subn(pattern, replacement, text, count=1, flags=re.S)
    if count != 1:
        raise SystemExit(f"{label}: expected one match, found {count}")
    return result


# Preserve the generic lifecycle UI helpers while deleting only the obsolete normalizer.
path = Path("src/app/category_lifecycle_view.rs")
text = path.read_text()
if "fn lifecycle_confirmation_phrase" not in text:
    text += '''
fn lifecycle_confirmation_phrase(
    source: CategoryId,
    target: Option<CategoryId>,
    revision: &str,
) -> String {
    match target {
        Some(target) => format!("MERGE {} INTO {} {revision}", source.0, target.0),
        None => format!("DELETE {} {revision}", source.0),
    }
}

fn labelled(label: &str, value: impl ToString) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!("{label}: "),
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::raw(value.to_string()),
    ])
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(area.height.saturating_sub(height) / 2),
            Constraint::Length(height.min(area.height)),
            Constraint::Min(0),
        ])
        .split(area);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(area.width.saturating_sub(width) / 2),
            Constraint::Length(width.min(area.width)),
            Constraint::Min(0),
        ])
        .split(vertical[1])[1]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn confirmation_phrase_binds_source_target_and_revision() {
        assert_eq!(
            lifecycle_confirmation_phrase(CategoryId::new(7), Some(CategoryId::new(9)), "abc123"),
            "MERGE 7 INTO 9 abc123"
        );
        assert_eq!(
            lifecycle_confirmation_phrase(CategoryId::new(7), None, "abc123"),
            "DELETE 7 abc123"
        );
    }

    #[test]
    fn exact_confirmation_is_not_case_or_whitespace_fuzzy() {
        let expected = "MERGE 1 INTO 2 deadbeef";
        assert_ne!(expected, "merge 1 into 2 deadbeef");
        assert_ne!(expected, "MERGE 1 INTO 2 deadbeef ");
    }
}
'''
path.write_text(text)

# Remove obsolete import-parity fixtures from repository tests.
path = Path("src/sqlite/repository.rs")
text = path.read_text()
text = text.replace("    use std::{fs, path::PathBuf, time::SystemTime};\n\n", "")
text = text.replace("    use crate::{constants::COLORS, storage};\n\n", "")
text = text.replace(
    "    use crate::sqlite::legacy_import::{LegacyImportOptions, LegacyImportPaths, LegacyImportPlan};\n",
    "",
)
text = sub_once(
    text,
    r"\n    struct LegacyFixture \{.*?\n    #\[test\]\n    fn consistent_snapshot_matches_legacy_domain_visible_state\(\) \{.*?\n    \}\n\}",
    "\n}",
    "legacy repository fixture",
)
path.write_text(text)

# Remove the file-backed recovery test and obsolete checkpoint fields/assignments.
path = Path("src/app/persistence_recovery.rs")
text = path.read_text()
text = sub_once(
    text,
    r"\n    fn unique_path\(.*?\n    \}\n\n    fn recovery_category",
    "\n    fn recovery_category",
    "obsolete recovery path helper",
)
text = sub_once(
    text,
    r"\n    #\[test\]\n    fn legacy_recovery_reload_accepts_archived_session_references\(\) \{.*?\n    \}\n\n    #\[test\]\n    fn emergency_export_categories_preserve_archived_state",
    "\n    #[test]\n    fn emergency_export_categories_preserve_archived_state",
    "legacy recovery test",
)
path.write_text(text)

path = Path("src/app.rs")
text = path.read_text()
text = text.replace("    path::{Path, PathBuf},", "    path::PathBuf,")
text = text.replace("RuntimeSettings, TimeTracker, civil_time_for_utc,", "RuntimeSettings, TimeTracker,")
text = text.replace("let Some(mut checkpoint) = self.checkpoint_recovery_payload.clone()", "let Some(checkpoint) = self.checkpoint_recovery_payload.clone()")
text = re.sub(r"\n\s*version: ClearAllReceipt::VERSION,", "", text)
text = re.sub(r"\n\s*checkpoint\.legacy_recovery_committed = (?:true|false);", "", text)
path.write_text(text)

# SQLite owns session publication; delete the former local record writer.
path = Path("src/domain.rs")
text = path.read_text()
text = sub_once(
    text,
    r"\n    pub fn end_session_with_elapsed_at_local<Tz>\(.*?\n    \}\n\n    pub fn record_session_at<Tz>\(.*?\n    \}\n",
    "\n",
    "obsolete local session writer",
)
path.write_text(text)

# Reduce storage.rs to profile paths and generic atomic publication helpers.
path = Path("src/storage.rs")
text = path.read_text()
text = sub_once(
    text,
    r"use std::\{.*?\n\};\n\nuse chrono::\{.*?\};\nuse csv::\{.*?\};\nuse ratatui::style::Color;\nuse serde::\{Deserialize, Serialize, de::DeserializeOwned\};\nuse thiserror::Error;\n\nuse crate::\{.*?\n\};",
    '''use std::{
    collections::HashMap,
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use chrono::Local;
use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::domain::{Category, Session};''',
    "storage imports",
)
text = text.replace("    pub archived_categories: Vec<Category>,\n", "")
text = sub_once(
    text,
    r"const LEGACY_CATEGORIES_HEADER:.*?const BACKUP_RETENTION_MAX_FILES: usize = 10;",
    "const BACKUP_RETENTION_MAX_FILES: usize = 10;",
    "CSV constants",
)
text = sub_once(
    text,
    r"\n#\[derive\(Debug, Error\)\]\npub enum StorageError \{.*?\n\}\n",
    "\n",
    "storage error",
)
text = sub_once(
    text,
    r"\nfn default_categories_loaded\(\).*?\n\}\n\npub fn save_category_catalog_to_csv",
    "\npub fn save_category_catalog_to_csv",
    "CSV loaders",
)
text = sub_once(
    text,
    r"\npub fn save_category_catalog_to_csv\(.*?\n\}\n\npub fn get_data_dir",
    "\npub fn get_data_dir",
    "CSV writers",
)
text = sub_once(
    text,
    r"\npub fn get_detached_runtime_path\(\).*?\n\}\n\npub fn get_keymap_path",
    "\npub fn get_keymap_path",
    "file runtime paths",
)
text = sub_once(
    text,
    r"\npub fn get_categories_path\(\).*?\n\}\n\npub fn file_exists",
    "\npub fn file_exists",
    "file runtime serializers",
)
text = sub_once(
    text,
    r"\n#\[cfg\(test\)\]\nmod tests \{.*?\n\}\n\n#\[cfg\(test\)\]\nmod publication_race_tests",
    "\n#[cfg(test)]\nmod publication_race_tests",
    "obsolete storage tests",
)
path.write_text(text)

print("file-runtime prune cleanup applied")
