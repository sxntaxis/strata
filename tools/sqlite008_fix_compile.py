from pathlib import Path

path = Path("src/sqlite/tui_runtime.rs")
text = path.read_text()
old = '''        assert_eq!(
            load_daily_snapshot(&path, "2026-08-01").unwrap(),
            Some(state)
        );'''
new = '''        assert_eq!(
            load_daily_snapshot(&path, "2026-08-01").unwrap(),
            Some(state.clone())
        );'''
if old not in text:
    raise SystemExit("missing recovery fixture ownership anchor")
path.write_text(text.replace(old, new, 1))
