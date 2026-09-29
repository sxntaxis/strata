use crate::{
    command::{self, CommandIntent},
    domain::{Category, CategoryId, DRIFT_CATEGORY_ID, ReportPeriod, is_drift_category_id},
    keybindings::{Action, InputContext},
    sqlite,
};
use chrono::{Duration as ChronoDuration, Local, NaiveDate};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use super::{App, PaletteCommand, ReportRangeBoundary, RuntimeMutation, ui_helpers};

#[cfg(debug_assertions)]
const TESTING_FILL_CATEGORY_SPECS: [(&str, usize); 6] = [
    ("Fixture Green", 0),
    ("Fixture Yellow", 2),
    ("Fixture Red", 6),
    ("Fixture Purple", 7),
    ("Fixture Blue", 9),
    ("Fixture Cyan", 11),
];

#[cfg(debug_assertions)]
fn ensure_testing_fill_categories_in_tracker(
    tracker: &mut crate::domain::TimeTracker,
) -> Result<(Vec<CategoryId>, bool), String> {
    let mut created = false;
    let mut ids = Vec::with_capacity(TESTING_FILL_CATEGORY_SPECS.len());
    for (name, new_category_color_cursor) in TESTING_FILL_CATEGORY_SPECS {
        if let Some(category) = tracker
            .categories_ordered()
            .into_iter()
            .find(|category| category.name.eq_ignore_ascii_case(name))
        {
            ids.push(category.id);
            continue;
        }
        let id = tracker
            .add_category(
                name.to_string(),
                "testingcheats fill fixture layer".to_string(),
                Some(new_category_color_cursor),
            )
            .ok_or_else(|| format!("failed to create testing fixture category {name}"))?;
        ids.push(id);
        created = true;
    }
    Ok((ids, created))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LedgerEntryEditKeyIntent {
    Append(char),
    Backspace,
    NextField,
    PreviousField,
    Left,
    Right,
    ShiftLeft,
    ShiftRight,
    Commit,
    Cancel,
    EmergencyQuit,
    Ignore,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ReportRangeEditKeyIntent {
    Append(char),
    Backspace,
    NextField,
    PreviousField,
    Commit,
    Cancel,
    EmergencyQuit,
    Ignore,
}

fn direct_command_or_fuzzy_fallback(
    query: &str,
    has_fuzzy_result: bool,
) -> Result<Option<CommandIntent>, String> {
    let typed = query.trim();
    if typed.is_empty() {
        return Ok(None);
    }
    match crate::command::parse(typed) {
        Ok(command) => Ok(Some(command)),
        Err(_) if has_fuzzy_result => Ok(None),
        Err(error) => Err(error),
    }
}

fn resolve_ledger_entry_edit_key(
    key: KeyEvent,
    keymap: &crate::keybindings::Keymap,
) -> LedgerEntryEditKeyIntent {
    if key
        .modifiers
        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
    {
        return if keymap.mandatory_action_for_key_event(key) == Some(Action::Quit) {
            LedgerEntryEditKeyIntent::EmergencyQuit
        } else {
            LedgerEntryEditKeyIntent::Ignore
        };
    }

    match key.code {
        KeyCode::Esc => LedgerEntryEditKeyIntent::Cancel,
        KeyCode::Enter => LedgerEntryEditKeyIntent::Commit,
        KeyCode::Backspace | KeyCode::Delete => LedgerEntryEditKeyIntent::Backspace,
        KeyCode::BackTab => LedgerEntryEditKeyIntent::PreviousField,
        KeyCode::Tab if key.modifiers.contains(KeyModifiers::SHIFT) => {
            LedgerEntryEditKeyIntent::PreviousField
        }
        KeyCode::Tab => LedgerEntryEditKeyIntent::NextField,
        KeyCode::Left if key.modifiers.contains(KeyModifiers::SHIFT) => {
            LedgerEntryEditKeyIntent::ShiftLeft
        }
        KeyCode::Right if key.modifiers.contains(KeyModifiers::SHIFT) => {
            LedgerEntryEditKeyIntent::ShiftRight
        }
        KeyCode::Left => LedgerEntryEditKeyIntent::Left,
        KeyCode::Right => LedgerEntryEditKeyIntent::Right,
        KeyCode::Char(character) => LedgerEntryEditKeyIntent::Append(character),
        _ => LedgerEntryEditKeyIntent::Ignore,
    }
}

fn resolve_report_range_edit_key(
    key: KeyEvent,
    keymap: &crate::keybindings::Keymap,
) -> ReportRangeEditKeyIntent {
    if key
        .modifiers
        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
    {
        return if keymap.mandatory_action_for_key_event(key) == Some(Action::Quit) {
            ReportRangeEditKeyIntent::EmergencyQuit
        } else {
            ReportRangeEditKeyIntent::Ignore
        };
    }

    match key.code {
        KeyCode::Esc => ReportRangeEditKeyIntent::Cancel,
        KeyCode::Enter => ReportRangeEditKeyIntent::Commit,
        KeyCode::Backspace | KeyCode::Delete => ReportRangeEditKeyIntent::Backspace,
        KeyCode::BackTab => ReportRangeEditKeyIntent::PreviousField,
        KeyCode::Tab if key.modifiers.contains(KeyModifiers::SHIFT) => {
            ReportRangeEditKeyIntent::PreviousField
        }
        KeyCode::Tab => ReportRangeEditKeyIntent::NextField,
        KeyCode::Char(character) if character.is_ascii_digit() || character == '-' => {
            ReportRangeEditKeyIntent::Append(character)
        }
        _ => ReportRangeEditKeyIntent::Ignore,
    }
}

impl App {
    #[cfg(debug_assertions)]
    fn ensure_testing_fill_categories(&mut self) -> Result<Vec<CategoryId>, String> {
        let (ids, created) = ensure_testing_fill_categories_in_tracker(&mut self.time_tracker)?;

        if created {
            self.persist_categories();
            if self.has_persistence_recovery() {
                return Err(
                    "testingcheats fill created fixture categories but category persistence entered recovery"
                        .to_string(),
                );
            }
        }
        Ok(ids)
    }

    pub(super) fn handle_key(&mut self, key: KeyEvent) -> bool {
        if key.kind == KeyEventKind::Release {
            return false;
        }

        if self.has_persistence_recovery() {
            if self.keymap.mandatory_action_for_key_event(key) == Some(Action::Quit) {
                return self.request_persistence_recovery_quit();
            }
            return self.handle_persistence_recovery_key(key);
        }

        if self.recovery_statement.is_some() {
            if self.keymap.mandatory_action_for_key_event(key) == Some(Action::Quit) {
                return true;
            }
            return self.handle_recovery_statement_key(key);
        }

        if self.keymap.mandatory_action_for_key_event(key) == Some(Action::Quit) {
            if self.in_category_modal() && !self.persist_modal_active_description() {
                return false;
            }
            return true;
        }

        if self.report_range_edit.is_some() {
            return self.handle_report_range_edit_key(key);
        }

        if self.ledger_entry_edit.is_some() {
            return self.handle_ledger_entry_edit_key(key);
        }

        if self.show_command_palette {
            return self.handle_command_palette_key(key);
        }

        if self.show_settings && self.settings_overlay.is_some() {
            return self.handle_settings_overlay_key(key);
        }

        if self.in_category_modal()
            && !self.show_settings
            && !key
                .modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
        {
            match key.code {
                KeyCode::Char(character) => {
                    let current_tag_empty = self
                        .modal_description
                        .rsplit(';')
                        .next()
                        .unwrap_or_default()
                        .trim()
                        .is_empty();
                    let polarity_command = !self.modal_renaming_category
                        && !self.is_on_insert_space()
                        && matches!(character, '+' | '=' | '-' | '_')
                        && (!self.modal_tag_text_editing || current_tag_empty);
                    if !polarity_command {
                        self.handle_modal_text_input(key);
                        return false;
                    }
                }
                KeyCode::Backspace | KeyCode::Delete => {
                    self.handle_modal_text_delete();
                    return false;
                }
                _ => {}
            }
        }

        if let Some(action) = self.resolve_action(key) {
            return self.route_action(action, key);
        }

        false
    }

    fn resolve_action(&self, key: KeyEvent) -> Option<Action> {
        if self.in_category_modal() && !self.show_settings && matches!(key.code, KeyCode::Char('?'))
        {
            return None;
        }

        let context = if self.in_balance_modal() {
            InputContext::Report
        } else if self.in_category_modal() || self.show_settings {
            InputContext::Other
        } else {
            InputContext::Main
        };
        self.keymap
            .resolve_key_event(context, key)
            .map(|resolved| resolved.action)
    }

    fn route_action(&mut self, action: Action, key: KeyEvent) -> bool {
        if action == Action::ToggleCommandPalette {
            self.toggle_command_palette();
            return false;
        }

        if action == Action::ToggleSettings {
            self.toggle_settings();
            return false;
        }

        if self.show_settings {
            return self.handle_settings_action(action);
        }

        if self.in_category_modal() {
            let handled = self.handle_modal_action(action);
            if !handled {
                self.handle_modal_text_input(key);
            }
            return false;
        }

        if self.in_balance_modal() {
            return self.handle_report_modal_action(action);
        }

        self.handle_main_action(action)
    }

    fn handle_command_palette_key(&mut self, key: KeyEvent) -> bool {
        if self
            .keymap
            .resolve_key_event(InputContext::Other, key)
            .is_some_and(|resolved| resolved.action == Action::ToggleCommandPalette)
        {
            self.close_command_palette();
            return false;
        }

        let entries = self.filtered_command_palette_entries();
        self.clamp_command_palette_selection(entries.len());

        match key.code {
            KeyCode::Esc => self.close_command_palette(),
            KeyCode::Enter => {
                match direct_command_or_fuzzy_fallback(
                    &self.command_palette_query,
                    !entries.is_empty(),
                ) {
                    Ok(Some(command)) => {
                        let keep_open = command.keeps_palette_open();
                        match self.execute_command(command) {
                            Ok(message) if keep_open => {
                                self.command_palette_feedback = Some(message);
                                self.render_needed = true;
                            }
                            Ok(_) => self.close_command_palette(),
                            Err(error) => {
                                self.command_palette_feedback = Some(format!("Error: {error}"));
                                self.render_needed = true;
                            }
                        }
                        return false;
                    }
                    Ok(None) => {}
                    Err(error) => {
                        self.command_palette_feedback = Some(format!("Error: {error}"));
                        self.render_needed = true;
                        return false;
                    }
                }
                if let Some(entry) = entries.get(self.command_palette_selected_index) {
                    return self.execute_palette_command(entry.command);
                }
                self.close_command_palette();
            }
            KeyCode::Up => {
                if !entries.is_empty() {
                    self.command_palette_selected_index = ui_helpers::wrap_prev_index(
                        self.command_palette_selected_index,
                        entries.len(),
                    );
                    self.render_needed = true;
                }
            }
            KeyCode::Down => {
                if !entries.is_empty() {
                    self.command_palette_selected_index = ui_helpers::wrap_next_index(
                        self.command_palette_selected_index,
                        entries.len(),
                    );
                    self.render_needed = true;
                }
            }
            KeyCode::Home => {
                self.command_palette_selected_index = 0;
                self.command_palette_scroll = 0;
                self.render_needed = true;
            }
            KeyCode::End => {
                if !entries.is_empty() {
                    self.command_palette_selected_index = entries.len() - 1;
                    self.render_needed = true;
                }
            }
            KeyCode::Backspace => {
                self.command_palette_query.pop();
                self.command_palette_feedback = None;
                let updated = self.filtered_command_palette_entries();
                self.clamp_command_palette_selection(updated.len());
                self.render_needed = true;
            }
            KeyCode::Char(c)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                self.command_palette_query.push(c);
                self.command_palette_feedback = None;
                let updated = self.filtered_command_palette_entries();
                self.clamp_command_palette_selection(updated.len());
                self.render_needed = true;
            }
            _ => {}
        }

        false
    }

    fn resolve_layer_case_insensitive(&self, layer: &str) -> Option<Category> {
        let trimmed = layer.trim();
        if trimmed.is_empty() {
            return None;
        }
        let categories = self.time_tracker.categories_ordered();
        if let Ok(id) = trimmed.parse::<u64>()
            && let Some(found) = categories.iter().find(|category| category.id.0 == id)
        {
            return Some(found.clone());
        }
        categories
            .into_iter()
            .find(|category| category.name.eq_ignore_ascii_case(trimmed))
    }

    fn layer_suggestions(&self, layer: &str) -> Vec<String> {
        let needle = layer.trim().to_ascii_lowercase();
        if needle.is_empty() {
            return Vec::new();
        }
        self.time_tracker
            .categories_ordered()
            .into_iter()
            .map(|category| category.name)
            .filter(|name| name.to_ascii_lowercase().contains(&needle))
            .take(3)
            .collect()
    }

    fn canonicalize_tag_for_layer(&self, layer_id: CategoryId, tag: &str) -> String {
        self.canonicalize_description_for_category(layer_id, tag)
    }

    fn remember_tag_for_layer(&mut self, layer_id: CategoryId, tag: &str) {
        self.remember_description_tags_for_category(layer_id, tag);
    }

    pub(super) fn execute_command(&mut self, command: CommandIntent) -> Result<String, String> {
        match command {
            CommandIntent::Status => self.command_status(),
            CommandIntent::Start { layer, tag } => self.command_start(layer, tag),
            CommandIntent::Stop { layer, tag } => self.command_stop(layer, tag),
            CommandIntent::Balance {
                selector,
                layer,
                tag,
            } => self.command_balance(selector, layer, tag),
            CommandIntent::DeleteLastSession { layer, tag } => {
                self.command_delete_last_session(layer, tag)
            }
            CommandIntent::DataDir => Ok(format!(
                "Data dir: {}",
                crate::profile::data_dir().display()
            )),
            CommandIntent::ConfigDir => Ok(format!(
                "Config dir: {}",
                crate::profile::config_dir().display()
            )),
            CommandIntent::Timer { duration_seconds } => {
                let end = Local::now()
                    + ChronoDuration::seconds(i64::try_from(duration_seconds).unwrap_or(i64::MAX));
                Ok(format!(
                    "Timer {} (ends {})",
                    command::format_hms(duration_seconds as usize),
                    end.format("%Y-%m-%d %H:%M:%S")
                ))
            }
            #[cfg(debug_assertions)]
            CommandIntent::TestingCheatsHelp => Ok(
                "testingcheats: default sandbox classic · model <h4|classic|hybrid|oslo-zero|oslo-box|oslo-vessel|oslo-vessel-momentum|oslo-vessel-front|oslo-vessel-front-flowviz|oslo-vessel-front-parcels|oslo-vessel-front-grains|oslo-vessel-fluid> · classic texture [baseline|textured|rugged|terraced] · classic experiment [rugged|memory|slope|memory-slope|anchored|momentum|momentum-repose|momentum-tangent|momentum-soft|momentum-contact|momentum-repose-contact|momentum-surface|momentum-grounded-contact] · classic colorblend [rgb|rgb-additive|rgb-luma|rgb-luma-safe|rgb-mid|rgb-contrast|linear|oklab|dominant|dominant-soft] · classic colorbackground [neutral|dark|light] · classic stratigraphy · classic rainmetrics · fallspeed [1x|4x|16x|64x|128x] · advance <duration> · fill (classic/hybrid/Oslo; ensures six Fixture categories) · fillhalf (same fill, centered half-width) · clear · status · provenance · reset"
                    .to_string(),
            ),
            #[cfg(debug_assertions)]
            CommandIntent::TestingCheatsModel { model } => {
                self.set_testing_cheats_model(&model)?;
                let source = match model.as_str() {
                    "h4" => "fresh clone of authoritative sediment",
                    "classic" => "fresh empty pre-pause grain physics + modern canonical/VW walls",
                    "hybrid" => "classic physics + RAIN-004 full-width correlated meander and broad golden-small focus bias",
                    "oslo-vessel-momentum" => {
                        "oslo-vessel + causal moving-grain momentum phase on steep local failures"
                    }
                    "oslo-vessel-front" => {
                        "frozen oslo-vessel + rolling/static erosion-deposition exchange and uphill support-loss front"
                    }
                    "oslo-vessel-front-flowviz" => {
                        "frozen oslo-vessel-front physics + Eulerian edge-flux field and bounded Lagrangian tracer cloud"
                    }
                    "oslo-vessel-front-parcels" => {
                        "frozen oslo-vessel-front physics + conservative visual shadow surface and weighted local-flow parcels"
                    }
                    "oslo-vessel-front-grains" => {
                        "frozen oslo-vessel-front physics + one visual carrier per active CategoryId unit, causal observed-edge replay, and distance-aware long-drop playback"
                    }
                    "oslo-vessel-fluid" => {
                        "oslo-vessel-front + explicit partial-fluidization order field with start/stop hysteresis"
                    }
                    _ => "testing sandbox",
                };
                Ok(format!(
                    "Testing sandbox model: {model} ({source}; never persisted)"
                ))
            }
            #[cfg(debug_assertions)]
            CommandIntent::TestingCheatsFallSpeed { multiplier } => {
                self.ensure_testing_cheats_preview()?;
                let selected = multiplier.unwrap_or_else(|| self.testing_cheats_cycle_fallspeed());
                let testing = self.testing_cheats.as_mut().expect("testing preview exists");
                testing.set_speed_multiplier(selected);
                self.render_needed = true;
                Ok(format!(
                    "Testing sandbox {} fallspeed: {selected}x (stale multiplier debt cleared; explicit advance preserved; authoritative sediment unchanged)",
                    testing.engine.model_name()
                ))
            }
            #[cfg(debug_assertions)]
            CommandIntent::TestingCheatsAdvance { duration_seconds } => {
                self.ensure_testing_cheats_preview()?;
                self.queue_testing_cheats_simulated(std::time::Duration::from_secs(
                    duration_seconds,
                ));
                let (model, grains, queued) = self.testing_cheats.as_ref().map_or(
                    ("none", 0, std::time::Duration::ZERO),
                    |testing| {
                        (
                            testing.engine.model_name(),
                            testing.engine.grain_count(),
                            testing.total_queued_simulated(),
                        )
                    },
                );
                self.render_needed = true;
                Ok(format!(
                    "Testing sandbox {model} advancing {} ({} grains now; {} simulated remains; authoritative sediment unchanged)",
                    command::format_hms(duration_seconds as usize),
                    grains,
                    command::format_hms(queued.as_secs() as usize)
                ))
            }
            #[cfg(debug_assertions)]
            CommandIntent::TestingCheatsClassicTexture { profile } => {
                self.ensure_testing_cheats_preview()?;
                let testing = self.testing_cheats.as_mut().expect("testing preview exists");
                let model = testing.engine.model_name();
                if let Some(profile) = profile {
                    testing.engine.set_classic_texture_profile(&profile)?;
                    self.render_needed = true;
                    Ok(format!(
                        "Testing sandbox {model} texture profile: {profile} (new repose resamples only; use `testingcheats fill` or `testingcheats fillhalf` for a clean comparison)"
                    ))
                } else {
                    let profile = testing.engine.classic_texture_profile()?;
                    Ok(format!(
                        "Testing sandbox {model} texture profile: {profile} (baseline preserves CLASSIC-002 exactly)"
                    ))
                }
            }
            #[cfg(debug_assertions)]
            CommandIntent::TestingCheatsClassicExperiment { profile } => {
                self.ensure_testing_cheats_preview()?;
                let testing = self.testing_cheats.as_mut().expect("testing preview exists");
                let model = testing.engine.model_name();
                if let Some(profile) = profile {
                    testing.engine.set_classic_experiment_profile(&profile)?;
                    self.render_needed = true;
                    Ok(format!(
                        "Testing sandbox {model} Classic experiment: {profile} (rugged texture base; use `testingcheats fill` or `testingcheats fillhalf` for a clean comparison)"
                    ))
                } else {
                    let profile = testing.engine.classic_experiment_profile()?;
                    Ok(format!(
                        "Testing sandbox {model} Classic experiment: {profile}"
                    ))
                }
            }
            #[cfg(debug_assertions)]
            CommandIntent::TestingCheatsClassicColorBlend { profile } => {
                self.ensure_testing_cheats_preview()?;
                let testing = self.testing_cheats.as_mut().expect("testing preview exists");
                let model = testing.engine.model_name();
                if let Some(profile) = profile {
                    testing.engine.set_classic_color_blend_profile(&profile)?;
                    self.render_needed = true;
                    Ok(format!(
                        "Testing sandbox {model} Braille color blend: {profile} (render-only; physics/grid/categories unchanged)"
                    ))
                } else {
                    let profile = testing.engine.classic_color_blend_profile()?;
                    Ok(format!(
                        "Testing sandbox {model} Braille color blend: {profile} (rgb-additive is the owner-selected CLASSIC-014 baseline; rgb is the legacy control)"
                    ))
                }
            }
            #[cfg(debug_assertions)]
            CommandIntent::TestingCheatsClassicColorBackground { policy } => {
                self.ensure_testing_cheats_preview()?;
                let testing = self.testing_cheats.as_mut().expect("testing preview exists");
                let model = testing.engine.model_name();
                if let Some(policy) = policy {
                    testing.engine.set_classic_color_background_policy(&policy)?;
                    self.render_needed = true;
                    Ok(format!(
                        "Testing sandbox {model} Braille background policy: {policy} (render-only preview; rgb-contrast only; no terminal query or persistence)"
                    ))
                } else {
                    let policy = testing.engine.classic_color_background_policy()?;
                    Ok(format!(
                        "Testing sandbox {model} Braille background policy: {policy} (neutral is theme-independent; dark/light preview rgb-contrast)"
                    ))
                }
            }
            #[cfg(debug_assertions)]
            CommandIntent::TestingCheatsClassicStratigraphy => {
                let Some(testing) = self.testing_cheats.as_ref() else {
                    return Err(
                        "testingcheats classic stratigraphy requires an active classic/hybrid testing sandbox after testingcheats fill or fillhalf"
                            .to_string(),
                    );
                };
                let categories = self.time_tracker.categories_ordered();
                let report = testing.engine.classic_stratigraphy_report(&categories)?;
                let cache_root = std::env::var_os("XDG_CACHE_HOME")
                    .map(std::path::PathBuf::from)
                    .or_else(|| {
                        std::env::var_os("HOME")
                            .map(std::path::PathBuf::from)
                            .map(|home| home.join(".cache"))
                    })
                    .ok_or_else(|| {
                        "testingcheats classic stratigraphy could not resolve a cache directory"
                            .to_string()
                    })?;
                let path = cache_root.join("strata").join("classic-stratigraphy.txt");
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent).map_err(|error| {
                        format!(
                            "testingcheats classic stratigraphy could not create {}: {error}",
                            parent.display()
                        )
                    })?;
                }
                std::fs::write(&path, report.as_bytes()).map_err(|error| {
                    format!(
                        "testingcheats classic stratigraphy could not write {}: {error}",
                        path.display()
                    )
                })?;
                Ok(format!(
                    "Classic stratigraphy report written to {} (read-only distribution diagnostic; physics and testing sediment unchanged)",
                    path.display()
                ))
            }
            #[cfg(debug_assertions)]
            CommandIntent::TestingCheatsClassicRainMetrics => {
                let Some(testing) = self.testing_cheats.as_ref() else {
                    return Err(
                        "testingcheats classic rainmetrics requires an active classic/hybrid testing sandbox"
                            .to_string(),
                    );
                };
                let categories = self.time_tracker.categories_ordered();
                let report = testing
                    .engine
                    .classic_rain_morphology_diagnostics_report(&categories)?;
                let cache_root = std::env::var_os("XDG_CACHE_HOME")
                    .map(std::path::PathBuf::from)
                    .or_else(|| {
                        std::env::var_os("HOME")
                            .map(std::path::PathBuf::from)
                            .map(|home| home.join(".cache"))
                    })
                    .ok_or_else(|| {
                        "testingcheats classic rainmetrics could not resolve a cache directory"
                            .to_string()
                    })?;
                let path = cache_root.join("strata").join("classic-rain-metrics.txt");
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent).map_err(|error| {
                        format!(
                            "testingcheats classic rainmetrics could not create {}: {error}",
                            parent.display()
                        )
                    })?;
                }
                std::fs::write(&path, report.as_bytes()).map_err(|error| {
                    format!(
                        "testingcheats classic rainmetrics could not write {}: {error}",
                        path.display()
                    )
                })?;
                Ok(format!(
                    "Classic rain morphology diagnostics written to {} (measurement-only; RAIN-004 behavior unchanged)",
                    path.display()
                ))
            }
            #[cfg(debug_assertions)]
            CommandIntent::TestingCheatsFill => {
                self.ensure_testing_cheats_preview()?;
                let category_ids = self.ensure_testing_fill_categories()?;
                let testing = self.testing_cheats.as_mut().expect("testing preview exists");
                let model = testing.engine.model_name();
                let grains = testing.engine.fill_rainbow_80(&category_ids)?;
                testing.spawn_accumulator = std::time::Duration::ZERO;
                testing.physics_accumulator = std::time::Duration::ZERO;
                testing.queued_speed_simulated = std::time::Duration::ZERO;
                testing.queued_explicit_simulated = std::time::Duration::ZERO;
                testing.flow_wall_accumulator = std::time::Duration::ZERO;
                testing.visual_dirty = false;
                self.render_needed = true;
                Ok(format!(
                    "Testing sandbox {model} rainbow-filled to 80% of the visible window using six idempotent Fixture categories ({grains} grains; testing sediment isolated, category catalog persisted)"
                ))
            }
            #[cfg(debug_assertions)]
            CommandIntent::TestingCheatsFillHalf => {
                self.ensure_testing_cheats_preview()?;
                let category_ids = self.ensure_testing_fill_categories()?;
                let testing = self.testing_cheats.as_mut().expect("testing preview exists");
                let model = testing.engine.model_name();
                let grains = testing.engine.fill_rainbow_80_centered_half(&category_ids)?;
                testing.spawn_accumulator = std::time::Duration::ZERO;
                testing.physics_accumulator = std::time::Duration::ZERO;
                testing.queued_speed_simulated = std::time::Duration::ZERO;
                testing.queued_explicit_simulated = std::time::Duration::ZERO;
                testing.flow_wall_accumulator = std::time::Duration::ZERO;
                testing.visual_dirty = false;
                self.render_needed = true;
                Ok(format!(
                    "Testing sandbox {model} rainbow-filled to 80% visible height across the centered half-width using six idempotent Fixture categories ({grains} grains; testing sediment isolated, category catalog persisted)"
                ))
            }
            #[cfg(debug_assertions)]
            CommandIntent::TestingCheatsClear => {
                self.ensure_testing_cheats_preview()?;
                let testing = self.testing_cheats.as_mut().expect("testing preview exists");
                let model = testing.engine.model_name();
                testing.engine.clear();
                testing.spawn_accumulator = std::time::Duration::ZERO;
                testing.physics_accumulator = std::time::Duration::ZERO;
                testing.queued_speed_simulated = std::time::Duration::ZERO;
                testing.queued_explicit_simulated = std::time::Duration::ZERO;
                testing.flow_wall_accumulator = std::time::Duration::ZERO;
                testing.visual_dirty = false;
                self.render_needed = true;
                Ok(format!(
                    "Testing sandbox {model} cleared (authoritative sediment unchanged)"
                ))
            }
            #[cfg(debug_assertions)]
            CommandIntent::TestingCheatsStatus => {
                if let Some(testing) = self.testing_cheats.as_ref() {
                    Ok(format!(
                        "Testing sandbox: model={}, speed={}x, grains={} · {} · queued={} (speed={} advance={}) · reset returns to authoritative sediment",
                        testing.engine.model_name(),
                        testing.speed_multiplier,
                        testing.engine.grain_count(),
                        testing.engine.detail_status(),
                        command::format_hms(testing.total_queued_simulated().as_secs() as usize),
                        command::format_hms(testing.queued_speed_simulated.as_secs() as usize),
                        command::format_hms(testing.queued_explicit_simulated.as_secs() as usize)
                    ))
                } else {
                    Ok("Testing sandbox inactive; authoritative live sediment is displayed".to_string())
                }
            }
            #[cfg(debug_assertions)]
            CommandIntent::TestingCheatsProvenance => {
                let Some(testing) = self.testing_cheats.as_ref() else {
                    return Err("testingcheats provenance requires an active testing sandbox".to_string());
                };
                let report = testing.engine.live_rainbow_provenance_report()?;
                let cache_root = std::env::var_os("XDG_CACHE_HOME")
                    .map(std::path::PathBuf::from)
                    .or_else(|| {
                        std::env::var_os("HOME")
                            .map(std::path::PathBuf::from)
                            .map(|home| home.join(".cache"))
                    })
                    .ok_or_else(|| "testingcheats provenance could not resolve a cache directory".to_string())?;
                let path = cache_root.join("strata").join("rainbow-provenance.txt");
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent).map_err(|error| {
                        format!("testingcheats provenance could not create {}: {error}", parent.display())
                    })?;
                }
                std::fs::write(&path, report.as_bytes()).map_err(|error| {
                    format!("testingcheats provenance could not write {}: {error}", path.display())
                })?;
                Ok(format!(
                    "Rainbow provenance written to {} (read-only diagnostic; testing sediment unchanged)",
                    path.display()
                ))
            }
            #[cfg(debug_assertions)]
            CommandIntent::TestingCheatsReset => {
                self.testing_cheats = None;
                self.render_needed = true;
                Ok("Testing sandbox reset; authoritative live sediment restored".to_string())
            }
        }
    }

    fn command_status(&self) -> Result<String, String> {
        let active_id = self.time_tracker.active_category_id();
        let category_name = self
            .time_tracker
            .category_by_id(active_id)
            .map(|category| self.display_layer_name(&category.name))
            .unwrap_or_else(|| "Idle".to_string());
        let elapsed = self
            .time_tracker
            .current_session_start
            .map(|start| start.elapsed().as_secs() as usize)
            .unwrap_or(0);
        if is_drift_category_id(active_id) {
            return Ok(format!("Status: idle for {}", command::format_hms(elapsed)));
        }
        let description = self.time_tracker.active_description().trim();
        let started = self
            .session
            .active_session_started_at_utc
            .map(|started| {
                started
                    .with_timezone(&Local)
                    .format("%Y-%m-%d %H:%M:%S")
                    .to_string()
            })
            .unwrap_or_else(|| "unknown".to_string());
        if description.is_empty() {
            Ok(format!(
                "Status: active layer '{}' since {} (elapsed {})",
                category_name,
                started,
                command::format_hms(elapsed)
            ))
        } else {
            Ok(format!(
                "Status: active layer '{}' tag '{}' since {} (elapsed {})",
                category_name,
                description,
                started,
                command::format_hms(elapsed)
            ))
        }
    }

    fn command_start(&mut self, layer: String, tag: Option<String>) -> Result<String, String> {
        let Some(category) = self.resolve_layer_case_insensitive(&layer) else {
            let suggestions = self.layer_suggestions(&layer);
            return if suggestions.is_empty() {
                Err(format!("Layer '{layer}' not found"))
            } else {
                Err(format!(
                    "Layer '{layer}' not found. Did you mean: {}",
                    suggestions.join(", ")
                ))
            };
        };
        let canonical_tag = tag
            .as_deref()
            .map(|value| self.canonicalize_tag_for_layer(category.id, value))
            .filter(|value| !value.is_empty());
        if let Some(value) = canonical_tag.as_deref() {
            self.remember_tag_for_layer(category.id, value);
        }
        let description = canonical_tag.clone().unwrap_or_default();
        if self.time_tracker.active_category_id() == category.id {
            if !description.is_empty() {
                let database_path = self
                    .sqlite_database_path
                    .clone()
                    .ok_or_else(|| "SQLite authority is unavailable".to_string())?;
                let stable_id = self
                    .session
                    .active_session_stable_id
                    .clone()
                    .ok_or_else(|| "active session has no stable identity".to_string())?;
                sqlite::update_tui_active_description(&database_path, &stable_id, &description)?;
                self.time_tracker
                    .set_active_description(description.clone());
                self.refresh_active_runtime_checkpoint();
            }
        } else {
            self.apply_runtime_mutation(RuntimeMutation::SwitchLayer {
                category_id: category.id,
                description,
            });
        }
        let display_name = self.display_layer_name(&category.name);
        Ok(match canonical_tag {
            Some(tag) => format!("Started layer '{display_name}' with tag '{tag}'"),
            None => format!("Started layer '{display_name}'"),
        })
    }

    fn command_stop(
        &mut self,
        layer_filter: Option<String>,
        tag_filter: Option<String>,
    ) -> Result<String, String> {
        let active_id = self.time_tracker.active_category_id();
        if is_drift_category_id(active_id) {
            return Err("No active layer session to stop (already idle)".to_string());
        }
        if let Some(layer) = layer_filter {
            let Some(expected) = self.resolve_layer_case_insensitive(&layer) else {
                return Err(format!("Layer '{layer}' not found"));
            };
            if expected.id != active_id {
                let active_name = self
                    .time_tracker
                    .category_by_id(active_id)
                    .map(|category| self.display_layer_name(&category.name))
                    .unwrap_or_else(|| "Idle".to_string());
                return Err(format!(
                    "Active layer is '{}' (not '{}')",
                    active_name,
                    self.display_layer_name(&expected.name)
                ));
            }
        }
        if let Some(tag) = tag_filter {
            let canonical_tag = self.canonicalize_tag_for_layer(active_id, &tag);
            if canonical_tag.is_empty() {
                return Err("Tag filter must contain at least one non-empty tag".to_string());
            }
            let active_tag = self.time_tracker.active_description();
            if !super::tagging::contains_all_tags(active_tag, &canonical_tag) {
                return Err(format!(
                    "Active tags are '{}' (do not contain '{}')",
                    active_tag, canonical_tag
                ));
            }
        }
        let active_name = self
            .time_tracker
            .category_by_id(active_id)
            .map(|category| self.display_layer_name(&category.name))
            .unwrap_or_else(|| "Idle".to_string());
        self.apply_runtime_mutation(RuntimeMutation::SwitchLayer {
            category_id: DRIFT_CATEGORY_ID,
            description: String::new(),
        });
        Ok(format!("Stopped layer '{active_name}'"))
    }

    fn command_balance(
        &self,
        selector: command::BalanceSelector,
        layer_filter: Option<String>,
        tag_filter: Option<String>,
    ) -> Result<String, String> {
        let window = command::resolve_balance_window(
            &selector,
            crate::domain::operational_day_key_now(),
            self.runtime_settings.first_day_of_week,
        )?;
        let categories = self.time_tracker.categories_ordered();
        let layer = if let Some(layer) = layer_filter {
            Some(
                self.resolve_layer_case_insensitive(&layer)
                    .ok_or_else(|| format!("Layer '{layer}' not found"))?,
            )
        } else {
            None
        };
        let layer_id = layer.as_ref().map(|category| category.id);
        let canonical_tag = match tag_filter.as_deref().map(str::trim) {
            Some(value) if !value.is_empty() => {
                let canonical = layer
                    .as_ref()
                    .map(|category| self.canonicalize_tag_for_layer(category.id, value))
                    .unwrap_or_else(|| super::tagging::parse_tags(value).join("; "));
                if canonical.is_empty() {
                    return Err("Tag filter must contain at least one non-empty tag".to_string());
                }
                Some(canonical)
            }
            _ => None,
        };
        let mut total_elapsed = 0usize;
        let mut total_balance = 0isize;
        for session in &self.time_tracker.sessions {
            let Ok(day) = NaiveDate::parse_from_str(&session.date, "%Y-%m-%d") else {
                continue;
            };
            if day < window.start || day > window.end {
                continue;
            }
            if layer_id.is_some_and(|expected| session.category_id != expected) {
                continue;
            }
            if canonical_tag
                .as_ref()
                .is_some_and(|tag| !super::tagging::contains_all_tags(&session.description, tag))
            {
                continue;
            }
            let effect = categories
                .iter()
                .find(|category| category.id == session.category_id)
                .map(|category| {
                    if is_drift_category_id(category.id) {
                        0
                    } else {
                        category.balance_effect
                    }
                })
                .unwrap_or(0);
            total_elapsed = total_elapsed.saturating_add(session.elapsed_seconds);
            total_balance = total_balance
                .saturating_add((session.elapsed_seconds as isize).saturating_mul(effect as isize));
        }
        if let Some(start) = self.time_tracker.current_session_start {
            let active_id = self.time_tracker.active_category_id();
            let live_day = crate::domain::operational_day_key_now();
            let layer_matches = layer_id.is_none_or(|expected| expected == active_id);
            let tag_matches = canonical_tag.as_ref().is_none_or(|tag| {
                super::tagging::contains_all_tags(self.time_tracker.active_description(), tag)
            });
            if live_day >= window.start && live_day <= window.end && layer_matches && tag_matches {
                let elapsed = start.elapsed().as_secs() as usize;
                let effect = categories
                    .iter()
                    .find(|category| category.id == active_id)
                    .map(|category| {
                        if is_drift_category_id(category.id) {
                            0
                        } else {
                            category.balance_effect
                        }
                    })
                    .unwrap_or(0);
                total_elapsed = total_elapsed.saturating_add(elapsed);
                total_balance = total_balance
                    .saturating_add((elapsed as isize).saturating_mul(effect as isize));
            }
        }
        let mut scope = String::new();
        if let Some(category) = layer {
            scope.push_str(&format!(
                " layer '{}'",
                self.display_layer_name(&category.name)
            ));
        }
        if let Some(tag) = canonical_tag {
            scope.push_str(&format!(" tag '{tag}'"));
        }
        Ok(format!(
            "Balance {}{}: {} (elapsed {})",
            window.label,
            scope,
            command::format_signed_hms(total_balance),
            command::format_hms(total_elapsed)
        ))
    }

    fn command_delete_last_session(
        &mut self,
        layer: String,
        tag: Option<String>,
    ) -> Result<String, String> {
        let category = self
            .resolve_layer_case_insensitive(&layer)
            .ok_or_else(|| format!("Layer '{layer}' not found"))?;
        let canonical_tag = match tag.as_deref().map(str::trim) {
            Some(value) if !value.is_empty() => {
                let canonical = self.canonicalize_tag_for_layer(category.id, value);
                if canonical.is_empty() {
                    return Err("Tag filter must contain at least one non-empty tag".to_string());
                }
                Some(canonical)
            }
            _ => None,
        };
        let session_id = self
            .time_tracker
            .sessions
            .iter()
            .filter(|session| session.category_id == category.id)
            .filter(|session| {
                canonical_tag
                    .as_ref()
                    .is_none_or(|tag| super::tagging::contains_all_tags(&session.description, tag))
            })
            .max_by_key(|session| session.id)
            .map(|session| session.id)
            .ok_or_else(|| "No matching session found".to_string())?;
        let database_path = self
            .sqlite_database_path
            .clone()
            .ok_or_else(|| "SQLite authority is unavailable".to_string())?;
        sqlite::delete_tui_session(&database_path, session_id)?;
        self.reload_sqlite_sessions();
        Ok(format!(
            "Deleted last session for layer '{}'",
            self.display_layer_name(&category.name)
        ))
    }

    fn execute_palette_command(&mut self, command: PaletteCommand) -> bool {
        self.close_command_palette();

        match command {
            PaletteCommand::Action(Action::ToggleCommandPalette) => false,
            PaletteCommand::Action(Action::ToggleSettings) => {
                self.toggle_settings();
                false
            }
            PaletteCommand::Action(Action::ReportRangeStart) => {
                if !self.in_balance_modal() {
                    self.open_report_modal();
                } else {
                    self.ledger_entry_edit = None;
                }
                self.select_report_range_boundary(ReportRangeBoundary::Start);
                false
            }
            PaletteCommand::Action(Action::ReportRangeEnd) => {
                if !self.in_balance_modal() {
                    self.open_report_modal();
                } else {
                    self.ledger_entry_edit = None;
                }
                self.select_report_range_boundary(ReportRangeBoundary::End);
                false
            }
            PaletteCommand::Action(Action::ReportRange) => {
                if !self.in_balance_modal() {
                    self.open_report_modal();
                }
                self.begin_report_range_edit();
                false
            }
            PaletteCommand::Action(Action::LogActivity) => {
                if !self.in_balance_modal() {
                    self.open_report_modal();
                }
                self.begin_selected_layer_ledger_add();
                false
            }
            PaletteCommand::Action(action) => self.handle_main_action(action),
            PaletteCommand::SetReportPeriod(period) => {
                if !self.in_balance_modal() {
                    self.open_report_modal();
                }
                self.set_report_period(period);
                self.render_needed = true;
                false
            }
            PaletteCommand::SwitchLayer(category_id) => {
                self.switch_to_layer_from_palette(category_id);
                false
            }
        }
    }

    fn switch_to_layer_from_palette(&mut self, category_id: CategoryId) {
        self.apply_runtime_mutation(RuntimeMutation::SwitchLayer {
            category_id,
            description: String::new(),
        });
    }

    fn handle_settings_action(&mut self, action: Action) -> bool {
        match action {
            Action::Cancel => self.close_settings(),
            Action::Up | Action::Left => self.select_previous_settings_item(),
            Action::Down | Action::Right => self.select_next_settings_item(),
            Action::Confirm => self.open_settings_editor_for_selection(),
            Action::SettingsTop => self.jump_settings_top(),
            Action::SettingsBottom => self.jump_settings_bottom(),
            Action::Quit => return true,
            _ => {}
        }

        false
    }

    fn handle_settings_overlay_key(&mut self, key: KeyEvent) -> bool {
        let Some(overlay) = self.settings_overlay.clone() else {
            return false;
        };

        match overlay {
            super::SettingsOverlay::CaptureKey { action } => {
                self.handle_settings_capture_key_input(action, key);
            }
            super::SettingsOverlay::SelectTheme { .. } => {
                self.handle_settings_theme_dropdown(key);
            }
            super::SettingsOverlay::SelectWeekStartDay { .. } => {
                self.handle_settings_week_start_dropdown(key);
            }
        }

        false
    }

    fn handle_settings_capture_key_input(&mut self, action: Action, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.close_settings_overlay();
            }
            KeyCode::Backspace => {
                let keymap_path = crate::storage::get_keymap_path();
                match crate::keybindings::set_action_binding(&keymap_path, action, None) {
                    Ok(loaded) => {
                        self.apply_loaded_keybindings(loaded);
                        self.close_settings_overlay();
                    }
                    Err(err) => {
                        self.keymap_error = Some(err);
                        self.close_settings_overlay();
                    }
                }
            }
            KeyCode::Delete => {
                let keymap_path = crate::storage::get_keymap_path();
                match crate::keybindings::set_action_unbound(&keymap_path, action) {
                    Ok(loaded) => {
                        self.apply_loaded_keybindings(loaded);
                        self.close_settings_overlay();
                    }
                    Err(err) => {
                        self.keymap_error = Some(err);
                        self.close_settings_overlay();
                    }
                }
            }
            _ => {
                if let Some(binding) = crate::keybindings::KeyBinding::from_key_event(key) {
                    let keymap_path = crate::storage::get_keymap_path();
                    match crate::keybindings::set_action_binding(
                        &keymap_path,
                        action,
                        Some(binding),
                    ) {
                        Ok(loaded) => {
                            self.apply_loaded_keybindings(loaded);
                            self.close_settings_overlay();
                        }
                        Err(err) => {
                            self.keymap_error = Some(err);
                            self.close_settings_overlay();
                        }
                    }
                }
            }
        }
    }

    fn handle_settings_theme_dropdown(&mut self, key: KeyEvent) {
        let Some(super::SettingsOverlay::SelectTheme { mut selected }) =
            self.settings_overlay.take()
        else {
            return;
        };

        let themes = self.appearance.theme_descriptors();
        if themes.is_empty() {
            self.close_settings_overlay();
            return;
        }
        match key.code {
            KeyCode::Esc => {
                self.close_settings_overlay();
                return;
            }
            KeyCode::Up | KeyCode::Left => {
                selected = if selected == 0 {
                    themes.len() - 1
                } else {
                    selected - 1
                };
            }
            KeyCode::Down | KeyCode::Right => {
                selected = (selected + 1) % themes.len();
            }
            KeyCode::Enter => {
                let id = themes
                    .get(selected)
                    .map(|theme| theme.id.clone())
                    .unwrap_or_else(|| themes[0].id.clone());
                match self.appearance.select_theme(&id) {
                    Ok(()) => {
                        self.close_settings_overlay();
                        self.render_needed = true;
                    }
                    Err(error) => {
                        self.keymap_error = Some(format!("appearance: {error}"));
                        self.close_settings_overlay();
                    }
                }
                return;
            }
            _ => {}
        }

        self.settings_overlay = Some(super::SettingsOverlay::SelectTheme { selected });
        self.render_needed = true;
    }

    fn handle_settings_week_start_dropdown(&mut self, key: KeyEvent) {
        let Some(super::SettingsOverlay::SelectWeekStartDay { mut selected }) =
            self.settings_overlay.take()
        else {
            return;
        };

        let options = Self::week_start_options();
        match key.code {
            KeyCode::Esc => {
                self.close_settings_overlay();
                return;
            }
            KeyCode::Up | KeyCode::Left => {
                selected = if selected == 0 {
                    options.len().saturating_sub(1)
                } else {
                    selected - 1
                };
            }
            KeyCode::Down | KeyCode::Right => {
                selected = (selected + 1) % options.len().max(1);
            }
            KeyCode::Enter => {
                let week_start = options.get(selected).copied().unwrap_or(options[0]);
                let keymap_path = crate::storage::get_keymap_path();
                match crate::keybindings::set_first_day_of_week(&keymap_path, week_start) {
                    Ok(loaded) => {
                        self.apply_loaded_keybindings(loaded);
                        self.close_settings_overlay();
                    }
                    Err(err) => {
                        self.keymap_error = Some(err);
                        self.close_settings_overlay();
                    }
                }
                return;
            }
            _ => {}
        }

        self.settings_overlay = Some(super::SettingsOverlay::SelectWeekStartDay { selected });
        self.render_needed = true;
    }

    fn handle_modal_action(&mut self, action: Action) -> bool {
        if self.modal_renaming_category {
            match action {
                Action::Cancel => {
                    self.leave_category_rename();
                }
                Action::Confirm => {
                    self.commit_category_rename();
                }
                Action::RenameCategory => {}
                _ => {}
            }
            self.render_needed = true;
            return true;
        }

        let mut handled = true;

        match action {
            Action::Cancel => self.cancel_modal(),
            Action::Up => {
                let total_rows = self.time_tracker.category_count() + 1;
                if total_rows > 0 {
                    self.selected_index =
                        ui_helpers::wrap_prev_index(self.selected_index, total_rows);
                    self.sync_modal_description_from_selection();
                }
            }
            Action::Down => {
                let total_rows = self.time_tracker.category_count() + 1;
                if total_rows > 0 {
                    self.selected_index =
                        ui_helpers::wrap_next_index(self.selected_index, total_rows);
                    self.sync_modal_description_from_selection();
                }
            }
            Action::Left => {
                if self.is_on_insert_space() {
                    let count = self.appearance.sand_color_count();
                    self.new_category_color_cursor =
                        (self.new_category_color_cursor + count - 1) % count;
                } else {
                    self.cycle_selected_tag(-1);
                }
            }
            Action::Right => {
                if self.is_on_insert_space() {
                    let count = self.appearance.sand_color_count();
                    self.new_category_color_cursor = (self.new_category_color_cursor + 1) % count;
                } else {
                    self.cycle_selected_tag(1);
                }
            }
            Action::MoveLayerUp => {
                if !self.is_on_insert_space()
                    && self.time_tracker.move_category_up(self.selected_index)
                {
                    self.selected_index = self.selected_index.saturating_sub(1);
                    self.persist_categories();
                    if self.has_persistence_recovery() {
                        return true;
                    }
                }
            }
            Action::MoveLayerDown => {
                if !self.is_on_insert_space()
                    && self.time_tracker.move_category_down(self.selected_index)
                {
                    self.selected_index += 1;
                    self.persist_categories();
                    if self.has_persistence_recovery() {
                        return true;
                    }
                }
            }
            Action::PreviousLayerColor | Action::NextLayerColor => {
                if !self.is_on_insert_space() && self.selected_index > 0 {
                    let Some(current_color) = self
                        .time_tracker
                        .category_by_index(self.selected_index)
                        .map(|category| category.color)
                    else {
                        self.render_needed = true;
                        return true;
                    };
                    let direction = if matches!(action, Action::PreviousLayerColor) {
                        -1
                    } else {
                        1
                    };
                    let new_color = self
                        .appearance
                        .cycle_category_anchor(current_color, direction);
                    if self
                        .time_tracker
                        .set_category_color_by_index(self.selected_index, new_color)
                    {
                        self.persist_categories();
                        // Historical sediment keeps Layer identity, not frozen RGB.
                        // Invalidate any presentation cache so the new color resolves
                        // retroactively anywhere that Layer appears.
                        self.clear_report_snapshot_cache();
                    }
                }
            }
            Action::ShiftLeft | Action::ShiftRight => {}
            Action::Confirm => {
                if self.is_on_insert_space() {
                    if !self.new_category_name.is_empty() {
                        self.add_category();
                        self.close_modal();
                    }
                } else {
                    self.remember_selected_tag();
                    if self.has_persistence_recovery() {
                        self.render_needed = true;
                        return true;
                    }
                    let selected = self
                        .time_tracker
                        .category_by_index(self.selected_index)
                        .map(|category| category.id);
                    if let Some(category_id) = selected
                        && self.time_tracker.active_category_id() != category_id
                    {
                        if !self.persist_modal_active_description() {
                            self.render_needed = true;
                            return true;
                        }
                        self.apply_runtime_mutation(RuntimeMutation::SwitchLayer {
                            category_id,
                            description: self.modal_description.clone(),
                        });
                    }
                    self.close_modal();
                }
            }
            Action::RenameCategory => {
                self.begin_category_rename();
            }
            Action::DeleteCategory => {
                if !self.is_on_insert_space() && self.selected_index > 0 {
                    self.delete_category();
                }
            }
            Action::IncreaseBalance => {
                if !self.is_on_insert_space()
                    && self.selected_index > 0
                    && self.selected_index < self.time_tracker.category_count()
                    && self
                        .time_tracker
                        .set_category_balance_by_index(self.selected_index, 1)
                {
                    self.persist_categories();
                }
            }
            Action::DecreaseBalance => {
                if !self.is_on_insert_space()
                    && self.selected_index > 0
                    && self.selected_index < self.time_tracker.category_count()
                    && self
                        .time_tracker
                        .set_category_balance_by_index(self.selected_index, -1)
                {
                    self.persist_categories();
                }
            }
            _ => handled = false,
        }

        if handled {
            self.render_needed = true;
        }

        handled
    }

    fn handle_modal_text_input(&mut self, key: KeyEvent) {
        if key
            .modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
        {
            return;
        }

        if let KeyCode::Char(c) = key.code {
            if self.modal_renaming_category {
                self.modal_category_name_draft.push(c);
                self.modal_category_name_error = None;
                self.render_needed = true;
            } else if self.is_on_insert_space() {
                self.new_category_name.push(c);
                self.render_needed = true;
            } else if self.selected_index < self.time_tracker.category_count() {
                self.modal_tag_index = None;
                self.modal_tag_cycle_prefix = None;
                self.modal_tag_text_editing = true;
                self.modal_description.push(c);
                self.preview_active_description_from_modal();
                self.render_needed = true;
            }
        }
    }

    fn handle_modal_text_delete(&mut self) {
        if self.modal_renaming_category {
            self.modal_category_name_draft.pop();
            self.modal_category_name_error = None;
        } else if self.is_on_insert_space() {
            self.new_category_name.pop();
        } else if self.selected_index < self.time_tracker.category_count() {
            self.modal_tag_index = None;
            self.modal_tag_cycle_prefix = None;
            self.modal_tag_text_editing = true;
            self.modal_description.pop();
            self.preview_active_description_from_modal();
        }
        self.render_needed = true;
    }

    fn handle_report_modal_action(&mut self, action: Action) -> bool {
        if self.report_layer_delete_confirmation.is_some() {
            match action {
                Action::Confirm => {
                    self.confirm_report_layer_delete();
                }
                Action::Cancel => self.cancel_report_layer_delete_confirmation(),
                Action::Quit => return true,
                _ => {}
            }
            return false;
        }

        let in_logs_view = self.report_logs_category_id.is_some();
        let summary = if in_logs_view {
            self.report_rows()
        } else {
            self.report_visible_rows()
        };
        self.sync_report_selection_to_summary(&summary);
        let logs = self.report_current_logs();
        let ledger_row_count = if in_logs_view {
            self.report_ledger_row_count()
        } else {
            0
        };
        self.clamp_report_log_selection(ledger_row_count);

        let mut handled = true;

        match action {
            Action::Cancel => {
                if self.report_range_boundary.is_some() {
                    self.cancel_report_range_boundary();
                } else if in_logs_view {
                    if !self.leave_report_filter_focus() && !self.clear_report_tag_filter() {
                        self.ledger_entry_edit = None;
                        self.report_logs_category_id = None;
                        self.report_log_selected_index = 0;
                    }
                } else {
                    self.close_report_modal();
                }
            }
            Action::Confirm => {
                if self.report_range_boundary.is_some() {
                    self.clear_report_range_boundary();
                } else if in_logs_view && self.report_filter_focus_active() {
                    handled = self.toggle_selected_report_filter();
                } else if in_logs_view {
                    handled = self.begin_ledger_entry_edit();
                } else if let Some(entry) = summary.entries.get(self.report_selected_index) {
                    self.clear_report_range_boundary();
                    self.clear_report_tag_filter();
                    self.ledger_entry_edit = None;
                    self.report_logs_category_id = Some(entry.category_id);
                    self.report_log_selected_index = 0;
                }
            }
            Action::Up => {
                if in_logs_view && self.report_filter_focus_active() {
                    // A multi-tag selector owns its row until it is applied or cancelled.
                } else if in_logs_view {
                    if ledger_row_count > 0 {
                        self.report_log_selected_index = ui_helpers::wrap_prev_index(
                            self.report_log_selected_index,
                            ledger_row_count,
                        );
                    }
                } else if !summary.entries.is_empty() {
                    let index = ui_helpers::wrap_prev_index(
                        self.report_selected_index,
                        summary.entries.len(),
                    );
                    self.select_report_summary_index(&summary, index);
                }
            }
            Action::Down => {
                if in_logs_view && self.report_filter_focus_active() {
                    // A multi-tag selector owns its row until it is applied or cancelled.
                } else if in_logs_view {
                    if ledger_row_count > 0 {
                        self.report_log_selected_index = ui_helpers::wrap_next_index(
                            self.report_log_selected_index,
                            ledger_row_count,
                        );
                    }
                } else if !summary.entries.is_empty() {
                    let index = ui_helpers::wrap_next_index(
                        self.report_selected_index,
                        summary.entries.len(),
                    );
                    self.select_report_summary_index(&summary, index);
                }
            }
            Action::Left => {
                if in_logs_view && self.report_filter_focus_active() {
                    handled = self.move_report_filter_focus(-1);
                } else if self.report_range_boundary.is_some() {
                    self.move_report_range_boundary(-1);
                } else {
                    self.shift_report_interval_older();
                }
            }
            Action::Right => {
                if in_logs_view && self.report_filter_focus_active() {
                    handled = self.move_report_filter_focus(1);
                } else if self.report_range_boundary.is_some() {
                    self.move_report_range_boundary(1);
                } else {
                    self.shift_report_interval_newer();
                }
            }
            Action::ShiftLeft => {
                if in_logs_view && self.report_filter_focus_active() {
                    // Shift never escapes the active tag selector into period navigation.
                } else if self.report_range_boundary.is_some() {
                    self.move_report_range_boundary_month(-1);
                } else {
                    handled = false;
                }
            }
            Action::ShiftRight => {
                if in_logs_view && self.report_filter_focus_active() {
                    // Shift never escapes the active tag selector into period navigation.
                } else if self.report_range_boundary.is_some() {
                    self.move_report_range_boundary_month(1);
                } else {
                    handled = false;
                }
            }
            Action::ReportToday => self.set_report_period(ReportPeriod::Today),
            Action::ReportWeek => self.set_report_period(ReportPeriod::Week),
            Action::ReportMonth => self.set_report_period(ReportPeriod::Month),
            Action::ReportRange => self.begin_report_range_edit(),
            Action::ReportRangeStart => {
                self.select_report_range_boundary(ReportRangeBoundary::Start);
            }
            Action::ReportRangeEnd => {
                self.select_report_range_boundary(ReportRangeBoundary::End);
            }
            Action::LogActivity => {
                self.clear_report_range_boundary();
                handled = self.begin_selected_layer_ledger_add();
            }
            Action::ReportFilter => {
                handled = in_logs_view && self.toggle_selected_report_filter();
            }
            Action::DeleteCategory => {
                if in_logs_view && self.report_log_selected_index < logs.len() {
                    handled = self.delete_selected_report_session();
                } else if !in_logs_view {
                    handled = self.begin_report_layer_delete_confirmation(&summary);
                } else {
                    handled = false;
                }
            }
            Action::Quit => return true,
            _ => handled = false,
        }

        if handled {
            self.render_needed = true;
        }
        false
    }

    fn handle_main_action(&mut self, action: Action) -> bool {
        match action {
            Action::Quit => true,
            Action::ClearAllSand => {
                self.apply_runtime_mutation(RuntimeMutation::ClearAllSand);
                false
            }
            Action::ClearNoneSand => {
                self.apply_runtime_mutation(RuntimeMutation::ClearDriftSand);
                false
            }
            Action::OpenReportModal => {
                self.open_report_modal();
                false
            }
            Action::OpenCategoryModal => {
                self.open_modal();
                false
            }
            Action::Confirm => false,
            Action::SwitchToNone => {
                self.apply_runtime_mutation(RuntimeMutation::SwitchLayer {
                    category_id: DRIFT_CATEGORY_ID,
                    description: String::new(),
                });
                false
            }
            Action::Detach => {
                self.detach_requested = true;
                true
            }
            Action::Cancel => false,
            _ => false,
        }
    }

    fn handle_report_range_edit_key(&mut self, key: KeyEvent) -> bool {
        match resolve_report_range_edit_key(key, &self.keymap) {
            ReportRangeEditKeyIntent::Append(character) => {
                if let Some(edit) = self.report_range_edit.as_mut() {
                    edit.append(character);
                    self.render_needed = true;
                }
            }
            ReportRangeEditKeyIntent::Backspace => {
                if let Some(edit) = self.report_range_edit.as_mut() {
                    edit.backspace();
                    self.render_needed = true;
                }
            }
            ReportRangeEditKeyIntent::NextField | ReportRangeEditKeyIntent::PreviousField => {
                if let Some(edit) = self.report_range_edit.as_mut() {
                    edit.switch_field();
                    self.render_needed = true;
                }
            }
            ReportRangeEditKeyIntent::Commit => {
                self.commit_report_range_edit();
            }
            ReportRangeEditKeyIntent::Cancel => {
                self.cancel_report_range_edit();
            }
            ReportRangeEditKeyIntent::EmergencyQuit => return true,
            ReportRangeEditKeyIntent::Ignore => {}
        }
        false
    }

    fn handle_ledger_entry_edit_key(&mut self, key: KeyEvent) -> bool {
        let intent = resolve_ledger_entry_edit_key(key, &self.keymap);
        if self
            .ledger_entry_edit
            .as_ref()
            .is_some_and(|edit| edit.confirmation.is_some())
        {
            match intent {
                LedgerEntryEditKeyIntent::Commit => {
                    self.commit_ledger_entry_edit();
                }
                LedgerEntryEditKeyIntent::Cancel => {
                    self.dismiss_ledger_entry_confirmation();
                }
                LedgerEntryEditKeyIntent::EmergencyQuit => return true,
                _ => {}
            }
            return false;
        }

        match intent {
            LedgerEntryEditKeyIntent::Append(character) => {
                if let Some(edit) = self.ledger_entry_edit.as_mut() {
                    edit.append(character);
                    self.render_needed = true;
                }
            }
            LedgerEntryEditKeyIntent::Backspace => {
                if let Some(edit) = self.ledger_entry_edit.as_mut() {
                    edit.backspace();
                    self.render_needed = true;
                }
            }
            LedgerEntryEditKeyIntent::NextField => {
                if let Some(edit) = self.ledger_entry_edit.as_mut() {
                    edit.next_field();
                    self.render_needed = true;
                }
            }
            LedgerEntryEditKeyIntent::PreviousField => {
                if let Some(edit) = self.ledger_entry_edit.as_mut() {
                    edit.previous_field();
                    self.render_needed = true;
                }
            }
            LedgerEntryEditKeyIntent::Left | LedgerEntryEditKeyIntent::Right => {
                let direction: i64 = if matches!(intent, LedgerEntryEditKeyIntent::Left) {
                    -1
                } else {
                    1
                };
                let description_active = self
                    .ledger_entry_edit
                    .as_ref()
                    .is_some_and(|edit| edit.active_field == super::LedgerEntryField::Description);
                if description_active {
                    self.cycle_ledger_tag(direction as isize);
                } else if let Some(edit) = self.ledger_entry_edit.as_mut()
                    && edit.adjust_active_temporal(direction, false)
                {
                    self.render_needed = true;
                }
            }
            LedgerEntryEditKeyIntent::ShiftLeft | LedgerEntryEditKeyIntent::ShiftRight => {
                let direction: i64 = if matches!(intent, LedgerEntryEditKeyIntent::ShiftLeft) {
                    -1
                } else {
                    1
                };
                let description_active = self
                    .ledger_entry_edit
                    .as_ref()
                    .is_some_and(|edit| edit.active_field == super::LedgerEntryField::Description);
                if !description_active
                    && let Some(edit) = self.ledger_entry_edit.as_mut()
                    && edit.adjust_active_temporal(direction, true)
                {
                    self.render_needed = true;
                }
            }
            LedgerEntryEditKeyIntent::Commit => {
                self.commit_ledger_entry_edit();
            }
            LedgerEntryEditKeyIntent::Cancel => {
                self.cancel_ledger_entry_edit();
            }
            LedgerEntryEditKeyIntent::EmergencyQuit => return true,
            LedgerEntryEditKeyIntent::Ignore => {}
        }
        false
    }
}

#[cfg(test)]
mod report_edit_tests {
    use super::{
        LedgerEntryEditKeyIntent, ReportRangeEditKeyIntent, direct_command_or_fuzzy_fallback,
        resolve_ledger_entry_edit_key, resolve_report_range_edit_key,
    };
    use crate::keybindings::default_keymap;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    #[test]
    fn range_edit_accepts_iso_date_input_and_owns_plain_range_keys() {
        let keymap = default_keymap();
        for character in ['2', '0', '2', '6', '-', '0', '8'] {
            assert_eq!(
                resolve_report_range_edit_key(
                    KeyEvent::new(KeyCode::Char(character), KeyModifiers::NONE),
                    &keymap,
                ),
                ReportRangeEditKeyIntent::Append(character)
            );
        }
        assert_eq!(
            resolve_report_range_edit_key(
                KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE),
                &keymap,
            ),
            ReportRangeEditKeyIntent::Ignore
        );
        assert_eq!(
            resolve_report_range_edit_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE), &keymap),
            ReportRangeEditKeyIntent::NextField
        );
        assert_eq!(
            resolve_report_range_edit_key(
                KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
                &keymap
            ),
            ReportRangeEditKeyIntent::Commit
        );
    }

    #[test]
    fn plain_command_letters_are_text_only_in_edit_mode() {
        let keymap = default_keymap();
        for character in ['q', 'w', 'm', 't', 'f', 'k', 'd', 'x'] {
            assert_eq!(
                resolve_ledger_entry_edit_key(
                    KeyEvent::new(KeyCode::Char(character), KeyModifiers::NONE),
                    &keymap,
                ),
                LedgerEntryEditKeyIntent::Append(character)
            );
        }
    }

    #[test]
    fn unicode_and_spaces_are_supported() {
        let keymap = default_keymap();
        assert_eq!(
            resolve_ledger_entry_edit_key(
                KeyEvent::new(KeyCode::Char('界'), KeyModifiers::NONE),
                &keymap,
            ),
            LedgerEntryEditKeyIntent::Append('界')
        );
        assert_eq!(
            resolve_ledger_entry_edit_key(
                KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE),
                &keymap,
            ),
            LedgerEntryEditKeyIntent::Append(' ')
        );
    }

    #[test]
    fn enter_commits_and_escape_cancels() {
        let keymap = default_keymap();
        assert_eq!(
            resolve_ledger_entry_edit_key(
                KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
                &keymap,
            ),
            LedgerEntryEditKeyIntent::Commit
        );
        assert_eq!(
            resolve_ledger_entry_edit_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE), &keymap,),
            LedgerEntryEditKeyIntent::Cancel
        );
    }

    #[test]
    fn configured_modified_quit_is_the_only_emergency_action() {
        let keymap = default_keymap();
        assert_eq!(
            resolve_ledger_entry_edit_key(
                KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
                &keymap,
            ),
            LedgerEntryEditKeyIntent::EmergencyQuit
        );
        assert_eq!(
            resolve_ledger_entry_edit_key(
                KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL),
                &keymap,
            ),
            LedgerEntryEditKeyIntent::Ignore
        );
    }
    #[test]
    fn ledger_entry_editor_owns_tab_navigation() {
        let keymap = default_keymap();
        assert_eq!(
            resolve_ledger_entry_edit_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE), &keymap,),
            LedgerEntryEditKeyIntent::NextField
        );
        assert_eq!(
            resolve_ledger_entry_edit_key(
                KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT),
                &keymap,
            ),
            LedgerEntryEditKeyIntent::PreviousField
        );
    }

    #[test]
    fn ledger_entry_editor_owns_temporal_arrow_adjustments() {
        let keymap = default_keymap();
        assert_eq!(
            resolve_ledger_entry_edit_key(
                KeyEvent::new(KeyCode::Left, KeyModifiers::NONE),
                &keymap,
            ),
            LedgerEntryEditKeyIntent::Left
        );
        assert_eq!(
            resolve_ledger_entry_edit_key(
                KeyEvent::new(KeyCode::Right, KeyModifiers::SHIFT),
                &keymap,
            ),
            LedgerEntryEditKeyIntent::ShiftRight
        );
        assert_eq!(
            resolve_ledger_entry_edit_key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE), &keymap,),
            LedgerEntryEditKeyIntent::Ignore
        );
    }

    #[test]
    fn fuzzy_palette_query_falls_back_when_it_is_not_a_direct_command() {
        assert_eq!(
            direct_command_or_fuzzy_fallback("report", true).unwrap(),
            None
        );
    }

    #[test]
    fn valid_direct_command_wins_over_fuzzy_results() {
        let resolved = direct_command_or_fuzzy_fallback("status", true)
            .unwrap()
            .expect("status should resolve as a direct command");
        assert_eq!(resolved, crate::command::CommandIntent::Status);
    }
}

#[cfg(test)]
mod testing_fill_category_tests {
    use super::{TESTING_FILL_CATEGORY_SPECS, ensure_testing_fill_categories_in_tracker};
    use crate::{constants::COLORS, domain::TimeTracker};

    #[test]
    fn testing_fill_categories_are_exact_and_idempotent() {
        let mut tracker = TimeTracker::new();
        let (first_ids, created) = ensure_testing_fill_categories_in_tracker(&mut tracker).unwrap();
        assert!(created);
        assert_eq!(first_ids.len(), TESTING_FILL_CATEGORY_SPECS.len());

        for ((name, new_category_color_cursor), id) in
            TESTING_FILL_CATEGORY_SPECS.into_iter().zip(&first_ids)
        {
            let category = tracker.category_by_id(*id).unwrap();
            assert_eq!(category.name, name);
            assert_eq!(category.color, COLORS[new_category_color_cursor]);
        }

        let count_after_first = tracker.categories_ordered().len();
        let (second_ids, created_again) =
            ensure_testing_fill_categories_in_tracker(&mut tracker).unwrap();
        assert!(!created_again);
        assert_eq!(second_ids, first_ids);
        assert_eq!(tracker.categories_ordered().len(), count_after_first);
    }
}
