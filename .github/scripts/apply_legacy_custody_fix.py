from pathlib import Path
import re


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected one marker, found {count}")
    return text.replace(old, new, 1)


# Legacy state parsers must distinguish absence from damaged evidence.
path = Path("src/storage.rs")
text = path.read_text()
old = '''pub fn load_sand_state(path: &Path) -> Option<SandState> {
    if !path.exists() {
        return None;
    }

    match read_json::<SandState>(path) {
        Ok(state) if state.version == SandState::VERSION => Some(state),
        Ok(_) => {
            eprintln!("Warning: Unsupported sand state version, ignoring saved layout");
            None
        }
        Err(e) => {
            eprintln!("Warning: Could not load sand state: {}", e);
            None
        }
    }
}
'''
new = '''pub fn try_load_sand_state(path: &Path) -> Result<Option<SandState>, String> {
    if !path.exists() {
        return Ok(None);
    }

    let state = read_json::<SandState>(path)
        .map_err(|error| format!("Could not load sand state {}: {error}", path.display()))?;
    if state.version != SandState::VERSION && state.version != SandState::LEGACY_VERSION {
        return Err(format!(
            "Unsupported sand state version {} in {}",
            state.version,
            path.display()
        ));
    }
    Ok(Some(state))
}
'''
text = replace_once(text, old, new, "sand loader")
old = '''pub fn load_category_tags(path: &Path) -> CategoryTagsState {
    if !path.exists() {
        return CategoryTagsState::default();
    }

    match read_json::<CategoryTagsState>(path) {
        Ok(mut state) if state.version == CategoryTagsState::VERSION => {
            for tags in state.tags_by_category.values_mut() {
                tags.retain(|tag| !tag.trim().is_empty());
            }
            state
        }
        Ok(_) => {
            eprintln!("Warning: Unsupported category tags version, ignoring saved tags");
            CategoryTagsState::default()
        }
        Err(e) => {
            eprintln!("Warning: Could not load category tags: {}", e);
            CategoryTagsState::default()
        }
    }
}
'''
new = '''pub fn try_load_category_tags(path: &Path) -> Result<CategoryTagsState, String> {
    if !path.exists() {
        return Ok(CategoryTagsState::default());
    }

    let mut state = read_json::<CategoryTagsState>(path)
        .map_err(|error| format!("Could not load category tags {}: {error}", path.display()))?;
    if state.version != CategoryTagsState::VERSION {
        return Err(format!(
            "Unsupported category tags version {} in {}",
            state.version,
            path.display()
        ));
    }
    for tags in state.tags_by_category.values_mut() {
        tags.retain(|tag| !tag.trim().is_empty());
    }
    Ok(state)
}
'''
text = replace_once(text, old, new, "category tags loader")
text = replace_once(
    text,
    '        let loaded = load_sand_state(&path).expect("sand state should load");',
    '        let loaded = try_load_sand_state(&path)\n            .unwrap()\n            .expect("sand state should load");',
    "sand round-trip test",
)
text = replace_once(
    text,
    '        let loaded = load_category_tags(&path);',
    '        let loaded = try_load_category_tags(&path).unwrap();',
    "category tags round-trip test",
)
path.write_text(text)

# Category tags are authority at startup; parsing failure must stop construction before any rewrite.
path = Path("src/app.rs")
text = path.read_text()
text = replace_once(
    text,
    '                let tags = storage::load_category_tags(&storage::get_category_tags_path());',
    '                let tags =\n                    storage::try_load_category_tags(&storage::get_category_tags_path())?;',
    "app category tags load",
)
path.write_text(text)

# Legacy sediment must use the same visible reload failure boundary as SQLite state.
path = Path("src/app/category_state.rs")
text = path.read_text()
old = '''    pub(super) fn restore_sand_state(&mut self) {
        let state = if let Some(database_path) = self.sqlite_database_path.clone() {
            match sqlite::load_tui_sand_state(&database_path) {
                Ok(value) => value,
                Err(error) => {
                    self.record_storage_result_for::<()>(
                        PersistenceOperation::StateReload,
                        RecoveryAction::ReloadAuthority,
                        Err(error),
                    );
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
'''
new = '''    pub(super) fn restore_sand_state(&mut self) {
        let state = if let Some(database_path) = self.sqlite_database_path.clone() {
            match sqlite::load_tui_sand_state(&database_path) {
                Ok(value) => value,
                Err(error) => {
                    self.record_storage_result_for::<()>(
                        PersistenceOperation::StateReload,
                        RecoveryAction::ReloadAuthority,
                        Err(error),
                    );
                    return;
                }
            }
        } else {
            match storage::try_load_sand_state(&storage::get_sand_state_path()) {
                Ok(value) => value,
                Err(error) => {
                    self.record_storage_result_for::<()>(
                        PersistenceOperation::StateReload,
                        RecoveryAction::ReloadAuthority,
                        Err(error),
                    );
                    return;
                }
            }
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
        if let Err(error) = self.sand_engine.restore_state(&state, &valid_category_ids) {
            self.record_storage_result_for::<()>(
                PersistenceOperation::StateReload,
                RecoveryAction::ReloadAuthority,
                Err(error),
            );
        }
    }
'''
text = replace_once(text, old, new, "app sediment restore")
path.write_text(text)

# Sediment restoration validates a complete candidate before mutating the engine.
path = Path("src/sand/engine.rs")
text = path.read_text()
text = replace_once(
    text,
    '    domain::{Category, CategoryId, DRIFT_CATEGORY_ID},',
    '    domain::{Category, CategoryId},',
    "unused drift import",
)
start = text.index(
    '    pub fn restore_state(&mut self, state: &SandState, valid_category_ids: &HashSet<CategoryId>) {'
)
end = text.index('    fn next_random_u64(&mut self) -> u64 {', start)
replacement = '''    pub fn restore_state(
        &mut self,
        state: &SandState,
        valid_category_ids: &HashSet<CategoryId>,
    ) -> Result<(), String> {
        if state.version != SandState::VERSION && state.version != SandState::LEGACY_VERSION {
            return Err(format!("unsupported sand state version {}", state.version));
        }
        if (state.grid_width == 0 || state.grid_height == 0) && !state.grains.is_empty() {
            return Err("zero-sized sand state cannot contain placed grains".to_string());
        }

        let mut restored = vec![vec![None; state.grid_width]; state.grid_height];
        let mut occupied = HashSet::with_capacity(state.grains.len());
        for grain in &state.grains {
            if grain.x >= state.grid_width || grain.y >= state.grid_height {
                return Err(format!(
                    "sand grain ({}, {}) is outside the {}x{} canonical grid",
                    grain.x, grain.y, state.grid_width, state.grid_height
                ));
            }
            let category_id = CategoryId::new(grain.category_id);
            if !valid_category_ids.contains(&category_id) {
                return Err(format!(
                    "sand state references unknown category ID {}",
                    grain.category_id
                ));
            }
            if !occupied.insert((grain.x, grain.y)) {
                return Err(format!(
                    "sand state contains duplicate grain coordinate ({}, {})",
                    grain.x, grain.y
                ));
            }
            restored[grain.y][grain.x] = Some(category_id);
        }

        let mut pending_runs = VecDeque::new();
        let mut append_serialized_run = |category_id: u64, count: usize| -> Result<(), String> {
            if count == 0 {
                return Err(format!(
                    "sand state contains a zero-count pending run for category {category_id}"
                ));
            }
            let category_id = CategoryId::new(category_id);
            if !valid_category_ids.contains(&category_id) {
                return Err(format!(
                    "sand state references unknown pending category ID {}",
                    category_id.0
                ));
            }
            Self::append_pending_run(&mut pending_runs, category_id, count)
        };

        if state.version == SandState::LEGACY_VERSION {
            if !state.pending_runs.is_empty() {
                return Err("legacy sand state cannot contain version-two pending runs".to_string());
            }
            for category_id in &state.pending_grains {
                append_serialized_run(*category_id, 1)?;
            }
        } else {
            if !state.pending_runs.is_empty() && !state.pending_grains.is_empty() {
                return Err(
                    "sand state contains both legacy pending grains and compressed pending runs"
                        .to_string(),
                );
            }
            if state.pending_runs.is_empty() {
                for category_id in &state.pending_grains {
                    append_serialized_run(*category_id, 1)?;
                }
            } else {
                for run in &state.pending_runs {
                    append_serialized_run(run.category_id, run.count)?;
                }
            }
        }

        let physical_count = state.grains.len();
        let pending_count = pending_runs.iter().try_fold(0usize, |total, run| {
            total.checked_add(run.count).ok_or_else(|| {
                "pending sediment count exceeds the supported range".to_string()
            })
        })?;
        let logical_count = physical_count
            .checked_add(pending_count)
            .ok_or_else(|| "logical sediment count exceeds the supported range".to_string())?;

        self.grid = restored;
        self.grid_width_dots = state.grid_width;
        self.grid_height_dots = state.grid_height;
        self.pending_runs = pending_runs;
        self.grain_count = logical_count;
        self.frame_count = state.frame_count;
        self.sweep_left_to_right = state.sweep_left_to_right;
        self.rng_state = if state.rng_state == 0 {
            default_rng_state()
        } else {
            state.rng_state
        };
        Ok(())
    }

'''
text = text[:start] + replacement + text[end:]
old_test = '''    #[test]
    fn test_sand_state_restore_maps_unknown_category_to_none() {
        let mut se = SandEngine::new(20, 20);
        se.clear();
        se.grid[2][2] = Some(CategoryId::new(99));
        se.grain_count = 1;

        let state = se.snapshot_state();

        let mut restored = SandEngine::new(20, 20);
        let valid = HashSet::from([CategoryId::new(0), CategoryId::new(1)]);
        restored.restore_state(&state, &valid);

        assert_eq!(restored.grid[2][2], Some(CategoryId::new(0)));
        assert_eq!(restored.grain_count, 1);
    }
'''
new_test = '''    #[test]
    fn test_sand_state_restore_rejects_unknown_category_without_mutation() {
        let state = super::SandState {
            version: super::SandState::VERSION,
            grid_width: 2,
            grid_height: 2,
            grains: vec![super::SandStateGrain {
                x: 1,
                y: 1,
                category_id: 99,
            }],
            frame_count: 0,
            sweep_left_to_right: true,
            rng_state: 1,
            pending_grains: Vec::new(),
            pending_runs: Vec::new(),
        };

        let mut restored = SandEngine::new(20, 20);
        let before = restored.snapshot_state();
        let valid = HashSet::from([CategoryId::new(0), CategoryId::new(1)]);
        let error = restored.restore_state(&state, &valid).unwrap_err();

        assert!(error.contains("unknown category ID 99"));
        assert_eq!(restored.snapshot_state(), before);
    }
'''
text = replace_once(text, old_test, new_test, "unknown sediment test")
# Every remaining in-file test call is expected to accept a valid fixture.
text, count = re.subn(
    r'(?ms)^(\s*)([A-Za-z_][A-Za-z0-9_]*)\.restore_state\((.*?)\);$',
    lambda match: (
        f"{match.group(1)}{match.group(2)}.restore_state({match.group(3)})"
        ".unwrap();"
    ),
    text,
)
if count < 5:
    raise SystemExit(f"restore test call conversion: expected at least 5, found {count}")
path.write_text(text)

print("legacy state custody transform applied")
