use ratatui::prelude::{Line, Span};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, BorderType, Borders, Paragraph},
};

use crate::{
    constants::{APP_LAYOUT_SETTINGS, COMMAND_PALETTE_SETTINGS},
    domain::ReportPeriod,
    keybindings::{Action, ActionBindingState},
};

use super::{App, PaletteCommand, PaletteEntry, view_style};

impl App {
    pub(super) fn render_command_palette(&mut self, f: &mut Frame, terminal_size: Rect) {
        let entries = self.filtered_command_palette_entries();
        self.clamp_command_palette_selection(entries.len());

        let accent = Color::Cyan;
        let rect = self.command_palette_rect(terminal_size);

        let footer = if let Some(feedback) = self.command_palette_feedback.as_ref() {
            Line::from(Span::styled(
                feedback.clone(),
                Style::default().fg(Color::LightRed),
            ))
            .alignment(Alignment::Right)
        } else {
            Line::from(Span::styled(
                "Enter run · Esc close · ↑↓ move",
                Style::default().fg(Color::DarkGray),
            ))
            .alignment(Alignment::Right)
        };

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
            .title_bottom(footer)
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
                "Open balance pop-up",
                &["balance", "report", "popup"],
            ),
            self.palette_action_entry(
                Action::Detach,
                "Detach Strata (keep tracking)",
                &["detach", "detached", "dettached", "headless", "background"],
            ),
            self.palette_action_entry(
                Action::ToggleKeybindingsHelp,
                "Open command atlas",
                &["atlas", "help", "keybindings", "commands"],
            ),
            self.palette_action_entry(
                Action::SwitchToNone,
                "Switch layer: idle",
                &["drift", "idle", "none", "neutral"],
            ),
            self.palette_period_entry(
                ReportPeriod::Today,
                Action::ReportToday,
                "Balance range: day",
            ),
            self.palette_period_entry(
                ReportPeriod::Week,
                Action::ReportWeek,
                "Balance range: week",
            ),
            self.palette_period_entry(
                ReportPeriod::Month,
                Action::ReportMonth,
                "Balance range: month",
            ),
            self.palette_action_entry(
                Action::ClearAllSand,
                "Clear all sand",
                &["reset", "wipe", "clear"],
            ),
            self.palette_action_entry(
                Action::ClearNoneSand,
                "Clear idle sand",
                &["clear", "drift", "none", "idle"],
            ),
            self.palette_action_entry(Action::Quit, "Quit Strata", &["exit", "close", "stop"]),
        ];

        for category in self.time_tracker.categories_ordered() {
            if crate::domain::is_drift_category_id(category.id) {
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

        entries.retain(|entry| match entry.command {
            PaletteCommand::Action(action) => {
                self.keymap.action_state(action) != ActionBindingState::Disabled
            }
            PaletteCommand::SetReportPeriod(period) => {
                let action = match period {
                    ReportPeriod::Today => Action::ReportToday,
                    ReportPeriod::Week => Action::ReportWeek,
                    ReportPeriod::Month => Action::ReportMonth,
                };
                self.keymap.action_state(action) != ActionBindingState::Disabled
            }
            PaletteCommand::SwitchLayer(_) => true,
        });
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
            "{} balance report {}",
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
        if !keys.is_empty() {
            return keys
                .into_iter()
                .map(|key| key.to_string())
                .collect::<Vec<_>>()
                .join(", ");
        }
        match self.keymap.action_state(action) {
            ActionBindingState::Unbound => "unbound".to_string(),
            ActionBindingState::Disabled => "disabled".to_string(),
            ActionBindingState::Bound => String::new(),
        }
    }

    fn command_palette_line(
        &self,
        entry: &PaletteEntry,
        row_width: usize,
        is_selected: bool,
        accent: Color,
    ) -> Line<'static> {
        let max_hint_width = row_width
            .saturating_div(COMMAND_PALETTE_SETTINGS.hint_width_divisor)
            .max(1);
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
        let target_width = terminal_size
            .width
            .saturating_mul(COMMAND_PALETTE_SETTINGS.rect_width_num)
            / COMMAND_PALETTE_SETTINGS.rect_width_den;
        let target_height = terminal_size
            .height
            .saturating_mul(COMMAND_PALETTE_SETTINGS.rect_height_num)
            / COMMAND_PALETTE_SETTINGS.rect_height_den;

        let max_width = terminal_size
            .width
            .saturating_sub(APP_LAYOUT_SETTINGS.frame_margin)
            .saturating_sub(APP_LAYOUT_SETTINGS.frame_margin)
            .max(1);
        let max_height = terminal_size
            .height
            .saturating_sub(APP_LAYOUT_SETTINGS.frame_margin)
            .saturating_sub(APP_LAYOUT_SETTINGS.frame_margin)
            .max(1);

        let modal_width = target_width.clamp(1, max_width);
        let modal_height = target_height
            .max(COMMAND_PALETTE_SETTINGS.min_height)
            .clamp(1, max_height);

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
        return Some(COMMAND_PALETTE_SETTINGS.score_prefix);
    }

    if haystack
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .any(|word| !word.is_empty() && word.starts_with(token))
    {
        return Some(COMMAND_PALETTE_SETTINGS.score_word_prefix);
    }

    if let Some(position) = haystack.find(token) {
        return Some(COMMAND_PALETTE_SETTINGS.score_contains_base + position);
    }

    if let Some(distance) = best_typo_distance(token, haystack, 2) {
        return Some(
            COMMAND_PALETTE_SETTINGS.score_typo_base
                + distance * COMMAND_PALETTE_SETTINGS.score_typo_distance_weight,
        );
    }

    palette_subsequence_score(token, haystack)
        .map(|score| COMMAND_PALETTE_SETTINGS.score_subsequence_base + score)
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
        let exact = palette_token_score("balance", "balance range day").expect("exact match");
        let typo = palette_token_score("balnce", "balance range day").expect("typo match");

        assert!(exact < typo);
    }

    #[test]
    fn test_levenshtein_distance_limit_rejects_large_typos() {
        assert_eq!(levenshtein_distance_with_limit("balance", "krmzz", 2), None);
    }
}
