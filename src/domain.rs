use std::{
    collections::{HashMap, HashSet},
    time::Instant,
};

use chrono::{Duration as ChronoDuration, Local};
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
            karma_effect: 1,
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
        let cat_name = self
            .category_store
            .get_by_id(cat_id)
            .map(|category| category.name.as_str())
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
}
