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
    r"\n    #\[test\]\n    fn legacy_recovery_reload_accepts_archived_session_references\(\) \{.*?\n    \}\n\n    #\[test\]\n    fn emergency_export_categories_preserve_archived_state",
    "\n    #[test]\n    fn emergency_export_categories_preserve_archived_state",
    "legacy recovery test",
)
path.write_text(text)

path = Path("src/app.rs")
text = path.read_text()
text = text.replace("    path::{Path, PathBuf},", "    path::PathBuf,")
text = text.replace("RuntimeSettings, TimeTracker, civil_time_for_utc,", "RuntimeSettings, TimeTracker,")
text = re.sub(r"\n\s*version: ClearAllReceipt::VERSION,", "", text)
text = re.sub(r"\n\s*checkpoint\.legacy_recovery_committed = (?:true|false);", "", text)
path.write_text(text)

print("file-runtime prune cleanup applied")
