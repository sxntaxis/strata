from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    if old not in text:
        raise SystemExit(f"{label} anchor missing")
    return text.replace(old, new, 1)

# Export the transaction boundary and keep the removed destructive API absent.
root_path = Path("src/sqlite.rs")
root = root_path.read_text()
root = replace_once(
    root,
    "    archive_category as archive_tui_category, clear_checkpoint as clear_tui_checkpoint,",
    "    archive_category as archive_tui_category, clear_all_state as clear_tui_state,\n    clear_checkpoint as clear_tui_checkpoint,",
    "SQLite clear-all export",
)
root_path.write_text(root)


# Remove the now-obsolete persistence fault case referencing the deleted API.
fault_path = Path("src/sqlite/fault_certification.rs")
fault = fault_path.read_text()
marker = 'tui_runtime::delete_drift_sessions_for_day(path, "2026-08-01")'
index = fault.find(marker)
if index != -1:
    start = fault.rfind("        PersistenceFaultCase {", 0, index)
    end = fault.find("        PersistenceFaultCase {", index)
    if start == -1 or end == -1:
        raise SystemExit("could not bound obsolete drift fault case")
    fault = fault[:start] + fault[end:]
fault_path.write_text(fault)
