use ratatui::prelude::{Line, Span};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, BorderType, Borders, Paragraph},
};

use crate::{
    domain::{CategoryId, ReportPeriod},
    keybindings::Action,
};

use super::{App, PaletteCommand, PaletteEntry, view_style};

impl App {
    pub(super) fn render_command_palette(&mut self, f: &mut Frame, terminal_size: Rect) {
        let entries = self.filtered_command_palette_entries();
        self.clamp_command_palette_selection(entries.len());

        let accent = Color::Cyan;
        let rect = self.command_palette_rect(terminal_size);

        let frame_block = Block::default()
            .title(
                Line::from(Span::styled(
                    "command palette",
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ))
                .alignment(Alignment::Center),
            )
            .title_bottom(
                Line::from(Span::styled(
                    "Enter run · Esc close · ↑↓ move",
                    Style::default().fg(Color::DarkGray),
                ))
                .alignment(Alignment::Right),
            )
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(accent));

        f.render_widget(ratatui::widgets::Clear, rect);
        f.render_widget(frame_block.clone(), rect);

        let inner = frame_block.inner(rect);
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(1)])
            .split(inner);

        let prompt_line = if self.command_palette_query.is_empty() {
            Line::from(vec![
                Span::styled(
                    "> ",
                    Style::default().fg(accent).add_modifier(Modifier::BOLD),
                ),
                Span::styled("Type a command...", Style::default().fg(Color::DarkGray)),
            ])
        } else {
            Line::from(vec![
                Span::styled(
                    "> ",
                    Style::default().fg(accent).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    self.command_palette_query.clone(),
                    Style::default().fg(Color::White),
                ),
            ])
        };
        f.render_widget(Paragraph::new(prompt_line), chunks[0]);

        let list_area = chunks[1];
        let selected_index = if entries.is_empty() {
            None
        } else {
            Some(self.command_palette_selected_index)
        };

        let viewport_rows = list_area.height as usize;
        if entries.is_empty() {
            self.command_palette_scroll = 0;
        } else if let Some(selected) = selected_index {
            if self.command_palette_scroll > selected {
                self.command_palette_scroll = selected;
            } else if selected >= self.command_palette_scroll.saturating_add(viewport_rows)
                && viewport_rows > 0
            {
                self.command_palette_scroll = selected + 1 - viewport_rows;
            }

            let max_scroll = entries.len().saturating_sub(viewport_rows);
            self.command_palette_scroll = self.command_palette_scroll.min(max_scroll);
        }

        let row_width = list_area.width as usize;
        let lines: Vec<Line<'static>> = if entries.is_empty() {
            vec![Line::from(Span::styled(
                "No commands found.",
                Style::default().fg(Color::DarkGray),
            ))]
        } else {
            entries
                .iter()
                .enumerate()
                .map(|(idx, entry)| {
                    let is_selected = selected_index == Some(idx);
                    self.command_palette_line(entry, row_width, is_selected, accent)
                })
                .collect()
        };

        let list = Paragraph::new(lines).scroll((self.command_palette_scroll as u16, 0));
        f.render_widget(list, list_area);
    }

    pub(super) fn filtered_command_palette_entries(&self) -> Vec<PaletteEntry> {
        let query = self.command_palette_query.trim().to_ascii_lowercase();

        let mut scored: Vec<(usize, usize, PaletteEntry)> = self
            .command_palette_entries()
            .into_iter()
            .enumerate()
            .filter_map(|(idx, entry)| {
                let score = palette_match_score(&query, &entry.search_text)?;
                Some((score, idx, entry))
            })
            .collect();

        scored.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
        scored.into_iter().map(|(_, _, entry)| entry).collect()
    }

    fn command_palette_entries(&self) -> Vec<PaletteEntry> {
        let mut entries = vec![
            self.palette_action_entry(
                Action::OpenCategoryModal,
                "Open layer pop-up",
                &["layers", "strata", "edit", "popup"],
            ),
            self.palette_action_entry(
                Action::OpenReportModal,
                "Open karma pop-up",
                &["karma", "report", "popup"],
            ),
            self.palette_action_entry(
                Action::ToggleKeybindingsHelp,
                "Open command atlas",
                &["atlas", "help", "keybindings", "commands"],
            ),
            self.palette_action_entry(
                Action::SwitchToNone,
                "Switch layer: drift",
                &["drift", "idle", "none", "neutral"],
            ),
            self.palette_period_entry(ReportPeriod::Today, Action::ReportToday, "Karma range: day"),
            self.palette_period_entry(ReportPeriod::Week, Action::ReportWeek, "Karma range: week"),
            self.palette_period_entry(
                ReportPeriod::Month,
                Action::ReportMonth,
                "Karma range: month",
            ),
            self.palette_action_entry(
                Action::ClearAllSand,
                "Clear all sand",
                &["reset", "wipe", "clear"],
            ),
            self.palette_action_entry(
                Action::ClearNoneSand,
                "Clear drift sand",
                &["clear", "drift", "none", "idle"],
            ),
            self.palette_action_entry(Action::Quit, "Quit Strata", &["exit", "close", "stop"]),
        ];

        for category in self.time_tracker.categories_ordered() {
            if category.id == CategoryId::new(0) {
                continue;
            }

            let layer_name = self.display_layer_name(&category.name);
            let title = format!("Switch layer: {layer_name}");
            let search_text = format!(
                "switch layer focus activate {} {}",
                layer_name.to_ascii_lowercase(),
                category.description.to_ascii_lowercase()
            );

            entries.push(PaletteEntry {
                command: PaletteCommand::SwitchLayer(category.id),
                title,
                search_text,
                hint: String::new(),
            });
        }

        entries
    }

    fn palette_action_entry(&self, action: Action, title: &str, aliases: &[&str]) -> PaletteEntry {
        let hint = self.palette_hint_for_action(action);
        let search_text = format!(
            "{} {} {}",
            title.to_ascii_lowercase(),
            action.config_name(),
            aliases.join(" ")
        );

        PaletteEntry {
            command: PaletteCommand::Action(action),
            title: title.to_string(),
            search_text,
            hint,
        }
    }

    fn palette_period_entry(
        &self,
        period: ReportPeriod,
        hint_action: Action,
        title: &str,
    ) -> PaletteEntry {
        let hint = self.palette_hint_for_action(hint_action);
        let search_text = format!(
            "{} karma report {}",
            title.to_ascii_lowercase(),
            period_search_label(period)
        );

        PaletteEntry {
            command: PaletteCommand::SetReportPeriod(period),
            title: title.to_string(),
            search_text,
            hint,
        }
    }

    fn palette_hint_for_action(&self, action: Action) -> String {
        let keys = self.effective_keys_for_action(action);
        if keys.is_empty() {
            String::new()
        } else {
            keys.into_iter()
                .map(|key| key.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        }
    }

    fn command_palette_line(
        &self,
        entry: &PaletteEntry,
        row_width: usize,
        is_selected: bool,
        accent: Color,
    ) -> Line<'static> {
        let max_hint_width = row_width.saturating_div(3).max(1);
        let hint = if entry.hint.is_empty() {
            String::new()
        } else {
            self.truncate_label(&entry.hint, max_hint_width)
        };
        let hint_width = hint.chars().count();
        let separator_width = if hint.is_empty() { 0 } else { 1 };
        let title_width = row_width
            .saturating_sub(hint_width + separator_width)
            .max(1);

        let title = self.truncate_label(&entry.title, title_width);
        let title_pad = title_width.saturating_sub(title.chars().count());
        let spacer = if hint.is_empty() {
            String::new()
        } else {
            " ".to_string()
        };

        if is_selected {
            let text_color = view_style::text_color_for_bg(accent);
            Line::from(vec![
                Span::styled(title, Style::default().fg(text_color).bg(accent)),
                Span::styled(
                    " ".repeat(title_pad),
                    Style::default().fg(text_color).bg(accent),
                ),
                Span::styled(spacer, Style::default().fg(text_color).bg(accent)),
                Span::styled(
                    hint,
                    Style::default()
                        .fg(text_color)
                        .bg(accent)
                        .add_modifier(Modifier::BOLD),
                ),
            ])
        } else {
            Line::from(vec![
                Span::styled(title, Style::default().fg(Color::White)),
                Span::styled(" ".repeat(title_pad), Style::default().fg(Color::White)),
                Span::styled(spacer, Style::default().fg(Color::White)),
                Span::styled(hint, Style::default().fg(Color::DarkGray)),
            ])
        }
    }

    fn command_palette_rect(&self, terminal_size: Rect) -> Rect {
        let target_width = terminal_size.width.saturating_mul(5) / 6;
        let target_height = terminal_size.height.saturating_mul(1) / 2;

        let max_width = terminal_size.width.saturating_sub(2).max(1);
        let max_height = terminal_size.height.saturating_sub(2).max(1);

        let modal_width = target_width.clamp(1, max_width);
        let modal_height = target_height.max(8).clamp(1, max_height);

        let modal_x = (terminal_size.width.saturating_sub(modal_width)) / 2;
        let modal_y = (terminal_size.height.saturating_sub(modal_height)) / 3;

        Rect::new(modal_x, modal_y, modal_width, modal_height)
    }
}

fn period_search_label(period: ReportPeriod) -> &'static str {
    match period {
        ReportPeriod::Today => "day today d",
        ReportPeriod::Week => "week w",
        ReportPeriod::Month => "month m",
    }
}

fn palette_match_score(query: &str, haystack: &str) -> Option<usize> {
    if query.is_empty() {
        return Some(0);
    }

    let mut total_score = 0usize;
    for token in query.split_whitespace().filter(|token| !token.is_empty()) {
        total_score += palette_token_score(token, haystack)?;
    }

    Some(total_score)
}

fn palette_token_score(token: &str, haystack: &str) -> Option<usize> {
    if haystack == token {
        return Some(0);
    }

    if haystack.starts_with(token) {
        return Some(2);
    }

    if haystack
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .any(|word| !word.is_empty() && word.starts_with(token))
    {
        return Some(6);
    }

    if let Some(position) = haystack.find(token) {
        return Some(12 + position);
    }

    if let Some(distance) = best_typo_distance(token, haystack, 2) {
        return Some(24 + distance * 4);
    }

    palette_subsequence_score(token, haystack).map(|score| 60 + score)
}

fn best_typo_distance(token: &str, haystack: &str, max_distance: usize) -> Option<usize> {
    let mut best = levenshtein_distance_with_limit(token, haystack, max_distance);

    for word in haystack.split(|ch: char| !ch.is_ascii_alphanumeric()) {
        if word.is_empty() {
            continue;
        }

        if let Some(distance) = levenshtein_distance_with_limit(token, word, max_distance) {
            best = Some(best.map_or(distance, |existing| existing.min(distance)));
        }
    }

    best
}

fn levenshtein_distance_with_limit(a: &str, b: &str, limit: usize) -> Option<usize> {
    if a == b {
        return Some(0);
    }

    let a_bytes = a.as_bytes();
    let b_bytes = b.as_bytes();

    if a_bytes.len().abs_diff(b_bytes.len()) > limit {
        return None;
    }

    let mut previous: Vec<usize> = (0..=b_bytes.len()).collect();
    let mut current = vec![0usize; b_bytes.len() + 1];

    for (i, &a_char) in a_bytes.iter().enumerate() {
        current[0] = i + 1;
        let mut row_min = current[0];

        for (j, &b_char) in b_bytes.iter().enumerate() {
            let replace_cost = if a_char == b_char { 0 } else { 1 };
            current[j + 1] = (previous[j + 1] + 1)
                .min(current[j] + 1)
                .min(previous[j] + replace_cost);
            row_min = row_min.min(current[j + 1]);
        }

        if row_min > limit {
            return None;
        }

        std::mem::swap(&mut previous, &mut current);
    }

    let distance = previous[b_bytes.len()];
    if distance <= limit {
        Some(distance)
    } else {
        None
    }
}

fn palette_subsequence_score(token: &str, haystack: &str) -> Option<usize> {
    let mut cursor = 0usize;
    let mut score = 0usize;

    for query_char in token.chars() {
        let slice = haystack.get(cursor..)?;
        let mut found = None;

        for (idx, candidate) in slice.char_indices() {
            if candidate == query_char {
                found = Some(idx);
                break;
            }
        }

        let index = found?;
        score += index;
        cursor += index + query_char.len_utf8();
    }

    Some(score)
}

#[cfg(test)]
mod tests {
    use super::{
        levenshtein_distance_with_limit, palette_match_score, palette_subsequence_score,
        palette_token_score,
    };

    #[test]
    fn test_palette_match_score_handles_empty_query() {
        assert_eq!(palette_match_score("", "open layer pop-up"), Some(0));
    }

    #[test]
    fn test_palette_token_score_prefers_prefix_over_contains() {
        let prefix = palette_token_score("open", "open layer pop-up").expect("prefix match");
        let contains = palette_token_score("layer", "open layer pop-up").expect("contains");

        assert!(prefix < contains);
    }

    #[test]
    fn test_palette_subsequence_score_matches_initials() {
        assert!(palette_subsequence_score("sl", "switch layer: work").is_some());
    }

    #[test]
    fn test_palette_token_score_accepts_small_typos() {
        let exact = palette_token_score("karma", "karma range day").expect("exact match");
        let typo = palette_token_score("karmma", "karma range day").expect("typo match");

        assert!(exact < typo);
    }

    #[test]
    fn test_levenshtein_distance_limit_rejects_large_typos() {
        assert_eq!(levenshtein_distance_with_limit("karma", "krmzz", 2), None);
    }
}
