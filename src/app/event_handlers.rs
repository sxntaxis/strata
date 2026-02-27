use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::{
    constants::COLORS,
    domain::{CategoryId, ReportPeriod},
};

use super::{ui_helpers, App};

impl App {
    pub(super) fn handle_key(&mut self, key: KeyEvent) -> bool {
        if self.in_category_modal() {
            self.handle_modal_key(key);
            false
        } else if self.in_karma_modal() {
            self.handle_report_modal_key(key);
            false
        } else {
            self.handle_normal_key(key)
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
                } else {
                    let total_rows = self.time_tracker.category_count() + 1;
                    if total_rows > 0 {
                        self.selected_index =
                            ui_helpers::wrap_prev_index(self.selected_index, total_rows);
                        self.sync_modal_description_from_selection();
                    }
                }
            }
            KeyCode::Down => {
                if shift {
                    if self.time_tracker.move_category_down(self.selected_index) {
                        self.selected_index += 1;
                        self.persist_categories();
                    }
                } else {
                    let total_rows = self.time_tracker.category_count() + 1;
                    if total_rows > 0 {
                        self.selected_index =
                            ui_helpers::wrap_next_index(self.selected_index, total_rows);
                        self.sync_modal_description_from_selection();
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
                } else if !shift {
                    self.cycle_selected_tag(-1);
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
                } else if !shift {
                    self.cycle_selected_tag(1);
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
                        self.remember_selected_tag();
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
                    self.modal_tag_index = None;
                    self.modal_description.push(c);
                }
            }
            KeyCode::Backspace => {
                if self.is_on_insert_space() {
                    self.new_category_name.pop();
                } else if self.selected_index < self.time_tracker.category_count() {
                    self.modal_tag_index = None;
                    self.modal_description.pop();
                }
            }
            _ => {}
        }
    }

    fn handle_report_modal_key(&mut self, key: KeyEvent) {
        let shift = key.modifiers.contains(KeyModifiers::SHIFT);

        let summary = self.report_rows();
        self.clamp_report_selection(summary.entries.len());
        let logs = self.report_current_logs();
        self.clamp_report_log_selection(logs.len());
        let in_logs_view = self.report_logs_category_id.is_some();

        match key.code {
            KeyCode::Esc => {
                if in_logs_view {
                    self.report_logs_category_id = None;
                    self.report_log_selected_index = 0;
                } else {
                    self.close_report_modal();
                }
            }
            KeyCode::Enter => {
                if in_logs_view {
                    self.report_logs_category_id = None;
                    self.report_log_selected_index = 0;
                } else if let Some(entry) = summary.entries.get(self.report_selected_index) {
                    if entry.category_id != CategoryId::new(0) {
                        self.report_logs_category_id = Some(entry.category_id);
                        self.report_log_selected_index = 0;
                    }
                }
            }
            KeyCode::Up => {
                if in_logs_view {
                    if !logs.is_empty() {
                        self.report_log_selected_index =
                            ui_helpers::wrap_prev_index(self.report_log_selected_index, logs.len());
                    }
                } else if !summary.entries.is_empty() {
                    self.report_selected_index = ui_helpers::wrap_prev_index(
                        self.report_selected_index,
                        summary.entries.len(),
                    );
                }
            }
            KeyCode::Down => {
                if in_logs_view {
                    if !logs.is_empty() {
                        self.report_log_selected_index =
                            ui_helpers::wrap_next_index(self.report_log_selected_index, logs.len());
                    }
                } else if !summary.entries.is_empty() {
                    self.report_selected_index = ui_helpers::wrap_next_index(
                        self.report_selected_index,
                        summary.entries.len(),
                    );
                }
            }
            KeyCode::Left if shift => {
                self.set_report_period(ui_helpers::report_period_prev(self.report_period));
            }
            KeyCode::Right if shift => {
                self.set_report_period(ui_helpers::report_period_next(self.report_period));
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

    fn handle_normal_key(&mut self, key: KeyEvent) -> bool {
        let shift = key.modifiers.contains(KeyModifiers::SHIFT);

        match key.code {
            KeyCode::Char('q') => true,
            KeyCode::Char('c') | KeyCode::Char('C') => {
                let is_shift_clear = shift || matches!(key.code, KeyCode::Char('C'));
                if is_shift_clear {
                    self.sand_engine.clear_category(CategoryId::new(0));
                } else {
                    self.sand_engine.clear();
                    self.time_tracker.reset_none_counter_today();
                    self.persist_sessions();
                }
                self.persist_sand_state();
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
