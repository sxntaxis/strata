from pathlib import Path
import re


path = Path("src/category_lifecycle.rs")
text = path.read_text()
text = text.replace("if !(1..=3).contains(&version)", "if version != 1")
text = text.replace(
    'Ok(["legacy_transition", "legacy_finish", "clear_all"]\n        .iter()\n        .any(|field| object.get(*field).is_some_and(|value| !value.is_null())))',
    'Ok(object\n        .get("clear_all")\n        .is_some_and(|value| !value.is_null()))',
)
text = text.replace('"schema_version": 3,', '"schema_version": 1,')
text = re.sub(r'\n\s*"legacy_transition": (?:null|\{.*?\}),', '', text)
text = re.sub(r'\n\s*"legacy_finish": null,', '', text)
text = text.replace(
    '"clear_all": null\n        })\n        .to_string();\n        assert!(checkpoint_has_transition_receipt(&payload).unwrap());',
    '"clear_all": {"operation_id": "pending-clear"}\n        })\n        .to_string();\n        assert!(checkpoint_has_transition_receipt(&payload).unwrap());',
    1,
)
path.write_text(text)

path = Path("src/sqlite/category_lifecycle.rs")
text = path.read_text()
pattern = r'''    fn checkpoint_json\(with_receipt: bool\) -> String \{.*?\n    \}\n\n    fn merge'''
replacement = '''    fn checkpoint_json(with_receipt: bool) -> String {
        serde_json::json!({
            "schema_version": 1,
            "detached_at_utc": "2026-08-03T18:00:00Z",
            "simulation_time_utc": "2026-08-03T18:00:00Z",
            "spawn_accumulator_nanos": 0,
            "physics_accumulator_nanos": 0,
            "active_category_id": 1,
            "active_description": "active",
            "active_session_started_at_utc": "2026-08-03T18:00:00Z",
            "sand_state": sand_state(),
            "pending_mutations": [{"SwitchLayer": {"category_id": 1}}],
            "recovery_target_utc": null,
            "clear_all": if with_receipt {
                serde_json::json!({"operation_id": "pending-clear"})
            } else {
                serde_json::Value::Null
            }
        })
        .to_string()
    }

    fn merge'''
text, count = re.subn(pattern, replacement, text, count=1, flags=re.S)
if count != 1:
    raise SystemExit(f"checkpoint fixture: expected one match, found {count}")
text = re.sub(r'\n\s*legacy_recovery_committed: false,', '', text)
text = re.sub(r'\n\s*legacy_transition: None,', '', text)
text = re.sub(r'\n\s*legacy_finish: None,', '', text)
path.write_text(text)

print("obsolete checkpoint receipt fields removed")
