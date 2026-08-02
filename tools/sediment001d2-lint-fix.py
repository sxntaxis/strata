from pathlib import Path


# Daily fault certification now reads the typed envelope payload.
fault = Path("src/sqlite/fault_certification.rs")
text = fault.read_text()
start = text.index('    with_database("daily-snapshot",')
end = text.index('    with_database("daily-snapshot-delete",', start)
block = text[start:end]
old = """                .unwrap()
                .frame_count,"""
new = """                .unwrap()
                .state
                .frame_count,"""
if block.count(old) != 1:
    raise SystemExit("typed daily fault assertion was not found")
block = block.replace(old, new, 1)
fault.write_text(text[:start] + block + text[end:])

# The full-state flush writes canonical state directly and no longer needs a local copy.
persistence = Path("src/app/persistence_recovery.rs")
text = persistence.read_text()
start = text.index("    pub(super) fn try_flush_current_state(")
end = text.index("    fn try_reload_authority", start)
block = text[start:end]
old = "        let state = self.sand_engine.snapshot_state();\n"
if block.count(old) != 1:
    raise SystemExit("unused flush state binding was not found")
block = block.replace(old, "", 1)
persistence.write_text(text[:start] + block + text[end:])

# Legacy-envelope constructors remain test evidence after D2 stops reading legacy rows as authority.
snapshot = Path("src/sand/snapshot.rs")
text = snapshot.read_text()
for signature in [
    "    pub fn cumulative_checkpoint(\n",
    "    pub fn legacy_daily_payload(operational_day: String, state: SandState) -> Self {\n",
]:
    if text.count(signature) != 1:
        raise SystemExit(f"snapshot test constructor was not found: {signature!r}")
    text = text.replace(signature, "    #[cfg(test)]\n" + signature, 1)
snapshot.write_text(text)

# The old daily history path remains available only to custody/regression tests.
storage = Path("src/storage.rs")
text = storage.read_text()
signature = "pub fn get_sand_history_path_for_day(day: NaiveDate) -> PathBuf {\n"
if text.count(signature) != 1:
    raise SystemExit("legacy history path function was not found")
storage.write_text(text.replace(signature, "#[cfg(test)]\n" + signature, 1))

Path(__file__).unlink(missing_ok=True)
