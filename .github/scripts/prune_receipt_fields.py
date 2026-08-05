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
# The fail-closed receipt proof now uses the only current transitional receipt.
text = text.replace(
    '"clear_all": null\n        })\n        .to_string();\n        assert!(checkpoint_has_transition_receipt(&payload).unwrap());',
    '"clear_all": {"operation_id": "pending-clear"}\n        })\n        .to_string();\n        assert!(checkpoint_has_transition_receipt(&payload).unwrap());',
    1,
)
path.write_text(text)

path = Path("src/sqlite/category_lifecycle.rs")
text = path.read_text()
text = re.sub(r'\n\s*legacy_recovery_committed: false,', '', text)
text = re.sub(r'\n\s*legacy_transition: None,', '', text)
text = re.sub(r'\n\s*legacy_finish: None,', '', text)
text = text.replace('"schema_version": 3,', '"schema_version": 1,')
text = re.sub(r'\n\s*"legacy_transition": (?:null|\{.*?\}),', '', text)
text = re.sub(r'\n\s*"legacy_finish": null,', '', text)
path.write_text(text)

print("obsolete checkpoint receipt fields removed")
