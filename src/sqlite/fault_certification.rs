use std::{
    fs,
    path::{Path, PathBuf},
    time::Instant,
};

use chrono::{TimeZone, Utc};
use rusqlite::{Connection, params};

use crate::{
    constants::COLORS,
    domain::{Category, CategoryId, Session},
    sand::SandState,
    storage::CategoryTagsState,
};

use super::{
    NewActiveSession, SqliteRepository, authority,
    repository::{CheckpointStatus, NewCategoryRecord, SandStateRecord},
    runtime_coordination, tui_runtime,
};

fn database_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "strata-sqlite010-{name}-{}-{}.sqlite3",
        std::process::id(),
        Utc::now().timestamp_nanos_opt().unwrap_or_default()
    ))
}

fn remove_database(path: &Path) {
    fs::remove_file(path).ok();
    fs::remove_file(format!("{}-wal", path.display())).ok();
    fs::remove_file(format!("{}-shm", path.display())).ok();
}

fn seed(path: &Path) {
    let mut repository = SqliteRepository::open(path).unwrap();
    repository
        .transition_storage_authority("sqlite-candidate", "sqlite-cli", "2026-08-01T12:00:00Z")
        .unwrap();
    repository
        .create_category(&NewCategoryRecord {
            name: "Work",
            description: "original-work",
            color_index: 0,
            balance_effect: 1,
        })
        .unwrap();
    repository
        .create_category(&NewCategoryRecord {
            name: "Rest",
            description: "original-rest",
            color_index: 1,
            balance_effect: -1,
        })
        .unwrap();
}

fn start_active(path: &Path, stable_id: &str, category_id: i64) {
    let mut repository = SqliteRepository::open(path).unwrap();
    runtime_coordination::start_active_session(
        &mut repository,
        &NewActiveSession {
            stable_id,
            project: "",
            category_id,
            description: "",
            started_at_utc: "2026-08-01T12:00:00Z",
            recovery_kind: "live",
        },
    )
    .unwrap();
}

fn insert_session(path: &Path, id: i64, category_id: i64, description: &str) {
    let repository = SqliteRepository::open(path).unwrap();
    repository
        .connection
        .execute(
            "INSERT INTO sessions (
                id, stable_id, project, category_id, description, started_at_utc,
                ended_at_utc, operational_day, elapsed_seconds, source
             ) VALUES (?1, ?2, '', ?3, ?4, '2026-08-01T12:00:00Z',
                '2026-08-01T13:00:00Z', '2026-08-01', 3600, 'tui-runtime')",
            params![id, format!("session-{id}"), category_id, description],
        )
        .unwrap();
}

fn category(id: u64, name: &str, description: &str, color_index: usize, balance: i8) -> Category {
    Category {
        id: CategoryId::new(id),
        name: name.to_string(),
        color: COLORS[color_index % COLORS.len()],
        description: description.to_string(),
        karma_effect: balance,
    }
}

fn session_description(path: &Path, id: i64) -> String {
    let repository = SqliteRepository::open(path).unwrap();
    repository
        .connection
        .query_row(
            "SELECT description FROM sessions WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )
        .unwrap()
}

fn active_id(path: &Path) -> Option<String> {
    SqliteRepository::open(path)
        .unwrap()
        .active_session()
        .unwrap()
        .map(|active| active.stable_id)
}

fn count(path: &Path, table: &str) -> i64 {
    let repository = SqliteRepository::open(path).unwrap();
    repository
        .connection
        .query_row(&format!("SELECT count(*) FROM {table}"), [], |row| {
            row.get(0)
        })
        .unwrap()
}

fn sand_state(frame_count: usize) -> SandState {
    SandState {
        version: SandState::VERSION,
        grid_width: 2,
        grid_height: 2,
        grains: Vec::new(),
        frame_count,
        sweep_left_to_right: true,
        rng_state: u64::try_from(frame_count).unwrap(),
    }
}

fn with_database(name: &str, action: impl FnOnce(&Path)) {
    let path = database_path(name);
    seed(&path);
    action(&path);
    remove_database(&path);
}

#[test]
fn every_authoritative_persistence_family_rolls_back_or_remains_recoverable() {
    with_database("active-start", |path| {
        let started = Utc.with_ymd_and_hms(2026, 8, 1, 12, 30, 0).unwrap();
        let error =
            runtime_coordination::with_test_fault("active-start", "commit", "commit", || {
                tui_runtime::ensure_active_session(path, CategoryId::new(1), "", started)
            })
            .unwrap_err();
        assert!(error.contains("active-start commit"));
        assert_eq!(active_id(path), None);
    });

    with_database("active-finish", |path| {
        start_active(path, "active-a", 1);
        let ended = Utc.with_ymd_and_hms(2026, 8, 1, 13, 0, 0).unwrap();
        runtime_coordination::with_test_fault("finish", "commit", "commit", || {
            tui_runtime::finish_active_session(
                path,
                "active-a",
                "finish:active-a",
                ended,
                "2026-08-01",
                3600,
            )
        })
        .unwrap_err();
        assert_eq!(active_id(path).as_deref(), Some("active-a"));
        assert_eq!(count(path, "sessions"), 0);
        assert_eq!(count(path, "runtime_transitions"), 0);
    });

    with_database("active-switch", |path| {
        start_active(path, "active-a", 1);
        let switched = Utc.with_ymd_and_hms(2026, 8, 1, 13, 0, 0).unwrap();
        runtime_coordination::with_test_fault("switch", "commit", "commit", || {
            tui_runtime::switch_active_session(
                path,
                "active-a",
                "switch:active-a",
                "active-b",
                CategoryId::new(2),
                "",
                switched,
                "2026-08-01",
                3600,
            )
        })
        .unwrap_err();
        assert_eq!(active_id(path).as_deref(), Some("active-a"));
        assert_eq!(count(path, "sessions"), 0);
        assert_eq!(count(path, "runtime_transitions"), 0);
    });

    with_database("active-reset", |path| {
        start_active(path, "active-a", 1);
        let reset_at = Utc.with_ymd_and_hms(2026, 8, 1, 13, 0, 0).unwrap();
        runtime_coordination::with_test_fault("reset", "commit", "commit", || {
            tui_runtime::reset_active_session(
                path,
                "active-a",
                "reset:active-a",
                "active-b",
                reset_at,
            )
        })
        .unwrap_err();
        assert_eq!(active_id(path).as_deref(), Some("active-a"));
        assert_eq!(count(path, "runtime_transitions"), 0);
    });

    with_database("category-sync", |path| {
        start_active(path, "active-a", 1);
        let categories = vec![
            category(0, "None", "", 0, 0),
            category(1, "Work", "changed", 0, 1),
            category(2, "Rest", "changed", 1, -1),
        ];
        runtime_coordination::with_test_fault("category-sync", "commit", "commit", || {
            tui_runtime::sync_categories(path, &categories, CategoryId::new(1), Some("active-a"))
        })
        .unwrap_err();
        let state = tui_runtime::load_state(path).unwrap();
        assert_eq!(
            state.loaded_categories.categories[1].description,
            "original-work"
        );
        assert_eq!(
            state.loaded_categories.categories[2].description,
            "original-rest"
        );
    });

    with_database("category-archive", |path| {
        runtime_coordination::with_test_fault("category-archive", "commit", "commit", || {
            tui_runtime::archive_category(path, CategoryId::new(1))
        })
        .unwrap_err();
        let state = tui_runtime::load_state(path).unwrap();
        assert!(state.archived_categories.is_empty());
        assert!(
            state
                .loaded_categories
                .categories
                .iter()
                .any(|entry| entry.id.0 == 1)
        );
    });

    with_database("category-tags", |path| {
        let mut repository = SqliteRepository::open(path).unwrap();
        repository
            .replace_category_tags(1, &["old-work".to_string()])
            .unwrap();
        repository
            .replace_category_tags(2, &["old-rest".to_string()])
            .unwrap();
        drop(repository);
        let mut tags = CategoryTagsState::default();
        tags.tags_by_category
            .insert(1, vec!["new-work".to_string()]);
        tags.tags_by_category
            .insert(2, vec!["new-rest".to_string()]);
        runtime_coordination::with_test_fault("category-tags", "commit", "commit", || {
            tui_runtime::sync_category_tags(path, &tags, &[CategoryId::new(1), CategoryId::new(2)])
        })
        .unwrap_err();
        let stored = SqliteRepository::open(path)
            .unwrap()
            .category_tags()
            .unwrap();
        assert_eq!(stored.get(&1).unwrap(), &vec!["old-work".to_string()]);
        assert_eq!(stored.get(&2).unwrap(), &vec!["old-rest".to_string()]);
    });

    with_database("session-sync", |path| {
        insert_session(path, 7, 1, "old");
        let sessions = vec![Session {
            id: 7,
            date: "2026-08-01".to_string(),
            category_id: CategoryId::new(1),
            description: "new".to_string(),
            start_time: "12:00:00".to_string(),
            end_time: "13:00:00".to_string(),
            elapsed_seconds: 3600,
        }];
        runtime_coordination::with_test_fault("session-sync", "commit", "commit", || {
            tui_runtime::sync_sessions(path, &sessions)
        })
        .unwrap_err();
        assert_eq!(session_description(path, 7), "old");
    });

    with_database("session-edit", |path| {
        insert_session(path, 7, 1, "old");
        runtime_coordination::with_test_fault("session-edit", "commit", "commit", || {
            tui_runtime::update_session_description(path, 7, "new")
        })
        .unwrap_err();
        assert_eq!(session_description(path, 7), "old");
    });

    with_database("session-delete", |path| {
        insert_session(path, 7, 1, "old");
        runtime_coordination::with_test_fault("session-delete", "commit", "commit", || {
            tui_runtime::delete_session(path, 7)
        })
        .unwrap_err();
        assert_eq!(count(path, "sessions"), 1);
    });

    with_database("drift-delete", |path| {
        insert_session(path, 7, 0, "drift");
        runtime_coordination::with_test_fault("drift-session-delete", "commit", "commit", || {
            tui_runtime::delete_drift_sessions_for_day(path, "2026-08-01")
        })
        .unwrap_err();
        assert_eq!(count(path, "sessions"), 1);
    });

    with_database("sand-state", |path| {
        tui_runtime::save_sand_state(path, &sand_state(1)).unwrap();
        runtime_coordination::with_test_fault("sand-state", "commit", "commit", || {
            tui_runtime::save_sand_state(path, &sand_state(2))
        })
        .unwrap_err();
        assert_eq!(
            tui_runtime::load_sand_state(path)
                .unwrap()
                .unwrap()
                .frame_count,
            1
        );
    });

    with_database("daily-snapshot", |path| {
        tui_runtime::save_daily_snapshot(path, "2026-08-01", &sand_state(1)).unwrap();
        runtime_coordination::with_test_fault("daily-snapshot", "commit", "commit", || {
            tui_runtime::save_daily_snapshot(path, "2026-08-01", &sand_state(2))
        })
        .unwrap_err();
        assert_eq!(
            tui_runtime::load_daily_snapshot(path, "2026-08-01")
                .unwrap()
                .unwrap()
                .frame_count,
            1
        );
    });

    with_database("daily-snapshot-delete", |path| {
        tui_runtime::save_daily_snapshot(path, "2026-08-01", &sand_state(1)).unwrap();
        runtime_coordination::with_test_fault("daily-snapshot-delete", "commit", "commit", || {
            tui_runtime::delete_daily_snapshot(path, "2026-08-01")
        })
        .unwrap_err();
        assert!(
            tui_runtime::load_daily_snapshot(path, "2026-08-01")
                .unwrap()
                .is_some()
        );
    });

    with_database("checkpoint-save", |path| {
        start_active(path, "active-a", 1);
        let at = Utc.with_ymd_and_hms(2026, 8, 1, 13, 0, 0).unwrap();
        runtime_coordination::with_test_fault("checkpoint-save", "commit", "commit", || {
            tui_runtime::save_checkpoint(path, "active-a", at, at, &sand_state(1))
        })
        .unwrap_err();
        assert!(
            SqliteRepository::open(path)
                .unwrap()
                .checkpoint()
                .unwrap()
                .is_none()
        );
        assert_eq!(active_id(path).as_deref(), Some("active-a"));
    });

    with_database("checkpoint-claim", |path| {
        start_active(path, "active-a", 1);
        let at = Utc.with_ymd_and_hms(2026, 8, 1, 13, 0, 0).unwrap();
        tui_runtime::save_checkpoint(path, "active-a", at, at, &sand_state(1)).unwrap();
        runtime_coordination::with_test_fault("checkpoint-claim", "commit", "commit", || {
            tui_runtime::load_checkpoint::<SandState>(path)
        })
        .unwrap_err();
        assert_eq!(
            SqliteRepository::open(path)
                .unwrap()
                .checkpoint()
                .unwrap()
                .unwrap()
                .status,
            CheckpointStatus::Pending
        );
    });

    with_database("checkpoint-quarantine", |path| {
        start_active(path, "active-a", 1);
        let mut repository = SqliteRepository::open(path).unwrap();
        runtime_coordination::save_checkpoint(
            &mut repository,
            "active-a",
            "2026-08-01T13:00:00Z",
            "2026-08-01T13:00:00Z",
            "{}",
        )
        .unwrap();
        runtime_coordination::claim_checkpoint(&mut repository)
            .unwrap()
            .unwrap();
        drop(repository);
        runtime_coordination::with_test_fault("checkpoint-quarantine", "commit", "commit", || {
            tui_runtime::quarantine_checkpoint(path)
        })
        .unwrap_err();
        assert_eq!(
            SqliteRepository::open(path)
                .unwrap()
                .checkpoint()
                .unwrap()
                .unwrap()
                .status,
            CheckpointStatus::Recovering
        );
    });

    with_database("checkpoint-recovery", |path| {
        start_active(path, "active-a", 1);
        tui_runtime::save_sand_state(path, &sand_state(1)).unwrap();
        let mut repository = SqliteRepository::open(path).unwrap();
        runtime_coordination::save_checkpoint(
            &mut repository,
            "active-a",
            "2026-08-01T13:00:00Z",
            "2026-08-01T13:00:00Z",
            "{}",
        )
        .unwrap();
        runtime_coordination::claim_checkpoint(&mut repository)
            .unwrap()
            .unwrap();
        drop(repository);
        runtime_coordination::with_test_fault("checkpoint-recovery", "commit", "commit", || {
            tui_runtime::commit_checkpoint_recovery(path, "active-a", "2026-08-01", &sand_state(2))
        })
        .unwrap_err();
        assert_eq!(
            tui_runtime::load_sand_state(path)
                .unwrap()
                .unwrap()
                .frame_count,
            1
        );
        assert_eq!(
            SqliteRepository::open(path)
                .unwrap()
                .checkpoint()
                .unwrap()
                .unwrap()
                .status,
            CheckpointStatus::Recovering
        );
    });

    with_database("checkpoint-clear", |path| {
        start_active(path, "active-a", 1);
        let mut repository = SqliteRepository::open(path).unwrap();
        runtime_coordination::save_checkpoint(
            &mut repository,
            "active-a",
            "2026-08-01T13:00:00Z",
            "2026-08-01T13:00:00Z",
            "{}",
        )
        .unwrap();
        runtime_coordination::claim_checkpoint(&mut repository)
            .unwrap()
            .unwrap();
        runtime_coordination::commit_checkpoint_recovery(
            &mut repository,
            "active-a",
            "2026-08-01",
            &SandStateRecord {
                formation_id: "default".to_string(),
                quantum_seconds: 1,
                grid_width: 2,
                grid_height: 2,
                payload_json: serde_json::to_string(&sand_state(1)).unwrap(),
                updated_at_utc: "2026-08-01T13:00:00Z".to_string(),
            },
            "2026-08-01T13:00:00Z",
        )
        .unwrap();
        drop(repository);
        runtime_coordination::with_test_fault("checkpoint-clear", "commit", "commit", || {
            tui_runtime::clear_checkpoint(path)
        })
        .unwrap_err();
        assert_eq!(
            SqliteRepository::open(path)
                .unwrap()
                .checkpoint()
                .unwrap()
                .unwrap()
                .status,
            CheckpointStatus::Committed
        );
    });

    with_database("state-load", |path| {
        runtime_coordination::with_test_fault("state-load", "before-read", "corrupt", || {
            tui_runtime::load_state(path)
        })
        .unwrap_err();
        assert_eq!(count(path, "categories"), 3);
    });
}

#[test]
fn real_sqlite_busy_full_constraint_and_corruption_fail_without_false_success() {
    with_database("real-busy", |path| {
        start_active(path, "active-a", 1);
        let lock = Connection::open(path).unwrap();
        lock.execute_batch("BEGIN IMMEDIATE").unwrap();
        let started = Instant::now();
        let error = tui_runtime::update_session_description(path, 999, "blocked").unwrap_err();
        assert!(
            error.to_ascii_lowercase().contains("locked")
                || error.to_ascii_lowercase().contains("busy")
        );
        assert!(started.elapsed().as_secs() >= 4);
        lock.execute_batch("ROLLBACK").unwrap();
        assert_eq!(active_id(path).as_deref(), Some("active-a"));
    });

    with_database("real-constraint", |path| {
        insert_session(path, 7, 1, "old");
        let repository = SqliteRepository::open(path).unwrap();
        repository
            .connection
            .execute_batch(
                "CREATE TRIGGER reject_session_edit
                 BEFORE UPDATE OF description ON sessions
                 BEGIN SELECT RAISE(ABORT, 'injected constraint'); END;",
            )
            .unwrap();
        drop(repository);
        let error = tui_runtime::update_session_description(path, 7, "new").unwrap_err();
        assert!(error.to_ascii_lowercase().contains("constraint"));
        assert_eq!(session_description(path, 7), "old");
    });

    with_database("real-full", |path| {
        let categories = vec![
            category(0, "None", "", 0, 0),
            category(1, "Work", &"x".repeat(2 * 1024 * 1024), 0, 1),
            category(2, "Rest", "original-rest", 1, -1),
        ];
        let error = authority::with_test_page_limit(|| {
            tui_runtime::sync_categories(path, &categories, CategoryId::new(0), None)
        })
        .unwrap_err();
        assert!(error.to_ascii_lowercase().contains("full"));
        let state = tui_runtime::load_state(path).unwrap();
        assert_eq!(
            state.loaded_categories.categories[1].description,
            "original-work"
        );
    });

    let path = database_path("real-corrupt");
    seed(&path);
    fs::write(&path, b"not a sqlite database").unwrap();
    let error = tui_runtime::load_state(&path).unwrap_err();
    let normalized = error.to_ascii_lowercase();
    assert!(normalized.contains("not a database") || normalized.contains("malformed"));
    remove_database(&path);
}

#[cfg(unix)]
#[test]
fn real_read_only_authority_rejects_writes_and_preserves_rows() {
    use std::os::unix::fs::PermissionsExt;

    let root = std::env::temp_dir().join(format!(
        "strata-sqlite010-readonly-{}-{}",
        std::process::id(),
        Utc::now().timestamp_nanos_opt().unwrap_or_default()
    ));
    fs::create_dir_all(&root).unwrap();
    let path = root.join("strata.sqlite3");
    seed(&path);
    insert_session(&path, 7, 1, "old");
    fs::remove_file(format!("{}-wal", path.display())).ok();
    fs::remove_file(format!("{}-shm", path.display())).ok();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o444)).unwrap();
    fs::set_permissions(&root, fs::Permissions::from_mode(0o555)).unwrap();

    let error = tui_runtime::update_session_description(&path, 7, "new").unwrap_err();

    fs::set_permissions(&root, fs::Permissions::from_mode(0o755)).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
    assert!(
        error.to_ascii_lowercase().contains("readonly")
            || error.to_ascii_lowercase().contains("read-only")
    );
    assert_eq!(session_description(&path, 7), "old");
    remove_database(&path);
    fs::remove_dir_all(root).ok();
}
