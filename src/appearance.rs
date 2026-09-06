use std::{
    fs,
    path::{Path, PathBuf},
};

use ratatui::style::Color;
use serde::{Deserialize, Serialize};

use crate::{constants::COLORS, storage};

mod color;
mod theme;

pub(crate) use color::nearest_legacy_color_index;
pub(crate) use theme::{ThemeAppearance, ThemeDescriptor, UiColorRef};
use theme::Theme;

const APPEARANCE_SCHEMA_VERSION: u8 = 1;
const COLOR_ANCHOR_TAG: i64 = 1_i64 << 62;
const COLOR_ANCHOR_MASK: i64 = 0x00ff_ffff;
pub(super) const BUILTIN_THEME_ID: &str = "default";
pub(super) const BUILTIN_THEME_SOURCE: &str = include_str!("../themes/strata-default.toml");

#[derive(Clone, Debug, Serialize, Deserialize)]
struct AppearanceConfig {
    schema: u8,
    theme: String,
}

impl Default for AppearanceConfig {
    fn default() -> Self {
        Self {
            schema: APPEARANCE_SCHEMA_VERSION,
            theme: BUILTIN_THEME_ID.to_string(),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct AppearanceState {
    config_path: PathBuf,
    config: AppearanceConfig,
    themes: Vec<Theme>,
    active_index: usize,
}

impl AppearanceState {
    pub(crate) fn load(ignore_config: bool) -> Result<Self, String> {
        Self::load_from_paths(
            ignore_config,
            storage::get_appearance_path(),
            &storage::get_themes_dir(),
        )
    }

    fn load_from_paths(
        ignore_config: bool,
        config_path: PathBuf,
        themes_directory: &Path,
    ) -> Result<Self, String> {
        let config = if ignore_config {
            AppearanceConfig::default()
        } else {
            load_config(&config_path)?
        };
        let themes = if ignore_config {
            vec![Theme::parse(BUILTIN_THEME_ID, BUILTIN_THEME_SOURCE)?]
        } else {
            theme::load_themes_from(themes_directory)?
        };
        let active_index = themes
            .iter()
            .position(|theme| theme.id == config.theme)
            .ok_or_else(|| {
                format!(
                    "Theme '{}' is not installed. Put {}.toml in {} or choose another theme in {}",
                    config.theme,
                    config.theme,
                    themes_directory.display(),
                    config_path.display()
                )
            })?;
        Ok(Self {
            config_path,
            config,
            themes,
            active_index,
        })
    }

    fn active_theme(&self) -> &Theme {
        &self.themes[self.active_index]
    }

    pub(crate) fn theme_descriptors(&self) -> Vec<ThemeDescriptor> {
        self.themes
            .iter()
            .map(|theme| ThemeDescriptor {
                id: theme.id.clone(),
                name: theme.name.clone(),
                appearance: theme.appearance,
            })
            .collect()
    }

    pub(crate) fn active_theme_id(&self) -> &str {
        &self.config.theme
    }

    pub(crate) fn active_theme_name(&self) -> &str {
        &self.active_theme().name
    }

    pub(crate) fn select_theme(&mut self, id: &str) -> Result<(), String> {
        let active_index = self
            .themes
            .iter()
            .position(|theme| theme.id == id)
            .ok_or_else(|| format!("Theme '{id}' is not installed"))?;
        let mut config = self.config.clone();
        config.theme = id.to_string();
        save_config(&self.config_path, &config)?;
        self.config = config;
        self.active_index = active_index;
        Ok(())
    }

    pub(crate) fn sand_color_count(&self) -> usize {
        self.active_theme().sand_color_count()
    }

    pub(crate) fn sand_color_at(&self, index: usize) -> Color {
        self.active_theme().sand_color_at(index)
    }

    pub(crate) fn resolve_category_color(&self, anchor: Color) -> Color {
        self.active_theme().resolve_category_color(anchor)
    }

    pub(crate) fn cycle_category_anchor(&self, anchor: Color, direction: isize) -> Color {
        self.active_theme().cycle_category_anchor(anchor, direction)
    }

    pub(crate) fn idle_color(&self) -> Color {
        self.active_theme().ui.idle
    }

    pub(crate) fn background_ref(&self) -> UiColorRef {
        self.active_theme().ui.background
    }

    pub(crate) fn foreground_ref(&self) -> UiColorRef {
        self.active_theme().ui.foreground
    }

    pub(crate) fn status_ref(&self) -> UiColorRef {
        self.active_theme().ui.status
    }

    pub(crate) fn border_ref(&self) -> UiColorRef {
        self.active_theme().ui.border
    }

    pub(crate) fn accent_ref(&self) -> UiColorRef {
        self.active_theme().ui.accent
    }

    pub(crate) fn report_ref(&self) -> UiColorRef {
        self.active_theme().ui.report
    }

    pub(crate) fn warning_ref(&self) -> UiColorRef {
        self.active_theme().ui.warning
    }

    pub(crate) fn error_ref(&self) -> UiColorRef {
        self.active_theme().ui.error
    }

    pub(crate) fn success_ref(&self) -> UiColorRef {
        self.active_theme().ui.success
    }
}

fn load_config(path: &Path) -> Result<AppearanceConfig, String> {
    if !path.exists() {
        return Ok(AppearanceConfig::default());
    }
    let source = fs::read_to_string(path)
        .map_err(|error| format!("Cannot read appearance config {}: {error}", path.display()))?;
    let config: AppearanceConfig = toml::from_str(&source)
        .map_err(|error| format!("Cannot parse appearance config {}: {error}", path.display()))?;
    if config.schema != APPEARANCE_SCHEMA_VERSION {
        return Err(format!(
            "Appearance config schema {} is unsupported; expected {}",
            config.schema, APPEARANCE_SCHEMA_VERSION
        ));
    }
    if config.theme.trim().is_empty()
        || config.theme.trim() != config.theme
        || config.theme.contains(['/', '\\'])
    {
        return Err("Appearance theme identifier is invalid".to_string());
    }
    Ok(config)
}

fn save_config(path: &Path, config: &AppearanceConfig) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let source = toml::to_string_pretty(config).map_err(|error| error.to_string())?;
    storage::atomic_write(path, &source)
}

pub(crate) fn encode_color_anchor(color: Color) -> Result<i64, String> {
    let (r, g, b) = color::rgb_tuple(color)
        .ok_or_else(|| "Category colors must resolve to explicit RGB anchors".to_string())?;
    let packed = (i64::from(r) << 16) | (i64::from(g) << 8) | i64::from(b);
    Ok(COLOR_ANCHOR_TAG | packed)
}

pub(crate) fn decode_color_anchor(value: i64) -> Result<Color, String> {
    if value < 0 {
        return Err(format!("Category color value {value} is invalid"));
    }
    if value & COLOR_ANCHOR_TAG != 0 {
        let packed = value & COLOR_ANCHOR_MASK;
        return Ok(Color::Rgb(
            ((packed >> 16) & 0xff) as u8,
            ((packed >> 8) & 0xff) as u8,
            (packed & 0xff) as u8,
        ));
    }
    let legacy = usize::try_from(value)
        .map_err(|_| format!("Category legacy color index {value} is invalid"))?;
    Ok(COLORS[legacy % COLORS.len()])
}

#[cfg(test)]
mod tests;
