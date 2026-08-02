from pathlib import Path

path = Path("tools/reconciliation001b1-apply.py")
text = path.read_text()
old_identity = '''    let claimed_stable_id = claimed
        .active_session_stable_id
        .as_deref()
        .ok_or_else(|| "Runtime checkpoint has no active stable identity".to_string())?;
'''
new_identity = '''    let claimed_stable_id = match claimed.active_session_stable_id.as_deref() {
        Some(stable_id) => stable_id,
        None => {
            runtime_coordination::quarantine_checkpoint(&mut repository)
                .map_err(|error| error.to_string())?;
            return Err(
                "Runtime checkpoint has no active stable identity; evidence quarantined"
                    .to_string(),
            );
        }
    };
'''
if old_identity not in text:
    raise SystemExit("claimed checkpoint identity template not found")
text = text.replace(old_identity, new_identity, 1)
text = text.replace(
    "fn startup_quarantines_checkpoint_for_replaced_active_identity()",
    "fn startup_quarantines_checkpoint_without_active_identity()",
    1,
)
old_mutation = '''        repository
            .connection
            .execute(
                "UPDATE active_session SET stable_id = 'active-b' WHERE singleton = 1",
                [],
            )
            .unwrap();
'''
new_mutation = '''        repository
            .connection
            .execute(
                "UPDATE runtime_checkpoint SET active_session_stable_id = NULL WHERE singleton = 1",
                [],
            )
            .unwrap();
'''
if old_mutation not in text:
    raise SystemExit("checkpoint identity test mutation not found")
text = text.replace(old_mutation, new_mutation, 1)
text = text.replace(
    'assert!(error.contains("does not match authoritative active session active-b"));',
    'assert!(error.contains("has no active stable identity; evidence quarantined"));',
    1,
)
authority_anchor = '''        let mut repository = SqliteRepository::open(&path).unwrap();
        repository
            .create_category(&NewCategoryRecord {
'''
authority_replacement = '''        let mut repository = SqliteRepository::open(&path).unwrap();
        repository
            .connection
            .execute(
                "UPDATE database_metadata SET value = 'sqlite-cli' WHERE key = 'storage_authority'",
                [],
            )
            .unwrap();
        repository
            .create_category(&NewCategoryRecord {
'''
if authority_anchor not in text:
    raise SystemExit("checkpoint fixture authority anchor not found")
text = text.replace(authority_anchor, authority_replacement, 1)
path.write_text(text)
Path(__file__).unlink(missing_ok=True)