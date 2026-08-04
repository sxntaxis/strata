from pathlib import Path

path = Path("src/app/recovery_statement.rs")
content = path.read_text()
content = content.replace("use super::{App, RecoveryStatement};", "use super::App;", 1)
content = content.replace(
    "            labelled(\"Active identity\", active_identity),",
    "            labelled(\"Active identity\", active_identity),",
    1,
)
content = content.replace(
    "            labelled(\"Category\", &statement.active_category_id.to_string()),",
    "            labelled(\"Category\", statement.active_category_id.to_string()),",
    1,
)
content = content.replace(
    "            labelled(\"Description\", description),",
    "            labelled(\"Description\", description),",
    1,
)
for field, label in [
    ("active_session_started_at_utc", "Active started"),
    ("checkpoint_captured_at_utc", "Checkpoint captured"),
    ("checkpoint_simulation_at_utc", "Durable sediment through"),
    ("recovery_target_utc", "Recovery target"),
]:
    old = f'''            labelled(\n                "{label}",\n                &format_timestamp(statement.{field}),\n            ),'''
    new = f'''            labelled(\n                "{label}",\n                format_timestamp(statement.{field}),\n            ),'''
    if old not in content:
        raise SystemExit(f"timestamp render marker missing for {field}")
    content = content.replace(old, new, 1)
content = content.replace(
    '''            labelled(
                "Reconstructed duration",
                &format_duration(statement.reconstructed_duration_nanos),
            ),''',
    '''            labelled(
                "Reconstructed duration",
                format_duration(statement.reconstructed_duration_nanos),
            ),''',
    1,
)
old = '''fn labelled<'a>(label: &'a str, value: &'a str) -> Line<'a> {
    Line::from(vec![
        Span::styled(
            format!("{label}: "),
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::raw(value),
    ])
}
'''
new = '''fn labelled(label: &str, value: impl Into<String>) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!("{label}: "),
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::raw(value.into()),
    ])
}
'''
if old not in content:
    raise SystemExit("labelled helper marker missing")
path.write_text(content.replace(old, new, 1))
