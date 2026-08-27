use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fmt, fs,
    path::Path,
};

use chrono::FixedOffset;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde::{Deserialize, Serialize};

use crate::domain::{DayBoundaryConfig, DayBoundaryMode, FirstDayOfWeek, RuntimeSettings};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum ActionCategory {
    Global,
    Navigation,
    CategoryModal,
    ReportModal,
    HelpModal,
}

impl ActionCategory {
    pub(crate) const fn all() -> [Self; 5] {
        [
            Self::Global,
            Self::Navigation,
            Self::CategoryModal,
            Self::ReportModal,
            Self::HelpModal,
        ]
    }

    pub(crate) const fn title(self) -> &'static str {
        match self {
            Self::Global => "Core Commands",
            Self::Navigation => "Flow Controls",
            Self::CategoryModal => "Layer Pop-up",
            Self::ReportModal => "Balance Pop-up",
            Self::HelpModal => "Command Atlas",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Action {
    Quit,
    ToggleCommandPalette,
    OpenCategoryModal,
    OpenReportModal,
    Detach,
    SwitchToNone,
    ClearAllSand,
    ClearNoneSand,
    ToggleKeybindingsHelp,

    Up,
    Down,
    Left,
    Right,
    ShiftUp,
    ShiftDown,
    ShiftLeft,
    ShiftRight,
    Confirm,
    Cancel,

    DeleteCategory,
    EditCategoryDescription,
    IncreaseBalance,
    DecreaseBalance,
    Backspace,

    ReportToday,
    ReportWeek,
    ReportMonth,
    ReportRange,
    LogActivity,

    HelpTop,
    HelpBottom,
}

impl Action {
    const ALL: [Action; 31] = [
        Action::Quit,
        Action::ToggleCommandPalette,
        Action::OpenCategoryModal,
        Action::OpenReportModal,
        Action::Detach,
        Action::SwitchToNone,
        Action::ClearAllSand,
        Action::ClearNoneSand,
        Action::ToggleKeybindingsHelp,
        Action::Up,
        Action::Down,
        Action::Left,
        Action::Right,
        Action::ShiftUp,
        Action::ShiftDown,
        Action::ShiftLeft,
        Action::ShiftRight,
        Action::Confirm,
        Action::Cancel,
        Action::DeleteCategory,
        Action::EditCategoryDescription,
        Action::IncreaseBalance,
        Action::DecreaseBalance,
        Action::Backspace,
        Action::ReportToday,
        Action::ReportWeek,
        Action::ReportMonth,
        Action::ReportRange,
        Action::LogActivity,
        Action::HelpTop,
        Action::HelpBottom,
    ];

    pub(crate) fn all() -> &'static [Action] {
        &Self::ALL
    }

    pub(crate) fn config_name(self) -> &'static str {
        match self {
            Action::Quit => "quit",
            Action::ToggleCommandPalette => "toggle_command_palette",
            Action::OpenCategoryModal => "open_layer_popup",
            Action::OpenReportModal => "open_balance_popup",
            Action::Detach => "detach",
            Action::SwitchToNone => "switch_to_drift",
            Action::ClearAllSand => "clear_all_sand",
            Action::ClearNoneSand => "clear_drift_sand",
            Action::ToggleKeybindingsHelp => "toggle_command_atlas",

            Action::Up => "up",
            Action::Down => "down",
            Action::Left => "left",
            Action::Right => "right",
            Action::ShiftUp => "shift_up",
            Action::ShiftDown => "shift_down",
            Action::ShiftLeft => "shift_left",
            Action::ShiftRight => "shift_right",
            Action::Confirm => "confirm",
            Action::Cancel => "cancel",

            Action::DeleteCategory => "delete_layer",
            Action::EditCategoryDescription => "edit_layer_metadata",
            Action::IncreaseBalance => "boost_layer_balance",
            Action::DecreaseBalance => "drain_layer_balance",
            Action::Backspace => "backspace",

            Action::ReportToday => "balance_today",
            Action::ReportWeek => "balance_week",
            Action::ReportMonth => "balance_month",
            Action::ReportRange => "balance_range",
            Action::LogActivity => "balance_log_activity",

            Action::HelpTop => "atlas_top",
            Action::HelpBottom => "atlas_bottom",
        }
    }

    pub(crate) fn from_config_name(name: &str) -> Option<Self> {
        match name.trim().to_ascii_lowercase().as_str() {
            "quit" => Some(Self::Quit),
            "toggle_command_palette" | "toggle_palette" => Some(Self::ToggleCommandPalette),

            "open_layer_popup" | "open_category_modal" => Some(Self::OpenCategoryModal),
            "open_balance_popup" | "open_report_modal" => Some(Self::OpenReportModal),
            "detach" | "detach_from_main" => Some(Self::Detach),
            "switch_to_drift" | "switch_to_none" => Some(Self::SwitchToNone),

            "clear_all_sand" => Some(Self::ClearAllSand),
            "clear_drift_sand" | "clear_none_sand" => Some(Self::ClearNoneSand),
            "toggle_command_atlas" | "toggle_keybindings_help" => Some(Self::ToggleKeybindingsHelp),

            "up" => Some(Self::Up),
            "down" => Some(Self::Down),
            "left" => Some(Self::Left),
            "right" => Some(Self::Right),
            "shift_up" => Some(Self::ShiftUp),
            "shift_down" => Some(Self::ShiftDown),
            "shift_left" => Some(Self::ShiftLeft),
            "shift_right" => Some(Self::ShiftRight),
            "confirm" => Some(Self::Confirm),
            "cancel" => Some(Self::Cancel),

            "delete_layer" | "delete_category" => Some(Self::DeleteCategory),
            "edit_layer_metadata" | "edit_category_description" => {
                Some(Self::EditCategoryDescription)
            }
            "boost_layer_balance" | "increase_balance" => Some(Self::IncreaseBalance),
            "drain_layer_balance" | "decrease_balance" => Some(Self::DecreaseBalance),
            "backspace" => Some(Self::Backspace),

            "balance_today" | "report_today" => Some(Self::ReportToday),
            "balance_week" | "report_week" => Some(Self::ReportWeek),
            "balance_month" | "report_month" => Some(Self::ReportMonth),
            "balance_range" | "report_range" => Some(Self::ReportRange),
            "balance_log_activity" | "report_log_activity" => Some(Self::LogActivity),

            "atlas_top" | "help_top" => Some(Self::HelpTop),
            "atlas_bottom" | "help_bottom" => Some(Self::HelpBottom),

            _ => None,
        }
    }

    pub(crate) fn description(self) -> &'static str {
        match self {
            Action::Quit => "Exit Strata",
            Action::ToggleCommandPalette => "Open/close command palette",
            Action::OpenCategoryModal => "Open layer pop-up from main view",
            Action::OpenReportModal => "Open balance pop-up from main view",
            Action::Detach => "Detach from main view (in balance pop-up: day range)",
            Action::SwitchToNone => "Switch active layer to idle",
            Action::ClearAllSand => "Clear all sand and reset idle timer",
            Action::ClearNoneSand => "Clear only idle sand",
            Action::ToggleKeybindingsHelp => "Open/close command atlas",

            Action::Up => "Move up / previous item",
            Action::Down => "Move down / next item",
            Action::Left => "Context ← action (layer tags or older balance interval)",
            Action::Right => "Context → action (layer tags or newer balance interval)",
            Action::ShiftUp => "Shift+↑ action (layer reorder)",
            Action::ShiftDown => "Shift+↓ action (layer reorder)",
            Action::ShiftLeft => "Shift+← action (color or period)",
            Action::ShiftRight => "Shift+→ action (color or period)",
            Action::Confirm => "Confirm / open",
            Action::Cancel => "Cancel / close",

            Action::DeleteCategory => "Archive selected layer / delete selected report session",
            Action::EditCategoryDescription => "Toggle durable layer-metadata editing",
            Action::IncreaseBalance => "Set selected layer balance to +1",
            Action::DecreaseBalance => "Set selected layer balance to -1",
            Action::Backspace => "Delete one typed character in layer pop-up",

            Action::ReportToday => "Set balance pop-up range to day",
            Action::ReportWeek => "Set balance pop-up range to week",
            Action::ReportMonth => "Set balance pop-up range to month",
            Action::ReportRange => "Edit an explicit From/To balance range",
            Action::LogActivity => "Log or correct an arbitrary past activity interval",

            Action::HelpTop => "Jump command atlas to top",
            Action::HelpBottom => "Jump command atlas to bottom",
        }
    }

    pub(crate) fn category(self) -> ActionCategory {
        match self {
            Action::Quit
            | Action::ToggleCommandPalette
            | Action::OpenCategoryModal
            | Action::OpenReportModal
            | Action::Detach
            | Action::SwitchToNone
            | Action::ClearAllSand
            | Action::ClearNoneSand
            | Action::ToggleKeybindingsHelp => ActionCategory::Global,

            Action::Up
            | Action::Down
            | Action::Left
            | Action::Right
            | Action::ShiftUp
            | Action::ShiftDown
            | Action::ShiftLeft
            | Action::ShiftRight
            | Action::Confirm
            | Action::Cancel => ActionCategory::Navigation,

            Action::DeleteCategory
            | Action::EditCategoryDescription
            | Action::IncreaseBalance
            | Action::DecreaseBalance
            | Action::Backspace => ActionCategory::CategoryModal,

            Action::ReportToday
            | Action::ReportWeek
            | Action::ReportMonth
            | Action::ReportRange
            | Action::LogActivity => ActionCategory::ReportModal,

            Action::HelpTop | Action::HelpBottom => ActionCategory::HelpModal,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ActionBindingState {
    Bound,
    Unbound,
    Disabled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum InputContext {
    Main,
    Report,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ResolvedActionSource {
    Mandatory,
    Direct,
    Contextual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ResolvedAction {
    pub action: Action,
    pub source: ResolvedActionSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AliasCondition {
    Always,
    TargetUnbound,
}

impl AliasCondition {
    const fn label(self) -> &'static str {
        match self {
            Self::Always => "always",
            Self::TargetUnbound => "when target unbound",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ContextualAlias {
    name: String,
    context: InputContext,
    source: Action,
    target: Action,
    condition: AliasCondition,
}

impl ContextualAlias {
    pub(crate) fn display_label(&self) -> String {
        format!(
            "{} → {} ({})",
            self.name,
            self.target.config_name(),
            self.condition.label()
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum KeyCodeSpec {
    Char(char),
    Enter,
    Esc,
    Up,
    Down,
    Left,
    Right,
    Backspace,
    Delete,
    Tab,
    Home,
    End,
    PageUp,
    PageDown,
    F(u8),
}

impl KeyCodeSpec {
    fn label(self) -> String {
        match self {
            KeyCodeSpec::Char(' ') => "Space".to_string(),
            KeyCodeSpec::Char(c) => c.to_string(),
            KeyCodeSpec::Enter => "Enter".to_string(),
            KeyCodeSpec::Esc => "Esc".to_string(),
            KeyCodeSpec::Up => "↑".to_string(),
            KeyCodeSpec::Down => "↓".to_string(),
            KeyCodeSpec::Left => "←".to_string(),
            KeyCodeSpec::Right => "→".to_string(),
            KeyCodeSpec::Backspace => "Backspace".to_string(),
            KeyCodeSpec::Delete => "Delete".to_string(),
            KeyCodeSpec::Tab => "Tab".to_string(),
            KeyCodeSpec::Home => "Home".to_string(),
            KeyCodeSpec::End => "End".to_string(),
            KeyCodeSpec::PageUp => "PageUp".to_string(),
            KeyCodeSpec::PageDown => "PageDown".to_string(),
            KeyCodeSpec::F(n) => format!("F{n}"),
        }
    }

    fn config_token(self) -> String {
        match self {
            KeyCodeSpec::Char(' ') => "space".to_string(),
            KeyCodeSpec::Char(c) => c.to_string(),
            KeyCodeSpec::Enter => "enter".to_string(),
            KeyCodeSpec::Esc => "esc".to_string(),
            KeyCodeSpec::Up => "up".to_string(),
            KeyCodeSpec::Down => "down".to_string(),
            KeyCodeSpec::Left => "left".to_string(),
            KeyCodeSpec::Right => "right".to_string(),
            KeyCodeSpec::Backspace => "backspace".to_string(),
            KeyCodeSpec::Delete => "delete".to_string(),
            KeyCodeSpec::Tab => "tab".to_string(),
            KeyCodeSpec::Home => "home".to_string(),
            KeyCodeSpec::End => "end".to_string(),
            KeyCodeSpec::PageUp => "pageup".to_string(),
            KeyCodeSpec::PageDown => "pagedown".to_string(),
            KeyCodeSpec::F(n) => format!("f{n}"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct KeyBinding {
    ctrl: bool,
    alt: bool,
    shift: bool,
    code: KeyCodeSpec,
}

impl fmt::Display for KeyBinding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut parts: Vec<String> = Vec::new();
        if self.ctrl {
            parts.push("Ctrl".to_string());
        }
        if self.alt {
            parts.push("Alt".to_string());
        }
        if self.shift {
            parts.push("Shift".to_string());
        }
        parts.push(self.code.label());

        write!(f, "{}", parts.join("-"))
    }
}

impl KeyBinding {
    pub(crate) fn parse(input: &str) -> Result<Self, String> {
        let mut rest = input.trim();
        if rest.is_empty() {
            return Err("key string is empty".to_string());
        }

        let mut ctrl = false;
        let mut alt = false;
        let mut shift = false;

        loop {
            if let Some(stripped) = strip_prefix_ascii_case(rest, "ctrl-") {
                ctrl = true;
                rest = stripped;
                continue;
            }
            if let Some(stripped) = strip_prefix_ascii_case(rest, "control-") {
                ctrl = true;
                rest = stripped;
                continue;
            }
            if let Some(stripped) = strip_prefix_ascii_case(rest, "alt-") {
                alt = true;
                rest = stripped;
                continue;
            }
            if let Some(stripped) = strip_prefix_ascii_case(rest, "shift-") {
                shift = true;
                rest = stripped;
                continue;
            }
            break;
        }

        if rest.is_empty() {
            return Err("missing key code after modifiers".to_string());
        }

        let (code, implied_shift) = parse_key_code(rest)?;
        shift |= implied_shift;

        Ok(Self {
            ctrl,
            alt,
            shift,
            code,
        })
    }

    pub(crate) fn from_key_event(event: KeyEvent) -> Option<Self> {
        let ctrl = event.modifiers.contains(KeyModifiers::CONTROL);
        let alt = event.modifiers.contains(KeyModifiers::ALT);
        let mut shift = event.modifiers.contains(KeyModifiers::SHIFT);

        let code = match event.code {
            KeyCode::Char(c) => {
                if c.is_ascii_uppercase() {
                    shift = true;
                    KeyCodeSpec::Char(c.to_ascii_lowercase())
                } else if c.is_ascii_alphabetic() {
                    KeyCodeSpec::Char(c.to_ascii_lowercase())
                } else {
                    shift = false;
                    KeyCodeSpec::Char(c)
                }
            }
            KeyCode::Enter => KeyCodeSpec::Enter,
            KeyCode::Esc => KeyCodeSpec::Esc,
            KeyCode::Up => KeyCodeSpec::Up,
            KeyCode::Down => KeyCodeSpec::Down,
            KeyCode::Left => KeyCodeSpec::Left,
            KeyCode::Right => KeyCodeSpec::Right,
            KeyCode::Backspace => KeyCodeSpec::Backspace,
            KeyCode::Delete => KeyCodeSpec::Delete,
            KeyCode::Tab => KeyCodeSpec::Tab,
            KeyCode::BackTab => {
                shift = true;
                KeyCodeSpec::Tab
            }
            KeyCode::Home => KeyCodeSpec::Home,
            KeyCode::End => KeyCodeSpec::End,
            KeyCode::PageUp => KeyCodeSpec::PageUp,
            KeyCode::PageDown => KeyCodeSpec::PageDown,
            KeyCode::F(n) => KeyCodeSpec::F(n),
            _ => return None,
        };

        Some(Self {
            ctrl,
            alt,
            shift,
            code,
        })
    }

    fn mandatory_quit() -> Self {
        Self {
            ctrl: true,
            alt: false,
            shift: false,
            code: KeyCodeSpec::Char('c'),
        }
    }

    fn is_mandatory_quit(self) -> bool {
        self == Self::mandatory_quit()
    }

    fn to_config_string(self) -> String {
        let mut parts: Vec<String> = Vec::new();
        if self.ctrl {
            parts.push("ctrl".to_string());
        }
        if self.alt {
            parts.push("alt".to_string());
        }
        if self.shift {
            parts.push("shift".to_string());
        }
        parts.push(self.code.config_token());

        parts.join("-")
    }
}

fn strip_prefix_ascii_case<'a>(value: &'a str, prefix: &str) -> Option<&'a str> {
    let head = value.get(..prefix.len())?;
    if head.eq_ignore_ascii_case(prefix) {
        value.get(prefix.len()..)
    } else {
        None
    }
}

fn parse_key_code(raw: &str) -> Result<(KeyCodeSpec, bool), String> {
    let token = raw.trim();
    if token.is_empty() {
        return Err("empty key token".to_string());
    }

    let normalized = token.to_ascii_lowercase();

    let parsed = match normalized.as_str() {
        "enter" | "return" => (KeyCodeSpec::Enter, false),
        "esc" | "escape" => (KeyCodeSpec::Esc, false),
        "up" => (KeyCodeSpec::Up, false),
        "down" => (KeyCodeSpec::Down, false),
        "left" => (KeyCodeSpec::Left, false),
        "right" => (KeyCodeSpec::Right, false),
        "backspace" => (KeyCodeSpec::Backspace, false),
        "delete" | "del" => (KeyCodeSpec::Delete, false),
        "tab" => (KeyCodeSpec::Tab, false),
        "backtab" => (KeyCodeSpec::Tab, true),
        "home" => (KeyCodeSpec::Home, false),
        "end" => (KeyCodeSpec::End, false),
        "pageup" | "page_up" => (KeyCodeSpec::PageUp, false),
        "pagedown" | "page_down" => (KeyCodeSpec::PageDown, false),
        "space" => (KeyCodeSpec::Char(' '), false),
        _ => {
            if normalized.starts_with('f')
                && normalized.len() > 1
                && let Ok(n) = normalized[1..].parse::<u8>()
                && (1..=24).contains(&n)
            {
                return Ok((KeyCodeSpec::F(n), false));
            }

            let mut chars = token.chars();
            let Some(first) = chars.next() else {
                return Err("empty key token".to_string());
            };

            if chars.next().is_some() {
                return Err(format!("unsupported key token '{token}'"));
            }

            if first.is_ascii_uppercase() {
                (KeyCodeSpec::Char(first.to_ascii_lowercase()), true)
            } else {
                (KeyCodeSpec::Char(first), false)
            }
        }
    };

    Ok(parsed)
}

#[derive(Debug, Clone)]
pub(crate) struct Keymap {
    bindings: HashMap<KeyBinding, Action>,
    disabled_actions: HashSet<Action>,
    contextual_aliases: Vec<ContextualAlias>,
}

impl Keymap {
    fn empty() -> Self {
        Self {
            bindings: HashMap::new(),
            disabled_actions: HashSet::new(),
            contextual_aliases: Vec::new(),
        }
    }

    fn bind(&mut self, key: KeyBinding, action: Action) {
        self.bindings.insert(key, action);
    }

    fn unbind_key(&mut self, key: &KeyBinding) {
        self.bindings.remove(key);
    }

    fn unbind_action(&mut self, action: Action) {
        self.bindings
            .retain(|_, current_action| *current_action != action);
    }

    fn disable_action(&mut self, action: Action) {
        self.unbind_action(action);
        self.disabled_actions.insert(action);
    }

    pub(crate) fn action_state(&self, action: Action) -> ActionBindingState {
        if self.disabled_actions.contains(&action) {
            ActionBindingState::Disabled
        } else if self.bindings.values().any(|mapped| *mapped == action) {
            ActionBindingState::Bound
        } else {
            ActionBindingState::Unbound
        }
    }

    #[cfg(test)]
    pub(crate) fn action_for_key_event(&self, event: KeyEvent) -> Option<Action> {
        self.resolve_key_event(InputContext::Other, event)
            .map(|resolved| resolved.action)
    }

    pub(crate) fn mandatory_action_for_key_event(&self, event: KeyEvent) -> Option<Action> {
        let key = KeyBinding::from_key_event(event)?;
        key.is_mandatory_quit().then_some(Action::Quit)
    }

    pub(crate) fn resolve_key_event(
        &self,
        context: InputContext,
        event: KeyEvent,
    ) -> Option<ResolvedAction> {
        if let Some(action) = self.mandatory_action_for_key_event(event) {
            return Some(ResolvedAction {
                action,
                source: ResolvedActionSource::Mandatory,
            });
        }

        let key = KeyBinding::from_key_event(event)?;
        let source_action = self.bindings.get(&key).copied()?;
        if self.action_state(source_action) == ActionBindingState::Disabled {
            return None;
        }

        if let Some(alias) = self
            .contextual_aliases
            .iter()
            .find(|alias| alias.context == context && alias.source == source_action)
        {
            let target_state = self.action_state(alias.target);
            let applies = target_state != ActionBindingState::Disabled
                && match alias.condition {
                    AliasCondition::Always => true,
                    AliasCondition::TargetUnbound => target_state == ActionBindingState::Unbound,
                };
            if applies {
                return Some(ResolvedAction {
                    action: alias.target,
                    source: ResolvedActionSource::Contextual,
                });
            }
        }

        Some(ResolvedAction {
            action: source_action,
            source: ResolvedActionSource::Direct,
        })
    }

    pub(crate) fn keys_for_action(&self, action: Action) -> Vec<KeyBinding> {
        let mut keys: Vec<KeyBinding> = self
            .bindings
            .iter()
            .filter_map(|(key, mapped_action)| (*mapped_action == action).then_some(*key))
            .collect();
        keys.sort_by_key(|key| key.to_string());
        keys
    }

    pub(crate) fn mandatory_keys_for_action(&self, action: Action) -> Vec<KeyBinding> {
        if action == Action::Quit {
            vec![KeyBinding::mandatory_quit()]
        } else {
            Vec::new()
        }
    }

    pub(crate) fn aliases_for_action(&self, action: Action) -> Vec<ContextualAlias> {
        let mut aliases = self
            .contextual_aliases
            .iter()
            .filter(|alias| alias.target == action)
            .cloned()
            .collect::<Vec<_>>();
        aliases.sort_by(|left, right| left.name.cmp(&right.name));
        aliases
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct KeymapConfig {
    #[serde(default = "default_true")]
    keymap_inherit: bool,
    #[serde(default)]
    keymap: BTreeMap<String, Option<String>>,
    #[serde(default)]
    unbind_actions: Vec<String>,
    #[serde(default = "default_true")]
    contextual_aliases_inherit: bool,
    #[serde(default)]
    contextual_aliases: BTreeMap<String, Option<String>>,
    #[serde(default)]
    day_start_mode: Option<String>,
    #[serde(default)]
    day_start_hour: Option<u32>,
    #[serde(default)]
    day_start_minute: Option<u32>,
    #[serde(default)]
    utc_offset_seconds: Option<i32>,
    #[serde(default)]
    first_day_of_week: Option<String>,
    #[serde(default)]
    time_log_path: Option<String>,
}

impl Default for KeymapConfig {
    fn default() -> Self {
        Self {
            keymap_inherit: true,
            keymap: BTreeMap::new(),
            unbind_actions: Vec::new(),
            contextual_aliases_inherit: true,
            contextual_aliases: BTreeMap::new(),
            day_start_mode: None,
            day_start_hour: None,
            day_start_minute: None,
            utc_offset_seconds: None,
            first_day_of_week: None,
            time_log_path: None,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct LoadedKeybindings {
    pub keymap: Keymap,
    pub runtime_settings: RuntimeSettings,
}

fn default_true() -> bool {
    true
}

const DEFAULT_BINDINGS: [(&str, Action); 33] = [
    ("q", Action::Quit),
    ("ctrl-p", Action::ToggleCommandPalette),
    ("enter", Action::Confirm),
    ("esc", Action::Cancel),
    ("b", Action::OpenReportModal),
    ("d", Action::Detach),
    ("c", Action::ClearAllSand),
    ("shift-c", Action::ClearNoneSand),
    ("f1", Action::ToggleKeybindingsHelp),
    ("?", Action::ToggleKeybindingsHelp),
    ("up", Action::Up),
    ("down", Action::Down),
    ("left", Action::Left),
    ("right", Action::Right),
    ("shift-up", Action::ShiftUp),
    ("shift-down", Action::ShiftDown),
    ("shift-left", Action::ShiftLeft),
    ("shift-right", Action::ShiftRight),
    ("x", Action::DeleteCategory),
    ("shift-e", Action::EditCategoryDescription),
    ("+", Action::IncreaseBalance),
    ("=", Action::IncreaseBalance),
    ("-", Action::DecreaseBalance),
    ("_", Action::DecreaseBalance),
    ("backspace", Action::Backspace),
    ("t", Action::ReportToday),
    ("w", Action::ReportWeek),
    ("m", Action::ReportMonth),
    ("r", Action::ReportRange),
    ("l", Action::LogActivity),
    ("home", Action::HelpTop),
    ("g", Action::HelpTop),
    ("end", Action::HelpBottom),
];

#[derive(Debug, Clone, Copy)]
struct ContextualAliasDefinition {
    name: &'static str,
    context: InputContext,
    source: Action,
    target: Action,
    condition: AliasCondition,
}

const DEFAULT_CONTEXTUAL_ALIASES: [ContextualAliasDefinition; 4] = [
    ContextualAliasDefinition {
        name: "main.confirm",
        context: InputContext::Main,
        source: Action::Confirm,
        target: Action::OpenCategoryModal,
        condition: AliasCondition::TargetUnbound,
    },
    ContextualAliasDefinition {
        name: "main.cancel",
        context: InputContext::Main,
        source: Action::Cancel,
        target: Action::SwitchToNone,
        condition: AliasCondition::TargetUnbound,
    },
    ContextualAliasDefinition {
        name: "main.balance_today",
        context: InputContext::Main,
        source: Action::ReportToday,
        target: Action::Detach,
        condition: AliasCondition::TargetUnbound,
    },
    ContextualAliasDefinition {
        name: "report.detach",
        context: InputContext::Report,
        source: Action::Detach,
        target: Action::ReportToday,
        condition: AliasCondition::Always,
    },
];

fn contextual_alias_definition(name: &str) -> Option<ContextualAliasDefinition> {
    DEFAULT_CONTEXTUAL_ALIASES
        .iter()
        .copied()
        .find(|definition| definition.name == name)
}

fn default_contextual_aliases() -> Vec<ContextualAlias> {
    DEFAULT_CONTEXTUAL_ALIASES
        .iter()
        .map(|definition| ContextualAlias {
            name: definition.name.to_string(),
            context: definition.context,
            source: definition.source,
            target: definition.target,
            condition: definition.condition,
        })
        .collect()
}

fn parse_contextual_aliases(
    config: &KeymapConfig,
    path: &Path,
) -> Result<Vec<ContextualAlias>, String> {
    let mut aliases = if config.contextual_aliases_inherit {
        default_contextual_aliases()
    } else {
        Vec::new()
    };

    for (name, configured_target) in &config.contextual_aliases {
        let definition = contextual_alias_definition(name).ok_or_else(|| {
            let available = DEFAULT_CONTEXTUAL_ALIASES
                .iter()
                .map(|definition| definition.name)
                .collect::<Vec<_>>()
                .join(", ");
            format!(
                "Unknown contextual alias '{}' in {}. Available aliases: {available}",
                name,
                path.display()
            )
        })?;
        aliases.retain(|alias| alias.name != *name);
        let Some(target_name) = configured_target else {
            continue;
        };
        let target = Action::from_config_name(target_name).ok_or_else(|| {
            format!(
                "Unknown contextual target '{}' for '{}' in {}",
                target_name,
                name,
                path.display()
            )
        })?;
        aliases.push(ContextualAlias {
            name: name.clone(),
            context: definition.context,
            source: definition.source,
            target,
            condition: definition.condition,
        });
    }

    Ok(aliases)
}

pub(crate) fn default_keymap() -> Keymap {
    let mut keymap = Keymap::empty();
    for (raw_key, action) in DEFAULT_BINDINGS {
        if let Ok(key) = KeyBinding::parse(raw_key) {
            keymap.bind(key, action);
        }
    }
    keymap.contextual_aliases = default_contextual_aliases();
    keymap
}

pub(crate) fn default_runtime_settings() -> RuntimeSettings {
    RuntimeSettings::default()
}

pub(crate) fn default_loaded_keybindings() -> LoadedKeybindings {
    LoadedKeybindings {
        keymap: default_keymap(),
        runtime_settings: default_runtime_settings(),
    }
}

fn parse_runtime_settings(config: &KeymapConfig, path: &Path) -> Result<RuntimeSettings, String> {
    let mut settings = default_runtime_settings();

    if let Some(mode) = config.day_start_mode.as_deref() {
        settings.day_boundary.mode = match mode.trim().to_ascii_lowercase().as_str() {
            "fixed" | "fixed_hour" => DayBoundaryMode::FixedHour,
            _ => {
                return Err(format!(
                    "Invalid day_start_mode '{}' in {}. Expected 'fixed'",
                    mode,
                    path.display()
                ));
            }
        };
    }

    if let Some(hour) = config.day_start_hour {
        if hour > 23 {
            return Err(format!(
                "Invalid day_start_hour '{}' in {}. Expected 0..23",
                hour,
                path.display()
            ));
        }
        settings.day_boundary.fixed_hour = hour;
    }

    if let Some(minute) = config.day_start_minute {
        if minute > 59 {
            return Err(format!(
                "Invalid day_start_minute '{}' in {}. Expected 0..59",
                minute,
                path.display()
            ));
        }
        settings.day_boundary.fixed_minute = minute;
    }

    if let Some(offset) = config.utc_offset_seconds {
        if FixedOffset::east_opt(offset).is_none() {
            return Err(format!(
                "Invalid utc_offset_seconds '{}' in {}. Expected an offset between -86399 and 86399",
                offset,
                path.display()
            ));
        }
        settings.day_boundary.utc_offset_seconds = offset;
    }

    if let Some(first_day) = config.first_day_of_week.as_deref() {
        let parsed = FirstDayOfWeek::from_config_name(first_day).ok_or_else(|| {
            format!(
                "Invalid first_day_of_week '{}' in {}. Expected monday..sunday",
                first_day,
                path.display()
            )
        })?;
        settings.first_day_of_week = parsed;
    }

    settings.day_boundary = DayBoundaryConfig {
        mode: settings.day_boundary.mode,
        fixed_hour: settings.day_boundary.fixed_hour,
        fixed_minute: settings.day_boundary.fixed_minute,
        utc_offset_seconds: settings.day_boundary.utc_offset_seconds,
    };

    Ok(settings)
}

fn parse_unbound_actions(config: &KeymapConfig, path: &Path) -> Result<HashSet<Action>, String> {
    let mut actions = HashSet::new();

    for raw in &config.unbind_actions {
        let action = Action::from_config_name(raw).ok_or_else(|| {
            format!(
                "Unknown action '{}' in {}. Expected known action names in unbind_actions",
                raw,
                path.display()
            )
        })?;
        actions.insert(action);
    }

    Ok(actions)
}

fn load_config_or_default(path: &Path) -> Result<KeymapConfig, String> {
    if !path.exists() {
        return Ok(KeymapConfig::default());
    }

    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed reading keymap at {}: {e}", path.display()))?;

    let parsed: KeymapConfig = serde_json::from_str(&content)
        .map_err(|e| format!("Failed parsing keymap JSON at {}: {e}", path.display()))?;

    Ok(parsed)
}

fn save_config(path: &Path, config: &KeymapConfig) -> Result<(), String> {
    let serialized = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
    crate::storage::atomic_write(path, &serialized)
}

pub(crate) fn load_keybindings(path: &Path) -> Result<LoadedKeybindings, String> {
    let mut config = load_config_or_default(path)?;
    if config
        .day_start_mode
        .as_deref()
        .is_some_and(|mode| mode.trim().eq_ignore_ascii_case("sunrise"))
    {
        config.day_start_mode = Some("fixed".to_string());
        save_config(path, &config).map_err(|error| {
            format!(
                "Failed migrating removed sunrise mode in {} to fixed-clock policy: {error}",
                path.display()
            )
        })?;
        eprintln!(
            "Warning: migrated removed day_start_mode 'sunrise' in {} to fixed-clock policy at {:02}:{:02}; Strata never implemented solar sunrise calculation",
            path.display(),
            config.day_start_hour.unwrap_or(6),
            config.day_start_minute.unwrap_or(0)
        );
    }

    let runtime_settings = parse_runtime_settings(&config, path)?;
    if config.time_log_path.is_some() {
        return Err(format!(
            "time_log_path in {} is no longer supported because it hot-redirected one file without switching complete profile authority; exit Strata and use --profile <directory>",
            path.display()
        ));
    }
    let contextual_aliases = parse_contextual_aliases(&config, path)?;
    let disabled_actions = parse_unbound_actions(&config, path)?;

    let mut parsed_overrides: Vec<(KeyBinding, Option<Action>)> = Vec::new();
    let mut overridden_actions: HashSet<Action> = HashSet::new();

    for (raw_key, raw_action) in config.keymap {
        let parsed_key = KeyBinding::parse(&raw_key)
            .map_err(|e| format!("Invalid key '{}' in {}: {e}", raw_key, path.display()))?;

        if parsed_key.is_mandatory_quit() {
            return Err(format!(
                "Key 'ctrl-c' in {} is mandatory Quit policy and cannot be configured",
                path.display()
            ));
        }

        let parsed_action = match raw_action {
            Some(action_name) => {
                let action = Action::from_config_name(&action_name).ok_or_else(|| {
                    let available = Action::all()
                        .iter()
                        .map(|action| action.config_name())
                        .collect::<Vec<_>>()
                        .join(", ");

                    format!(
                        "Unknown action '{}' in {}. Available actions: {available}",
                        action_name,
                        path.display()
                    )
                })?;
                if disabled_actions.contains(&action) {
                    return Err(format!(
                        "Action '{}' in {} is both bound and disabled",
                        action.config_name(),
                        path.display()
                    ));
                }
                overridden_actions.insert(action);
                Some(action)
            }
            None => None,
        };

        parsed_overrides.push((parsed_key, parsed_action));
    }

    let mut keymap = if config.keymap_inherit {
        default_keymap()
    } else {
        Keymap::empty()
    };

    if config.keymap_inherit {
        for action in overridden_actions {
            keymap.unbind_action(action);
        }
    }

    for (key, action) in parsed_overrides {
        if let Some(action) = action {
            keymap.bind(key, action);
        } else {
            keymap.unbind_key(&key);
        }
    }
    for action in disabled_actions {
        keymap.disable_action(action);
    }
    keymap.contextual_aliases = contextual_aliases;

    Ok(LoadedKeybindings {
        keymap,
        runtime_settings,
    })
}

fn remove_action_keymap_entries(config: &mut KeymapConfig, action: Action) {
    config.keymap.retain(|_, mapped| match mapped {
        Some(action_name) => Action::from_config_name(action_name) != Some(action),
        None => true,
    });
}

fn remove_unbound_action_marker(config: &mut KeymapConfig, action: Action) {
    config
        .unbind_actions
        .retain(|name| Action::from_config_name(name) != Some(action));
}

pub(crate) fn set_action_binding(
    path: &Path,
    action: Action,
    binding: Option<KeyBinding>,
) -> Result<LoadedKeybindings, String> {
    if binding.is_some_and(KeyBinding::is_mandatory_quit) {
        return Err("Ctrl-C is mandatory Quit policy and cannot be rebound".to_string());
    }
    let mut config = load_config_or_default(path)?;
    remove_action_keymap_entries(&mut config, action);
    remove_unbound_action_marker(&mut config, action);

    if let Some(binding) = binding {
        config.keymap.insert(
            binding.to_config_string(),
            Some(action.config_name().to_string()),
        );
    } else {
        config.unbind_actions.push(action.config_name().to_string());
        config.unbind_actions.sort();
        config.unbind_actions.dedup();
    }

    save_config(path, &config)?;
    load_keybindings(path)
}

pub(crate) fn set_action_unbound(path: &Path, action: Action) -> Result<LoadedKeybindings, String> {
    let mut config = load_config_or_default(path)?;
    remove_action_keymap_entries(&mut config, action);
    remove_unbound_action_marker(&mut config, action);

    if config.keymap_inherit {
        for (raw_key, default_action) in DEFAULT_BINDINGS {
            if default_action == action {
                config.keymap.insert(raw_key.to_string(), None);
            }
        }
    }

    save_config(path, &config)?;
    load_keybindings(path)
}

pub(crate) fn set_first_day_of_week(
    path: &Path,
    day: FirstDayOfWeek,
) -> Result<LoadedKeybindings, String> {
    let mut config = load_config_or_default(path)?;
    config.first_day_of_week = Some(day.as_config_name().to_string());
    save_config(path, &config)?;
    load_keybindings(path)
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf, time::SystemTime};

    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    use crate::domain::{DayBoundaryMode, FirstDayOfWeek};

    use super::{
        Action, ActionBindingState, InputContext, KeyBinding, Keymap, ResolvedActionSource,
        default_keymap, load_keybindings, set_action_binding, set_action_unbound,
        set_first_day_of_week,
    };

    fn unique_path(prefix: &str) -> PathBuf {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        PathBuf::from(format!("/tmp/{prefix}_{now}.json"))
    }

    fn load_keymap_for_test(path: &std::path::Path) -> Result<Keymap, String> {
        load_keybindings(path).map(|loaded| loaded.keymap)
    }

    #[test]
    fn test_parse_key_binding_with_modifiers() {
        let key = KeyBinding::parse("ctrl-shift-left").expect("key should parse");
        assert_eq!(key.to_string(), "Ctrl-Shift-←");
    }

    #[test]
    fn test_parse_single_symbol_key() {
        let key = KeyBinding::parse("?").expect("key should parse");
        assert_eq!(key.to_string(), "?");
    }

    #[test]
    fn test_key_event_normalizes_uppercase_letter_to_shift() {
        let event = KeyEvent::new(KeyCode::Char('C'), KeyModifiers::SHIFT);
        let key = KeyBinding::from_key_event(event).expect("event should normalize");
        assert_eq!(key.to_string(), "Shift-c");
    }

    #[test]
    fn test_key_event_drops_shift_for_symbol_chars() {
        let event = KeyEvent::new(KeyCode::Char('?'), KeyModifiers::SHIFT);
        let key = KeyBinding::from_key_event(event).expect("event should normalize");
        assert_eq!(key.to_string(), "?");
    }

    #[test]
    fn test_default_keymap_has_ctrl_p_for_command_palette() {
        let keymap = default_keymap();
        let ctrl_p = KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL);

        assert_eq!(
            keymap.action_for_key_event(ctrl_p),
            Some(Action::ToggleCommandPalette)
        );
    }

    #[test]
    fn test_default_keymap_has_b_for_balance() {
        let keymap = default_keymap();
        let b = KeyEvent::new(KeyCode::Char('b'), KeyModifiers::NONE);

        assert_eq!(
            keymap.action_for_key_event(b),
            Some(Action::OpenReportModal)
        );
    }

    #[test]
    fn test_default_keymap_has_d_for_detach() {
        let keymap = default_keymap();
        let d = KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE);

        assert_eq!(keymap.action_for_key_event(d), Some(Action::Detach));
    }

    #[test]
    fn default_x_action_is_the_contextual_remove_action() {
        let keymap = default_keymap();
        assert_eq!(
            keymap.action_for_key_event(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE)),
            Some(Action::DeleteCategory)
        );
        assert_eq!(
            Action::DeleteCategory.description(),
            "Archive selected layer / delete selected report session"
        );
    }

    #[test]
    fn test_default_keymap_has_t_for_balance_day() {
        let keymap = default_keymap();
        let t = KeyEvent::new(KeyCode::Char('t'), KeyModifiers::NONE);

        assert_eq!(keymap.action_for_key_event(t), Some(Action::ReportToday));
    }

    #[test]
    fn test_default_keymap_has_r_for_balance_range() {
        let keymap = default_keymap();
        let r = KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE);

        assert_eq!(keymap.action_for_key_event(r), Some(Action::ReportRange));
    }

    #[test]
    fn test_default_keymap_has_l_for_balance_log_activity() {
        let keymap = default_keymap();
        let l = KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE);

        assert_eq!(
            keymap.action_for_key_event(l),
            Some(Action::LogActivity)
        );
    }

    #[test]
    fn test_from_config_name_supports_command_palette_aliases() {
        assert_eq!(
            Action::from_config_name("toggle_command_palette"),
            Some(Action::ToggleCommandPalette)
        );
        assert_eq!(
            Action::from_config_name("toggle_palette"),
            Some(Action::ToggleCommandPalette)
        );
    }

    #[test]
    fn test_from_config_name_supports_detach_aliases() {
        assert_eq!(Action::from_config_name("detach"), Some(Action::Detach));
        assert_eq!(
            Action::from_config_name("detach_from_main"),
            Some(Action::Detach)
        );
    }

    #[test]
    fn test_load_keymap_inherit_replaces_default_action_bindings() {
        let path = unique_path("strata_keymap_override");
        let raw = r#"{
  "keymap_inherit": true,
  "keymap": {
    "ctrl-q": "quit"
  }
}"#;
        fs::write(&path, raw).expect("write config");

        let keymap = load_keymap_for_test(&path).expect("keymap should load");

        let q = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
        let ctrl_q = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::CONTROL);

        assert_eq!(keymap.action_for_key_event(q), None);
        assert_eq!(keymap.action_for_key_event(ctrl_q), Some(Action::Quit));

        fs::remove_file(path).ok();
    }

    #[test]
    fn test_load_keymap_allows_unbinding() {
        let path = unique_path("strata_keymap_unbind");
        let raw = r#"{
  "keymap_inherit": true,
  "keymap": {
    "k": null
  }
}"#;
        fs::write(&path, raw).expect("write config");

        let keymap = load_keymap_for_test(&path).expect("keymap should load");
        let key = KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE);

        assert_eq!(keymap.action_for_key_event(key), None);

        fs::remove_file(path).ok();
    }

    #[test]
    fn test_load_keymap_without_inherit_uses_only_custom_bindings() {
        let path = unique_path("strata_keymap_no_inherit");
        let raw = r#"{
  "keymap_inherit": false,
  "keymap": {
    "f": "open_report_modal"
  }
}"#;
        fs::write(&path, raw).expect("write config");

        let keymap = load_keymap_for_test(&path).expect("keymap should load");
        let f = KeyEvent::new(KeyCode::Char('f'), KeyModifiers::NONE);
        let k = KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE);

        assert_eq!(
            keymap.action_for_key_event(f),
            Some(Action::OpenReportModal)
        );
        assert_eq!(keymap.action_for_key_event(k), None);

        fs::remove_file(path).ok();
    }

    #[test]
    fn test_load_keymap_unknown_action_returns_error() {
        let path = unique_path("strata_keymap_unknown_action");
        let raw = r#"{
  "keymap_inherit": true,
  "keymap": {
    "f": "not_real"
  }
}"#;
        fs::write(&path, raw).expect("write config");

        let err = load_keymap_for_test(&path).expect_err("config should fail");
        assert!(err.contains("Unknown action 'not_real'"));

        fs::remove_file(path).ok();
    }

    #[test]
    fn test_load_keybindings_parses_runtime_settings() {
        let path = unique_path("strata_keymap_runtime_settings");
        let raw = r#"{
  "keymap_inherit": true,
  "day_start_mode": "sunrise",
  "day_start_hour": 5,
  "day_start_minute": 45,
  "first_day_of_week": "sunday"
}"#;
        fs::write(&path, raw).expect("write config");

        let loaded = load_keybindings(&path).expect("config should parse");
        assert_eq!(
            loaded.runtime_settings.day_boundary.mode,
            DayBoundaryMode::FixedHour
        );
        let migrated = fs::read_to_string(&path).expect("migrated config should be readable");
        assert!(migrated.contains("\"day_start_mode\": \"fixed\""));
        assert_eq!(loaded.runtime_settings.day_boundary.fixed_hour, 5);
        assert_eq!(loaded.runtime_settings.day_boundary.fixed_minute, 45);
        assert_eq!(
            loaded.runtime_settings.first_day_of_week,
            FirstDayOfWeek::Sunday
        );

        fs::remove_file(path).ok();
    }

    #[test]
    fn test_load_keybindings_invalid_first_day_returns_error() {
        let path = unique_path("strata_keymap_invalid_week_start");
        let raw = r#"{
  "first_day_of_week": "funday"
}"#;
        fs::write(&path, raw).expect("write config");

        let err = load_keybindings(&path).expect_err("config should fail");
        assert!(err.contains("Invalid first_day_of_week"));

        fs::remove_file(path).ok();
    }

    #[test]
    fn test_set_action_binding_assigns_and_persists() {
        let path = unique_path("strata_keymap_set_action_binding");

        let key = KeyBinding::parse("ctrl-l").expect("key should parse");
        let loaded = set_action_binding(&path, Action::OpenCategoryModal, Some(key))
            .expect("binding should persist");

        let event = KeyEvent::new(KeyCode::Char('l'), KeyModifiers::CONTROL);
        assert_eq!(
            loaded.keymap.action_for_key_event(event),
            Some(Action::OpenCategoryModal)
        );

        fs::remove_file(path).ok();
    }

    #[test]
    fn test_set_first_day_of_week_persists_setting() {
        let path = unique_path("strata_keymap_set_week_start");

        let loaded = set_first_day_of_week(&path, FirstDayOfWeek::Sunday)
            .expect("week-start setting should persist");
        assert_eq!(
            loaded.runtime_settings.first_day_of_week,
            FirstDayOfWeek::Sunday
        );

        fs::remove_file(path).ok();
    }

    #[test]
    fn test_load_keybindings_malformed_json_identifies_path() {
        let path = unique_path("strata_keymap_malformed_json");
        fs::write(&path, "{ not-json").expect("write config");

        let err = load_keybindings(&path).expect_err("config should fail");
        assert!(err.contains("Failed parsing keymap JSON"));
        assert!(err.contains(&path.display().to_string()));

        fs::remove_file(path).ok();
    }

    #[test]
    fn test_load_keybindings_invalid_utc_offset_returns_error() {
        let path = unique_path("strata_keymap_invalid_offset");
        fs::write(&path, r#"{"utc_offset_seconds": 86400}"#).expect("write config");

        let err = load_keybindings(&path).expect_err("config should fail");
        assert!(err.contains("Invalid utc_offset_seconds '86400'"));
        assert!(err.contains(&path.display().to_string()));

        fs::remove_file(path).ok();
    }

    #[test]
    fn test_load_keybindings_rejects_time_log_path_hot_redirect() {
        let path = unique_path("strata_keymap_obsolete_time_log_path");
        let raw = r#"{
  "time_log_path": "/tmp/strata-other-profile/time_log.csv"
}"#;
        fs::write(&path, raw).expect("write config");

        let err = load_keybindings(&path).expect_err("config should fail");
        assert!(err.contains("time_log_path"));
        assert!(err.contains("--profile"));

        fs::remove_file(path).ok();
    }

    #[test]
    fn contradictory_bound_and_disabled_action_is_rejected() {
        let path = unique_path("strata_keymap_contradictory_state");
        fs::write(
            &path,
            r#"{
              "keymap":{"ctrl-r":"open_balance_popup"},
              "unbind_actions":["open_balance_popup"]
            }"#,
        )
        .unwrap();
        let error = load_keymap_for_test(&path).unwrap_err();
        assert!(error.contains("both bound and disabled"));
        fs::remove_file(path).ok();
    }

    #[test]
    fn atlas_writer_rejects_mandatory_ctrl_c_before_persisting() {
        let path = unique_path("strata_keymap_mandatory_writer");
        let ctrl_c = KeyBinding::parse("ctrl-c").unwrap();
        let error = set_action_binding(&path, Action::OpenReportModal, Some(ctrl_c)).unwrap_err();
        assert!(error.contains("mandatory Quit policy"));
        assert!(
            !path.exists(),
            "invalid mandatory override must not be persisted"
        );
    }

    #[test]
    fn mandatory_ctrl_c_is_separate_from_configured_quit_state() {
        let path = unique_path("strata_keymap_mandatory_quit");
        fs::write(&path, r#"{"unbind_actions":["quit"]}"#).unwrap();
        let keymap = load_keymap_for_test(&path).unwrap();
        assert_eq!(
            keymap.action_state(Action::Quit),
            ActionBindingState::Disabled
        );
        let ctrl_c = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        let resolved = keymap
            .resolve_key_event(InputContext::Main, ctrl_c)
            .unwrap();
        assert_eq!(resolved.action, Action::Quit);
        assert_eq!(resolved.source, ResolvedActionSource::Mandatory);
        fs::remove_file(path).ok();
    }

    #[test]
    fn configuring_mandatory_ctrl_c_is_rejected() {
        let path = unique_path("strata_keymap_mandatory_conflict");
        fs::write(&path, r#"{"keymap":{"ctrl-c":"open_balance_popup"}}"#).unwrap();
        let error = load_keymap_for_test(&path).unwrap_err();
        assert!(error.contains("mandatory Quit policy"));
        fs::remove_file(path).ok();
    }

    #[test]
    fn null_key_removal_produces_unbound_not_disabled() {
        let path = unique_path("strata_keymap_unbound_state");
        fs::write(&path, r#"{"keymap":{"f1":null,"?":null}}"#).unwrap();
        let keymap = load_keymap_for_test(&path).unwrap();
        assert_eq!(
            keymap.action_state(Action::ToggleKeybindingsHelp),
            ActionBindingState::Unbound
        );
        let f1 = KeyEvent::new(KeyCode::F(1), KeyModifiers::NONE);
        assert_eq!(keymap.resolve_key_event(InputContext::Main, f1), None);
        fs::remove_file(path).ok();
    }

    #[test]
    fn disabled_action_is_not_reached_by_direct_or_contextual_routes() {
        let path = unique_path("strata_keymap_disabled_alias");
        fs::write(&path, r#"{"unbind_actions":["open_layer_popup"]}"#).unwrap();
        let keymap = load_keymap_for_test(&path).unwrap();
        assert_eq!(
            keymap.action_state(Action::OpenCategoryModal),
            ActionBindingState::Disabled
        );
        let enter = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        let resolved = keymap.resolve_key_event(InputContext::Main, enter).unwrap();
        assert_eq!(resolved.action, Action::Confirm);
        assert_eq!(resolved.source, ResolvedActionSource::Direct);
        fs::remove_file(path).ok();
    }

    #[test]
    fn inherited_aliases_follow_declared_conditions() {
        let keymap = default_keymap();
        let enter = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        let escape = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        let today = KeyEvent::new(KeyCode::Char('t'), KeyModifiers::NONE);
        let detach = KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE);

        assert_eq!(
            keymap
                .resolve_key_event(InputContext::Main, enter)
                .unwrap()
                .action,
            Action::OpenCategoryModal
        );
        assert_eq!(
            keymap
                .resolve_key_event(InputContext::Main, escape)
                .unwrap()
                .action,
            Action::SwitchToNone
        );
        assert_eq!(
            keymap
                .resolve_key_event(InputContext::Main, today)
                .unwrap()
                .action,
            Action::ReportToday,
            "detach is directly bound, so target-unbound alias must not apply"
        );
        assert_eq!(
            keymap
                .resolve_key_event(InputContext::Report, detach)
                .unwrap()
                .action,
            Action::ReportToday
        );
    }

    #[test]
    fn alias_can_be_removed_and_target_can_be_bound() {
        let path = unique_path("strata_keymap_alias_override");
        fs::write(
            &path,
            r#"{
              "keymap":{"ctrl-l":"open_layer_popup"},
              "contextual_aliases":{"main.cancel":null}
            }"#,
        )
        .unwrap();
        let keymap = load_keymap_for_test(&path).unwrap();
        let enter = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        let escape = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        assert_eq!(
            keymap
                .resolve_key_event(InputContext::Main, enter)
                .unwrap()
                .action,
            Action::Confirm,
            "bound target disables target-unbound alias"
        );
        assert_eq!(
            keymap
                .resolve_key_event(InputContext::Main, escape)
                .unwrap()
                .action,
            Action::Cancel,
            "removed alias leaves source action unchanged"
        );
        fs::remove_file(path).ok();
    }

    #[test]
    fn atlas_unbind_and_disable_persist_distinct_states() {
        let path = unique_path("strata_keymap_atlas_states");
        let unbound = set_action_unbound(&path, Action::OpenReportModal).unwrap();
        assert_eq!(
            unbound.keymap.action_state(Action::OpenReportModal),
            ActionBindingState::Unbound
        );
        let disabled = set_action_binding(&path, Action::OpenReportModal, None).unwrap();
        assert_eq!(
            disabled.keymap.action_state(Action::OpenReportModal),
            ActionBindingState::Disabled
        );
        fs::remove_file(path).ok();
    }
}
