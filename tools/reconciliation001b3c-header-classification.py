from pathlib import Path

path = Path("src/app/recovery_statement.rs")
content = path.read_text()
old = '''            Line::from(Span::styled(
                "RECOVERY EVIDENCE",
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
'''
new = '''            Line::from(Span::styled(
                "RECOVERY EVIDENCE",
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                format!(
                    "{} -> {}",
                    statement.recovered_interval_class.label(),
                    statement.post_target_class.label()
                ),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
'''
if old not in content:
    raise SystemExit("recovery modal heading marker missing")
path.write_text(content.replace(old, new, 1))
