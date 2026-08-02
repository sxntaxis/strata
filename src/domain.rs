use std::{
    collections::{HashMap, HashSet},
    sync::{OnceLock, RwLock},
    time::{Duration, Instant},
};

#[cfg(test)]
use chrono::Local;
use chrono::{DateTime, Datelike, Duration as ChronoDuration, FixedOffset, NaiveDate, Utc};
use ratatui::style::Color;

use crate::{constants::COLORS, temporal};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct CategoryId(pub u64);

impl CategoryId {
    pub fn new(id: u64) -> Self {
        CategoryId(id)
    }
}

pub const DRIFT_CATEGORY_ID: CategoryId = CategoryId(0);
pub const DRIFT_CATEGORY_CONFIG_NAME: &str = "idle";
pub const DRIFT_CATEGORY_DISPLAY_NAME: &str = "idle";

pub fn is_drift_category_id(category_id: CategoryId) -> bool {
    category_id == DRIFT_CATEGORY_ID
}

pub fn is_drift_name(name: &str) -> bool {
    matches!(
        name.trim().to_ascii_lowercase().as_str(),
        "idle" | "none" | "drift"
    )
}

#[derive(Clone, Debug)]
pub struct Category {
    pub id: CategoryId,
    pub name: String,
    pub color: Color,
    pub description: String,
    pub karma_effect: i8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OperationalDayPolicy {
    pub utc_offset_seconds: i32,
    pub start_minutes: u16,
}

impl OperationalDayPolicy {
    pub fn from_config(config: DayBoundaryConfig) -> Self {
        let minutes = config
            .fixed_hour
            .saturating_mul(60)
            .saturating_add(config.fixed_minute);
        Self {
            utc_offset_seconds: config.utc_offset_seconds,
            start_minutes: u16::try_from(minutes).unwrap_or(0),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Session {
    pub id: usize,
    pub date: String,
    pub category_id: CategoryId,
    pub project: String,
    pub description: String,
    pub start_time: String,
    pub end_time: String,
    pub elapsed_seconds: usize,
    pub started_at_utc: Option<DateTime<Utc>>,
    pub ended_at_utc: Option<DateTime<Utc>>,
    pub operational_day_policy: Option<OperationalDayPolicy>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SessionSlice {
    pub operational_day: NaiveDate,
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

#[derive(Debug, Clone)]
pub struct CategoryLogEntry {
    pub session_id: Option<usize>,
    pub date: String,
    pub start_time: String,
    pub end_time: String,
    pub description: String,
    pub elapsed_seconds: usize,
    pub karma_effect: i8,
    pub karma_seconds: isize,
}

#[derive(Debug, Clone)]
pub struct LiveSessionPreview {
    pub category_id: CategoryId,
    pub description: String,
    pub elapsed_seconds: usize,
    pub started_at_utc: DateTime<Utc>,
    pub ended_at_utc: DateTime<Utc>,
    pub operational_day_policy: OperationalDayPolicy,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReportPeriod {
    Today,
    Week,
    Month,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DayBoundaryMode {
    FixedHour,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FirstDayOfWeek {
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
    Sunday,
}

impl FirstDayOfWeek {
    pub fn from_config_name(name: &str) -> Option<Self> {
        match name.trim().to_ascii_lowercase().as_str() {
            "monday" | "mon" => Some(Self::Monday),
            "tuesday" | "tue" | "tues" => Some(Self::Tuesday),
            "wednesday" | "wed" => Some(Self::Wednesday),
            "thursday" | "thu" | "thurs" => Some(Self::Thursday),
            "friday" | "fri" => Some(Self::Friday),
            "saturday" | "sat" => Some(Self::Saturday),
            "sunday" | "sun" => Some(Self::Sunday),
            _ => None,
        }
    }

    pub const fn as_config_name(self) -> &'static str {
        match self {
            Self::Monday => "monday",
            Self::Tuesday => "tuesday",
            Self::Wednesday => "wednesday",
            Self::Thursday => "thursday",
            Self::Friday => "friday",
            Self::Saturday => "saturday",
            Self::Sunday => "sunday",
        }
    }

    const fn num_days_from_monday(self) -> u32 {
        match self {
            Self::Monday => 0,
            Self::Tuesday => 1,
            Self::Wednesday => 2,
            Self::Thursday => 3,
            Self::Friday => 4,
            Self::Saturday => 5,
            Self::Sunday => 6,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DayBoundaryConfig {
    pub mode: DayBoundaryMode,
    pub fixed_hour: u32,
    pub fixed_minute: u32,
    pub utc_offset_seconds: i32,
}

impl Default for DayBoundaryConfig {
    fn default() -> Self {
        Self {
            mode: DayBoundaryMode::FixedHour,
            fixed_hour: 6,
            fixed_minute: 0,
            utc_offset_seconds: -6 * 60 * 60,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuntimeSettings {
    pub day_boundary: DayBoundaryConfig,
    pub first_day_of_week: FirstDayOfWeek,
}

impl Default for RuntimeSettings {
    fn default() -> Self {
        Self {
            day_boundary: DayBoundaryConfig::default(),
            first_day_of_week: FirstDayOfWeek::Monday,
        }
    }
}

fn runtime_settings_cell() -> &'static RwLock<RuntimeSettings> {
    static CELL: OnceLock<RwLock<RuntimeSettings>> = OnceLock::new();
    CELL.get_or_init(|| RwLock::new(RuntimeSettings::default()))
}

pub fn runtime_settings() -> RuntimeSettings {
    runtime_settings_cell()
        .read()
        .map(|guard| *guard)
        .unwrap_or_default()
}

pub fn set_runtime_settings(settings: RuntimeSettings) {
    if let Ok(mut guard) = runtime_settings_cell().write() {
        *guard = settings;
    }
}

pub fn day_boundary_config() -> DayBoundaryConfig {
    runtime_settings().day_boundary
}

pub fn operational_day_key_now() -> NaiveDate {
    operational_day_key_from_utc(Utc::now(), &day_boundary_config())
}

pub fn operational_day_key_for_utc(timestamp: DateTime<Utc>) -> NaiveDate {
    operational_day_key_from_utc(timestamp, &day_boundary_config())
}

pub fn civil_time_for_utc(timestamp: DateTime<Utc>) -> DateTime<FixedOffset> {
    temporal::civil_from_utc(timestamp, &day_boundary_config())
        .expect("runtime UTC offset must be validated before time authority is used")
}

pub fn report_period_date_bounds_with_offset(
    period: ReportPeriod,
    offset: usize,
) -> (NaiveDate, NaiveDate) {
    let (start, end, _) = period_bounds_with_offset(period, offset);
    (start, end)
}

pub(crate) fn operational_day_key_from_utc(
    now_utc: DateTime<Utc>,
    config: &DayBoundaryConfig,
) -> NaiveDate {
    temporal::operational_day_from_utc(now_utc, config)
        .expect("runtime time policy must be validated before operational-day allocation")
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
            id: DRIFT_CATEGORY_ID,
            name: DRIFT_CATEGORY_CONFIG_NAME.to_string(),
            color: Color::White,
            description: String::new(),
            karma_effect: 0,
        };
        by_id.insert(none.id, none);

        Self {
            by_id,
            order: vec![DRIFT_CATEGORY_ID],
            next_id: 1,
        }
    }

    pub fn from_loaded(categories: Vec<Category>, next_id: u64) -> Self {
        let mut store = Self::new();
        let mut seen_names: HashSet<String> = HashSet::new();
        seen_names.insert(DRIFT_CATEGORY_CONFIG_NAME.to_string());

        let mut max_id = 0u64;

        for category in categories {
            max_id = max_id.max(category.id.0);

            if is_drift_category_id(category.id) || is_drift_name(&category.name) {
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

    pub fn restore_category(&mut self, mut category: Category) -> bool {
        let trimmed = category.name.trim();
        if category.id == DRIFT_CATEGORY_ID || trimmed.is_empty() {
            return false;
        }
        if self.by_id.contains_key(&category.id)
            || self
                .order
                .iter()
                .filter_map(|id| self.by_id.get(id))
                .any(|existing| existing.name.eq_ignore_ascii_case(trimmed))
        {
            return false;
        }

        category.name = trimmed.to_string();
        self.next_id = self.next_id.max(category.id.0.saturating_add(1));
        self.order.push(category.id);
        self.by_id.insert(category.id, category);
        true
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
            session_id_counter: 1,
            active_category_id: DRIFT_CATEGORY_ID,
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
            self.active_category_id = DRIFT_CATEGORY_ID;
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

    pub fn category_by_id(&self, id: CategoryId) -> Option<&Category> {
        self.category_store.get_by_id(id)
    }

    pub fn category_description_by_index(&self, index: usize) -> Option<String> {
        self.category_by_index(index)
            .map(|category| category.description.clone())
    }

    pub fn category_id_by_name(&self, name: &str) -> Option<CategoryId> {
        self.category_store.category_id_by_name(name)
    }

    pub fn category_color_by_id(&self, id: CategoryId) -> Option<Color> {
        self.category_by_id(id).map(|category| category.color)
    }

    pub fn category_description_by_id(&self, id: CategoryId) -> Option<&str> {
        self.category_by_id(id)
            .map(|category| category.description.as_str())
    }

    pub fn active_category_id(&self) -> CategoryId {
        self.active_category_id
    }

    pub fn active_category_index(&self) -> Option<usize> {
        self.category_store.index_of_id(self.active_category_id)
    }

    pub fn set_active_category_by_id(&mut self, category_id: CategoryId) -> bool {
        if self.category_store.get_by_id(category_id).is_none() {
            return false;
        }

        self.active_category_id = category_id;
        true
    }

    pub fn set_category_description_by_index(&mut self, index: usize, description: String) -> bool {
        self.category_store
            .set_description_by_index(index, description)
    }

    pub fn set_category_description_by_id(
        &mut self,
        category_id: CategoryId,
        description: String,
    ) -> bool {
        let Some(category) = self.category_store.get_mut_by_id(category_id) else {
            return false;
        };

        category.description = description;
        true
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

    pub fn restore_category(&mut self, category: Category) -> bool {
        self.category_store.restore_category(category)
    }

    pub fn delete_category(&mut self, index: usize) -> bool {
        let removed = self.category_store.delete_by_index(index);
        if let Some(removed_id) = removed {
            if self.active_category_id == removed_id {
                self.active_category_id = DRIFT_CATEGORY_ID;
            }
            return true;
        }
        false
    }

    pub fn start_session(&mut self) {
        self.current_session_start = Some(Instant::now());
    }

    pub fn start_session_with_elapsed(&mut self, elapsed_seconds: usize) -> Result<(), String> {
        let offset = Duration::from_secs(elapsed_seconds as u64);
        let start = Instant::now().checked_sub(offset).ok_or_else(|| {
            format!(
                "elapsed interval of {elapsed_seconds} seconds exceeds the monotonic clock range"
            )
        })?;
        self.current_session_start = Some(start);
        Ok(())
    }

    pub fn current_elapsed(&self) -> Option<Duration> {
        self.current_session_start.map(|start| start.elapsed())
    }

    pub fn end_session_with_elapsed_at_local<Tz>(
        &mut self,
        elapsed: usize,
        end_local: DateTime<Tz>,
    ) -> Option<usize>
    where
        Tz: chrono::TimeZone,
        Tz::Offset: std::fmt::Display,
    {
        self.current_session_start?;
        let cat_id = self.active_category_id;
        let cat_description = self
            .category_store
            .get_by_id(cat_id)
            .map(|category| category.description.clone())
            .unwrap_or_default();

        if elapsed > 0 {
            self.record_session_at(cat_id, &cat_description, elapsed, end_local);
        }

        if let Some(category) = self.category_store.get_mut_by_id(cat_id) {
            category.description.clear();
        }

        self.current_session_start = None;
        Some(elapsed)
    }

    pub fn record_session_at<Tz>(
        &mut self,
        cat_id: CategoryId,
        cat_description: &str,
        elapsed: usize,
        end_local: DateTime<Tz>,
    ) where
        Tz: chrono::TimeZone,
        Tz::Offset: std::fmt::Display,
    {
        if elapsed == 0 {
            return;
        }
        let end_utc = end_local.with_timezone(&Utc);
        let start_utc = end_utc - ChronoDuration::seconds(elapsed as i64);
        let start_time = end_local.clone() - ChronoDuration::seconds(elapsed as i64);
        let today = operational_day_key_for_utc(end_utc)
            .format("%Y-%m-%d")
            .to_string();

        self.sessions.push(Session {
            id: self.session_id_counter,
            date: today,
            category_id: cat_id,
            project: String::new(),
            description: cat_description.to_string(),
            start_time: start_time.format("%H:%M:%S").to_string(),
            end_time: end_local.format("%H:%M:%S").to_string(),
            elapsed_seconds: elapsed,
            started_at_utc: Some(start_utc),
            ended_at_utc: Some(end_utc),
            operational_day_policy: Some(OperationalDayPolicy::from_config(day_boundary_config())),
        });
        self.session_id_counter += 1;
    }

    pub fn get_todays_time(&self) -> usize {
        let today = operational_day_key_now();
        self.sessions
            .iter()
            .filter(|session| !is_drift_category_id(session.category_id))
            .flat_map(session_slices)
            .filter(|slice| slice.operational_day == today)
            .map(|slice| slice.elapsed_seconds)
            .sum()
    }

    pub fn get_category_time(&self, category_name: &str) -> usize {
        let cat_id = self
            .category_id_by_name(category_name)
            .unwrap_or(DRIFT_CATEGORY_ID);
        let today = operational_day_key_now();
        self.sessions
            .iter()
            .filter(|session| session.category_id == cat_id)
            .flat_map(session_slices)
            .filter(|slice| slice.operational_day == today)
            .map(|slice| slice.elapsed_seconds)
            .sum()
    }

    pub fn delete_session_by_id(&mut self, session_id: usize) -> bool {
        let Some(index) = self
            .sessions
            .iter()
            .position(|session| session.id == session_id)
        else {
            return false;
        };

        self.sessions.remove(index);
        true
    }

    pub fn set_session_description_by_id(
        &mut self,
        session_id: usize,
        description: String,
    ) -> bool {
        let Some(session) = self
            .sessions
            .iter_mut()
            .find(|session| session.id == session_id)
        else {
            return false;
        };

        session.description = description;
        true
    }

    pub fn clear_drift_sessions_for_day(&mut self, day: NaiveDate) {
        let day_key = day.format("%Y-%m-%d").to_string();
        self.sessions.retain(|session| {
            !(is_drift_category_id(session.category_id) && session.date == day_key)
        });
    }
}

pub(crate) fn session_slices(session: &Session) -> Vec<SessionSlice> {
    let complete = match (
        session.started_at_utc,
        session.ended_at_utc,
        session.operational_day_policy,
    ) {
        (Some(started_at_utc), Some(ended_at_utc), Some(policy)) => {
            Some((started_at_utc, ended_at_utc, policy))
        }
        _ => None,
    };

    if let Some((started_at_utc, ended_at_utc, policy)) = complete
        && let Ok(slices) = temporal::allocate_operational_day_slices(
            started_at_utc,
            ended_at_utc,
            session.elapsed_seconds,
            policy,
        )
    {
        return slices
            .into_iter()
            .map(|slice| SessionSlice {
                operational_day: slice.operational_day,
                start_time: temporal::civil_from_policy(slice.started_at_utc, policy)
                    .map(|value| value.format("%H:%M:%S").to_string())
                    .unwrap_or_else(|_| session.start_time.clone()),
                end_time: temporal::civil_from_policy(slice.ended_at_utc, policy)
                    .map(|value| value.format("%H:%M:%S").to_string())
                    .unwrap_or_else(|_| session.end_time.clone()),
                elapsed_seconds: slice.elapsed_seconds,
            })
            .collect();
    }

    let Some(day) = NaiveDate::parse_from_str(&session.date, "%Y-%m-%d").ok() else {
        return Vec::new();
    };
    if session.elapsed_seconds == 0 {
        return Vec::new();
    }
    vec![SessionSlice {
        operational_day: day,
        start_time: session.start_time.clone(),
        end_time: session.end_time.clone(),
        elapsed_seconds: session.elapsed_seconds,
    }]
}

fn live_session_slices(live: &LiveSessionPreview) -> Vec<SessionSlice> {
    temporal::allocate_operational_day_slices(
        live.started_at_utc,
        live.ended_at_utc,
        live.elapsed_seconds,
        live.operational_day_policy,
    )
    .unwrap_or_default()
    .into_iter()
    .map(|slice| SessionSlice {
        operational_day: slice.operational_day,
        start_time: temporal::civil_from_policy(slice.started_at_utc, live.operational_day_policy)
            .map(|value| value.format("%H:%M:%S").to_string())
            .unwrap_or_default(),
        end_time: temporal::civil_from_policy(slice.ended_at_utc, live.operational_day_policy)
            .map(|value| value.format("%H:%M:%S").to_string())
            .unwrap_or_default(),
        elapsed_seconds: slice.elapsed_seconds,
    })
    .collect()
}

pub fn build_today_report(sessions: &[Session], categories: &[Category]) -> ReportSummary {
    let today = operational_day_key_now().format("%Y-%m-%d").to_string();
    build_report_for_date(sessions, categories, &today)
}

pub fn build_period_report(
    sessions: &[Session],
    categories: &[Category],
    period: ReportPeriod,
) -> ReportSummary {
    build_period_report_with_offset(sessions, categories, period, 0)
}

pub fn build_period_report_with_offset(
    sessions: &[Session],
    categories: &[Category],
    period: ReportPeriod,
    offset: usize,
) -> ReportSummary {
    if period == ReportPeriod::Today && offset == 0 {
        return build_today_report(sessions, categories);
    }

    let (start, end, label) = period_bounds_with_offset(period, offset);
    build_report_for_date_range(sessions, categories, start, end, label)
}

fn period_bounds_with_offset(
    period: ReportPeriod,
    offset: usize,
) -> (NaiveDate, NaiveDate, String) {
    let today = operational_day_key_now();
    let settings = runtime_settings();

    match period {
        ReportPeriod::Today => {
            let day = today - ChronoDuration::days(offset as i64);
            let label = day.format("%Y-%m-%d").to_string();
            (day, day, label)
        }
        ReportPeriod::Week => {
            let current_start = start_of_week(today, settings.first_day_of_week);
            let start = current_start - ChronoDuration::days((offset as i64) * 7);
            let end = if offset == 0 {
                today
            } else {
                start + ChronoDuration::days(6)
            };
            let label = format!("{}..{}", start.format("%Y-%m-%d"), end.format("%Y-%m-%d"));
            (start, end, label)
        }
        ReportPeriod::Month => {
            let current_month_start = today.with_day(1).unwrap_or(today);

            let (start, end) = if offset == 0 {
                (current_month_start, today)
            } else {
                let start = shift_month_start(current_month_start, -(offset as i32));
                let next_start = shift_month_start(start, 1);
                let end = next_start - ChronoDuration::days(1);
                (start, end)
            };

            let label = format!("{}..{}", start.format("%Y-%m-%d"), end.format("%Y-%m-%d"));
            (start, end, label)
        }
    }
}

fn shift_month_start(base_start: NaiveDate, delta_months: i32) -> NaiveDate {
    let total_months = base_start.year() * 12 + (base_start.month0() as i32) + delta_months;
    let year = total_months.div_euclid(12);
    let month0 = total_months.rem_euclid(12) as u32;
    NaiveDate::from_ymd_opt(year, month0 + 1, 1).unwrap_or(base_start)
}

pub fn build_period_karma_report_with_offset(
    sessions: &[Session],
    categories: &[Category],
    period: ReportPeriod,
    offset: usize,
) -> KarmaReportSummary {
    if period == ReportPeriod::Today && offset == 0 {
        return build_today_karma_report(sessions, categories);
    }

    let (start, end, label) = period_bounds_with_offset(period, offset);
    build_karma_report_for_date_range(sessions, categories, start, end, label)
}

fn start_of_week(day: NaiveDate, first_day: FirstDayOfWeek) -> NaiveDate {
    let day_index = day.weekday().num_days_from_monday() as i64;
    let first_index = first_day.num_days_from_monday() as i64;
    let distance = (7 + day_index - first_index) % 7;
    day - ChronoDuration::days(distance)
}

pub fn build_today_karma_report(
    sessions: &[Session],
    categories: &[Category],
) -> KarmaReportSummary {
    let today = operational_day_key_now().format("%Y-%m-%d").to_string();
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
            karma_effect: if is_drift_category_id(category.id) || is_drift_name(&category.name) {
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
        if let Some(idx) = by_id.get(&session.category_id).copied() {
            for slice in session_slices(session) {
                if slice.operational_day >= start && slice.operational_day <= end {
                    entries[idx].elapsed_seconds += slice.elapsed_seconds;
                }
            }
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
        .filter(|category| !is_drift_category_id(category.id) && !is_drift_name(&category.name))
        .map(|category| (category.id, category.name.clone()))
        .collect();

    let mut totals: HashMap<CategoryId, usize> = HashMap::new();
    for session in sessions {
        if category_names.contains_key(&session.category_id) {
            for slice in session_slices(session) {
                if slice.operational_day >= start && slice.operational_day <= end {
                    *totals.entry(session.category_id).or_insert(0) += slice.elapsed_seconds;
                }
            }
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

fn category_karma_effect(categories: &[Category], category_id: CategoryId) -> i8 {
    categories
        .iter()
        .find(|category| category.id == category_id)
        .map(|category| {
            if is_drift_category_id(category.id) || is_drift_name(&category.name) {
                0
            } else {
                category.karma_effect
            }
        })
        .unwrap_or(0)
}

pub fn sort_karma_entries_for_display(entries: &mut [KarmaReportEntry]) {
    entries.sort_by(|a, b| {
        let group = |entry: &KarmaReportEntry| -> u8 {
            if is_drift_category_id(entry.category_id) {
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
                let a_is_none = is_drift_category_id(a.category_id);
                let b_is_none = is_drift_category_id(b.category_id);
                b_is_none
                    .cmp(&a_is_none)
                    .then(b.elapsed_seconds.cmp(&a.elapsed_seconds))
                    .then(a.category_name.cmp(&b.category_name))
            }
            _ => a
                .karma_seconds
                .cmp(&b.karma_seconds)
                .reverse()
                .then(a.elapsed_seconds.cmp(&b.elapsed_seconds))
                .then(a.category_name.cmp(&b.category_name)),
        })
    });
}

pub fn build_period_karma_report_with_live_and_offset(
    sessions: &[Session],
    categories: &[Category],
    period: ReportPeriod,
    offset: usize,
    live_session: Option<&LiveSessionPreview>,
) -> KarmaReportSummary {
    let mut summary = build_period_karma_report_with_offset(sessions, categories, period, offset);

    let (start, end) = report_period_date_bounds_with_offset(period, offset);
    if let Some(live) = live_session
        && let Some(entry) = summary
            .entries
            .iter_mut()
            .find(|entry| entry.category_id == live.category_id)
    {
        let seconds: usize = live_session_slices(live)
            .into_iter()
            .filter(|slice| slice.operational_day >= start && slice.operational_day <= end)
            .map(|slice| slice.elapsed_seconds)
            .sum();
        entry.elapsed_seconds += seconds;
        entry.karma_seconds += seconds as isize * entry.karma_effect as isize;
        summary.total_seconds += seconds;
        summary.total_karma_seconds += seconds as isize * entry.karma_effect as isize;
    }

    sort_karma_entries_for_display(&mut summary.entries);
    summary
}

pub fn build_category_logs_for_period_with_offset(
    sessions: &[Session],
    categories: &[Category],
    category_id: CategoryId,
    period: ReportPeriod,
    offset: usize,
    live_session: Option<&LiveSessionPreview>,
) -> Vec<CategoryLogEntry> {
    let (start, end) = report_period_date_bounds_with_offset(period, offset);
    let karma_effect = category_karma_effect(categories, category_id);

    let mut logs: Vec<CategoryLogEntry> = sessions
        .iter()
        .filter(|session| session.category_id == category_id)
        .flat_map(|session| {
            session_slices(session)
                .into_iter()
                .filter(move |slice| slice.operational_day >= start && slice.operational_day <= end)
                .map(move |slice| CategoryLogEntry {
                    session_id: Some(session.id),
                    date: slice.operational_day.format("%Y-%m-%d").to_string(),
                    start_time: slice.start_time,
                    end_time: slice.end_time,
                    description: session.description.clone(),
                    elapsed_seconds: slice.elapsed_seconds,
                    karma_effect,
                    karma_seconds: slice.elapsed_seconds as isize * karma_effect as isize,
                })
        })
        .collect();

    if let Some(live) = live_session
        && live.category_id == category_id
    {
        logs.extend(
            live_session_slices(live)
                .into_iter()
                .filter(|slice| slice.operational_day >= start && slice.operational_day <= end)
                .map(|slice| CategoryLogEntry {
                    session_id: None,
                    date: slice.operational_day.format("%Y-%m-%d").to_string(),
                    start_time: slice.start_time,
                    end_time: slice.end_time,
                    description: live.description.clone(),
                    elapsed_seconds: slice.elapsed_seconds,
                    karma_effect,
                    karma_seconds: slice.elapsed_seconds as isize * karma_effect as isize,
                }),
        );
    }

    logs.sort_by(|a, b| {
        a.date
            .cmp(&b.date)
            .then(a.start_time.cmp(&b.start_time))
            .then(a.end_time.cmp(&b.end_time))
            .then(a.session_id.cmp(&b.session_id))
    });
    logs
}

#[cfg(test)]
mod tests {
    use chrono::{Datelike, TimeZone};

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
    fn test_restore_category_reuses_stable_identity() {
        let mut tracker = TimeTracker::new();
        let id = tracker
            .add_category("Work".to_string(), "focus".to_string(), Some(0))
            .expect("category should be added");
        let archived = tracker
            .category_by_id(id)
            .cloned()
            .expect("category should exist");
        assert!(tracker.delete_category(1));
        assert!(tracker.restore_category(archived));
        assert_eq!(tracker.category_id_by_name("Work"), Some(id));
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

        let t1 = Local
            .with_ymd_and_hms(2026, 2, 1, 9, 0, 0)
            .single()
            .expect("valid datetime");
        let t2 = Local
            .with_ymd_and_hms(2026, 2, 1, 10, 0, 0)
            .single()
            .expect("valid datetime");

        tracker.record_session_at(CategoryId::new(1), "work session", 100, t1);
        tracker.record_session_at(CategoryId::new(2), "personal session", 200, t2);

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
    fn test_record_session_creates_distinct_rows_per_session() {
        let mut tracker = TimeTracker::new();
        let t1 = Local
            .with_ymd_and_hms(2026, 2, 1, 11, 0, 0)
            .single()
            .expect("valid datetime");
        let t2 = Local
            .with_ymd_and_hms(2026, 2, 1, 11, 30, 0)
            .single()
            .expect("valid datetime");

        tracker.record_session_at(CategoryId::new(1), "focus", 120, t1);
        tracker.record_session_at(CategoryId::new(1), "review", 180, t2);

        assert_eq!(tracker.sessions.len(), 2);
        assert_eq!(tracker.sessions[0].description, "focus");
        assert_eq!(tracker.sessions[1].description, "review");
        assert_eq!(tracker.sessions[0].id + 1, tracker.sessions[1].id);
    }

    #[test]
    fn test_operational_day_boundary_uses_6am_costa_rica() {
        let config = day_boundary_config();

        let before = Utc
            .with_ymd_and_hms(2026, 2, 10, 11, 59, 0)
            .single()
            .expect("valid datetime");
        let at_cutoff = Utc
            .with_ymd_and_hms(2026, 2, 10, 12, 0, 0)
            .single()
            .expect("valid datetime");

        assert_eq!(
            operational_day_key_from_utc(before, &config),
            NaiveDate::from_ymd_opt(2026, 2, 9).expect("valid date")
        );
        assert_eq!(
            operational_day_key_from_utc(at_cutoff, &config),
            NaiveDate::from_ymd_opt(2026, 2, 10).expect("valid date")
        );
    }

    #[test]
    fn test_start_of_week_respects_selected_first_day() {
        let sunday = NaiveDate::from_ymd_opt(2026, 3, 1).expect("valid date");

        let monday_start = start_of_week(sunday, FirstDayOfWeek::Monday);
        let sunday_start = start_of_week(sunday, FirstDayOfWeek::Sunday);

        assert_eq!(
            monday_start,
            NaiveDate::from_ymd_opt(2026, 2, 23).expect("valid date")
        );
        assert_eq!(sunday_start, sunday);
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
                project: String::new(),
                description: String::new(),
                start_time: "09:00:00".to_string(),
                end_time: "10:00:00".to_string(),
                elapsed_seconds: 3600,
                started_at_utc: None,
                ended_at_utc: None,
                operational_day_policy: None,
            },
            Session {
                id: 2,
                date: "2026-02-25".to_string(),
                category_id: CategoryId::new(2),
                project: String::new(),
                description: String::new(),
                start_time: "10:00:00".to_string(),
                end_time: "10:30:00".to_string(),
                elapsed_seconds: 1800,
                started_at_utc: None,
                ended_at_utc: None,
                operational_day_policy: None,
            },
            Session {
                id: 3,
                date: "2026-02-25".to_string(),
                category_id: CategoryId::new(0),
                project: String::new(),
                description: String::new(),
                start_time: "11:00:00".to_string(),
                end_time: "12:00:00".to_string(),
                elapsed_seconds: 3600,
                started_at_utc: None,
                ended_at_utc: None,
                operational_day_policy: None,
            },
            Session {
                id: 4,
                date: "2026-02-24".to_string(),
                category_id: CategoryId::new(1),
                project: String::new(),
                description: String::new(),
                start_time: "09:00:00".to_string(),
                end_time: "10:00:00".to_string(),
                elapsed_seconds: 3600,
                started_at_utc: None,
                ended_at_utc: None,
                operational_day_policy: None,
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
                project: String::new(),
                description: String::new(),
                start_time: "08:00:00".to_string(),
                end_time: "09:00:00".to_string(),
                elapsed_seconds: 3600,
                started_at_utc: None,
                ended_at_utc: None,
                operational_day_policy: None,
            },
            Session {
                id: 2,
                date: "2026-02-25".to_string(),
                category_id: CategoryId::new(2),
                project: String::new(),
                description: String::new(),
                start_time: "10:00:00".to_string(),
                end_time: "10:30:00".to_string(),
                elapsed_seconds: 1800,
                started_at_utc: None,
                ended_at_utc: None,
                operational_day_policy: None,
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
                project: String::new(),
                description: String::new(),
                start_time: "08:00:00".to_string(),
                end_time: "08:20:00".to_string(),
                elapsed_seconds: 1200,
                started_at_utc: None,
                ended_at_utc: None,
                operational_day_policy: None,
            },
            Session {
                id: 2,
                date: "2026-02-25".to_string(),
                category_id: CategoryId::new(1),
                project: String::new(),
                description: String::new(),
                start_time: "09:00:00".to_string(),
                end_time: "09:30:00".to_string(),
                elapsed_seconds: 1800,
                started_at_utc: None,
                ended_at_utc: None,
                operational_day_policy: None,
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
    fn test_build_period_report_week_uses_configured_week_start() {
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

        let today = operational_day_key_now();
        let week_start = start_of_week(today, runtime_settings().first_day_of_week);
        let in_window = week_start.format("%Y-%m-%d").to_string();
        let out_window = (week_start - ChronoDuration::days(1))
            .format("%Y-%m-%d")
            .to_string();

        let sessions = vec![
            Session {
                id: 1,
                date: today.format("%Y-%m-%d").to_string(),
                category_id: CategoryId::new(1),
                project: String::new(),
                description: String::new(),
                start_time: "09:00:00".to_string(),
                end_time: "10:00:00".to_string(),
                elapsed_seconds: 3600,
                started_at_utc: None,
                ended_at_utc: None,
                operational_day_policy: None,
            },
            Session {
                id: 2,
                date: in_window,
                category_id: CategoryId::new(1),
                project: String::new(),
                description: String::new(),
                start_time: "09:00:00".to_string(),
                end_time: "09:30:00".to_string(),
                elapsed_seconds: 1800,
                started_at_utc: None,
                ended_at_utc: None,
                operational_day_policy: None,
            },
            Session {
                id: 3,
                date: out_window,
                category_id: CategoryId::new(1),
                project: String::new(),
                description: String::new(),
                start_time: "09:00:00".to_string(),
                end_time: "11:00:00".to_string(),
                elapsed_seconds: 7200,
                started_at_utc: None,
                ended_at_utc: None,
                operational_day_policy: None,
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

        let today = operational_day_key_now();
        let month_start = today.with_day(1).unwrap_or(today);
        let previous_month_end = month_start - ChronoDuration::days(1);

        let sessions = vec![
            Session {
                id: 1,
                date: month_start.format("%Y-%m-%d").to_string(),
                category_id: CategoryId::new(1),
                project: String::new(),
                description: String::new(),
                start_time: "08:00:00".to_string(),
                end_time: "09:00:00".to_string(),
                elapsed_seconds: 3600,
                started_at_utc: None,
                ended_at_utc: None,
                operational_day_policy: None,
            },
            Session {
                id: 2,
                date: today.format("%Y-%m-%d").to_string(),
                category_id: CategoryId::new(2),
                project: String::new(),
                description: String::new(),
                start_time: "10:00:00".to_string(),
                end_time: "10:30:00".to_string(),
                elapsed_seconds: 1800,
                started_at_utc: None,
                ended_at_utc: None,
                operational_day_policy: None,
            },
            Session {
                id: 3,
                date: previous_month_end.format("%Y-%m-%d").to_string(),
                category_id: CategoryId::new(1),
                project: String::new(),
                description: String::new(),
                start_time: "12:00:00".to_string(),
                end_time: "13:00:00".to_string(),
                elapsed_seconds: 3600,
                started_at_utc: None,
                ended_at_utc: None,
                operational_day_policy: None,
            },
        ];

        let summary =
            build_period_karma_report_with_offset(&sessions, &categories, ReportPeriod::Month, 0);
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
    fn test_report_period_month_offset_uses_previous_calendar_month() {
        let (current_start, current_end) =
            report_period_date_bounds_with_offset(ReportPeriod::Month, 0);
        let (previous_start, previous_end) =
            report_period_date_bounds_with_offset(ReportPeriod::Month, 1);

        assert_eq!(current_start.day(), 1);
        assert_eq!(current_end, operational_day_key_now());
        assert_eq!(previous_start.day(), 1);
        assert_eq!(previous_end, current_start - ChronoDuration::days(1));
    }

    #[test]
    fn test_build_period_karma_report_month_offset_aggregates_previous_month_only() {
        let categories = vec![
            Category {
                id: CategoryId::new(0),
                name: "none".to_string(),
                color: Color::White,
                description: String::new(),
                karma_effect: 0,
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

        let (current_start, _) = report_period_date_bounds_with_offset(ReportPeriod::Month, 0);
        let (previous_start, previous_end) =
            report_period_date_bounds_with_offset(ReportPeriod::Month, 1);

        let sessions = vec![
            Session {
                id: 1,
                date: previous_start.format("%Y-%m-%d").to_string(),
                category_id: CategoryId::new(1),
                project: String::new(),
                description: String::new(),
                start_time: "08:00:00".to_string(),
                end_time: "08:20:00".to_string(),
                elapsed_seconds: 1200,
                started_at_utc: None,
                ended_at_utc: None,
                operational_day_policy: None,
            },
            Session {
                id: 2,
                date: previous_end.format("%Y-%m-%d").to_string(),
                category_id: CategoryId::new(2),
                project: String::new(),
                description: String::new(),
                start_time: "21:00:00".to_string(),
                end_time: "21:10:00".to_string(),
                elapsed_seconds: 600,
                started_at_utc: None,
                ended_at_utc: None,
                operational_day_policy: None,
            },
            Session {
                id: 3,
                date: current_start.format("%Y-%m-%d").to_string(),
                category_id: CategoryId::new(1),
                project: String::new(),
                description: String::new(),
                start_time: "10:00:00".to_string(),
                end_time: "10:40:00".to_string(),
                elapsed_seconds: 2400,
                started_at_utc: None,
                ended_at_utc: None,
                operational_day_policy: None,
            },
        ];

        let summary =
            build_period_karma_report_with_offset(&sessions, &categories, ReportPeriod::Month, 1);

        assert_eq!(
            summary.date,
            format!("{}..{}", previous_start, previous_end)
        );
        assert_eq!(summary.total_seconds, 1800);
        assert_eq!(summary.total_karma_seconds, 600);
    }

    #[test]
    fn test_report_period_day_offset_moves_back_by_days() {
        let today = operational_day_key_now();
        let (start0, end0) = report_period_date_bounds_with_offset(ReportPeriod::Today, 0);
        let (start2, end2) = report_period_date_bounds_with_offset(ReportPeriod::Today, 2);

        assert_eq!(start0, today);
        assert_eq!(end0, today);
        assert_eq!(start2, today - ChronoDuration::days(2));
        assert_eq!(end2, start2);
    }

    #[test]
    fn test_report_period_week_offset_previous_is_full_week() {
        let (current_start, current_end) =
            report_period_date_bounds_with_offset(ReportPeriod::Week, 0);
        let (previous_start, previous_end) =
            report_period_date_bounds_with_offset(ReportPeriod::Week, 1);

        assert_eq!(current_end, operational_day_key_now());
        assert_eq!(previous_start, current_start - ChronoDuration::days(7));
        assert_eq!(previous_end, previous_start + ChronoDuration::days(6));
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

        let today = operational_day_key_now().format("%Y-%m-%d").to_string();
        let sessions = vec![Session {
            id: 1,
            date: today.clone(),
            category_id: CategoryId::new(1),
            project: String::new(),
            description: String::new(),
            start_time: "09:00:00".to_string(),
            end_time: "09:10:00".to_string(),
            elapsed_seconds: 600,
            started_at_utc: None,
            ended_at_utc: None,
            operational_day_policy: None,
        }];

        let summary =
            build_period_karma_report_with_offset(&sessions, &categories, ReportPeriod::Today, 0);
        assert_eq!(summary.date, today);
        assert_eq!(summary.total_seconds, 600);
        assert_eq!(summary.total_karma_seconds, 600);
    }

    #[test]
    fn test_clear_drift_sessions_for_day_clears_only_target_day() {
        let mut tracker = TimeTracker::new();
        let today = operational_day_key_now().format("%Y-%m-%d").to_string();
        let yesterday = (operational_day_key_now() - ChronoDuration::days(1))
            .format("%Y-%m-%d")
            .to_string();

        tracker.sessions = vec![
            Session {
                id: 1,
                date: today.clone(),
                category_id: CategoryId::new(0),
                project: String::new(),
                description: String::new(),
                start_time: "08:00:00".to_string(),
                end_time: "08:10:00".to_string(),
                elapsed_seconds: 600,
                started_at_utc: None,
                ended_at_utc: None,
                operational_day_policy: None,
            },
            Session {
                id: 2,
                date: yesterday,
                category_id: CategoryId::new(0),
                project: String::new(),
                description: String::new(),
                start_time: "08:00:00".to_string(),
                end_time: "08:10:00".to_string(),
                elapsed_seconds: 600,
                started_at_utc: None,
                ended_at_utc: None,
                operational_day_policy: None,
            },
            Session {
                id: 3,
                date: today,
                category_id: CategoryId::new(1),
                project: String::new(),
                description: String::new(),
                start_time: "09:00:00".to_string(),
                end_time: "09:10:00".to_string(),
                elapsed_seconds: 600,
                started_at_utc: None,
                ended_at_utc: None,
                operational_day_policy: None,
            },
        ];

        tracker.clear_drift_sessions_for_day(operational_day_key_now());

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

    #[test]
    fn test_build_category_logs_for_period_keeps_per_session_rows() {
        let today = operational_day_key_now().format("%Y-%m-%d").to_string();
        let categories = vec![
            Category {
                id: CategoryId::new(0),
                name: "none".to_string(),
                color: Color::White,
                description: String::new(),
                karma_effect: 0,
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
                date: today.clone(),
                category_id: CategoryId::new(1),
                project: String::new(),
                description: "focus".to_string(),
                start_time: "09:00:00".to_string(),
                end_time: "09:10:00".to_string(),
                elapsed_seconds: 600,
                started_at_utc: None,
                ended_at_utc: None,
                operational_day_policy: None,
            },
            Session {
                id: 2,
                date: today,
                category_id: CategoryId::new(1),
                project: String::new(),
                description: "review".to_string(),
                start_time: "10:00:00".to_string(),
                end_time: "10:05:00".to_string(),
                elapsed_seconds: 300,
                started_at_utc: None,
                ended_at_utc: None,
                operational_day_policy: None,
            },
        ];

        let logs = build_category_logs_for_period_with_offset(
            &sessions,
            &categories,
            CategoryId::new(1),
            ReportPeriod::Today,
            0,
            None,
        );

        assert_eq!(logs.len(), 2);
        assert_eq!(logs[0].description, "focus");
        assert_eq!(logs[1].description, "review");
        assert_eq!(logs[0].session_id, Some(1));
        assert_eq!(logs[1].session_id, Some(2));
    }

    #[test]
    fn test_delete_session_by_id_removes_exact_session() {
        let mut tracker = TimeTracker::new();
        tracker.sessions = vec![
            Session {
                id: 1,
                date: "2026-03-01".to_string(),
                category_id: CategoryId::new(1),
                project: String::new(),
                description: "focus".to_string(),
                start_time: "09:00:00".to_string(),
                end_time: "09:30:00".to_string(),
                elapsed_seconds: 1800,
                started_at_utc: None,
                ended_at_utc: None,
                operational_day_policy: None,
            },
            Session {
                id: 2,
                date: "2026-03-01".to_string(),
                category_id: CategoryId::new(1),
                project: String::new(),
                description: "review".to_string(),
                start_time: "10:00:00".to_string(),
                end_time: "10:20:00".to_string(),
                elapsed_seconds: 1200,
                started_at_utc: None,
                ended_at_utc: None,
                operational_day_policy: None,
            },
        ];

        assert!(tracker.delete_session_by_id(1));
        assert_eq!(tracker.sessions.len(), 1);
        assert_eq!(tracker.sessions[0].id, 2);
        assert!(!tracker.delete_session_by_id(999));
    }

    #[test]
    fn test_set_session_description_by_id_updates_target_only() {
        let mut tracker = TimeTracker::new();
        tracker.sessions = vec![
            Session {
                id: 1,
                date: "2026-03-01".to_string(),
                category_id: CategoryId::new(1),
                project: String::new(),
                description: "focus".to_string(),
                start_time: "09:00:00".to_string(),
                end_time: "09:30:00".to_string(),
                elapsed_seconds: 1800,
                started_at_utc: None,
                ended_at_utc: None,
                operational_day_policy: None,
            },
            Session {
                id: 2,
                date: "2026-03-01".to_string(),
                category_id: CategoryId::new(1),
                project: String::new(),
                description: "review".to_string(),
                start_time: "10:00:00".to_string(),
                end_time: "10:20:00".to_string(),
                elapsed_seconds: 1200,
                started_at_utc: None,
                ended_at_utc: None,
                operational_day_policy: None,
            },
        ];

        assert!(tracker.set_session_description_by_id(2, "retro".to_string()));
        assert_eq!(tracker.sessions[0].description, "focus");
        assert_eq!(tracker.sessions[1].description, "retro");
        assert!(!tracker.set_session_description_by_id(999, "none".to_string()));
    }

    #[test]
    fn report_allocates_one_canonical_session_across_operational_days() {
        let category = Category {
            id: CategoryId::new(1),
            name: "Work".to_string(),
            color: COLORS[0],
            description: String::new(),
            karma_effect: 1,
        };
        let session = Session {
            id: 1,
            date: "2026-08-02".to_string(),
            category_id: category.id,
            project: String::new(),
            description: "boundary work".to_string(),
            start_time: "05:30:00".to_string(),
            end_time: "06:30:00".to_string(),
            elapsed_seconds: 3600,
            started_at_utc: Some(
                Utc.with_ymd_and_hms(2026, 8, 2, 11, 30, 0)
                    .single()
                    .unwrap(),
            ),
            ended_at_utc: Some(
                Utc.with_ymd_and_hms(2026, 8, 2, 12, 30, 0)
                    .single()
                    .unwrap(),
            ),
            operational_day_policy: Some(OperationalDayPolicy {
                utc_offset_seconds: -6 * 60 * 60,
                start_minutes: 6 * 60,
            }),
        };

        let first = build_report_for_date(
            std::slice::from_ref(&session),
            std::slice::from_ref(&category),
            "2026-08-01",
        );
        let second = build_report_for_date(&[session], &[category], "2026-08-02");

        assert_eq!(first.total_seconds, 1800);
        assert_eq!(second.total_seconds, 1800);
    }

    #[test]
    fn exact_boundary_end_creates_no_empty_next_day_slice() {
        let session = Session {
            id: 1,
            date: "2026-08-01".to_string(),
            category_id: CategoryId::new(1),
            project: String::new(),
            description: String::new(),
            start_time: "05:30:00".to_string(),
            end_time: "06:00:00".to_string(),
            elapsed_seconds: 1800,
            started_at_utc: Some(
                Utc.with_ymd_and_hms(2026, 8, 2, 11, 30, 0)
                    .single()
                    .unwrap(),
            ),
            ended_at_utc: Some(Utc.with_ymd_and_hms(2026, 8, 2, 12, 0, 0).single().unwrap()),
            operational_day_policy: Some(OperationalDayPolicy {
                utc_offset_seconds: -6 * 60 * 60,
                start_minutes: 6 * 60,
            }),
        };

        let slices = session_slices(&session);
        assert_eq!(slices.len(), 1);
        assert_eq!(slices[0].operational_day.to_string(), "2026-08-01");
        assert_eq!(slices[0].elapsed_seconds, 1800);
    }
}
