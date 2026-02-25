use std::{
    collections::{HashMap, HashSet},
    time::Instant,
};

use chrono::{Duration as ChronoDuration, Local, NaiveDate};
use ratatui::style::Color;

use crate::constants::COLORS;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct CategoryId(pub u64);

impl CategoryId {
    pub fn new(id: u64) -> Self {
        CategoryId(id)
    }
}

#[derive(Clone, Debug)]
pub struct Category {
    pub id: CategoryId,
    pub name: String,
    pub color: Color,
    pub description: String,
    pub karma_effect: i8,
}

#[derive(Clone, Debug)]
pub struct Session {
    pub id: usize,
    pub date: String,
    pub category_id: CategoryId,
    pub description: String,
    pub start_time: String,
    pub end_time: String,
    pub elapsed_seconds: usize,
}

#[derive(Debug, Clone)]
pub struct ReportEntry {
    pub category_name: String,
    pub elapsed_seconds: usize,
}

#[derive(Debug, Clone)]
pub struct ReportSummary {
    pub date: String,
    pub entries: Vec<ReportEntry>,
    pub total_seconds: usize,
}

#[derive(Debug, Clone)]
pub struct KarmaReportEntry {
    pub category_id: CategoryId,
    pub category_name: String,
    pub color: Color,
    pub elapsed_seconds: usize,
    pub karma_effect: i8,
    pub karma_seconds: isize,
}

#[derive(Debug, Clone)]
pub struct KarmaReportSummary {
    pub date: String,
    pub entries: Vec<KarmaReportEntry>,
    pub total_seconds: usize,
    pub total_karma_seconds: isize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReportPeriod {
    Today,
    Week,
    Month,
}

#[derive(Clone, Debug)]
pub struct CategoryStore {
    by_id: HashMap<CategoryId, Category>,
    order: Vec<CategoryId>,
    next_id: u64,
}

impl CategoryStore {
    pub fn new() -> Self {
        let mut by_id = HashMap::new();
        let none = Category {
            id: CategoryId::new(0),
            name: "none".to_string(),
            color: Color::White,
            description: String::new(),
            karma_effect: 0,
        };
        by_id.insert(none.id, none);

        Self {
            by_id,
            order: vec![CategoryId::new(0)],
            next_id: 1,
        }
    }

    pub fn from_loaded(categories: Vec<Category>, next_id: u64) -> Self {
        let mut store = Self::new();
        let mut seen_names: HashSet<String> = HashSet::new();
        seen_names.insert("none".to_string());

        let mut max_id = 0u64;

        for category in categories {
            max_id = max_id.max(category.id.0);

            if category.id.0 == 0 || category.name.eq_ignore_ascii_case("none") {
                continue;
            }

            if store.by_id.contains_key(&category.id) {
                continue;
            }

            let normalized = category.name.to_lowercase();
            if seen_names.contains(&normalized) {
                continue;
            }

            seen_names.insert(normalized);
            store.order.push(category.id);
            store.by_id.insert(category.id, category);
        }

        store.next_id = next_id.max(max_id + 1).max(1);
        store
    }

    pub fn len(&self) -> usize {
        self.order.len()
    }

    pub fn id_at_index(&self, index: usize) -> Option<CategoryId> {
        self.order.get(index).copied()
    }

    pub fn index_of_id(&self, id: CategoryId) -> Option<usize> {
        self.order.iter().position(|existing| *existing == id)
    }

    pub fn get_by_id(&self, id: CategoryId) -> Option<&Category> {
        self.by_id.get(&id)
    }

    pub fn get_mut_by_id(&mut self, id: CategoryId) -> Option<&mut Category> {
        self.by_id.get_mut(&id)
    }

    pub fn get_by_index(&self, index: usize) -> Option<&Category> {
        let id = self.id_at_index(index)?;
        self.by_id.get(&id)
    }

    pub fn category_id_by_name(&self, name: &str) -> Option<CategoryId> {
        self.order
            .iter()
            .copied()
            .find(|id| self.by_id.get(id).is_some_and(|cat| cat.name == name))
    }

    pub fn ordered_categories(&self) -> Vec<Category> {
        self.order
            .iter()
            .filter_map(|id| self.by_id.get(id).cloned())
            .collect()
    }

    pub fn add_category(
        &mut self,
        name: String,
        description: String,
        color_index: Option<usize>,
    ) -> Option<CategoryId> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return None;
        }

        if self
            .order
            .iter()
            .filter_map(|id| self.by_id.get(id))
            .any(|cat| cat.name.eq_ignore_ascii_case(trimmed))
        {
            return None;
        }

        let id = CategoryId::new(self.next_id);
        self.next_id += 1;

        let color_idx = color_index.unwrap_or(self.order.len() % COLORS.len());
        self.by_id.insert(
            id,
            Category {
                id,
                name: trimmed.to_string(),
                color: COLORS[color_idx % COLORS.len()],
                description,
                karma_effect: 1,
            },
        );
        self.order.push(id);

        Some(id)
    }

    pub fn delete_by_index(&mut self, index: usize) -> Option<CategoryId> {
        if index == 0 || index >= self.order.len() {
            return None;
        }

        let removed_id = self.order.remove(index);
        self.by_id.remove(&removed_id);
        Some(removed_id)
    }

    pub fn move_up(&mut self, index: usize) -> bool {
        if index <= 1 || index >= self.order.len() {
            return false;
        }
        self.order.swap(index - 1, index);
        true
    }

    pub fn move_down(&mut self, index: usize) -> bool {
        if index == 0 || index + 1 >= self.order.len() {
            return false;
        }
        self.order.swap(index, index + 1);
        true
    }

    pub fn set_color_by_index(&mut self, index: usize, color: Color) -> bool {
        if index == 0 {
            return false;
        }

        let Some(id) = self.id_at_index(index) else {
            return false;
        };

        let Some(category) = self.by_id.get_mut(&id) else {
            return false;
        };

        category.color = color;
        true
    }

    pub fn set_description_by_index(&mut self, index: usize, description: String) -> bool {
        let Some(id) = self.id_at_index(index) else {
            return false;
        };

        let Some(category) = self.by_id.get_mut(&id) else {
            return false;
        };

        category.description = description;
        true
    }

    pub fn set_karma_by_index(&mut self, index: usize, karma_effect: i8) -> bool {
        if index == 0 {
            return false;
        }

        let Some(id) = self.id_at_index(index) else {
            return false;
        };

        let Some(category) = self.by_id.get_mut(&id) else {
            return false;
        };

        category.karma_effect = karma_effect;
        true
    }
}

pub struct TimeTracker {
    pub sessions: Vec<Session>,
    category_store: CategoryStore,
    pub current_session_start: Option<Instant>,
    pub session_id_counter: usize,
    active_category_id: CategoryId,
}

impl TimeTracker {
    pub fn new() -> Self {
        Self {
            sessions: Vec::new(),
            category_store: CategoryStore::new(),
            current_session_start: None,
            session_id_counter: 0,
            active_category_id: CategoryId::new(0),
        }
    }

    pub fn apply_loaded_state(
        &mut self,
        categories: Vec<Category>,
        next_category_id: u64,
        sessions: Vec<Session>,
        next_session_id: usize,
    ) {
        self.category_store = CategoryStore::from_loaded(categories, next_category_id);
        self.sessions = sessions;
        self.session_id_counter = next_session_id;

        if self
            .category_store
            .get_by_id(self.active_category_id)
            .is_none()
        {
            self.active_category_id = CategoryId::new(0);
        }
    }

    pub fn category_count(&self) -> usize {
        self.category_store.len()
    }

    pub fn categories_for_storage(&self) -> Vec<Category> {
        self.category_store.ordered_categories()
    }

    pub fn categories_ordered(&self) -> Vec<Category> {
        self.category_store.ordered_categories()
    }

    pub fn category_by_index(&self, index: usize) -> Option<&Category> {
        self.category_store.get_by_index(index)
    }

    pub fn category_description_by_index(&self, index: usize) -> Option<String> {
        self.category_by_index(index)
            .map(|category| category.description.clone())
    }

    pub fn category_id_by_name(&self, name: &str) -> Option<CategoryId> {
        self.category_store.category_id_by_name(name)
    }

    pub fn active_category_id(&self) -> CategoryId {
        self.active_category_id
    }

    pub fn active_category_index(&self) -> Option<usize> {
        self.category_store.index_of_id(self.active_category_id)
    }

    pub fn set_active_category_by_index(&mut self, index: usize) -> bool {
        let Some(id) = self.category_store.id_at_index(index) else {
            return false;
        };
        self.active_category_id = id;
        true
    }

    pub fn set_category_description_by_index(&mut self, index: usize, description: String) -> bool {
        self.category_store
            .set_description_by_index(index, description)
    }

    pub fn set_category_color_by_index(&mut self, index: usize, color: Color) -> bool {
        self.category_store.set_color_by_index(index, color)
    }

    pub fn set_category_karma_by_index(&mut self, index: usize, karma_effect: i8) -> bool {
        self.category_store.set_karma_by_index(index, karma_effect)
    }

    pub fn move_category_up(&mut self, index: usize) -> bool {
        self.category_store.move_up(index)
    }

    pub fn move_category_down(&mut self, index: usize) -> bool {
        self.category_store.move_down(index)
    }

    pub fn add_category(
        &mut self,
        name: String,
        description: String,
        color_index: Option<usize>,
    ) -> Option<CategoryId> {
        self.category_store
            .add_category(name, description, color_index)
    }

    pub fn delete_category(&mut self, index: usize) -> bool {
        let removed = self.category_store.delete_by_index(index);
        if let Some(removed_id) = removed {
            if self.active_category_id == removed_id {
                self.active_category_id = CategoryId::new(0);
            }
            return true;
        }
        false
    }

    pub fn start_session(&mut self) {
        self.current_session_start = Some(Instant::now());
        self.session_id_counter += 1;
    }

    pub fn end_session(&mut self) -> Option<usize> {
        let Some(start_instant) = self.current_session_start else {
            return None;
        };

        let elapsed = start_instant.elapsed().as_secs() as usize;
        let cat_id = self.active_category_id;
        let cat_description = self
            .category_store
            .get_by_id(cat_id)
            .map(|category| category.description.clone())
            .unwrap_or_default();

        self.record_session(cat_id, &cat_description, elapsed);

        if let Some(category) = self.category_store.get_mut_by_id(cat_id) {
            category.description.clear();
        }

        self.current_session_start = None;
        Some(elapsed)
    }

    pub fn record_session(&mut self, cat_id: CategoryId, cat_description: &str, elapsed: usize) {
        let now = Local::now();
        let start_time = now - ChronoDuration::seconds(elapsed as i64);
        let today = now.format("%Y-%m-%d").to_string();

        if let Some(session) = self
            .sessions
            .iter_mut()
            .find(|session| session.category_id == cat_id && session.date == today)
        {
            session.elapsed_seconds += elapsed;
            session.end_time = now.format("%H:%M:%S").to_string();
        } else {
            self.sessions.push(Session {
                id: self.session_id_counter,
                date: today,
                category_id: cat_id,
                description: cat_description.to_string(),
                start_time: start_time.format("%H:%M:%S").to_string(),
                end_time: now.format("%H:%M:%S").to_string(),
                elapsed_seconds: elapsed,
            });
        }
    }

    pub fn get_todays_time(&self) -> usize {
        let today = Local::now().format("%Y-%m-%d").to_string();
        self.sessions
            .iter()
            .filter(|session| session.date == today && session.category_id != CategoryId::new(0))
            .map(|session| session.elapsed_seconds)
            .sum()
    }

    pub fn get_category_time(&self, category_name: &str) -> usize {
        let cat_id = self
            .category_id_by_name(category_name)
            .unwrap_or(CategoryId::new(0));
        let today = Local::now().format("%Y-%m-%d").to_string();
        self.sessions
            .iter()
            .filter(|session| session.date == today && session.category_id == cat_id)
            .map(|session| session.elapsed_seconds)
            .sum()
    }

    pub fn reset_none_counter_today(&mut self) {
        let today = Local::now().format("%Y-%m-%d").to_string();
        self.sessions.retain(|session| {
            !(session.category_id == CategoryId::new(0) && session.date == today)
        });

        if self.active_category_id == CategoryId::new(0) {
            self.current_session_start = Some(Instant::now());
        }
    }
}

pub fn build_today_report(sessions: &[Session], categories: &[Category]) -> ReportSummary {
    let today = Local::now().format("%Y-%m-%d").to_string();
    build_report_for_date(sessions, categories, &today)
}

pub fn build_period_report(
    sessions: &[Session],
    categories: &[Category],
    period: ReportPeriod,
) -> ReportSummary {
    if period == ReportPeriod::Today {
        return build_today_report(sessions, categories);
    }

    let (start, end, label) = period_bounds(period);

    build_report_for_date_range(sessions, categories, start, end, label)
}

pub fn build_period_karma_report(
    sessions: &[Session],
    categories: &[Category],
    period: ReportPeriod,
) -> KarmaReportSummary {
    if period == ReportPeriod::Today {
        return build_today_karma_report(sessions, categories);
    }

    let (start, end, label) = period_bounds(period);

    build_karma_report_for_date_range(sessions, categories, start, end, label)
}

fn period_bounds(period: ReportPeriod) -> (NaiveDate, NaiveDate, String) {
    let today = Local::now().date_naive();

    match period {
        ReportPeriod::Today => {
            let label = today.format("%Y-%m-%d").to_string();
            (today, today, label)
        }
        ReportPeriod::Week => {
            let start = today - ChronoDuration::days(6);
            let label = format!("{}..{}", start.format("%Y-%m-%d"), today.format("%Y-%m-%d"));
            (start, today, label)
        }
        ReportPeriod::Month => {
            let start = today - ChronoDuration::days(29);
            let label = format!("{}..{}", start.format("%Y-%m-%d"), today.format("%Y-%m-%d"));
            (start, today, label)
        }
    }
}

pub fn build_today_karma_report(
    sessions: &[Session],
    categories: &[Category],
) -> KarmaReportSummary {
    let today = Local::now().format("%Y-%m-%d").to_string();
    build_karma_report_for_date(sessions, categories, &today)
}

pub fn build_karma_report_for_date(
    sessions: &[Session],
    categories: &[Category],
    date: &str,
) -> KarmaReportSummary {
    let Some(date) = NaiveDate::parse_from_str(date, "%Y-%m-%d").ok() else {
        return KarmaReportSummary {
            date: String::new(),
            entries: vec![],
            total_seconds: 0,
            total_karma_seconds: 0,
        };
    };

    build_karma_report_for_date_range(
        sessions,
        categories,
        date,
        date,
        date.format("%Y-%m-%d").to_string(),
    )
}

fn build_karma_report_for_date_range(
    sessions: &[Session],
    categories: &[Category],
    start: NaiveDate,
    end: NaiveDate,
    label: String,
) -> KarmaReportSummary {
    let mut entries: Vec<KarmaReportEntry> = categories
        .iter()
        .map(|category| KarmaReportEntry {
            category_id: category.id,
            category_name: category.name.clone(),
            color: category.color,
            elapsed_seconds: 0,
            karma_effect: if category.id == CategoryId::new(0) || category.name == "none" {
                0
            } else {
                category.karma_effect
            },
            karma_seconds: 0,
        })
        .collect();

    let mut by_id: HashMap<CategoryId, usize> = HashMap::new();
    for (idx, entry) in entries.iter().enumerate() {
        by_id.insert(entry.category_id, idx);
    }

    for session in sessions {
        let Some(session_date) = NaiveDate::parse_from_str(&session.date, "%Y-%m-%d").ok() else {
            continue;
        };

        if session_date < start || session_date > end {
            continue;
        }

        if let Some(idx) = by_id.get(&session.category_id).copied() {
            entries[idx].elapsed_seconds += session.elapsed_seconds;
        }
    }

    for entry in &mut entries {
        entry.karma_seconds = entry.elapsed_seconds as isize * entry.karma_effect as isize;
    }

    let total_seconds = entries.iter().map(|entry| entry.elapsed_seconds).sum();
    let total_karma_seconds = entries.iter().map(|entry| entry.karma_seconds).sum();

    KarmaReportSummary {
        date: label,
        entries,
        total_seconds,
        total_karma_seconds,
    }
}

pub fn build_report_for_date(
    sessions: &[Session],
    categories: &[Category],
    date: &str,
) -> ReportSummary {
    let Some(date) = NaiveDate::parse_from_str(date, "%Y-%m-%d").ok() else {
        return ReportSummary {
            date: String::new(),
            entries: vec![],
            total_seconds: 0,
        };
    };

    build_report_for_date_range(
        sessions,
        categories,
        date,
        date,
        date.format("%Y-%m-%d").to_string(),
    )
}

fn build_report_for_date_range(
    sessions: &[Session],
    categories: &[Category],
    start: NaiveDate,
    end: NaiveDate,
    label: String,
) -> ReportSummary {
    let category_names: HashMap<CategoryId, String> = categories
        .iter()
        .filter(|category| category.id != CategoryId::new(0) && category.name != "none")
        .map(|category| (category.id, category.name.clone()))
        .collect();

    let mut totals: HashMap<CategoryId, usize> = HashMap::new();
    for session in sessions {
        let Some(session_date) = NaiveDate::parse_from_str(&session.date, "%Y-%m-%d").ok() else {
            continue;
        };

        if session_date < start || session_date > end {
            continue;
        }

        if category_names.contains_key(&session.category_id) {
            *totals.entry(session.category_id).or_insert(0) += session.elapsed_seconds;
        }
    }

    let mut entries: Vec<ReportEntry> = totals
        .into_iter()
        .filter_map(|(category_id, elapsed_seconds)| {
            category_names.get(&category_id).map(|name| ReportEntry {
                category_name: name.clone(),
                elapsed_seconds,
            })
        })
        .collect();
    entries.sort_by(|a, b| b.elapsed_seconds.cmp(&a.elapsed_seconds));

    let total_seconds = entries.iter().map(|entry| entry.elapsed_seconds).sum();

    ReportSummary {
        date: label,
        entries,
        total_seconds,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_category_id_new() {
        let id1 = CategoryId::new(1);
        let id2 = CategoryId::new(2);
        assert_ne!(id1, id2);
        assert_eq!(id1, CategoryId::new(1));
    }

    #[test]
    fn test_category_store_invariants() {
        let categories = vec![
            Category {
                id: CategoryId::new(0),
                name: "none".to_string(),
                color: Color::White,
                description: String::new(),
                karma_effect: 1,
            },
            Category {
                id: CategoryId::new(1),
                name: "Work".to_string(),
                color: COLORS[0],
                description: String::new(),
                karma_effect: 1,
            },
            Category {
                id: CategoryId::new(1),
                name: "Work Duplicate Id".to_string(),
                color: COLORS[1],
                description: String::new(),
                karma_effect: 1,
            },
            Category {
                id: CategoryId::new(2),
                name: "work".to_string(),
                color: COLORS[2],
                description: String::new(),
                karma_effect: 1,
            },
        ];

        let store = CategoryStore::from_loaded(categories, 3);
        let ordered = store.ordered_categories();

        assert_eq!(
            ordered.first().map(|category| category.id),
            Some(CategoryId::new(0))
        );
        assert_eq!(ordered.len(), 2, "none + one deduped category");
    }

    #[test]
    fn test_category_id_stability_on_reorder() {
        let mut tracker = TimeTracker::new();
        let _ = tracker.add_category("Work".to_string(), "Work category".to_string(), Some(0));
        let _ = tracker.add_category(
            "Personal".to_string(),
            "Personal category".to_string(),
            Some(1),
        );

        tracker.record_session(CategoryId::new(1), "work session", 100);
        tracker.record_session(CategoryId::new(2), "personal session", 200);

        let work_count_before = tracker
            .sessions
            .iter()
            .filter(|session| session.category_id == CategoryId::new(1))
            .count();
        let personal_count_before = tracker
            .sessions
            .iter()
            .filter(|session| session.category_id == CategoryId::new(2))
            .count();

        let moved_down = tracker.move_category_down(1);
        assert!(moved_down);

        let work_count_after = tracker
            .sessions
            .iter()
            .filter(|session| session.category_id == CategoryId::new(1))
            .count();
        let personal_count_after = tracker
            .sessions
            .iter()
            .filter(|session| session.category_id == CategoryId::new(2))
            .count();

        assert_eq!(work_count_before, work_count_after);
        assert_eq!(personal_count_before, personal_count_after);
    }

    #[test]
    fn test_build_report_for_date_excludes_none_and_sorts() {
        let categories = vec![
            Category {
                id: CategoryId::new(0),
                name: "none".to_string(),
                color: Color::White,
                description: String::new(),
                karma_effect: 1,
            },
            Category {
                id: CategoryId::new(1),
                name: "Work".to_string(),
                color: COLORS[0],
                description: String::new(),
                karma_effect: 1,
            },
            Category {
                id: CategoryId::new(2),
                name: "Personal".to_string(),
                color: COLORS[1],
                description: String::new(),
                karma_effect: 1,
            },
        ];

        let sessions = vec![
            Session {
                id: 1,
                date: "2026-02-25".to_string(),
                category_id: CategoryId::new(1),
                description: String::new(),
                start_time: "09:00:00".to_string(),
                end_time: "10:00:00".to_string(),
                elapsed_seconds: 3600,
            },
            Session {
                id: 2,
                date: "2026-02-25".to_string(),
                category_id: CategoryId::new(2),
                description: String::new(),
                start_time: "10:00:00".to_string(),
                end_time: "10:30:00".to_string(),
                elapsed_seconds: 1800,
            },
            Session {
                id: 3,
                date: "2026-02-25".to_string(),
                category_id: CategoryId::new(0),
                description: String::new(),
                start_time: "11:00:00".to_string(),
                end_time: "12:00:00".to_string(),
                elapsed_seconds: 3600,
            },
            Session {
                id: 4,
                date: "2026-02-24".to_string(),
                category_id: CategoryId::new(1),
                description: String::new(),
                start_time: "09:00:00".to_string(),
                end_time: "10:00:00".to_string(),
                elapsed_seconds: 3600,
            },
        ];

        let summary = build_report_for_date(&sessions, &categories, "2026-02-25");
        assert_eq!(summary.total_seconds, 5400);
        assert_eq!(summary.entries.len(), 2);
        assert_eq!(summary.entries[0].category_name, "Work");
        assert_eq!(summary.entries[0].elapsed_seconds, 3600);
        assert_eq!(summary.entries[1].category_name, "Personal");
        assert_eq!(summary.entries[1].elapsed_seconds, 1800);
    }

    #[test]
    fn test_build_karma_report_for_date_tracks_totals_and_zero_entries() {
        let categories = vec![
            Category {
                id: CategoryId::new(0),
                name: "none".to_string(),
                color: Color::White,
                description: String::new(),
                karma_effect: 1,
            },
            Category {
                id: CategoryId::new(1),
                name: "Work".to_string(),
                color: COLORS[0],
                description: String::new(),
                karma_effect: 1,
            },
            Category {
                id: CategoryId::new(2),
                name: "Gaming".to_string(),
                color: COLORS[5],
                description: String::new(),
                karma_effect: -1,
            },
            Category {
                id: CategoryId::new(3),
                name: "Reading".to_string(),
                color: COLORS[2],
                description: String::new(),
                karma_effect: 1,
            },
        ];

        let sessions = vec![
            Session {
                id: 1,
                date: "2026-02-25".to_string(),
                category_id: CategoryId::new(1),
                description: String::new(),
                start_time: "08:00:00".to_string(),
                end_time: "09:00:00".to_string(),
                elapsed_seconds: 3600,
            },
            Session {
                id: 2,
                date: "2026-02-25".to_string(),
                category_id: CategoryId::new(2),
                description: String::new(),
                start_time: "10:00:00".to_string(),
                end_time: "10:30:00".to_string(),
                elapsed_seconds: 1800,
            },
        ];

        let summary = build_karma_report_for_date(&sessions, &categories, "2026-02-25");
        assert_eq!(summary.entries.len(), 4, "all categories are listed");
        assert_eq!(summary.total_seconds, 5400);

        let work = summary
            .entries
            .iter()
            .find(|entry| entry.category_name == "Work")
            .expect("work entry");
        assert_eq!(work.elapsed_seconds, 3600);
        assert_eq!(work.karma_seconds, 3600);

        let gaming = summary
            .entries
            .iter()
            .find(|entry| entry.category_name == "Gaming")
            .expect("gaming entry");
        assert_eq!(gaming.elapsed_seconds, 1800);
        assert_eq!(gaming.karma_seconds, -1800);

        let reading = summary
            .entries
            .iter()
            .find(|entry| entry.category_name == "Reading")
            .expect("reading entry");
        assert_eq!(
            reading.elapsed_seconds, 0,
            "zero-time categories are included"
        );

        let none = summary
            .entries
            .iter()
            .find(|entry| entry.category_name == "none")
            .expect("none entry");
        assert_eq!(none.elapsed_seconds, 0);
        assert_eq!(none.karma_seconds, 0);
        assert_eq!(none.karma_effect, 0);

        assert_eq!(summary.total_karma_seconds, 1800);
    }

    #[test]
    fn test_build_karma_report_includes_none_as_neutral_counter() {
        let categories = vec![
            Category {
                id: CategoryId::new(0),
                name: "none".to_string(),
                color: Color::White,
                description: String::new(),
                karma_effect: 1,
            },
            Category {
                id: CategoryId::new(1),
                name: "Work".to_string(),
                color: COLORS[0],
                description: String::new(),
                karma_effect: 1,
            },
        ];

        let sessions = vec![
            Session {
                id: 1,
                date: "2026-02-25".to_string(),
                category_id: CategoryId::new(0),
                description: String::new(),
                start_time: "08:00:00".to_string(),
                end_time: "08:20:00".to_string(),
                elapsed_seconds: 1200,
            },
            Session {
                id: 2,
                date: "2026-02-25".to_string(),
                category_id: CategoryId::new(1),
                description: String::new(),
                start_time: "09:00:00".to_string(),
                end_time: "09:30:00".to_string(),
                elapsed_seconds: 1800,
            },
        ];

        let summary = build_karma_report_for_date(&sessions, &categories, "2026-02-25");

        assert_eq!(summary.total_seconds, 3000);
        assert_eq!(summary.total_karma_seconds, 1800);

        let none = summary
            .entries
            .iter()
            .find(|entry| entry.category_name == "none")
            .expect("none entry");
        assert_eq!(none.elapsed_seconds, 1200);
        assert_eq!(none.karma_effect, 0);
        assert_eq!(none.karma_seconds, 0);
    }

    #[test]
    fn test_build_period_report_week_includes_last_seven_days() {
        let categories = vec![
            Category {
                id: CategoryId::new(0),
                name: "none".to_string(),
                color: Color::White,
                description: String::new(),
                karma_effect: 1,
            },
            Category {
                id: CategoryId::new(1),
                name: "Work".to_string(),
                color: COLORS[0],
                description: String::new(),
                karma_effect: 1,
            },
        ];

        let today = Local::now().date_naive();
        let in_window = (today - ChronoDuration::days(6))
            .format("%Y-%m-%d")
            .to_string();
        let out_window = (today - ChronoDuration::days(7))
            .format("%Y-%m-%d")
            .to_string();

        let sessions = vec![
            Session {
                id: 1,
                date: today.format("%Y-%m-%d").to_string(),
                category_id: CategoryId::new(1),
                description: String::new(),
                start_time: "09:00:00".to_string(),
                end_time: "10:00:00".to_string(),
                elapsed_seconds: 3600,
            },
            Session {
                id: 2,
                date: in_window,
                category_id: CategoryId::new(1),
                description: String::new(),
                start_time: "09:00:00".to_string(),
                end_time: "09:30:00".to_string(),
                elapsed_seconds: 1800,
            },
            Session {
                id: 3,
                date: out_window,
                category_id: CategoryId::new(1),
                description: String::new(),
                start_time: "09:00:00".to_string(),
                end_time: "11:00:00".to_string(),
                elapsed_seconds: 7200,
            },
        ];

        let summary = build_period_report(&sessions, &categories, ReportPeriod::Week);
        assert_eq!(summary.total_seconds, 5400);
        assert_eq!(summary.entries.len(), 1);
        assert_eq!(summary.entries[0].category_name, "Work");
        assert_eq!(summary.entries[0].elapsed_seconds, 5400);
    }

    #[test]
    fn test_build_period_karma_report_month_aggregates_range() {
        let categories = vec![
            Category {
                id: CategoryId::new(0),
                name: "none".to_string(),
                color: Color::White,
                description: String::new(),
                karma_effect: 1,
            },
            Category {
                id: CategoryId::new(1),
                name: "Work".to_string(),
                color: COLORS[0],
                description: String::new(),
                karma_effect: 1,
            },
            Category {
                id: CategoryId::new(2),
                name: "Gaming".to_string(),
                color: COLORS[5],
                description: String::new(),
                karma_effect: -1,
            },
        ];

        let today = Local::now().date_naive();
        let in_window = (today - ChronoDuration::days(29))
            .format("%Y-%m-%d")
            .to_string();
        let out_window = (today - ChronoDuration::days(30))
            .format("%Y-%m-%d")
            .to_string();

        let sessions = vec![
            Session {
                id: 1,
                date: in_window,
                category_id: CategoryId::new(1),
                description: String::new(),
                start_time: "08:00:00".to_string(),
                end_time: "09:00:00".to_string(),
                elapsed_seconds: 3600,
            },
            Session {
                id: 2,
                date: today.format("%Y-%m-%d").to_string(),
                category_id: CategoryId::new(2),
                description: String::new(),
                start_time: "10:00:00".to_string(),
                end_time: "10:30:00".to_string(),
                elapsed_seconds: 1800,
            },
            Session {
                id: 3,
                date: out_window,
                category_id: CategoryId::new(1),
                description: String::new(),
                start_time: "12:00:00".to_string(),
                end_time: "13:00:00".to_string(),
                elapsed_seconds: 3600,
            },
        ];

        let summary = build_period_karma_report(&sessions, &categories, ReportPeriod::Month);
        assert_eq!(summary.total_seconds, 5400);
        assert_eq!(summary.total_karma_seconds, 1800);

        let work = summary
            .entries
            .iter()
            .find(|entry| entry.category_name == "Work")
            .expect("work entry");
        assert_eq!(work.elapsed_seconds, 3600);
        assert_eq!(work.karma_seconds, 3600);

        let gaming = summary
            .entries
            .iter()
            .find(|entry| entry.category_name == "Gaming")
            .expect("gaming entry");
        assert_eq!(gaming.elapsed_seconds, 1800);
        assert_eq!(gaming.karma_seconds, -1800);
    }

    #[test]
    fn test_build_period_karma_report_today_path_is_non_recursive() {
        let categories = vec![
            Category {
                id: CategoryId::new(0),
                name: "none".to_string(),
                color: Color::White,
                description: String::new(),
                karma_effect: 1,
            },
            Category {
                id: CategoryId::new(1),
                name: "Work".to_string(),
                color: COLORS[0],
                description: String::new(),
                karma_effect: 1,
            },
        ];

        let today = Local::now().format("%Y-%m-%d").to_string();
        let sessions = vec![Session {
            id: 1,
            date: today.clone(),
            category_id: CategoryId::new(1),
            description: String::new(),
            start_time: "09:00:00".to_string(),
            end_time: "09:10:00".to_string(),
            elapsed_seconds: 600,
        }];

        let summary = build_period_karma_report(&sessions, &categories, ReportPeriod::Today);
        assert_eq!(summary.date, today);
        assert_eq!(summary.total_seconds, 600);
        assert_eq!(summary.total_karma_seconds, 600);
    }

    #[test]
    fn test_reset_none_counter_today_clears_only_today_none() {
        let mut tracker = TimeTracker::new();
        let today = Local::now().format("%Y-%m-%d").to_string();
        let yesterday = (Local::now().date_naive() - ChronoDuration::days(1))
            .format("%Y-%m-%d")
            .to_string();

        tracker.sessions = vec![
            Session {
                id: 1,
                date: today.clone(),
                category_id: CategoryId::new(0),
                description: String::new(),
                start_time: "08:00:00".to_string(),
                end_time: "08:10:00".to_string(),
                elapsed_seconds: 600,
            },
            Session {
                id: 2,
                date: yesterday,
                category_id: CategoryId::new(0),
                description: String::new(),
                start_time: "08:00:00".to_string(),
                end_time: "08:10:00".to_string(),
                elapsed_seconds: 600,
            },
            Session {
                id: 3,
                date: today,
                category_id: CategoryId::new(1),
                description: String::new(),
                start_time: "09:00:00".to_string(),
                end_time: "09:10:00".to_string(),
                elapsed_seconds: 600,
            },
        ];

        tracker.reset_none_counter_today();

        assert_eq!(
            tracker
                .sessions
                .iter()
                .filter(|session| session.category_id == CategoryId::new(0))
                .count(),
            1
        );
        assert_eq!(tracker.sessions.len(), 2);
    }
}
