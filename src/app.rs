use std::{
    io,
    path::Path,
    time::{Duration, Instant},
};

use chrono::{Datelike, Local, NaiveDate};
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::prelude::{Line, Span};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph},
};

use crate::{
    constants::{BLINK_SETTINGS, COLORS, FACE_SETTINGS, FILE_PATHS, SAND_ENGINE, TIME_SETTINGS},
    domain::{
        CategoryId, KarmaReportSummary, ReportPeriod, TimeTracker, build_period_karma_report,
    },
    sand::SandEngine,
    storage,
};

struct App {
    time_tracker: TimeTracker,
    sand_engine: SandEngine,
    blink_state: i32,
    modal_open: bool,
    report_modal_open: bool,
    selected_index: usize,
    new_category_name: String,
    color_index: usize,
    modal_description: String,
    report_selected_index: usize,
    report_period: ReportPeriod,
    report_show_help: bool,
    render_needed: bool,
}

impl App {
    fn new(width: u16, height: u16) -> Self {
        let mut tracker = TimeTracker::new();
        let loaded_categories = storage::load_categories_from_csv(Path::new(FILE_PATHS.categories));
        let loaded_sessions = storage::load_sessions_from_csv(
            Path::new(FILE_PATHS.time_log),
            &loaded_categories.categories,
        );
        tracker.apply_loaded_state(
            loaded_categories.categories,
            loaded_categories.next_category_id,
            loaded_sessions.sessions,
            loaded_sessions.next_session_id,
        );

        let mut app = Self {
            time_tracker: tracker,
            sand_engine: SandEngine::new(width, height),
            blink_state: 0,
            modal_open: false,
            report_modal_open: false,
            selected_index: 0,
            new_category_name: String::new(),
            color_index: 0,
            modal_description: String::new(),
            report_selected_index: 0,
            report_period: ReportPeriod::Today,
            report_show_help: false,
            render_needed: true,
        };

        app.time_tracker.start_session();
        if app.time_tracker.active_category_index() == Some(0) {
            app.blink_state = app.next_blink_interval();
        }

        app
    }

    fn open_modal(&mut self) {
        self.modal_open = true;
        self.report_modal_open = false;
        self.selected_index = self.time_tracker.active_category_index().unwrap_or(0);
        self.new_category_name = String::new();
        self.color_index = 0;
        self.modal_description = self
            .time_tracker
            .category_description_by_index(self.selected_index)
            .unwrap_or_default();
        self.render_needed = true;
    }

    fn close_modal(&mut self) {
        self.modal_open = false;
        self.modal_description = String::new();
        self.render_needed = true;
    }

    fn open_report_modal(&mut self) {
        self.report_modal_open = true;
        self.modal_open = false;
        self.report_selected_index = 0;
        self.report_period = ReportPeriod::Today;
        self.report_show_help = false;
        self.render_needed = true;
    }

    fn close_report_modal(&mut self) {
        self.report_modal_open = false;
        self.report_show_help = false;
        self.render_needed = true;
    }

    fn modal_rect(&self, terminal_size: Rect) -> Rect {
        self.modal_rect_ratio(terminal_size, 1, 3)
    }

    fn modal_rect_ratio(&self, terminal_size: Rect, numerator: u16, denominator: u16) -> Rect {
        let target_width = terminal_size.width.saturating_mul(numerator) / denominator;
        let target_height = (terminal_size.height.saturating_mul(numerator) / denominator).max(10);

        let max_width = terminal_size.width.saturating_sub(2).max(1);
        let max_height = terminal_size.height.saturating_sub(2).max(1);

        let modal_width = target_width.clamp(1, max_width);
        let modal_height = target_height.clamp(1, max_height);

        let modal_x = (terminal_size.width.saturating_sub(modal_width)) / 2;
        let modal_y = (terminal_size.height.saturating_sub(modal_height)) / 2;

        Rect::new(modal_x, modal_y, modal_width, modal_height)
    }

    fn report_modal_rect(&self, terminal_size: Rect, row_count: usize) -> Rect {
        let compact = self.modal_rect(terminal_size);
        let inner_width = compact.width.saturating_sub(2);
        let inner_height = compact.height.saturating_sub(2);
        let footer_height = if self.report_show_help { 1 } else { 0 };
        let visible_rows = inner_height.saturating_sub(footer_height) as usize;

        let content_is_cramped = inner_width < 28 || row_count > visible_rows;
        if content_is_cramped {
            self.modal_rect_ratio(terminal_size, 2, 3)
        } else {
            compact
        }
    }

    fn report_chip(label: &str, active: bool) -> Span<'static> {
        let style = if active {
            Style::default()
                .fg(Color::Black)
                .bg(Color::White)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Gray)
        };

        let text = if active {
            format!("[{}]", label)
        } else {
            label.to_string()
        };

        Span::styled(text, style)
    }

    fn report_period_prev(period: ReportPeriod) -> ReportPeriod {
        match period {
            ReportPeriod::Today => ReportPeriod::Month,
            ReportPeriod::Week => ReportPeriod::Today,
            ReportPeriod::Month => ReportPeriod::Week,
        }
    }

    fn report_period_next(period: ReportPeriod) -> ReportPeriod {
        match period {
            ReportPeriod::Today => ReportPeriod::Week,
            ReportPeriod::Week => ReportPeriod::Month,
            ReportPeriod::Month => ReportPeriod::Today,
        }
    }

    fn format_report_interval_label(&self, raw: &str) -> String {
        let parse = |value: &str| NaiveDate::parse_from_str(value, "%Y-%m-%d").ok();

        if let Some((start_raw, end_raw)) = raw.split_once("..") {
            let (Some(start), Some(end)) = (parse(start_raw), parse(end_raw)) else {
                return raw.to_string();
            };

            if start.year() == end.year() && start.month() == end.month() {
                return format!("{}-{}", start.format("%b %-d"), end.format("%-d"));
            }

            if start.year() == end.year() {
                return format!("{}-{}", start.format("%b %-d"), end.format("%b %-d"));
            }

            return format!(
                "{}-{}",
                start.format("%b %-d, %Y"),
                end.format("%b %-d, %Y")
            );
        }

        parse(raw)
            .map(|date| date.format("%b %-d").to_string())
            .unwrap_or_else(|| raw.to_string())
    }

    fn report_rows(&self) -> KarmaReportSummary {
        let categories = self.time_tracker.categories_for_storage();
        let mut summary =
            build_period_karma_report(&self.time_tracker.sessions, &categories, self.report_period);
        summary.entries.sort_by(|a, b| {
            let none_id = CategoryId::new(0);
            let group = |entry: &crate::domain::KarmaReportEntry| -> u8 {
                if entry.category_id == none_id {
                    1
                } else if entry.karma_effect < 0 {
                    2
                } else {
                    0
                }
            };

            let ga = group(a);
            let gb = group(b);

            ga.cmp(&gb).then_with(|| match ga {
                0 => b
                    .karma_seconds
                    .cmp(&a.karma_seconds)
                    .then(b.elapsed_seconds.cmp(&a.elapsed_seconds))
                    .then(a.category_name.cmp(&b.category_name)),
                1 => {
                    let a_is_none = a.category_id == none_id;
                    let b_is_none = b.category_id == none_id;
                    b_is_none
                        .cmp(&a_is_none)
                        .then(b.elapsed_seconds.cmp(&a.elapsed_seconds))
                        .then(a.category_name.cmp(&b.category_name))
                }
                _ => a
                    .karma_seconds
                    .cmp(&b.karma_seconds)
                    .then(b.elapsed_seconds.cmp(&a.elapsed_seconds))
                    .then(a.category_name.cmp(&b.category_name)),
            })
        });
        summary
    }

    fn set_report_period(&mut self, period: ReportPeriod) {
        self.report_period = period;
        let row_count = self.report_rows().entries.len();
        self.clamp_report_selection(row_count);
    }

    fn clamp_report_selection(&mut self, row_count: usize) {
        if row_count == 0 {
            self.report_selected_index = 0;
        } else if self.report_selected_index >= row_count {
            self.report_selected_index = row_count - 1;
        }
    }

    fn persist_categories(&self) {
        let categories = self.time_tracker.categories_for_storage();
        let _ = storage::save_categories_to_csv(Path::new(FILE_PATHS.categories), &categories);
    }

    fn persist_sessions(&self) {
        let categories = self.time_tracker.categories_for_storage();
        let _ = storage::save_sessions_to_csv(
            Path::new(FILE_PATHS.time_log),
            &self.time_tracker.sessions,
            &categories,
        );
    }

    fn is_on_insert_space(&self) -> bool {
        self.selected_index == self.time_tracker.category_count()
    }

    fn add_category(&mut self) {
        if !self.new_category_name.is_empty() {
            let added = self.time_tracker.add_category(
                self.new_category_name.clone(),
                String::new(),
                Some(self.color_index),
            );
            if added.is_some() {
                let index = self.time_tracker.category_count().saturating_sub(1);
                let _ = self.time_tracker.set_active_category_by_index(index);
                self.time_tracker.start_session();
                self.persist_categories();
            }
        }
    }

    fn delete_category(&mut self) {
        if !self.is_on_insert_space()
            && self.selected_index < self.time_tracker.category_count()
            && self.selected_index > 0
        {
            if self.time_tracker.delete_category(self.selected_index) {
                if self.selected_index > 0
                    && self.selected_index >= self.time_tracker.category_count()
                {
                    self.selected_index = self.time_tracker.category_count();
                }
                self.persist_categories();
            }
        }
    }

    fn get_selected_color(&self) -> Color {
        if self.is_on_insert_space() {
            COLORS[self.color_index]
        } else if let Some(category) = self.time_tracker.category_by_index(self.selected_index) {
            category.color
        } else {
            Color::White
        }
    }

    fn get_active_color(&self) -> Color {
        if let Some(idx) = self.time_tracker.active_category_index() {
            if let Some(category) = self.time_tracker.category_by_index(idx) {
                return category.color;
            }
        }
        Color::White
    }

    fn get_effective_time_today(&self) -> usize {
        self.time_tracker.get_todays_time()
    }

    fn get_effective_time_for_category(&self, category_name: &str) -> usize {
        self.time_tracker.get_category_time(category_name)
    }

    fn get_karma_adjusted_time(&self) -> isize {
        let today = Local::now().format("%Y-%m-%d").to_string();
        let mut total: isize = 0;
        for cat in self.time_tracker.categories_ordered() {
            if cat.name == "none" {
                continue;
            }
            let cat_time: isize = self
                .time_tracker
                .sessions
                .iter()
                .filter(|s| s.date == today && s.category_id == cat.id)
                .map(|s| s.elapsed_seconds as isize)
                .sum();
            total += cat_time * cat.karma_effect as isize;
        }
        total
    }

    fn get_category_karma_adjusted_time(&self, category_name: &str) -> isize {
        let today = Local::now().format("%Y-%m-%d").to_string();
        let categories = self.time_tracker.categories_ordered();
        let cat = categories.iter().find(|c| c.name == category_name);
        if let Some(cat) = cat {
            let cat_time: isize = self
                .time_tracker
                .sessions
                .iter()
                .filter(|s| s.date == today && s.category_id == cat.id)
                .map(|s| s.elapsed_seconds as isize)
                .sum();
            cat_time * cat.karma_effect as isize
        } else {
            0
        }
    }

    fn format_signed_time(&self, seconds: isize) -> String {
        let abs_secs = seconds.abs() as usize;
        let sign = if seconds < 0 { "-" } else { "" };
        format!(
            "{}{:02}:{:02}:{:02}",
            sign,
            abs_secs / 3600,
            (abs_secs % 3600) / 60,
            abs_secs % 60
        )
    }

    fn format_karma_time(&self, seconds: isize) -> String {
        let abs_secs = seconds.abs() as usize;
        let sign = if seconds < 0 { "-" } else { "+" };
        format!(
            "{}{:02}:{:02}:{:02}",
            sign,
            abs_secs / 3600,
            (abs_secs % 3600) / 60,
            abs_secs % 60
        )
    }

    fn format_time(&self, seconds: usize) -> String {
        format!(
            "{:02}:{:02}:{:02}",
            seconds / 3600,
            (seconds % 3600) / 60,
            seconds % 60
        )
    }

    fn truncate_label(&self, value: &str, max_chars: usize) -> String {
        let count = value.chars().count();
        if count <= max_chars {
            return value.to_string();
        }

        if max_chars <= 3 {
            return value.chars().take(max_chars).collect();
        }

        let prefix: String = value.chars().take(max_chars - 3).collect();
        format!("{}...", prefix)
    }

    fn get_idle_face(&self) -> String {
        let idle_seconds = self
            .time_tracker
            .current_session_start
            .map_or(0, |s| s.elapsed().as_secs() as usize);

        if self.blink_state < 0 {
            "(-_-)".to_string()
        } else if self.blink_state > 0 {
            "(o_o)".to_string()
        } else {
            let faces = FACE_SETTINGS.faces;
            let thresholds = FACE_SETTINGS.thresholds;

            let mut face = faces[0];
            for (i, &threshold) in thresholds.iter().enumerate() {
                if idle_seconds >= threshold {
                    face = faces[i + 1];
                }
            }
            face.to_string()
        }
    }

    fn update_blink(&mut self) {
        if self.blink_state < 0 {
            self.blink_state -= 1;
            let blink_duration = BLINK_SETTINGS.duration_min_frames
                + (rand::random::<i32>()
                    % (BLINK_SETTINGS.duration_max_frames - BLINK_SETTINGS.duration_min_frames));
            if self.blink_state < -blink_duration {
                self.blink_state = self.next_blink_interval();
            }
        } else if self.blink_state > 0 {
            self.blink_state -= 1;
            if self.blink_state == 0 {
                self.blink_state = -1;
            }
        }
    }

    fn next_blink_interval(&self) -> i32 {
        BLINK_SETTINGS.interval_min_frames
            + (rand::random::<i32>()
                % (BLINK_SETTINGS.interval_max_frames - BLINK_SETTINGS.interval_min_frames))
    }

    fn text_color_for_bg(bg_color: Color) -> Color {
        if let Color::Rgb(r, g, b) = bg_color {
            let brightness = (299 * r as u32 + 587 * g as u32 + 114 * b as u32) / 1000;
            if brightness > 128 {
                Color::Black
            } else {
                Color::White
            }
        } else {
            Color::White
        }
    }

    fn karma_color(seconds: isize) -> Color {
        if seconds < 0 {
            Color::Red
        } else if seconds > 0 {
            Color::Green
        } else {
            Color::Gray
        }
    }

    fn render_modal(&self, f: &mut Frame, terminal_size: Rect) {
        let modal_rect = self.modal_rect(terminal_size);

        let border_color = self.get_selected_color();
        let categories = self.time_tracker.categories_ordered();

        let items: Vec<ListItem> = categories
            .iter()
            .enumerate()
            .map(|(i, cat)| {
                let is_selected = i == self.selected_index;
                let dot = if cat.karma_effect < 0 { "◯ " } else { "● " };

                if is_selected {
                    let text_color = Self::text_color_for_bg(cat.color);
                    let description_text = if self.modal_description.is_empty() {
                        Span::raw("")
                    } else {
                        Span::styled(
                            format!(" {}", self.modal_description),
                            Style::default().add_modifier(Modifier::ITALIC),
                        )
                    };
                    ListItem::new(Line::from(vec![
                        Span::raw(dot).fg(cat.color),
                        Span::raw(&cat.name).fg(text_color),
                        description_text,
                    ]))
                    .style(Style::default().fg(text_color).bg(cat.color))
                } else {
                    ListItem::new(Line::from(vec![
                        Span::raw(dot).fg(cat.color),
                        Span::raw(&cat.name).fg(Color::White),
                    ]))
                }
            })
            .chain(std::iter::once({
                let is_selected = self.is_on_insert_space();
                let cycling_color = COLORS[self.color_index];

                if is_selected {
                    ListItem::new(Line::from(vec![
                        Span::raw("● ").fg(cycling_color),
                        Span::raw(if self.new_category_name.is_empty() {
                            "+ Add new..."
                        } else {
                            &self.new_category_name
                        }),
                    ]))
                    .style(Style::default().fg(Color::Black).bg(Color::White))
                } else {
                    ListItem::new(Line::from(vec![
                        Span::raw("● ").fg(cycling_color),
                        Span::raw(if self.new_category_name.is_empty() {
                            "+ Add new..."
                        } else {
                            &self.new_category_name
                        })
                        .fg(Color::White),
                    ]))
                }
            }))
            .collect();

        let mut list_state = ListState::default();
        list_state.select(Some(self.selected_index));

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .title(Line::from(Span::styled(
                        "strata",
                        Style::default().fg(Color::White),
                    )))
                    .title_alignment(ratatui::layout::Alignment::Center)
                    .border_style(Style::default().fg(border_color)),
            )
            .highlight_style(Style::default());

        f.render_widget(ratatui::widgets::Clear, modal_rect);
        f.render_stateful_widget(list, modal_rect, &mut list_state);
    }

    fn render_report_modal(&self, f: &mut Frame, terminal_size: Rect) {
        let summary = self.report_rows();
        let modal_rect = self.report_modal_rect(terminal_size, summary.entries.len());
        let selected_index = if summary.entries.is_empty() {
            None
        } else {
            Some(self.report_selected_index.min(summary.entries.len() - 1))
        };

        let interval_label = self.format_report_interval_label(&summary.date);

        let border_color = selected_index
            .and_then(|idx| summary.entries.get(idx))
            .map(|entry| entry.color)
            .unwrap_or(Color::White);

        let interval_title = Line::from(Span::styled(
            interval_label,
            Style::default().fg(Color::White),
        ))
        .alignment(Alignment::Left);

        let center_title = Line::from(Span::styled(
            "karma",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ))
        .alignment(Alignment::Center);

        let total_title = Line::from(Span::styled(
            self.format_karma_time(summary.total_karma_seconds),
            Style::default().fg(Self::karma_color(summary.total_karma_seconds)),
        ))
        .alignment(Alignment::Right);

        let period_bottom_title = Line::from(vec![
            Self::report_chip("today", self.report_period == ReportPeriod::Today),
            Span::raw(" "),
            Self::report_chip("week", self.report_period == ReportPeriod::Week),
            Span::raw(" "),
            Self::report_chip("month", self.report_period == ReportPeriod::Month),
        ])
        .alignment(Alignment::Center);

        let frame_block = Block::default()
            .title(interval_title)
            .title(center_title)
            .title(total_title)
            .title_bottom(period_bottom_title)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(border_color));

        f.render_widget(ratatui::widgets::Clear, modal_rect);
        f.render_widget(frame_block.clone(), modal_rect);

        let inner = frame_block.inner(modal_rect);
        let footer_height = if self.report_show_help { 1 } else { 0 };
        let vertical = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(3), Constraint::Length(footer_height)])
            .split(inner);

        let row_width = vertical[0].width as usize;
        let metric_width = 9;
        let name_width = row_width.saturating_sub(metric_width + 4).max(4);

        let items: Vec<ListItem> = summary
            .entries
            .iter()
            .enumerate()
            .map(|(idx, entry)| {
                let is_selected = selected_index == Some(idx);
                let dot = if entry.karma_effect < 0 {
                    "◯ "
                } else if entry.karma_effect == 0 {
                    "· "
                } else {
                    "● "
                };
                let name = self.truncate_label(&entry.category_name, name_width);
                let pad = name_width.saturating_sub(name.chars().count()) + 1;
                let is_none_row = entry.category_id == CategoryId::new(0);
                let metric_value = if is_none_row {
                    self.format_time(entry.elapsed_seconds)
                } else if entry.karma_seconds == 0 && entry.karma_effect < 0 {
                    "-00:00:00".to_string()
                } else {
                    self.format_karma_time(entry.karma_seconds)
                };
                let metric_color = if is_none_row {
                    Color::Gray
                } else if entry.karma_seconds == 0 {
                    if entry.karma_effect < 0 {
                        Color::Red
                    } else if entry.karma_effect > 0 {
                        Color::Green
                    } else {
                        Color::Gray
                    }
                } else {
                    Self::karma_color(entry.karma_seconds)
                };

                if is_selected {
                    let text_color = Self::text_color_for_bg(entry.color);
                    ListItem::new(Line::from(vec![
                        Span::raw(dot).fg(text_color),
                        Span::raw(name).fg(text_color),
                        Span::raw(" ".repeat(pad)).fg(text_color),
                        Span::raw(metric_value).fg(text_color),
                    ]))
                    .style(Style::default().fg(text_color).bg(entry.color))
                } else {
                    ListItem::new(Line::from(vec![
                        Span::raw(dot).fg(entry.color),
                        Span::raw(name).fg(Color::White),
                        Span::raw(" ".repeat(pad)).fg(Color::White),
                        Span::raw(metric_value).fg(metric_color),
                    ]))
                }
            })
            .collect();

        let mut list_state = ListState::default();
        list_state.select(selected_index);

        let list = if summary.entries.is_empty() {
            List::new(vec![ListItem::new(Line::from(vec![Span::styled(
                "No tracked sessions for this period.",
                Style::default().fg(Color::Gray),
            )]))])
        } else {
            List::new(items)
        };

        f.render_stateful_widget(list, vertical[0], &mut list_state);

        if self.report_show_help {
            let footer = Paragraph::new(Line::from(
                Span::raw("keys: up/down  shift+left/right  d/w/m  esc  ?").fg(Color::DarkGray),
            ));
            f.render_widget(footer, vertical[1]);
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> bool {
        if self.modal_open {
            self.handle_modal_key(key);
            false
        } else if self.report_modal_open {
            self.handle_report_modal_key(key);
            false
        } else {
            self.handle_normal_key(key.code)
        }
    }

    fn handle_modal_key(&mut self, key: KeyEvent) {
        let shift = key.modifiers.contains(KeyModifiers::SHIFT);

        match key.code {
            KeyCode::Esc => self.close_modal(),
            KeyCode::Up => {
                if shift {
                    if self.time_tracker.move_category_up(self.selected_index) {
                        self.selected_index -= 1;
                        self.persist_categories();
                    }
                } else if self.selected_index > 0 {
                    self.selected_index -= 1;
                }
            }
            KeyCode::Down => {
                if shift {
                    if self.time_tracker.move_category_down(self.selected_index) {
                        self.selected_index += 1;
                        self.persist_categories();
                    }
                } else {
                    let max_index = self.time_tracker.category_count();
                    if self.selected_index < max_index {
                        self.selected_index += 1;
                    }
                }
            }
            KeyCode::Left => {
                if shift && !self.is_on_insert_space() && self.selected_index > 0 {
                    let Some(current_color) = self
                        .time_tracker
                        .category_by_index(self.selected_index)
                        .map(|category| category.color)
                    else {
                        return;
                    };
                    let current_pos = COLORS
                        .iter()
                        .position(|&color| color == current_color)
                        .unwrap_or(0);
                    let new_pos = (current_pos + COLORS.len() - 1) % COLORS.len();
                    if self
                        .time_tracker
                        .set_category_color_by_index(self.selected_index, COLORS[new_pos])
                    {
                        self.persist_categories();
                    }
                } else if self.is_on_insert_space() {
                    self.color_index = (self.color_index + COLORS.len() - 1) % COLORS.len();
                }
            }
            KeyCode::Right => {
                if shift && !self.is_on_insert_space() && self.selected_index > 0 {
                    let Some(current_color) = self
                        .time_tracker
                        .category_by_index(self.selected_index)
                        .map(|category| category.color)
                    else {
                        return;
                    };
                    let current_pos = COLORS
                        .iter()
                        .position(|&color| color == current_color)
                        .unwrap_or(0);
                    let new_pos = (current_pos + 1) % COLORS.len();
                    if self
                        .time_tracker
                        .set_category_color_by_index(self.selected_index, COLORS[new_pos])
                    {
                        self.persist_categories();
                    }
                } else if self.is_on_insert_space() {
                    self.color_index = (self.color_index + 1) % COLORS.len();
                }
            }
            KeyCode::Enter => {
                if self.is_on_insert_space() {
                    if !self.new_category_name.is_empty() {
                        self.add_category();
                        self.close_modal();
                    }
                } else {
                    if self.selected_index < self.time_tracker.category_count() {
                        if self.time_tracker.set_category_description_by_index(
                            self.selected_index,
                            self.modal_description.clone(),
                        ) {
                            self.persist_categories();
                        }
                    }
                    if self.time_tracker.active_category_index() != Some(self.selected_index) {
                        self.time_tracker.end_session();
                        self.persist_sessions();
                        let _ = self
                            .time_tracker
                            .set_active_category_by_index(self.selected_index);
                        self.time_tracker.start_session();
                    }
                    self.close_modal();
                }
            }
            KeyCode::Char('x') => {
                if !self.is_on_insert_space() && self.selected_index > 0 {
                    self.delete_category();
                }
            }
            KeyCode::Char('+') | KeyCode::Char('=') => {
                if !self.is_on_insert_space()
                    && self.selected_index > 0
                    && self.selected_index < self.time_tracker.category_count()
                {
                    if self
                        .time_tracker
                        .set_category_karma_by_index(self.selected_index, 1)
                    {
                        self.persist_categories();
                    }
                }
            }
            KeyCode::Char('-') | KeyCode::Char('_') => {
                if !self.is_on_insert_space()
                    && self.selected_index > 0
                    && self.selected_index < self.time_tracker.category_count()
                {
                    if self
                        .time_tracker
                        .set_category_karma_by_index(self.selected_index, -1)
                    {
                        self.persist_categories();
                    }
                }
            }
            KeyCode::Char(c) => {
                if self.is_on_insert_space() {
                    self.new_category_name.push(c);
                } else if self.selected_index < self.time_tracker.category_count() {
                    self.modal_description.push(c);
                }
            }
            KeyCode::Backspace => {
                if self.is_on_insert_space() {
                    self.new_category_name.pop();
                } else if self.selected_index < self.time_tracker.category_count() {
                    self.modal_description.pop();
                }
            }
            _ => {}
        }
    }

    fn handle_report_modal_key(&mut self, key: KeyEvent) {
        let summary = self.report_rows();
        self.clamp_report_selection(summary.entries.len());
        let shift = key.modifiers.contains(KeyModifiers::SHIFT);

        match key.code {
            KeyCode::Esc | KeyCode::Enter => self.close_report_modal(),
            KeyCode::Up => {
                if self.report_selected_index > 0 {
                    self.report_selected_index -= 1;
                }
            }
            KeyCode::Down => {
                if self.report_selected_index + 1 < summary.entries.len() {
                    self.report_selected_index += 1;
                }
            }
            KeyCode::Left if shift => {
                self.set_report_period(Self::report_period_prev(self.report_period));
            }
            KeyCode::Right if shift => {
                self.set_report_period(Self::report_period_next(self.report_period));
            }
            KeyCode::Char('d') | KeyCode::Char('D') => {
                self.set_report_period(ReportPeriod::Today);
            }
            KeyCode::Char('w') | KeyCode::Char('W') => {
                self.set_report_period(ReportPeriod::Week);
            }
            KeyCode::Char('m') | KeyCode::Char('M') => {
                self.set_report_period(ReportPeriod::Month);
            }
            KeyCode::Char('?') => {
                self.report_show_help = !self.report_show_help;
            }
            _ => {}
        }

        self.render_needed = true;
    }

    fn handle_normal_key(&mut self, key: KeyCode) -> bool {
        match key {
            KeyCode::Char('q') => true,
            KeyCode::Char('c') => {
                self.sand_engine.clear();
                self.time_tracker.reset_none_counter_today();
                self.persist_sessions();
                false
            }
            KeyCode::Char('k') | KeyCode::Char('K') => {
                self.open_report_modal();
                false
            }
            KeyCode::Enter => {
                self.open_modal();
                false
            }
            KeyCode::Esc => {
                self.time_tracker.end_session();
                self.persist_sessions();
                let _ = self.time_tracker.set_active_category_by_index(0);
                self.time_tracker.start_session();
                false
            }
            _ => false,
        }
    }
}

pub fn run_ui() -> Result<(), io::Error> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let size = terminal.size()?;
    let mut app = App::new(size.width, size.height);

    let physics_rate = Duration::from_millis(TIME_SETTINGS.physics_ms);
    let tick_rate = Duration::from_millis(TIME_SETTINGS.tick_ms);
    let render_rate = Duration::from_millis(1000 / TIME_SETTINGS.target_fps);
    let save_rate = Duration::from_secs(60);
    let mut last_spawn = Instant::now();
    let mut last_physics = Instant::now();
    let mut last_render = Instant::now();
    let mut last_save = Instant::now();

    loop {
        if last_spawn.elapsed() >= tick_rate {
            let should_spawn = app.time_tracker.current_session_start.is_some()
                && app.time_tracker.active_category_index().is_some();

            if should_spawn {
                let cat_id = app.time_tracker.active_category_id();
                app.sand_engine.spawn(cat_id);
                app.render_needed = true;
            }

            last_spawn = Instant::now();
        }

        if last_physics.elapsed() >= physics_rate {
            app.sand_engine.update();
            app.render_needed = true;
            if app.time_tracker.active_category_index() == Some(0) {
                app.update_blink();
            }
            last_physics = Instant::now();
        }

        if last_save.elapsed() >= save_rate {
            app.persist_sessions();
            last_save = Instant::now();
        }

        if last_render.elapsed() >= render_rate && app.render_needed {
            terminal.draw(|f| {
                let size = f.size();

                let inner_width = size.width.saturating_sub(2);
                let inner_height = size.height.saturating_sub(2);

                if app.sand_engine.width != inner_width * SAND_ENGINE.dot_width as u16
                    || app.sand_engine.height != inner_height * SAND_ENGINE.dot_height as u16
                {
                    app.sand_engine.resize(inner_width, inner_height);
                }

                let categories = app.time_tracker.categories_ordered();
                let sand = app.sand_engine.render(&categories);
                let active_index = app.time_tracker.active_category_index();

                let category_name = if active_index == Some(0) {
                    app.get_idle_face()
                } else if let Some(idx) = active_index {
                    categories
                        .get(idx)
                        .map(|c| c.name.clone())
                        .unwrap_or_else(|| app.get_idle_face())
                } else {
                    app.get_idle_face()
                };

                let description = active_index
                    .and_then(|idx| {
                        categories
                            .get(idx)
                            .map(|category| category.description.clone())
                    })
                    .unwrap_or_default();

                let session_timer = if active_index == Some(0) {
                    Local::now().format("%H:%M:%S").to_string()
                } else if let Some(start) = app.time_tracker.current_session_start {
                    let elapsed = start.elapsed();
                    app.format_time(elapsed.as_secs() as usize)
                } else {
                    Local::now().format("%H:%M:%S").to_string()
                };

                let effective_time_str = if app.modal_open {
                    let cat_name = categories
                        .get(app.selected_index)
                        .map(|category| category.name.as_str())
                        .unwrap_or("none");
                    let karma_time = if cat_name == "none" {
                        app.get_karma_adjusted_time()
                    } else {
                        app.get_category_karma_adjusted_time(cat_name)
                    };
                    app.format_signed_time(karma_time)
                } else if active_index == Some(0) {
                    let karma_time = app.get_karma_adjusted_time();
                    app.format_signed_time(karma_time)
                } else if let Some(idx) = active_index {
                    let cat_name = categories
                        .get(idx)
                        .map(|category| category.name.as_str())
                        .unwrap_or("none");
                    let mut total = app.get_effective_time_for_category(cat_name);
                    if let Some(start) = app.time_tracker.current_session_start {
                        total += start.elapsed().as_secs() as usize;
                    }
                    app.format_time(total)
                } else {
                    app.format_time(app.get_effective_time_today())
                };

                let border_color = app.get_active_color();
                let block = Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .title(
                        Line::from(vec![
                            Span::styled(
                                &category_name,
                                Style::default()
                                    .fg(Color::White)
                                    .add_modifier(Modifier::BOLD),
                            ),
                            if description.is_empty() {
                                Span::raw("")
                            } else {
                                Span::styled(
                                    format!(" {}", description),
                                    Style::default()
                                        .fg(Color::White)
                                        .add_modifier(Modifier::ITALIC),
                                )
                            },
                        ])
                        .alignment(Alignment::Left),
                    )
                    .title(
                        Line::from(Span::styled(
                            session_timer.as_str(),
                            Style::default().fg(Color::White),
                        ))
                        .alignment(Alignment::Center),
                    )
                    .title(
                        Line::from(Span::styled(
                            effective_time_str.as_str(),
                            Style::default().fg(Color::White),
                        ))
                        .alignment(Alignment::Right),
                    )
                    .border_style(Style::default().fg(border_color));
                let paragraph = Paragraph::new(sand).block(block);
                f.render_widget(paragraph, size);

                if app.modal_open {
                    app.render_modal(f, size);
                } else if app.report_modal_open {
                    app.render_report_modal(f, size);
                }
            })?;
            app.render_needed = false;
            last_render = Instant::now();
        }

        if event::poll(Duration::from_millis(1))? {
            if let Event::Key(key) = event::read()? {
                if app.handle_key(key) {
                    break;
                }
            }
        }
    }

    app.time_tracker.end_session();
    app.persist_sessions();

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}
