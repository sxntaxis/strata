use chrono::{Duration as ChronoDuration, NaiveDate, NaiveDateTime, NaiveTime};

use super::LedgerCorrectionConfirmation;
use crate::domain::CategoryId;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum LedgerTimePrecision {
    Minute,
    Second,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct NormalizedLedgerTime {
    pub(super) day_carry: i64,
    pub(super) time: NaiveTime,
    pub(super) precision: LedgerTimePrecision,
}

fn ledger_time_components(value: &str) -> Option<(u64, u64, u64, LedgerTimePrecision)> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }

    if value.contains(':') {
        let parts = value.split(':').collect::<Vec<_>>();
        if !(2..=3).contains(&parts.len())
            || parts[0].is_empty()
            || parts.iter().any(|part| part.len() > 2)
        {
            return None;
        }
        let hour = parts[0].parse::<u64>().ok()?;
        let minute = if parts[1].is_empty() {
            0
        } else {
            parts[1].parse::<u64>().ok()?
        };
        let (second, precision) = if parts.len() == 3 {
            (
                if parts[2].is_empty() {
                    0
                } else {
                    parts[2].parse::<u64>().ok()?
                },
                LedgerTimePrecision::Second,
            )
        } else {
            (0, LedgerTimePrecision::Minute)
        };
        return Some((hour, minute, second, precision));
    }

    if !value.chars().all(|character| character.is_ascii_digit()) {
        return None;
    }
    match value.len() {
        1 | 2 => Some((
            value.parse::<u64>().ok()?,
            0,
            0,
            LedgerTimePrecision::Minute,
        )),
        3 => Some((
            value[..1].parse::<u64>().ok()?,
            value[1..].parse::<u64>().ok()?,
            0,
            LedgerTimePrecision::Minute,
        )),
        4 => Some((
            value[..2].parse::<u64>().ok()?,
            value[2..].parse::<u64>().ok()?,
            0,
            LedgerTimePrecision::Minute,
        )),
        5 => Some((
            value[..1].parse::<u64>().ok()?,
            value[1..3].parse::<u64>().ok()?,
            value[3..].parse::<u64>().ok()?,
            LedgerTimePrecision::Second,
        )),
        6 => Some((
            value[..2].parse::<u64>().ok()?,
            value[2..4].parse::<u64>().ok()?,
            value[4..].parse::<u64>().ok()?,
            LedgerTimePrecision::Second,
        )),
        _ => None,
    }
}

pub(super) fn normalize_ledger_time_input(value: &str) -> Option<NormalizedLedgerTime> {
    let (hour, minute, second, precision) = ledger_time_components(value)?;
    let total_seconds = hour
        .checked_mul(3_600)?
        .checked_add(minute.checked_mul(60)?)?
        .checked_add(second)?;
    let day_carry = i64::try_from(total_seconds / 86_400).ok()?;
    let within_day = total_seconds % 86_400;
    let hour = u32::try_from(within_day / 3_600).ok()?;
    let minute = u32::try_from((within_day % 3_600) / 60).ok()?;
    let second = u32::try_from(within_day % 60).ok()?;
    Some(NormalizedLedgerTime {
        day_carry,
        time: NaiveTime::from_hms_opt(hour, minute, second)?,
        precision,
    })
}

pub(super) fn parse_ledger_time_input(value: &str) -> Option<NaiveTime> {
    normalize_ledger_time_input(value).map(|normalized| normalized.time)
}

pub(super) fn format_time_for_precision(
    time: NaiveTime,
    precision: LedgerTimePrecision,
) -> String {
    match precision {
        LedgerTimePrecision::Minute => time.format("%H:%M").to_string(),
        LedgerTimePrecision::Second => time.format("%H:%M:%S").to_string(),
    }
}

pub(super) fn format_ledger_time_input(value: &str) -> String {
    let Some(normalized) = normalize_ledger_time_input(value) else {
        return value.to_string();
    };
    format_time_for_precision(normalized.time, normalized.precision)
}

fn char_to_byte_index(value: &str, char_index: usize) -> usize {
    value
        .char_indices()
        .nth(char_index)
        .map(|(index, _)| index)
        .unwrap_or(value.len())
}

fn time_candidate_is_well_shaped(value: &str) -> bool {
    if value.is_empty() {
        return true;
    }
    if !value
        .chars()
        .all(|character| character.is_ascii_digit() || character == ':')
    {
        return false;
    }
    let colon_count = value.matches(':').count();
    if colon_count > 2 {
        return false;
    }
    if colon_count == 0 {
        return value.chars().count() <= 6;
    }
    let parts = value.split(':').collect::<Vec<_>>();
    !parts[0].is_empty() && parts.iter().all(|part| part.chars().count() <= 2)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum LedgerEntryEditKind {
    Existing { session_id: usize },
    Active,
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
    pub(super) caret: usize,
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
        let caret = description.chars().count();
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
            caret,
            confirmation: None,
            tag_cycle_prefix: None,
        }
    }

    pub(super) fn active(
        category_id: CategoryId,
        description: String,
        start_date: String,
        start_time: String,
        end_date: String,
        end_time: String,
    ) -> Self {
        let dates_linked = start_date == end_date;
        let caret = description.chars().count();
        Self {
            kind: LedgerEntryEditKind::Active,
            category_id,
            description,
            start_date,
            start_time,
            end_date,
            end_time,
            dates_linked,
            active_field: LedgerEntryField::Description,
            select_all: true,
            caret,
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
            caret: 0,
            confirmation: None,
            tag_cycle_prefix: None,
        }
    }

    pub(super) fn is_add(&self) -> bool {
        matches!(self.kind, LedgerEntryEditKind::Add)
    }

    pub(super) fn is_active(&self) -> bool {
        matches!(self.kind, LedgerEntryEditKind::Active)
    }

    pub(super) fn session_id(&self) -> Option<usize> {
        match self.kind {
            LedgerEntryEditKind::Existing { session_id } => Some(session_id),
            LedgerEntryEditKind::Active | LedgerEntryEditKind::Add => None,
        }
    }

    fn fields(&self) -> Vec<LedgerEntryField> {
        if self.is_active() {
            return vec![
                LedgerEntryField::Description,
                LedgerEntryField::StartDate,
                LedgerEntryField::StartTime,
            ];
        }
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

    fn active_value(&self) -> &str {
        match self.active_field {
            LedgerEntryField::Description => &self.description,
            LedgerEntryField::StartDate => &self.start_date,
            LedgerEntryField::StartTime => &self.start_time,
            LedgerEntryField::EndDate => &self.end_date,
            LedgerEntryField::EndTime => &self.end_time,
        }
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

    fn active_time_precision(&self) -> Option<LedgerTimePrecision> {
        match self.active_field {
            LedgerEntryField::StartTime => normalize_ledger_time_input(&self.start_time)
                .map(|normalized| normalized.precision),
            LedgerEntryField::EndTime => normalize_ledger_time_input(&self.end_time)
                .map(|normalized| normalized.precision),
            _ => None,
        }
    }

    fn field_time_precision(value: &str) -> LedgerTimePrecision {
        normalize_ledger_time_input(value)
            .map(|normalized| normalized.precision)
            .unwrap_or(LedgerTimePrecision::Minute)
    }

    fn effective_naive_bounds(&self) -> Option<(NaiveDateTime, NaiveDateTime)> {
        let start_base_date =
            NaiveDate::parse_from_str(self.start_date.trim(), "%Y-%m-%d").ok()?;
        let start_normalized = normalize_ledger_time_input(&self.start_time)?;
        let start_date = start_base_date.checked_add_signed(ChronoDuration::days(
            start_normalized.day_carry,
        ))?;
        let start = start_date.and_time(start_normalized.time);

        // A linked end follows any carry introduced by the start value. This is
        // what makes permissive inputs such as `99:` update the civil Date rather
        // than leaving End several days behind the normalized Start.
        let end_base_date = if self.dates_linked {
            start_date
        } else {
            NaiveDate::parse_from_str(self.end_date.trim(), "%Y-%m-%d").ok()?
        };
        let end_normalized = normalize_ledger_time_input(&self.end_time)?;
        let mut end_date = end_base_date.checked_add_signed(ChronoDuration::days(
            end_normalized.day_carry,
        ))?;
        let mut end = end_date.and_time(end_normalized.time);
        if self.dates_linked && end <= start && end_normalized.day_carry == 0 {
            end_date = end_date.checked_add_signed(ChronoDuration::days(1))?;
            end = end_date.and_time(end_normalized.time);
        }
        Some((start, end))
    }

    pub(super) fn preview_naive_bounds(&self) -> Option<(NaiveDateTime, NaiveDateTime)> {
        self.effective_naive_bounds()
    }

    fn store_naive_bounds(
        &mut self,
        start: NaiveDateTime,
        end: NaiveDateTime,
        start_precision: LedgerTimePrecision,
        end_precision: LedgerTimePrecision,
    ) {
        self.start_date = start.date().format("%Y-%m-%d").to_string();
        self.start_time = format_time_for_precision(start.time(), start_precision);

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
        self.end_time = format_time_for_precision(end.time(), end_precision);
        self.caret = self.active_value().chars().count();
    }

    fn normalize_active_time(&mut self) -> Result<bool, String> {
        if !matches!(
            self.active_field,
            LedgerEntryField::StartTime | LedgerEntryField::EndTime
        ) {
            return Ok(false);
        }
        let start_precision = Self::field_time_precision(&self.start_time);
        let end_precision = Self::field_time_precision(&self.end_time);
        let (start, end) = self
            .effective_naive_bounds()
            .ok_or_else(|| "time could not be normalized".to_string())?;
        self.store_naive_bounds(start, end, start_precision, end_precision);
        Ok(true)
    }

    pub(super) fn normalize_active_date(&mut self, current: NaiveDate) -> Result<bool, String> {
        let field = self.active_field;
        let raw = match field {
            LedgerEntryField::StartDate => self.start_date.clone(),
            LedgerEntryField::EndDate => self.end_date.clone(),
            _ => return Ok(false),
        };
        let normalized = super::date_input::normalize_date_text(&raw, current)?;
        match field {
            LedgerEntryField::StartDate => {
                self.start_date = normalized;
                if self.dates_linked {
                    self.end_date = self.start_date.clone();
                }
            }
            LedgerEntryField::EndDate => self.end_date = normalized,
            _ => {}
        }
        self.caret = self.active_value().chars().count();
        self.select_all = false;
        self.confirmation = None;
        Ok(true)
    }

    pub(super) fn normalize_all_dates(&mut self, current: NaiveDate) -> Result<(), String> {
        self.start_date = super::date_input::normalize_date_text(&self.start_date, current)?;
        if self.dates_linked {
            self.end_date = self.start_date.clone();
        } else {
            self.end_date = super::date_input::normalize_date_text(&self.end_date, current)?;
        }
        if matches!(
            self.active_field,
            LedgerEntryField::StartDate | LedgerEntryField::EndDate
        ) {
            self.caret = self.active_value().chars().count();
        }
        self.confirmation = None;
        Ok(())
    }

    pub(super) fn normalize_all_temporal(&mut self, current: NaiveDate) -> Result<(), String> {
        self.normalize_all_dates(current)?;
        let start_precision = Self::field_time_precision(&self.start_time);
        let end_precision = Self::field_time_precision(&self.end_time);
        let (start, end) = self
            .effective_naive_bounds()
            .ok_or_else(|| "time could not be normalized".to_string())?;
        self.store_naive_bounds(start, end, start_precision, end_precision);
        Ok(())
    }

    pub(super) fn normalize_active_input(&mut self, current: NaiveDate) -> Result<(), String> {
        if self.normalize_active_date(current)? {
            return Ok(());
        }
        self.normalize_active_time()?;
        Ok(())
    }

    pub(super) fn reset_tag_cycle(&mut self) {
        self.tag_cycle_prefix = None;
    }

    pub(super) fn adjust_active_temporal(&mut self, direction: i64, accelerated: bool) -> bool {
        if direction == 0 {
            return false;
        }
        let start_precision = Self::field_time_precision(&self.start_time);
        let end_precision = Self::field_time_precision(&self.end_time);
        let active_precision = self.active_time_precision();
        let Some((mut start, mut end)) = self.effective_naive_bounds() else {
            return false;
        };
        let sign = direction.signum();
        match self.active_field {
            LedgerEntryField::Description => return false,
            LedgerEntryField::StartDate => {
                if accelerated {
                    if self.dates_linked {
                        let (Some(next_start_date), Some(next_end_date)) = (
                            crate::temporal::shift_civil_month(start.date(), sign),
                            crate::temporal::shift_civil_month(end.date(), sign),
                        ) else {
                            return false;
                        };
                        start = next_start_date.and_time(start.time());
                        end = next_end_date.and_time(end.time());
                    } else if let Some(next_date) =
                        crate::temporal::shift_civil_month(start.date(), sign)
                    {
                        start = next_date.and_time(start.time());
                    } else {
                        return false;
                    }
                } else {
                    let delta = ChronoDuration::days(sign);
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
            }
            LedgerEntryField::EndDate => {
                if accelerated {
                    if let Some(next_date) = crate::temporal::shift_civil_month(end.date(), sign) {
                        end = next_date.and_time(end.time());
                    } else {
                        return false;
                    }
                } else if let Some(next) = end.checked_add_signed(ChronoDuration::days(sign)) {
                    end = next;
                } else {
                    return false;
                }
            }
            LedgerEntryField::StartTime => {
                let precision = active_precision.unwrap_or(LedgerTimePrecision::Minute);
                let delta = match (precision, accelerated) {
                    (LedgerTimePrecision::Second, false) => ChronoDuration::seconds(sign),
                    (LedgerTimePrecision::Second, true) => ChronoDuration::minutes(sign),
                    (LedgerTimePrecision::Minute, false) => ChronoDuration::minutes(sign),
                    (LedgerTimePrecision::Minute, true) => ChronoDuration::hours(sign),
                };
                if let Some(next) = start.checked_add_signed(delta) {
                    start = next;
                } else {
                    return false;
                }
            }
            LedgerEntryField::EndTime => {
                let precision = active_precision.unwrap_or(LedgerTimePrecision::Minute);
                let delta = match (precision, accelerated) {
                    (LedgerTimePrecision::Second, false) => ChronoDuration::seconds(sign),
                    (LedgerTimePrecision::Second, true) => ChronoDuration::minutes(sign),
                    (LedgerTimePrecision::Minute, false) => ChronoDuration::minutes(sign),
                    (LedgerTimePrecision::Minute, true) => ChronoDuration::hours(sign),
                };
                if let Some(next) = end.checked_add_signed(delta) {
                    end = next;
                } else {
                    return false;
                }
            }
        }
        self.store_naive_bounds(start, end, start_precision, end_precision);
        self.select_all = false;
        self.confirmation = None;
        self.tag_cycle_prefix = None;
        true
    }

    fn activate_field(&mut self, field: LedgerEntryField) {
        self.active_field = field;
        self.select_all = true;
        self.caret = self.active_value().chars().count();
        self.confirmation = None;
    }

    pub(super) fn next_field(&mut self) {
        self.reset_tag_cycle();
        let fields = self.fields();
        let current = fields
            .iter()
            .position(|field| *field == self.active_field)
            .unwrap_or(0);
        self.activate_field(fields[(current + 1) % fields.len()]);
    }

    pub(super) fn previous_field(&mut self) {
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
        self.activate_field(fields[previous]);
    }

    pub(super) fn move_caret(&mut self, delta: isize) {
        self.select_all = false;
        let len = self.active_value().chars().count();
        if delta < 0 {
            self.caret = self.caret.saturating_sub(delta.unsigned_abs());
        } else {
            self.caret = self
                .caret
                .saturating_add(usize::try_from(delta).unwrap_or(usize::MAX))
                .min(len);
        }
        self.reset_tag_cycle();
    }

    pub(super) fn caret_home(&mut self) {
        self.select_all = false;
        self.caret = 0;
        self.reset_tag_cycle();
    }

    pub(super) fn caret_end(&mut self) {
        self.select_all = false;
        self.caret = self.active_value().chars().count();
        self.reset_tag_cycle();
    }

    fn candidate_with_insert(&self, character: char) -> String {
        let value = self.active_value();
        let index = char_to_byte_index(value, self.caret.min(value.chars().count()));
        let mut candidate = value.to_string();
        candidate.insert(index, character);
        candidate
    }

    pub(super) fn append(&mut self, character: char) {
        let active = self.active_field;
        let valid = match active {
            LedgerEntryField::Description => true,
            LedgerEntryField::StartDate | LedgerEntryField::EndDate => {
                super::date_input::date_candidate_accepts_character(character)
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
            self.caret = 0;
            self.select_all = false;
        }
        let candidate = self.candidate_with_insert(character);
        let accepted = match active {
            LedgerEntryField::Description => true,
            LedgerEntryField::StartDate | LedgerEntryField::EndDate => {
                super::date_input::date_candidate_is_well_shaped(&candidate)
            }
            LedgerEntryField::StartTime | LedgerEntryField::EndTime => {
                time_candidate_is_well_shaped(&candidate)
            }
        };
        if !accepted {
            return;
        }
        *self.active_value_mut() = candidate;
        self.caret = self.caret.saturating_add(1);
        if active == LedgerEntryField::StartDate && self.dates_linked {
            self.end_date = self.start_date.clone();
        }
        if active == LedgerEntryField::Description {
            self.reset_tag_cycle();
        }
        self.confirmation = None;
    }

    pub(super) fn backspace(&mut self) {
        let active = self.active_field;
        if self.select_all {
            self.active_value_mut().clear();
            self.caret = 0;
            self.select_all = false;
        } else if self.caret > 0 {
            let value = self.active_value().to_string();
            let end = char_to_byte_index(&value, self.caret);
            let start = char_to_byte_index(&value, self.caret - 1);
            let mut next = value;
            next.replace_range(start..end, "");
            *self.active_value_mut() = next;
            self.caret -= 1;
        }
        if active == LedgerEntryField::StartDate && self.dates_linked {
            self.end_date = self.start_date.clone();
        }
        if active == LedgerEntryField::Description {
            self.reset_tag_cycle();
        }
        self.confirmation = None;
    }

    pub(super) fn delete_forward(&mut self) {
        let active = self.active_field;
        if self.select_all {
            self.active_value_mut().clear();
            self.caret = 0;
            self.select_all = false;
        } else {
            let value = self.active_value().to_string();
            let len = value.chars().count();
            if self.caret < len {
                let start = char_to_byte_index(&value, self.caret);
                let end = char_to_byte_index(&value, self.caret + 1);
                let mut next = value;
                next.replace_range(start..end, "");
                *self.active_value_mut() = next;
            }
        }
        if active == LedgerEntryField::StartDate && self.dates_linked {
            self.end_date = self.start_date.clone();
        }
        if active == LedgerEntryField::Description {
            self.reset_tag_cycle();
        }
        self.confirmation = None;
    }
}

#[cfg(test)]
mod tests {
    use super::{
        LedgerEntryEditState, LedgerEntryField, format_ledger_time_input,
        normalize_ledger_time_input, parse_ledger_time_input,
    };
    use crate::domain::CategoryId;
    use chrono::NaiveDate;

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
        edit.caret = edit.start_date.chars().count();
        for character in "2026-09-28".chars() {
            edit.append(character);
        }
        assert_eq!(edit.start_date, "2026-09-28");
        assert_eq!(edit.end_date, "2026-09-28");
    }

    #[test]
    fn time_input_accepts_compact_partial_and_overflow_precision() {
        assert_eq!(format_ledger_time_input("6"), "06:00");
        assert_eq!(format_ledger_time_input("650"), "06:50");
        assert_eq!(format_ledger_time_input("6:50"), "06:50");
        assert_eq!(format_ledger_time_input("65030"), "06:50:30");
        let normalized = normalize_ledger_time_input("12:99").unwrap();
        assert_eq!(normalized.day_carry, 0);
        assert_eq!(normalized.time.format("%H:%M").to_string(), "13:39");
        let normalized = normalize_ledger_time_input("99:").unwrap();
        assert_eq!(normalized.day_carry, 4);
        assert_eq!(normalized.time.format("%H:%M").to_string(), "03:00");
        assert_eq!(
            parse_ledger_time_input("23:59:59")
                .unwrap()
                .format("%H:%M:%S")
                .to_string(),
            "23:59:59"
        );
    }

    #[test]
    fn colon_components_never_accept_a_third_digit() {
        let mut edit = LedgerEntryEditState::add(
            CategoryId::new(2),
            "2026-09-27".to_string(),
            "11:05".to_string(),
            "2026-09-27".to_string(),
            "11:33".to_string(),
        );
        edit.active_field = LedgerEntryField::StartTime;
        edit.select_all = true;
        edit.caret = edit.start_time.chars().count();
        for character in "5:20:222".chars() {
            edit.append(character);
        }
        assert_eq!(edit.start_time, "5:20:22");
    }

    #[test]
    fn semantic_time_arrows_follow_expressed_precision() {
        let mut edit = LedgerEntryEditState::existing(
            7,
            CategoryId::new(2),
            "tag".to_string(),
            "2026-09-27".to_string(),
            "05:20:37".to_string(),
            "2026-09-27".to_string(),
            "06:00:37".to_string(),
        );
        edit.active_field = LedgerEntryField::StartTime;
        assert!(edit.adjust_active_temporal(1, false));
        assert_eq!(edit.start_time, "05:20:38");
        assert!(edit.adjust_active_temporal(1, true));
        assert_eq!(edit.start_time, "05:21:38");

        edit.start_time = "5:20".to_string();
        edit.caret = edit.start_time.chars().count();
        assert!(edit.adjust_active_temporal(1, false));
        assert_eq!(edit.start_time, "05:21");
        assert!(edit.adjust_active_temporal(1, true));
        assert_eq!(edit.start_time, "06:21");
    }

    #[test]
    fn time_normalization_carries_into_civil_date() {
        let mut edit = LedgerEntryEditState::existing(
            7,
            CategoryId::new(2),
            "tag".to_string(),
            "2026-09-27".to_string(),
            "99:".to_string(),
            "2026-10-02".to_string(),
            "05:00".to_string(),
        );
        edit.dates_linked = false;
        edit.active_field = LedgerEntryField::StartTime;
        assert!(edit.adjust_active_temporal(1, false));
        assert_eq!(edit.start_date, "2026-10-01");
        assert_eq!(edit.start_time, "03:01");
    }

    #[test]
    fn linked_start_time_overflow_moves_the_linked_interval_with_it() {
        let mut edit = LedgerEntryEditState::existing(
            7,
            CategoryId::new(2),
            "tag".to_string(),
            "2026-09-27".to_string(),
            "99:".to_string(),
            "2026-09-27".to_string(),
            "05:00".to_string(),
        );
        edit.active_field = LedgerEntryField::StartTime;
        assert!(edit.adjust_active_temporal(1, false));
        assert_eq!(edit.start_date, "2026-10-01");
        assert_eq!(edit.start_time, "03:01");
        assert_eq!(edit.end_date, "2026-10-01");
        assert_eq!(edit.end_time, "05:00");
    }

    #[test]
    fn caret_edits_inside_values_without_stealing_plain_arrows() {
        let mut edit = LedgerEntryEditState::existing(
            7,
            CategoryId::new(2),
            "post-pnuk".to_string(),
            "2026-09-27".to_string(),
            "10:30".to_string(),
            "2026-09-27".to_string(),
            "12:00".to_string(),
        );
        edit.select_all = false;
        edit.caret = 6;
        edit.append('u');
        assert_eq!(edit.description, "post-punuk");
        edit.backspace();
        assert_eq!(edit.description, "post-pnuk");
        edit.move_caret(-1);
        assert_eq!(edit.caret, 5);
        edit.caret_end();
        assert_eq!(edit.caret, edit.description.chars().count());
    }

    #[test]
    fn temporal_shift_month_clamps_to_month_end() {
        let mut edit = LedgerEntryEditState::existing(
            7,
            CategoryId::new(2),
            "tag".to_string(),
            "2026-01-31".to_string(),
            "10:30:15".to_string(),
            "2026-01-31".to_string(),
            "12:00:15".to_string(),
        );
        edit.active_field = LedgerEntryField::StartDate;
        assert!(edit.adjust_active_temporal(1, true));
        assert_eq!(edit.start_date, "2026-02-28");
        assert_eq!(edit.end_date, "2026-02-28");
    }
    #[test]
    fn named_date_normalization_uses_current_civil_year_and_carries_overflow() {
        let current = NaiveDate::from_ymd_opt(2026, 9, 30).unwrap();
        let mut edit = LedgerEntryEditState::existing(
            7,
            CategoryId::new(2),
            "tag".to_string(),
            "Sep 31".to_string(),
            "10:30".to_string(),
            "Sep 31".to_string(),
            "12:00".to_string(),
        );
        edit.active_field = LedgerEntryField::StartDate;
        edit.normalize_active_input(current).unwrap();
        assert_eq!(edit.start_date, "2026-10-01");
        assert_eq!(edit.end_date, "2026-10-01");
    }

    #[test]
    fn invalid_time_is_rejected_at_the_semantic_boundary() {
        let current = NaiveDate::from_ymd_opt(2026, 9, 30).unwrap();
        let mut edit = LedgerEntryEditState::existing(
            7,
            CategoryId::new(2),
            "tag".to_string(),
            "2026-09-30".to_string(),
            "1:".to_string(),
            "2026-09-30".to_string(),
            "12:00".to_string(),
        );
        edit.active_field = LedgerEntryField::StartTime;
        assert!(edit.normalize_active_input(current).is_ok());
        assert_eq!(edit.start_time, "01:00");

        edit.start_time = "::".to_string();
        assert!(edit.normalize_active_input(current).is_err());
    }

}
