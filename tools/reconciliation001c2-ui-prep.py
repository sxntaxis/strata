from pathlib import Path

path = Path("tools/reconciliation001c2-ui.py")
content = path.read_text()
start = content.find('replace_once(\n    "src/sqlite/tui_runtime.rs",')
if start < 0:
    raise SystemExit("obsolete TUI high-watermark transform block missing")
end_marker = "\n\n# App state and module wiring."
end = content.find(end_marker, start)
if end < 0:
    raise SystemExit("TUI transform block end missing")
path.write_text(content[:start] + content[end:])
