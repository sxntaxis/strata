from pathlib import Path

path = Path("src/app/persistence_recovery.rs")
text = path.read_text()

# Add explicit helpers so recovery reload/export behavior is directly testable.
anchor = "\nimpl App {\n"
helpers = r'''

fn load_legacy_recovery_authority(
    categories_path: &Path,
    sessions_path: &Path,
) -> Result<(storage::LoadedCategories, storage::LoadedSessions), String> {
    let categories = storage::try_load_categories_from_csv(categories_path)
        .map_err(|error| error.to_string())?;
    let mut session_categories = categories.categories.clone();
    session_categories.extend(categories.archived_categories.iter().cloned());
    let sessions = storage::try_load_sessions_from_csv(sessions_path, &session_categories)
        .map_err(|error| error.to_string())?;
    Ok((categories, sessions))
}

fn emergency_categories(
    active_categories: impl IntoIterator<Item = crate::domain::Category>,
    archived_categories: &[crate::domain::Category],
) -> Vec<EmergencyCategory> {
    active_categories
        .into_iter()
        .map(|category| EmergencyCategory {
            id: category.id.0,
            name: category.name,
            description: category.description,
            color: format!("{:?}", category.color),
            balance_effect: category.karma_effect,
            archived: false,
        })
        .chain(archived_categories.iter().cloned().map(|category| {
            EmergencyCategory {
                id: category.id.0,
                name: category.name,
                description: category.description,
                color: format!("{:?}", category.color),
                balance_effect: category.karma_effect,
                archived: true,
            }
        }))
        .collect()
}
'''
if anchor not in text:
    raise SystemExit("App impl anchor not found")
text = text.replace(anchor, helpers + anchor, 1)

old_reload = r'''        } else {
            let categories = storage::try_load_categories_from_csv(&storage::get_categories_path())
                .map_err(|error| error.to_string())?;
            let sessions = storage::try_load_sessions_from_csv(
                &storage::get_time_log_path(),
                &categories.categories,
            )
            .map_err(|error| error.to_string())?;
            self.time_tracker.apply_loaded_state(
                categories.categories,
                categories.next_category_id,
                sessions.sessions,
                sessions.next_session_id,
            );
            self.category_tags = storage::load_category_tags(&storage::get_category_tags_path());
            if let Some(state) = storage::load_sand_state(&storage::get_sand_state_path()) {
                let valid_category_ids = self
                    .time_tracker
                    .categories_for_storage()
                    .into_iter()
                    .map(|category| category.id)
                    .collect();
                self.sand_engine.restore_state(&state, &valid_category_ids);
            }
        }
'''
new_reload = r'''        } else {
            let (categories, sessions) = load_legacy_recovery_authority(
                &storage::get_categories_path(),
                &storage::get_time_log_path(),
            )?;
            let archived_categories = categories.archived_categories;
            self.time_tracker.apply_loaded_state(
                categories.categories,
                categories.next_category_id,
                sessions.sessions,
                sessions.next_session_id,
            );
            self.archived_categories = archived_categories;
            self.category_tags = storage::load_category_tags(&storage::get_category_tags_path());
            if let Some(state) = storage::load_sand_state(&storage::get_sand_state_path()) {
                let valid_category_ids = self
                    .time_tracker
                    .categories_for_storage()
                    .into_iter()
                    .chain(self.archived_categories.iter().cloned())
                    .map(|category| category.id)
                    .collect();
                self.sand_engine.restore_state(&state, &valid_category_ids);
            }
        }
'''
if old_reload not in text:
    raise SystemExit("legacy reload anchor not found")
text = text.replace(old_reload, new_reload, 1)

old_export = r'''        let categories = self
            .time_tracker
            .categories_for_storage()
            .into_iter()
            .map(|category| EmergencyCategory {
                id: category.id.0,
                name: category.name,
                description: category.description,
                color: format!("{:?}", category.color),
                balance_effect: category.karma_effect,
            })
            .collect();
'''
new_export = r'''        let categories = emergency_categories(
            self.time_tracker.categories_for_storage(),
            &self.archived_categories,
        );
'''
if old_export not in text:
    raise SystemExit("emergency category export anchor not found")
text = text.replace(old_export, new_export, 1)

# Emergency bundle schema 2 explicitly records archival state.
text = text.replace(
    "            schema_version: 1,\n            created_at_utc:",
    "            schema_version: 2,\n            created_at_utc:",
    1,
)
struct_anchor = r'''struct EmergencyCategory {
    id: u64,
    name: String,
    description: String,
    color: String,
    balance_effect: i8,
}'''
struct_new = r'''struct EmergencyCategory {
    id: u64,
    name: String,
    description: String,
    color: String,
    balance_effect: i8,
    archived: bool,
}'''
if struct_anchor not in text:
    raise SystemExit("EmergencyCategory anchor not found")
text = text.replace(struct_anchor, struct_new, 1)

# Add focused tests to the existing test module.
test_anchor = r'''    #[test]
    fn persistence_failure_classes_are_actionable() {
'''
tests = r'''    fn unique_path(label: &str, extension: &str) -> PathBuf {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "strata-{label}-{}-{stamp}.{extension}",
            std::process::id()
        ))
    }

    fn recovery_category(
        id: u64,
        name: &str,
        description: &str,
    ) -> crate::domain::Category {
        crate::domain::Category {
            id: crate::domain::CategoryId::new(id),
            name: name.to_string(),
            color: if id == 0 {
                Color::White
            } else {
                crate::constants::COLORS[((id - 1) as usize) % crate::constants::COLORS.len()]
            },
            description: description.to_string(),
            karma_effect: if id == 0 { 0 } else { 1 },
        }
    }

    #[test]
    fn legacy_recovery_reload_accepts_archived_session_references() {
        let categories_path = unique_path("recovery-archived-categories", "csv");
        let sessions_path = unique_path("recovery-archived-sessions", "csv");
        let active = vec![recovery_category(0, "idle", "")];
        let archived = vec![recovery_category(7, "Archived work", "historical")];
        storage::save_category_catalog_to_csv(&categories_path, &active, &archived).unwrap();
        let session = crate::domain::Session {
            id: 1,
            date: "2026-08-02".to_string(),
            category_id: crate::domain::CategoryId::new(7),
            project: String::new(),
            description: "completed".to_string(),
            start_time: "10:00:00".to_string(),
            end_time: "11:00:00".to_string(),
            elapsed_seconds: 3600,
            started_at_utc: Some(
                chrono::TimeZone::with_ymd_and_hms(&Utc, 2026, 8, 2, 16, 0, 0).unwrap(),
            ),
            ended_at_utc: Some(
                chrono::TimeZone::with_ymd_and_hms(&Utc, 2026, 8, 2, 17, 0, 0).unwrap(),
            ),
            operational_day_policy: Some(crate::domain::OperationalDayPolicy {
                utc_offset_seconds: -21600,
                start_minutes: 360,
            }),
        };
        let mut catalog = active.clone();
        catalog.extend(archived.iter().cloned());
        storage::save_sessions_to_csv(&sessions_path, &[session], &catalog).unwrap();

        let (loaded_categories, loaded_sessions) =
            load_legacy_recovery_authority(&categories_path, &sessions_path).unwrap();
        assert_eq!(loaded_categories.archived_categories.len(), 1);
        assert_eq!(loaded_categories.archived_categories[0].id.0, 7);
        assert_eq!(loaded_sessions.sessions.len(), 1);
        assert_eq!(loaded_sessions.sessions[0].category_id.0, 7);

        fs::remove_file(categories_path).ok();
        fs::remove_file(sessions_path).ok();
    }

    #[test]
    fn emergency_export_categories_preserve_archived_state() {
        let active = vec![
            recovery_category(0, "idle", ""),
            recovery_category(1, "Active", "current"),
        ];
        let archived = vec![recovery_category(7, "Archived", "historical")];
        let exported = emergency_categories(active, &archived);
        assert_eq!(exported.len(), 3);
        assert!(exported.iter().any(|category| category.id == 7 && category.archived));
        assert!(
            exported
                .iter()
                .filter(|category| category.id != 7)
                .all(|category| !category.archived)
        );
    }

'''
if test_anchor not in text:
    raise SystemExit("test module anchor not found")
text = text.replace(test_anchor, tests + test_anchor, 1)

path.write_text(text)
Path("tools/reconciliation001b2b-archived-recovery.py").unlink()
Path(".github/workflows/reconciliation001b2b-archived-recovery.yml").unlink()
