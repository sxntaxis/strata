use chrono::{Duration as ChronoDuration, NaiveDate, NaiveDateTime, NaiveTime, Timelike};

use super::LedgerCorrectionConfirmation;
use crate::domain::CategoryId;

pub(super) fn parse_ledger_time_input(value: &str) -> Option<NaiveTime> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    let (hour, minute, second) = if value.contains(':') {
        let parts = value.split(':').collect::<Vec<_>>();
        if !(2..=3).contains(&parts.len()) {
            return None;
        }
        let hour = parts[0].parse::<u32>().ok()?;
        let minute = if parts[1].is_empty() {
            0
        } else {
            parts[1].parse::<u32>().ok()?
        };
        let second = if parts.len() == 3 {
            if parts[2].is_empty() {
                0
            } else {
                parts[2].parse::<u32>().ok()?
            }
        } else {
            0
        };
        (hour, minute, second)
    } else if value.chars().all(|character| character.is_ascii_digit()) {
        match value.len() {
            1 | 2 => (value.parse::<u32>().ok()?, 0, 0),
            3 => (
                value[..1].parse::<u32>().ok()?,
                value[1..].parse::<u32>().ok()?,
                0,
            ),
            4 => (
                value[..2].parse::<u32>().ok()?,
                value[2..].parse::<u32>().ok()?,
                0,
            ),
            5 => (
                value[..1].parse::<u32>().ok()?,
                value[1..3].parse::<u32>().ok()?,
                value[3..].parse::<u32>().ok()?,
            ),
            6 => (
                value[..2].parse::<u32>().ok()?,
                value[2..4].parse::<u32>().ok()?,
                value[4..].parse::<u32>().ok()?,
            ),
            _ => return None,
        }
    } else {
        return None;
    };
    NaiveTime::from_hms_opt(hour, minute, second)
}

pub(super) fn format_ledger_time_input(value: &str) -> String {
    let Some(time) = parse_ledger_time_input(value) else {
        return value.to_string();
    };
    if time.second() == 0 {
        time.format("%H:%M").to_string()
    } else {
        time.format("%H:%M:%S").to_string()
    }
}

fn append_time_character(value: &mut String, character: char) {
    if character == ':' {
        if !value.is_empty() && !value.ends_with(':') && value.matches(':').count() < 2 {
            value.push(':');
        }
        return;
    }
    if !character.is_ascii_digit() || value.len() >= 8 {
        return;
    }

    if !value.contains(':') {
        value.push(character);
        let digits = value.chars().count();
        if digits == 1 {
            if value.as_bytes()[0] > b'2' {
                value.push(':');
            }
        } else if digits == 2 {
            let hour = value.parse::<u32>().unwrap_or(24);
            if hour <= 23 {
                value.push(':');
            } else {
                value.insert(1, ':');
            }
        }
        return;
    }

    let colon_count = value.matches(':').count();
    let current_component_len = value.rsplit(':').next().unwrap_or_default().chars().count();
    if colon_count == 1 && current_component_len >= 2 {
        value.push(':');
    }
    if value.len() < 8 {
        value.push(character);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum LedgerEntryEditKind {
    Existing { session_id: usize },
    Add,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum LedgerEntryField {
    Description,
    StartDate,
    StartTime,
    EndDate,
    EndTime,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct LedgerEntryEditState {
    pub(super) kind: LedgerEntryEditKind,
    pub(super) category_id: CategoryId,
    pub(super) description: String,
    pub(super) start_date: String,
    pub(super) start_time: String,
    pub(super) end_date: String,
    pub(super) end_time: String,
    pub(super) dates_linked: bool,
    pub(super) active_field: LedgerEntryField,
    pub(super) select_all: bool,
    pub(super) error: Option<String>,
    pub(super) confirmation: Option<LedgerCorrectionConfirmation>,
    pub(super) tag_cycle_prefix: Option<String>,
}

impl LedgerEntryEditState {
    pub(super) fn existing(
        session_id: usize,
        category_id: CategoryId,
        description: String,
        start_date: String,
        start_time: String,
        end_date: String,
        end_time: String,
    ) -> Self {
        let dates_linked = start_date == end_date;
        Self {
            kind: LedgerEntryEditKind::Existing { session_id },
            category_id,
            description,
            start_date,
            start_time,
            end_date,
            end_time,
            dates_linked,
            active_field: LedgerEntryField::Description,
            select_all: true,
            error: None,
            confirmation: None,
            tag_cycle_prefix: None,
        }
    }

    pub(super) fn add(
        category_id: CategoryId,
        start_date: String,
        start_time: String,
        end_date: String,
        end_time: String,
    ) -> Self {
        let dates_linked = start_date == end_date;
        Self {
            kind: LedgerEntryEditKind::Add,
            category_id,
            description: String::new(),
            start_date,
            start_time,
            end_date,
            end_time,
            dates_linked,
            active_field: LedgerEntryField::Description,
            select_all: false,
            error: None,
            confirmation: None,
            tag_cycle_prefix: None,
        }
    }

    pub(super) fn is_add(&self) -> bool {
        matches!(self.kind, LedgerEntryEditKind::Add)
    }

    pub(super) fn session_id(&self) -> Option<usize> {
        match self.kind {
            LedgerEntryEditKind::Existing { session_id } => Some(session_id),
            LedgerEntryEditKind::Add => None,
        }
    }

    fn fields(&self) -> Vec<LedgerEntryField> {
        let mut fields = vec![
            LedgerEntryField::Description,
            LedgerEntryField::StartDate,
            LedgerEntryField::StartTime,
        ];
        if !self.dates_linked {
            fields.push(LedgerEntryField::EndDate);
        }
        fields.push(LedgerEntryField::EndTime);
        fields
    }

    fn normalize_active_time(&mut self) {
        let value = match self.active_field {
            LedgerEntryField::StartTime => Some(&mut self.start_time),
            LedgerEntryField::EndTime => Some(&mut self.end_time),
            _ => None,
        };
        if let Some(value) = value
            && parse_ledger_time_input(value).is_some()
        {
            *value = format_ledger_time_input(value);
        }
    }

    pub(super) fn reset_tag_cycle(&mut self) {
        self.tag_cycle_prefix = None;
    }

    fn effective_naive_bounds(&self) -> Option<(NaiveDateTime, NaiveDateTime)> {
        let start_date = NaiveDate::parse_from_str(self.start_date.trim(), "%Y-%m-%d").ok()?;
        let start_time = parse_ledger_time_input(&self.start_time)?;
        let mut end_date = NaiveDate::parse_from_str(self.end_date.trim(), "%Y-%m-%d").ok()?;
        let end_time = parse_ledger_time_input(&self.end_time)?;
        if self.dates_linked && end_time < start_time {
            end_date = end_date.checked_add_signed(ChronoDuration::days(1))?;
        }
        Some((start_date.and_time(start_time), end_date.and_time(end_time)))
    }

    fn store_naive_bounds(&mut self, start: NaiveDateTime, end: NaiveDateTime) {
        self.start_date = start.date().format("%Y-%m-%d").to_string();
        self.start_time = format_ledger_time_input(&start.time().format("%H:%M:%S").to_string());

        let linked_same_day = end.date() == start.date() && end.time() >= start.time();
        let linked_overnight = start
            .date()
            .checked_add_signed(ChronoDuration::days(1))
            .is_some_and(|next| next == end.date() && end.time() < start.time());
        self.dates_linked = linked_same_day || linked_overnight;
        self.end_date = if self.dates_linked {
            self.start_date.clone()
        } else {
            end.date().format("%Y-%m-%d").to_string()
        };
        self.end_time = format_ledger_time_input(&end.time().format("%H:%M:%S").to_string());
    }

    pub(super) fn adjust_active_temporal(&mut self, direction: i64, accelerated: bool) -> bool {
        if direction == 0 {
            return false;
        }
        let Some((mut start, mut end)) = self.effective_naive_bounds() else {
            return false;
        };
        let sign = direction.signum();
        match self.active_field {
            LedgerEntryField::Description => return false,
            LedgerEntryField::StartDate => {
                let delta = ChronoDuration::days(sign * if accelerated { 7 } else { 1 });
                if self.dates_linked {
                    let (Some(next_start), Some(next_end)) = (
                        start.checked_add_signed(delta),
                        end.checked_add_signed(delta),
                    ) else {
                        return false;
                    };
                    start = next_start;
                    end = next_end;
                } else if let Some(next) = start.checked_add_signed(delta) {
                    start = next;
                } else {
                    return false;
                }
            }
            LedgerEntryField::EndDate => {
                let delta = ChronoDuration::days(sign * if accelerated { 7 } else { 1 });
                if let Some(next) = end.checked_add_signed(delta) {
                    end = next;
                } else {
                    return false;
                }
            }
            LedgerEntryField::StartTime => {
                let delta = ChronoDuration::minutes(sign * if accelerated { 60 } else { 1 });
                if let Some(next) = start.checked_add_signed(delta) {
                    start = next;
                } else {
                    return false;
                }
            }
            LedgerEntryField::EndTime => {
                let delta = ChronoDuration::minutes(sign * if accelerated { 60 } else { 1 });
                if let Some(next) = end.checked_add_signed(delta) {
                    end = next;
                } else {
                    return false;
                }
            }
        }
        self.store_naive_bounds(start, end);
        self.select_all = true;
        self.error = None;
        self.confirmation = None;
        self.tag_cycle_prefix = None;
        true
    }

    pub(super) fn next_field(&mut self) {
        self.normalize_active_time();
        self.reset_tag_cycle();
        let fields = self.fields();
        let current = fields
            .iter()
            .position(|field| *field == self.active_field)
            .unwrap_or(0);
        self.active_field = fields[(current + 1) % fields.len()];
        self.select_all = true;
        self.error = None;
        self.confirmation = None;
    }

    pub(super) fn previous_field(&mut self) {
        self.normalize_active_time();
        self.reset_tag_cycle();
        let fields = self.fields();
        let current = fields
            .iter()
            .position(|field| *field == self.active_field)
            .unwrap_or(0);
        let previous = if current == 0 {
            fields.len() - 1
        } else {
            current - 1
        };
        self.active_field = fields[previous];
        self.select_all = true;
        self.error = None;
        self.confirmation = None;
    }

    pub(super) fn append(&mut self, character: char) {
        let active = self.active_field;
        let valid = match active {
            LedgerEntryField::Description => true,
            LedgerEntryField::StartDate | LedgerEntryField::EndDate => {
                character.is_ascii_digit() || character == '-'
            }
            LedgerEntryField::StartTime | LedgerEntryField::EndTime => {
                character.is_ascii_digit() || character == ':'
            }
        };
        if !valid {
            return;
        }
        if self.select_all {
            self.active_value_mut().clear();
            self.select_all = false;
        }
        if active == LedgerEntryField::Description {
            self.active_value_mut().push(character);
            self.reset_tag_cycle();
        } else if matches!(
            active,
            LedgerEntryField::StartTime | LedgerEntryField::EndTime
        ) {
            append_time_character(self.active_value_mut(), character);
        } else if self.active_value_mut().len() < 10 {
            self.active_value_mut().push(character);
        }
        if active == LedgerEntryField::StartDate && self.dates_linked {
            self.end_date = self.start_date.clone();
        }
        self.error = None;
        self.confirmation = None;
    }

    pub(super) fn backspace(&mut self) {
        let active = self.active_field;
        if self.select_all {
            self.active_value_mut().clear();
            self.select_all = false;
        } else {
            self.active_value_mut().pop();
        }
        if active == LedgerEntryField::StartDate && self.dates_linked {
            self.end_date = self.start_date.clone();
        }
        if active == LedgerEntryField::Description {
            self.reset_tag_cycle();
        }
        self.error = None;
        self.confirmation = None;
    }

    fn active_value_mut(&mut self) -> &mut String {
        match self.active_field {
            LedgerEntryField::Description => &mut self.description,
            LedgerEntryField::StartDate => &mut self.start_date,
            LedgerEntryField::StartTime => &mut self.start_time,
            LedgerEntryField::EndDate => &mut self.end_date,
            LedgerEntryField::EndTime => &mut self.end_time,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        LedgerEntryEditState, LedgerEntryField, format_ledger_time_input, parse_ledger_time_input,
    };
    use crate::domain::CategoryId;

    #[test]
    fn same_day_edit_cycles_without_redundant_end_date() {
        let mut edit = LedgerEntryEditState::existing(
            7,
            CategoryId::new(2),
            "tag".to_string(),
            "2026-09-27".to_string(),
            "11:05:44".to_string(),
            "2026-09-27".to_string(),
            "11:33:09".to_string(),
        );
        edit.next_field();
        assert_eq!(edit.active_field, LedgerEntryField::StartDate);
        edit.next_field();
        assert_eq!(edit.active_field, LedgerEntryField::StartTime);
        edit.next_field();
        assert_eq!(edit.active_field, LedgerEntryField::EndTime);
    }

    #[test]
    fn linked_date_edit_updates_both_boundaries() {
        let mut edit = LedgerEntryEditState::add(
            CategoryId::new(2),
            "2026-09-27".to_string(),
            "11:05:44".to_string(),
            "2026-09-27".to_string(),
            "11:33:09".to_string(),
        );
        edit.active_field = LedgerEntryField::StartDate;
        edit.select_all = true;
        for character in "2026-09-28".chars() {
            edit.append(character);
        }
        assert_eq!(edit.start_date, "2026-09-28");
        assert_eq!(edit.end_date, "2026-09-28");
    }
    #[test]
    fn time_input_accepts_compact_and_partial_precision() {
        assert_eq!(format_ledger_time_input("6"), "06:00");
        assert_eq!(format_ledger_time_input("650"), "06:50");
        assert_eq!(format_ledger_time_input("6:50"), "06:50");
        assert_eq!(format_ledger_time_input("65030"), "06:50:30");
        assert_eq!(
            parse_ledger_time_input("23:59:59")
                .unwrap()
                .format("%H:%M:%S")
                .to_string(),
            "23:59:59"
        );
    }

    #[test]
    fn typed_time_inserts_separators_without_requiring_seconds() {
        let mut edit = LedgerEntryEditState::add(
            CategoryId::new(2),
            "2026-09-27".to_string(),
            "11:05:44".to_string(),
            "2026-09-27".to_string(),
            "11:33:09".to_string(),
        );
        edit.active_field = LedgerEntryField::StartTime;
        edit.select_all = true;
        for character in "650".chars() {
            edit.append(character);
        }
        assert_eq!(edit.start_time, "6:50");
        edit.next_field();
        assert_eq!(edit.start_time, "06:50");
    }

    #[test]
    fn temporal_arrows_carry_across_hours_and_days() {
        let mut edit = LedgerEntryEditState::existing(
            7,
            CategoryId::new(2),
            "tag".to_string(),
            "2026-09-27".to_string(),
            "23:30:15".to_string(),
            "2026-09-28".to_string(),
            "01:00:15".to_string(),
        );
        edit.active_field = LedgerEntryField::StartTime;
        assert!(edit.adjust_active_temporal(1, true));
        assert_eq!(edit.start_date, "2026-09-28");
        assert_eq!(edit.start_time, "00:30:15");
        assert_eq!(edit.end_date, "2026-09-28");
        assert_eq!(edit.end_time, "01:00:15");
    }

    #[test]
    fn temporal_shift_arrows_use_large_steps_and_preserve_seconds() {
        let mut edit = LedgerEntryEditState::existing(
            7,
            CategoryId::new(2),
            "tag".to_string(),
            "2026-09-20".to_string(),
            "10:30:15".to_string(),
            "2026-09-20".to_string(),
            "12:00:15".to_string(),
        );
        edit.active_field = LedgerEntryField::StartDate;
        assert!(edit.adjust_active_temporal(1, true));
        assert_eq!(edit.start_date, "2026-09-27");
        assert_eq!(edit.end_date, "2026-09-27");

        edit.active_field = LedgerEntryField::EndTime;
        assert!(edit.adjust_active_temporal(-1, false));
        assert_eq!(edit.end_time, "11:59:15");
    }
}
