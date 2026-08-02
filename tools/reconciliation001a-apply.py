from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    target = Path(path)
    text = target.read_text()
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"expected one match in {path}, found {count}: {old[:160]!r}")
    target.write_text(text.replace(old, new, 1))


def replace_between(path: str, start: str, end: str, replacement: str) -> None:
    target = Path(path)
    text = target.read_text()
    start_index = text.index(start)
    end_index = text.index(end, start_index)
    target.write_text(text[:start_index] + replacement + text[end_index:])


# -------------------------------------------------------------------------
# Legacy storage catalog and session integrity.
# -------------------------------------------------------------------------
path = Path("src/storage.rs")
text = path.read_text()
text = text.replace(
    "    collections::HashMap,",
    "    collections::{HashMap, HashSet},",
    1,
)
text = text.replace(
    '''pub struct LoadedCategories {
    pub categories: Vec<Category>,
    pub next_category_id: u64,
}
''',
    '''pub struct LoadedCategories {
    pub categories: Vec<Category>,
    pub archived_categories: Vec<Category>,
    pub next_category_id: u64,
}
''',
    1,
)
text = text.replace(
    '''const CATEGORIES_HEADER: [&str; 5] = ["id", "name", "description", "color_index", "karma_effect"];
''',
    '''const LEGACY_CATEGORIES_HEADER: [&str; 5] = [
    "id",
    "name",
    "description",
    "color_index",
    "karma_effect",
];
const CATEGORIES_HEADER: [&str; 6] = [
    "id",
    "name",
    "description",
    "color_index",
    "karma_effect",
    "archived",
];
''',
    1,
)
text = text.replace(
    '''        categories: vec![Category {
            id: DRIFT_CATEGORY_ID,
            name: DRIFT_CATEGORY_CONFIG_NAME.to_string(),
            color: Color::White,
            description: String::new(),
            karma_effect: 0,
        }],
        next_category_id: 1,
''',
    '''        categories: vec![Category {
            id: DRIFT_CATEGORY_ID,
            name: DRIFT_CATEGORY_CONFIG_NAME.to_string(),
            color: Color::White,
            description: String::new(),
            karma_effect: 0,
        }],
        archived_categories: Vec::new(),
        next_category_id: 1,
''',
    1,
)
path.write_text(text)

replace_between(
    "src/storage.rs",
    "pub fn try_load_categories_from_csv(path: &Path) -> Result<LoadedCategories, StorageError> {",
    "\n#[cfg(test)]\npub fn load_sessions_from_csv",
    r'''pub fn try_load_categories_from_csv(path: &Path) -> Result<LoadedCategories, StorageError> {
    if !path.exists() {
        return Ok(default_categories_loaded());
    }

    let mut reader = ReaderBuilder::new().has_headers(true).from_path(path)?;
    let headers = reader.headers()?.clone();
    let has_archived_state = csv_header_matches(&headers, &CATEGORIES_HEADER);
    if !has_archived_state && !csv_header_matches(&headers, &LEGACY_CATEGORIES_HEADER) {
        return Err(StorageError::InvalidCsvSchema {
            file: "categories.csv",
            expected: format!(
                "{} or {}",
                LEGACY_CATEGORIES_HEADER.join(","),
                CATEGORIES_HEADER.join(",")
            ),
            found: csv_header_string(&headers),
        });
    }

    let mut loaded = default_categories_loaded();
    let mut seen_ids = HashSet::from([DRIFT_CATEGORY_ID.0]);
    let mut seen_names = HashSet::from([DRIFT_CATEGORY_CONFIG_NAME.to_string()]);

    for (index, record) in reader.records().enumerate() {
        let row = index + 2;
        let record = record?;
        let id_raw = record.get(0).unwrap_or_default();
        let id = id_raw.parse::<u64>().map_err(|error| StorageError::InvalidCsvData {
            file: "categories.csv",
            row,
            message: format!("invalid category ID '{id_raw}': {error}"),
        })?;
        if id == DRIFT_CATEGORY_ID.0 {
            return Err(StorageError::InvalidCsvData {
                file: "categories.csv",
                row,
                message: "idle category ID 0 is implicit and must not appear as a catalog row"
                    .to_string(),
            });
        }
        if !seen_ids.insert(id) {
            return Err(StorageError::InvalidCsvData {
                file: "categories.csv",
                row,
                message: format!("duplicate category ID {id}"),
            });
        }

        let name = record.get(1).unwrap_or_default().trim().to_string();
        if name.is_empty() || crate::domain::is_drift_name(&name) {
            return Err(StorageError::InvalidCsvData {
                file: "categories.csv",
                row,
                message: format!("invalid or reserved category name '{name}'"),
            });
        }
        let normalized_name = name.to_ascii_lowercase();
        if !seen_names.insert(normalized_name) {
            return Err(StorageError::InvalidCsvData {
                file: "categories.csv",
                row,
                message: format!("duplicate category name '{name}'"),
            });
        }

        let description = record.get(2).unwrap_or_default().to_string();
        let color_raw = record.get(3).unwrap_or_default();
        let color_idx = color_raw.parse::<usize>().map_err(|error| {
            StorageError::InvalidCsvData {
                file: "categories.csv",
                row,
                message: format!("invalid color index '{color_raw}': {error}"),
            }
        })?;
        if color_idx >= COLORS.len() {
            return Err(StorageError::InvalidCsvData {
                file: "categories.csv",
                row,
                message: format!(
                    "color index {color_idx} is outside 0..{}",
                    COLORS.len().saturating_sub(1)
                ),
            });
        }
        let karma_raw = record.get(4).unwrap_or_default();
        let karma_effect = karma_raw.parse::<i8>().map_err(|error| {
            StorageError::InvalidCsvData {
                file: "categories.csv",
                row,
                message: format!("invalid karma effect '{karma_raw}': {error}"),
            }
        })?;
        if !(-1..=1).contains(&karma_effect) {
            return Err(StorageError::InvalidCsvData {
                file: "categories.csv",
                row,
                message: format!("karma effect {karma_effect} is outside -1..1"),
            });
        }
        let archived = if has_archived_state {
            let raw = record.get(5).unwrap_or_default();
            raw.parse::<bool>().map_err(|error| StorageError::InvalidCsvData {
                file: "categories.csv",
                row,
                message: format!("invalid archived state '{raw}': {error}"),
            })?
        } else {
            false
        };

        let category = Category {
            id: CategoryId::new(id),
            name,
            color: COLORS[color_idx],
            description,
            karma_effect,
        };
        if archived {
            loaded.archived_categories.push(category);
        } else {
            loaded.categories.push(category);
        }
        loaded.next_category_id = loaded.next_category_id.max(id.saturating_add(1));
    }

    Ok(loaded)
}
''',
)

replace_between(
    "src/storage.rs",
    "pub fn save_categories_to_csv(path: &Path, categories: &[Category]) -> Result<(), String> {",
    "\npub fn save_sessions_to_csv(",
    r'''pub fn save_categories_to_csv(path: &Path, categories: &[Category]) -> Result<(), String> {
    save_category_catalog_to_csv(path, categories, &[])
}

pub fn save_category_catalog_to_csv(
    path: &Path,
    active_categories: &[Category],
    archived_categories: &[Category],
) -> Result<(), String> {
    let mut writer = WriterBuilder::new().has_headers(false).from_writer(vec![]);
    writer
        .write_record(CATEGORIES_HEADER)
        .map_err(|e| e.to_string())?;

    for (category, archived) in active_categories
        .iter()
        .map(|category| (category, false))
        .chain(archived_categories.iter().map(|category| (category, true)))
    {
        if category.id == DRIFT_CATEGORY_ID {
            continue;
        }

        let color_pos = COLORS
            .iter()
            .position(|&color| color == category.color)
            .ok_or_else(|| {
                format!(
                    "category {} uses a color outside the persisted palette",
                    category.id.0
                )
            })?;

        writer
            .write_record([
                category.id.0.to_string(),
                category.name.clone(),
                category.description.clone(),
                color_pos.to_string(),
                category.karma_effect.to_string(),
                archived.to_string(),
            ])
            .map_err(|e| e.to_string())?;
    }

    let bytes = writer.into_inner().map_err(|e| e.error().to_string())?;
    let content = String::from_utf8_lossy(&bytes).to_string();

    atomic_write(path, &content)
}
''',
)

path = Path("src/storage.rs")
text = path.read_text()
text = text.replace(
    '''        let category_id = record
            .get(2)
            .and_then(|value| value.parse::<u64>().ok())
            .and_then(|raw| category_by_id.get(&raw).copied())
            .unwrap_or(DRIFT_CATEGORY_ID);
''',
    '''        let category_raw = record.get(2).unwrap_or_default();
        let category_value =
            category_raw
                .parse::<u64>()
                .map_err(|error| StorageError::InvalidCsvData {
                    file: "time_log.csv",
                    row,
                    message: format!("invalid category ID '{category_raw}': {error}"),
                })?;
        let category_id = category_by_id.get(&category_value).copied().ok_or_else(|| {
            StorageError::InvalidCsvData {
                file: "time_log.csv",
                row,
                message: format!(
                    "unknown category ID {category_value}; restore its catalog row or explicitly reassign the session"
                ),
            }
        })?;
''',
    1,
)
text = text.replace(
    '''        let category_name = categories
            .iter()
            .find(|category| category.id == session.category_id)
            .map(|category| category.name.as_str())
            .unwrap_or(DRIFT_CATEGORY_CONFIG_NAME);
''',
    '''        let category_name = categories
            .iter()
            .find(|category| category.id == session.category_id)
            .map(|category| category.name.as_str())
            .ok_or_else(|| {
                format!(
                    "session {} references unknown category ID {}",
                    session.id, session.category_id.0
                )
            })?;
''',
    1,
)
# Storage proofs.
insert_marker = '''    #[test]
    fn test_sessions_round_trip() {
'''
proofs = r'''    #[test]
    fn category_catalog_is_backward_compatible_and_preserves_archived_rows() {
        let legacy_path = unique_path("strata_categories_legacy_catalog", "csv");
        fs::write(
            &legacy_path,
            "id,name,description,color_index,karma_effect\n1,Work,focus,0,1\n",
        )
        .unwrap();
        let legacy = try_load_categories_from_csv(&legacy_path).unwrap();
        assert_eq!(legacy.categories.len(), 2);
        assert!(legacy.archived_categories.is_empty());

        let catalog_path = unique_path("strata_categories_archived_catalog", "csv");
        let active = vec![
            Category {
                id: DRIFT_CATEGORY_ID,
                name: "idle".to_string(),
                color: Color::White,
                description: String::new(),
                karma_effect: 0,
            },
            Category {
                id: CategoryId::new(1),
                name: "Work".to_string(),
                color: COLORS[0],
                description: "focus".to_string(),
                karma_effect: 1,
            },
        ];
        let archived = vec![Category {
            id: CategoryId::new(2),
            name: "Old Client".to_string(),
            color: COLORS[1],
            description: "historical".to_string(),
            karma_effect: -1,
        }];
        save_category_catalog_to_csv(&catalog_path, &active, &archived).unwrap();
        let loaded = try_load_categories_from_csv(&catalog_path).unwrap();
        assert_eq!(loaded.categories.len(), 2);
        assert_eq!(loaded.archived_categories.len(), 1);
        assert_eq!(loaded.archived_categories[0].id, CategoryId::new(2));
        assert_eq!(loaded.archived_categories[0].name, "Old Client");
        assert_eq!(loaded.archived_categories[0].description, "historical");
        assert_eq!(loaded.archived_categories[0].karma_effect, -1);

        fs::remove_file(legacy_path).ok();
        fs::remove_file(catalog_path).ok();
    }

    #[test]
    fn unknown_or_malformed_session_category_fails_closed_but_idle_remains_valid() {
        let categories = default_categories_loaded().categories;
        let missing_path = unique_path("strata_sessions_missing_category", "csv");
        fs::write(
            &missing_path,
            "id,date,category_id,category_name,description,start_time,end_time,elapsed_seconds\n1,2026-08-01,42,Missing,work,10:00:00,11:00:00,3600\n",
        )
        .unwrap();
        let missing = try_load_sessions_from_csv(&missing_path, &categories).unwrap_err();
        assert!(missing.to_string().contains("unknown category ID 42"));

        let malformed_path = unique_path("strata_sessions_malformed_category", "csv");
        fs::write(
            &malformed_path,
            "id,date,category_id,category_name,description,start_time,end_time,elapsed_seconds\n1,2026-08-01,not-an-id,Missing,work,10:00:00,11:00:00,3600\n",
        )
        .unwrap();
        let malformed = try_load_sessions_from_csv(&malformed_path, &categories).unwrap_err();
        assert!(malformed.to_string().contains("invalid category ID 'not-an-id'"));

        let idle_path = unique_path("strata_sessions_idle_category", "csv");
        fs::write(
            &idle_path,
            "id,date,category_id,category_name,description,start_time,end_time,elapsed_seconds\n1,2026-08-01,0,idle,break,10:00:00,11:00:00,3600\n",
        )
        .unwrap();
        let idle = try_load_sessions_from_csv(&idle_path, &categories).unwrap();
        assert_eq!(idle.sessions[0].category_id, DRIFT_CATEGORY_ID);

        fs::remove_file(missing_path).ok();
        fs::remove_file(malformed_path).ok();
        fs::remove_file(idle_path).ok();
    }

    #[test]
    fn session_writer_refuses_unknown_category_identity() {
        let path = unique_path("strata_sessions_unknown_writer", "csv");
        let session = Session {
            id: 7,
            date: "2026-08-01".to_string(),
            category_id: CategoryId::new(99),
            project: String::new(),
            description: String::new(),
            start_time: "10:00:00".to_string(),
            end_time: "11:00:00".to_string(),
            elapsed_seconds: 3600,
            started_at_utc: None,
            ended_at_utc: None,
            operational_day_policy: None,
        };
        let error = save_sessions_to_csv(&path, &[session], &default_categories_loaded().categories)
            .unwrap_err();
        assert!(error.contains("unknown category ID 99"));
        assert!(!path.exists());
    }

    #[test]
    fn archived_category_metadata_keeps_session_labels_round_trippable() {
        let path = unique_path("strata_sessions_archived_label", "csv");
        let archived = Category {
            id: CategoryId::new(8),
            name: "Archived Work".to_string(),
            color: COLORS[2],
            description: "retained".to_string(),
            karma_effect: 1,
        };
        let session = Session {
            id: 8,
            date: "2026-08-01".to_string(),
            category_id: archived.id,
            project: String::new(),
            description: "historical".to_string(),
            start_time: "10:00:00".to_string(),
            end_time: "11:00:00".to_string(),
            elapsed_seconds: 3600,
            started_at_utc: None,
            ended_at_utc: None,
            operational_day_policy: None,
        };
        let mut catalog = default_categories_loaded().categories;
        catalog.push(archived.clone());
        save_sessions_to_csv(&path, &[session], &catalog).unwrap();
        let loaded = try_load_sessions_from_csv(&path, &catalog).unwrap();
        assert_eq!(loaded.sessions[0].category_id, archived.id);
        let csv = fs::read_to_string(&path).unwrap();
        assert!(csv.contains("Archived Work"));
        fs::remove_file(path).ok();
    }

    #[test]
    fn test_sessions_round_trip() {
'''
if insert_marker not in text:
    raise SystemExit("storage proof insertion marker not found")
text = text.replace(insert_marker, proofs, 1)
path.write_text(text)

# -------------------------------------------------------------------------
# App legacy authority loads and persists the complete catalog.
# -------------------------------------------------------------------------
replace_once(
    "src/app.rs",
    '''                let loaded_sessions = storage::try_load_sessions_from_csv(
                    &sessions_path,
                    &loaded_categories.categories,
                )
                .map_err(|error| error.to_string())?;
                let tags = storage::load_category_tags(&storage::get_category_tags_path());
                (
                    None,
                    loaded_categories,
                    loaded_sessions,
                    tags,
                    Vec::new(),
                    None,
                )
''',
    '''                let mut session_categories = loaded_categories.categories.clone();
                session_categories.extend(loaded_categories.archived_categories.iter().cloned());
                let loaded_sessions =
                    storage::try_load_sessions_from_csv(&sessions_path, &session_categories)
                        .map_err(|error| error.to_string())?;
                let tags = storage::load_category_tags(&storage::get_category_tags_path());
                let archived_categories = loaded_categories.archived_categories.clone();
                (
                    None,
                    loaded_categories,
                    loaded_sessions,
                    tags,
                    archived_categories,
                    None,
                )
''',
)

# -------------------------------------------------------------------------
# Category archive/restore and legacy persistence parity.
# -------------------------------------------------------------------------
replace_once(
    "src/app/category_state.rs",
    '''        } else {
            let path = storage::get_categories_path();
            if let Err(error) = storage::save_categories_to_csv(&path, &categories) {
                self.record_storage_result::<()>(Err(error));
            }
        }
''',
    '''        } else {
            let path = storage::get_categories_path();
            if let Err(error) = storage::save_category_catalog_to_csv(
                &path,
                &categories,
                &self.archived_categories,
            ) {
                self.record_storage_result::<()>(Err(error));
            }
        }
''',
)
replace_once(
    "src/app/category_state.rs",
    '''        } else {
            let categories = self.time_tracker.categories_for_storage();
            let path = storage::get_time_log_path();
            if let Err(error) =
                storage::save_sessions_to_csv(&path, &self.time_tracker.sessions, &categories)
            {
                self.record_storage_result::<()>(Err(error));
            }
        }
''',
    '''        } else {
            let mut categories = self.time_tracker.categories_for_storage();
            categories.extend(self.archived_categories.iter().cloned());
            let path = storage::get_time_log_path();
            if let Err(error) =
                storage::save_sessions_to_csv(&path, &self.time_tracker.sessions, &categories)
            {
                self.record_storage_result::<()>(Err(error));
            }
        }
''',
)
replace_once(
    "src/app/category_state.rs",
    '''            .and_then(|index| {
                let mut category = self.archived_categories[index].clone();
                category.color = COLORS[self.color_index % COLORS.len()];
                category.description.clear();
                self.time_tracker
                    .restore_category(category)
                    .then_some((index, self.archived_categories[index].id))
            });
''',
    '''            .and_then(|index| {
                let category = self.archived_categories[index].clone();
                self.time_tracker
                    .restore_category(category)
                    .then_some((index, self.archived_categories[index].id))
            });
''',
)
replace_once(
    "src/app/category_state.rs",
    '''            let removed_id = self
                .time_tracker
                .category_by_index(self.selected_index)
                .map(|category| category.id);
''',
    '''            let removed_category = self
                .time_tracker
                .category_by_index(self.selected_index)
                .cloned();
            let removed_id = removed_category.as_ref().map(|category| category.id);
''',
)
replace_once(
    "src/app/category_state.rs",
    '''            if self.time_tracker.delete_category(self.selected_index) {
                if let Some(category_id) = removed_id {
                    self.category_tags.tags_by_category.remove(&category_id.0);
                    self.persist_category_tags();
                }

                if self.selected_index > 0
''',
    '''            if self.time_tracker.delete_category(self.selected_index) {
                if self.sqlite_database_path.is_none()
                    && let Some(category) = removed_category
                    && !self
                        .archived_categories
                        .iter()
                        .any(|archived| archived.id == category.id)
                {
                    self.archived_categories.push(category);
                }

                if self.selected_index > 0
''',
)

# Legacy report edit writer must know archived labels.
replace_once(
    "src/app/report_state.rs",
    '''            let categories = self.time_tracker.categories_for_storage();
            let result = crate::storage::save_sessions_to_csv(
''',
    '''            let mut categories = self.time_tracker.categories_for_storage();
            categories.extend(self.archived_categories.iter().cloned());
            let result = crate::storage::save_sessions_to_csv(
''',
)

# -------------------------------------------------------------------------
# SQLite legacy importer accepts and preserves archived catalog state.
# -------------------------------------------------------------------------
path = Path("src/sqlite/legacy_import.rs")
text = path.read_text()
text = text.replace(
    '''const CATEGORIES_HEADER: [&str; 5] = ["id", "name", "description", "color_index", "karma_effect"];
''',
    '''const LEGACY_CATEGORIES_HEADER: [&str; 5] = [
    "id",
    "name",
    "description",
    "color_index",
    "karma_effect",
];
const CATEGORIES_HEADER: [&str; 6] = [
    "id",
    "name",
    "description",
    "color_index",
    "karma_effect",
    "archived",
];
''',
    1,
)
text = text.replace(
    '''    balance_effect: i64,
}
''',
    '''    balance_effect: i64,
    archived: bool,
}
''',
    1,
)
text = text.replace(
    '''                    color_index,
                    balance_effect
                 ) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    category.id,
                    category.name,
                    category.description,
                    category.color_index,
                    category.balance_effect,
                ],
''',
    '''                    color_index,
                    balance_effect,
                    archived_at_utc
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    category.id,
                    category.name,
                    category.description,
                    category.color_index,
                    category.balance_effect,
                    category.archived.then_some(started_at_utc.as_str()),
                ],
''',
    1,
)
path.write_text(text)

replace_between(
    "src/sqlite/legacy_import.rs",
    "fn parse_categories(bytes: &[u8]) -> Result<Vec<LegacyCategory>, LegacyImportError> {",
    "\nfn category_names(categories: &[LegacyCategory])",
    r'''fn parse_categories(bytes: &[u8]) -> Result<Vec<LegacyCategory>, LegacyImportError> {
    let source = "categories.csv";
    let mut reader = ReaderBuilder::new()
        .has_headers(true)
        .flexible(false)
        .from_reader(Cursor::new(bytes));
    let header = reader
        .headers()
        .map_err(|error| csv_error(source, error))?
        .clone();
    let has_archived_state = header.iter().eq(CATEGORIES_HEADER.iter().copied());
    if !has_archived_state && !header.iter().eq(LEGACY_CATEGORIES_HEADER.iter().copied()) {
        return Err(invalid(
            source,
            None,
            format!(
                "invalid header; expected '{}' or '{}'",
                LEGACY_CATEGORIES_HEADER.join(","),
                CATEGORIES_HEADER.join(",")
            ),
        ));
    }

    let mut categories = Vec::new();
    let mut ids = HashSet::new();
    let mut names = HashSet::new();
    for (index, record) in reader.records().enumerate() {
        let row = index + 2;
        let record = record.map_err(|error| csv_error(source, error))?;
        let id = parse_i64(source, row, &record, 0, "category ID")?;
        if id <= 0 {
            return Err(invalid(
                source,
                Some(row),
                "category ID must be greater than zero",
            ));
        }
        if !ids.insert(id) {
            return Err(invalid(
                source,
                Some(row),
                format!("duplicate category ID {id}"),
            ));
        }

        let name = required_field(source, row, &record, 1, "category name")?
            .trim()
            .to_string();
        if is_drift_name(&name) {
            return Err(invalid(
                source,
                Some(row),
                format!("category name '{name}' is reserved for idle time"),
            ));
        }
        let normalized_name = name.to_lowercase();
        if !names.insert(normalized_name) {
            return Err(invalid(
                source,
                Some(row),
                format!("duplicate category name '{name}'"),
            ));
        }

        let color_index = parse_i64(source, row, &record, 3, "color index")?;
        if color_index < 0 || color_index >= COLORS.len() as i64 {
            return Err(invalid(
                source,
                Some(row),
                format!(
                    "color index {color_index} is outside 0..{}",
                    COLORS.len() - 1
                ),
            ));
        }
        let balance_effect = parse_i64(source, row, &record, 4, "karma effect")?;
        if !(-1..=1).contains(&balance_effect) {
            return Err(invalid(
                source,
                Some(row),
                format!("karma effect {balance_effect} is outside -1..1"),
            ));
        }
        let archived = if has_archived_state {
            let raw = required_field(source, row, &record, 5, "archived state")?;
            raw.parse::<bool>().map_err(|error| {
                invalid(
                    source,
                    Some(row),
                    format!("invalid archived state '{raw}': {error}"),
                )
            })?
        } else {
            false
        };

        categories.push(LegacyCategory {
            id,
            name,
            description: record.get(2).unwrap_or_default().to_string(),
            color_index,
            balance_effect,
            archived,
        });
    }
    categories.sort_by_key(|category| category.id);
    Ok(categories)
}
''',
)

# Add importer proof near the test module's first test marker.
path = Path("src/sqlite/legacy_import.rs")
text = path.read_text()
marker = '''    #[test]
    fn imports_full_legacy_fixture_and_verifies_totals() {
'''
proof = r'''    #[test]
    fn imports_archived_category_catalog_without_reactivating_history() {
        let fixture = LegacyFixture::new("archived_category_catalog");
        fs::write(
            &fixture.paths.categories_csv,
            "id,name,description,color_index,karma_effect,archived\n1,Current,active,0,1,false\n2,Old Client,historical,1,-1,true\n",
        )
        .unwrap();
        fs::write(
            &fixture.paths.sessions_csv,
            "id,date,category_id,category_name,description,start_time,end_time,elapsed_seconds\n1,2026-08-01,2,Old Client,legacy work,10:00:00,11:00:00,3600\n",
        )
        .unwrap();

        let plan = LegacyImportPlan::from_paths(&fixture.paths, fixture.options()).unwrap();
        let mut repository = SqliteRepository::open(&fixture.database_path).unwrap();
        repository.import_legacy(&plan).unwrap();

        let archived: Option<String> = repository
            .connection
            .query_row(
                "SELECT archived_at_utc FROM categories WHERE id = 2",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(archived.is_some());
        let category_id: i64 = repository
            .connection
            .query_row("SELECT category_id FROM sessions WHERE id = 1", [], |row| row.get(0))
            .unwrap();
        assert_eq!(category_id, 2);
    }

    #[test]
    fn imports_full_legacy_fixture_and_verifies_totals() {
'''
if marker not in text:
    raise SystemExit("legacy import test insertion marker not found")
text = text.replace(marker, proof, 1)
path.write_text(text)

for temporary in [
    ".github/workflows/reconciliation001a-apply.yml",
    "tools/reconciliation001a-apply.py",
]:
    Path(temporary).unlink(missing_ok=True)
