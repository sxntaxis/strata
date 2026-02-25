use std::{path::Path, time::Instant};

use chrono::{Duration as ChronoDuration, Local};
use ratatui::style::Color;

use crate::{
    constants::{COLORS, FILE_PATHS},
    storage,
};

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

pub struct TimeTracker {
    pub sessions: Vec<Session>,
    pub categories: Vec<Category>,
    pub next_category_id: u64,
    pub current_session_start: Option<Instant>,
    pub session_id_counter: usize,
    pub active_category_index: Option<usize>,
}

impl TimeTracker {
    pub fn new() -> Self {
        let mut tt = Self {
            sessions: Vec::new(),
            categories: vec![Category {
                id: CategoryId::new(0),
                name: "none".to_string(),
                color: Color::White,
                description: String::new(),
                karma_effect: 1,
            }],
            next_category_id: 1,
            current_session_start: None,
            session_id_counter: 0,
            active_category_index: Some(0),
        };
        tt.load_sessions();
        tt
    }

    pub fn category_id_by_name(&self, name: &str) -> Option<CategoryId> {
        self.categories
            .iter()
            .find(|c| c.name == name)
            .map(|c| c.id)
    }

    pub fn load_sessions(&mut self) {
        self.load_categories();

        let loaded =
            storage::load_sessions_from_csv(Path::new(FILE_PATHS.time_log), &self.categories);
        self.sessions = loaded.sessions;
        self.session_id_counter = loaded.next_session_id;
    }

    pub fn save_sessions(&self) {
        let _ = storage::save_sessions_to_csv(
            Path::new(FILE_PATHS.time_log),
            &self.sessions,
            &self.categories,
        );
    }

    pub fn load_categories(&mut self) {
        let loaded = storage::load_categories_from_csv(Path::new(FILE_PATHS.categories));
        self.categories = loaded.categories;
        self.next_category_id = loaded.next_category_id;
    }

    pub fn save_categories(&self) {
        let _ = storage::save_categories_to_csv(Path::new(FILE_PATHS.categories), &self.categories);
    }

    pub fn start_session(&mut self) {
        if self.active_category_index.is_some() {
            self.current_session_start = Some(Instant::now());
            self.session_id_counter += 1;
        }
    }

    pub fn end_session(&mut self) -> Option<usize> {
        if let Some(start_instant) = self.current_session_start {
            let elapsed = start_instant.elapsed().as_secs() as usize;

            if let Some(cat_idx) = self.active_category_index {
                let cat_id = self.categories[cat_idx].id;
                let cat_description = self.categories[cat_idx].description.clone();
                self.record_session(cat_id, &cat_description, elapsed);
                self.categories[cat_idx].description.clear();
            }

            self.current_session_start = None;
            self.save_sessions();
            return Some(elapsed);
        }

        None
    }

    pub fn record_session(&mut self, cat_id: CategoryId, cat_description: &str, elapsed: usize) {
        let cat_name = self
            .categories
            .iter()
            .find(|c| c.id == cat_id)
            .map(|c| c.name.as_str())
            .unwrap_or("none");

        if cat_name == "none" {
            return;
        }

        let now = Local::now();
        let start_time = now - ChronoDuration::seconds(elapsed as i64);
        let today = now.format("%Y-%m-%d").to_string();

        if let Some(session) = self
            .sessions
            .iter_mut()
            .find(|s| s.category_id == cat_id && s.date == today)
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
        let none_id = CategoryId::new(0);
        self.sessions
            .iter()
            .filter(|s| s.date == today && s.category_id != none_id)
            .map(|s| s.elapsed_seconds)
            .sum()
    }

    pub fn get_category_time(&self, category_name: &str) -> usize {
        let cat_id = self
            .category_id_by_name(category_name)
            .unwrap_or(CategoryId::new(0));
        let today = Local::now().format("%Y-%m-%d").to_string();
        self.sessions
            .iter()
            .filter(|s| s.date == today && s.category_id == cat_id)
            .map(|s| s.elapsed_seconds)
            .sum()
    }

    pub fn add_category(&mut self, name: String, description: String, color_index: Option<usize>) {
        let color_idx = color_index.unwrap_or(self.categories.len() % COLORS.len());
        let id = CategoryId::new(self.next_category_id);
        self.next_category_id += 1;

        self.categories.push(Category {
            id,
            name,
            color: COLORS[color_idx],
            description,
            karma_effect: 1,
        });

        self.save_categories();
    }

    pub fn delete_category(&mut self, index: usize) {
        if index > 0 && index < self.categories.len() {
            self.categories.remove(index);
            if self.active_category_index == Some(index) {
                self.active_category_index = Some(0);
            }
            self.save_categories();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf, time::SystemTime};

    use super::*;

    #[test]
    fn test_category_id_new() {
        let id1 = CategoryId::new(1);
        let id2 = CategoryId::new(2);
        assert_ne!(id1, id2);
        assert_eq!(id1, CategoryId::new(1));
    }

    #[test]
    fn test_load_categories_idempotent() {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let path = PathBuf::from(format!("/tmp/strata_test_categories_{}.csv", now));

        let content =
            "id,name,description,color_index,karma_effect\n1,Work,,0,1\n2,Personal,,1,1\n";
        fs::write(&path, content).unwrap();

        let first = storage::load_categories_from_csv(&path);
        let second = storage::load_categories_from_csv(&path);

        assert_eq!(first.categories.len(), second.categories.len());
        assert_eq!(first.categories.len(), 3);
        assert_eq!(first.next_category_id, second.next_category_id);

        fs::remove_file(path).ok();
    }

    #[test]
    fn test_category_id_stability_on_reorder() {
        let mut tt = TimeTracker {
            sessions: Vec::new(),
            categories: vec![
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
                    description: "Work category".to_string(),
                    karma_effect: 1,
                },
                Category {
                    id: CategoryId::new(2),
                    name: "Personal".to_string(),
                    color: COLORS[1],
                    description: "Personal category".to_string(),
                    karma_effect: 1,
                },
            ],
            next_category_id: 3,
            current_session_start: None,
            session_id_counter: 1,
            active_category_index: Some(1),
        };

        tt.record_session(CategoryId::new(1), "work session", 100);
        tt.record_session(CategoryId::new(2), "personal session", 200);

        let work_count_before = tt
            .sessions
            .iter()
            .filter(|s| s.category_id == CategoryId::new(1))
            .count();
        let personal_count_before = tt
            .sessions
            .iter()
            .filter(|s| s.category_id == CategoryId::new(2))
            .count();

        tt.categories.swap(1, 2);

        let work_count_after = tt
            .sessions
            .iter()
            .filter(|s| s.category_id == CategoryId::new(1))
            .count();
        let personal_count_after = tt
            .sessions
            .iter()
            .filter(|s| s.category_id == CategoryId::new(2))
            .count();

        assert_eq!(work_count_before, work_count_after);
        assert_eq!(personal_count_before, personal_count_after);
    }
}
