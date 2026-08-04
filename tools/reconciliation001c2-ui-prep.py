from pathlib import Path

path = Path("tools/reconciliation001c2-ui.py")
content = path.read_text()

# Normalize the lifecycle insertion marker to the current mandatory-action API
# before the main transformation executes.
content = content.replace(
    "if self.keymap.matches_mandatory(key, Action::Quit)",
    "if self.keymap.mandatory_action_for_key_event(key) == Some(Action::Quit)",
)

# The retired-ID high-watermark is now added by the post-transform adapter
# against the current load-state structure, so remove the obsolete earlier block.
start = content.find('replace_once(\n    "src/sqlite/tui_runtime.rs",')
if start < 0:
    raise SystemExit("obsolete TUI high-watermark transform block missing")
end_marker = "\n\n# App state and module wiring."
end = content.find(end_marker, start)
if end < 0:
    raise SystemExit("TUI transform block end missing")
path.write_text(content[:start] + content[end:])
