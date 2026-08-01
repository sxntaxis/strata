from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    target = Path(path)
    text = target.read_text()
    if old not in text:
        raise SystemExit(f"missing correction anchor in {path}: {old!r}")
    target.write_text(text.replace(old, new, 1))


replace_once(
    "src/cli.rs",
    "pub fn run_cli() {\n    let cli = Cli::parse();\n    match cli {",
    "pub fn run_command(cli: Cli) {\n    match cli {",
)
replace_once(
    "src/app.rs",
    "            time_log_path: loaded_time_log_path,",
    "            time_log_path: _,",
)
replace_once(
    "tests/config_authority.rs",
    "    path::{Path, PathBuf},",
    "    path::PathBuf,",
)
