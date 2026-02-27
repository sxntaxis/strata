use crate::domain::operational_day_key_now;

use super::App;

impl App {
    pub(super) fn get_effective_time_today(&self) -> usize {
        self.time_tracker.get_todays_time()
    }

    pub(super) fn get_effective_time_for_category(&self, category_name: &str) -> usize {
        self.time_tracker.get_category_time(category_name)
    }

    pub(super) fn get_karma_adjusted_time(&self) -> isize {
        let today = operational_day_key_now().format("%Y-%m-%d").to_string();
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

    pub(super) fn get_category_karma_adjusted_time(&self, category_name: &str) -> isize {
        let today = operational_day_key_now().format("%Y-%m-%d").to_string();
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

    pub(super) fn format_signed_time(&self, seconds: isize) -> String {
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

    pub(super) fn format_karma_time(&self, seconds: isize) -> String {
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

    pub(super) fn format_time(&self, seconds: usize) -> String {
        format!(
            "{:02}:{:02}:{:02}",
            seconds / 3600,
            (seconds % 3600) / 60,
            seconds % 60
        )
    }

    pub(super) fn truncate_label(&self, value: &str, max_chars: usize) -> String {
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
}
