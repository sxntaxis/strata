use super::HistoricalActivityConfirmation;
use crate::domain::CategoryId;

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
    pub(super) confirmation: Option<HistoricalActivityConfirmation>,
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

    pub(super) fn next_field(&mut self) {
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
        let limit = match active {
            LedgerEntryField::Description => usize::MAX,
            LedgerEntryField::StartDate | LedgerEntryField::EndDate => 10,
            LedgerEntryField::StartTime | LedgerEntryField::EndTime => 8,
        };
        if self.active_value_mut().len() < limit {
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
    use super::{LedgerEntryEditState, LedgerEntryField};
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
}
