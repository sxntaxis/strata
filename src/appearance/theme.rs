use std::{
    collections::{BTreeMap, HashSet},
    fs,
    path::Path,
};

use ratatui::style::Color;
use serde::Deserialize;

use super::color::{ThemeSwatch, nearest_swatch, nearest_swatch_index};
use super::{BUILTIN_THEME_ID, BUILTIN_THEME_SOURCE};

const THEME_SCHEMA_VERSION: u8 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum UiColorRef {
    Default,
    Color(Color),
}

#[derive(Clone, Debug)]
pub(crate) struct ThemeUi {
    pub background: UiColorRef,
    pub foreground: UiColorRef,
    pub idle: Color,
    pub status: UiColorRef,
    pub border: UiColorRef,
    pub accent: UiColorRef,
    pub report: UiColorRef,
    pub warning: UiColorRef,
    pub error: UiColorRef,
    pub success: UiColorRef,
}

#[derive(Clone, Debug)]
pub(crate) struct Theme {
    pub id: String,
    pub name: String,
    pub appearance: ThemeAppearance,
    sand_wheel: Vec<ThemeSwatch>,
    pub ui: ThemeUi,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ThemeAppearance {
    Dark,
    Light,
    Any,
}

#[derive(Clone, Debug)]
pub(crate) struct ThemeDescriptor {
    pub id: String,
    pub name: String,
    pub appearance: ThemeAppearance,
}

#[derive(Debug, Deserialize)]
struct ThemeFile {
    schema: u8,
    theme: ThemeMetaFile,
    palette: BTreeMap<String, String>,
    #[serde(default)]
    sand: Option<SandFile>,
    #[serde(default)]
    ui: UiFile,
}

#[derive(Debug, Deserialize)]
struct ThemeMetaFile {
    name: String,
    #[serde(default = "default_theme_appearance")]
    appearance: String,
}

#[derive(Debug, Deserialize)]
struct SandFile {
    colors: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
struct UiFile {
    background: Option<String>,
    foreground: Option<String>,
    idle: Option<String>,
    status: Option<String>,
    border: Option<String>,
    accent: Option<String>,
    report: Option<String>,
    warning: Option<String>,
    error: Option<String>,
    success: Option<String>,
}

fn default_theme_appearance() -> String {
    "any".to_string()
}

impl Theme {
    pub(super) fn parse(id: &str, source: &str) -> Result<Self, String> {
        let file: ThemeFile = toml::from_str(source)
            .map_err(|error| format!("Theme '{id}' TOML is invalid: {error}"))?;
        if file.schema != THEME_SCHEMA_VERSION {
            return Err(format!(
                "Theme '{id}' schema {} is unsupported; expected {}",
                file.schema, THEME_SCHEMA_VERSION
            ));
        }
        if file.theme.name.trim().is_empty() {
            return Err(format!("Theme '{id}' has an empty name"));
        }
        let appearance = match file.theme.appearance.trim().to_ascii_lowercase().as_str() {
            "dark" => ThemeAppearance::Dark,
            "light" => ThemeAppearance::Light,
            "any" | "both" | "neutral" => ThemeAppearance::Any,
            other => {
                return Err(format!(
                    "Theme '{id}' appearance '{other}' is invalid; expected dark, light, or any"
                ));
            }
        };

        let mut palette = BTreeMap::new();
        for (key, raw) in file.palette {
            if key.trim().is_empty() || key.eq_ignore_ascii_case("default") {
                return Err(format!("Theme '{id}' has invalid palette key '{key}'"));
            }
            palette.insert(key.clone(), parse_hex_color(id, &key, &raw)?);
        }
        if palette.is_empty() {
            return Err(format!("Theme '{id}' palette is empty"));
        }

        let sand_keys = file
            .sand
            .map(|sand| sand.colors)
            .unwrap_or_else(|| palette.keys().cloned().collect());
        if sand_keys.is_empty() {
            return Err(format!("Theme '{id}' sand palette is empty"));
        }
        let mut seen = HashSet::new();
        let mut sand_wheel = Vec::with_capacity(sand_keys.len());
        for key in sand_keys {
            if !seen.insert(key.clone()) {
                return Err(format!("Theme '{id}' repeats sand swatch '{key}'"));
            }
            let color = *palette.get(&key).ok_or_else(|| {
                format!("Theme '{id}' sand swatch '{key}' is not defined in [palette]")
            })?;
            sand_wheel.push(ThemeSwatch::new(key, color)?);
        }
        sand_wheel.sort_by(ThemeSwatch::hue_order);

        let builtin_fallback = if id == BUILTIN_THEME_ID {
            None
        } else {
            Some(Self::parse(BUILTIN_THEME_ID, BUILTIN_THEME_SOURCE)?)
        };
        let ui = resolve_ui(id, &palette, file.ui, builtin_fallback.as_ref())?;

        Ok(Self {
            id: id.to_string(),
            name: file.theme.name.trim().to_string(),
            appearance,
            sand_wheel,
            ui,
        })
    }

    pub(super) fn sand_color_count(&self) -> usize {
        self.sand_wheel.len()
    }

    pub(super) fn sand_color_at(&self, index: usize) -> Color {
        self.sand_wheel[index % self.sand_wheel.len()].color
    }

    pub(super) fn resolve_category_color(&self, anchor: Color) -> Color {
        nearest_swatch(&self.sand_wheel, anchor)
            .map(|swatch| swatch.color)
            .unwrap_or(anchor)
    }

    pub(super) fn cycle_category_anchor(&self, anchor: Color, direction: isize) -> Color {
        if self.sand_wheel.len() <= 1 {
            return self
                .sand_wheel
                .first()
                .map(|swatch| swatch.color)
                .unwrap_or(anchor);
        }
        let current = nearest_swatch_index(&self.sand_wheel, anchor).unwrap_or(0);
        let next = if direction < 0 {
            (current + self.sand_wheel.len() - 1) % self.sand_wheel.len()
        } else {
            (current + 1) % self.sand_wheel.len()
        };
        self.sand_wheel[next].color
    }
}

pub(super) fn load_themes_from(directory: &Path) -> Result<Vec<Theme>, String> {
    let mut themes = vec![Theme::parse(BUILTIN_THEME_ID, BUILTIN_THEME_SOURCE)?];
    if directory.exists() {
        let mut entries = fs::read_dir(directory)
            .map_err(|error| {
                format!(
                    "Cannot read theme directory {}: {error}",
                    directory.display()
                )
            })?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| {
                format!(
                    "Cannot enumerate theme directory {}: {error}",
                    directory.display()
                )
            })?
            .into_iter()
            .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "toml"))
            .collect::<Vec<_>>();
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            let path = entry.path();
            let stem = path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .ok_or_else(|| {
                    format!(
                        "Theme filename {} is not valid UTF-8 and cannot be addressed",
                        path.display()
                    )
                })?;
            if stem.eq_ignore_ascii_case(BUILTIN_THEME_ID) {
                return Err(format!(
                    "{} shadows the reserved built-in theme id '{BUILTIN_THEME_ID}'",
                    path.display()
                ));
            }
            if themes
                .iter()
                .any(|theme| theme.id.eq_ignore_ascii_case(stem))
            {
                return Err(format!(
                    "{} duplicates an already installed theme id '{stem}' (theme ids are case-insensitive)",
                    path.display()
                ));
            }
            let source = fs::read_to_string(&path)
                .map_err(|error| format!("Cannot read theme {}: {error}", path.display()))?;
            themes.push(Theme::parse(stem, &source)?);
        }
    }
    themes.sort_by(|left, right| {
        (left.id != BUILTIN_THEME_ID)
            .cmp(&(right.id != BUILTIN_THEME_ID))
            .then_with(|| {
                left.name
                    .to_ascii_lowercase()
                    .cmp(&right.name.to_ascii_lowercase())
            })
            .then_with(|| left.id.cmp(&right.id))
    });
    Ok(themes)
}

fn resolve_ui(
    id: &str,
    palette: &BTreeMap<String, Color>,
    file: UiFile,
    fallback: Option<&Theme>,
) -> Result<ThemeUi, String> {
    let fallback_ui = fallback.map(|theme| &theme.ui);
    let background = resolve_ui_ref(id, palette, file.background.as_deref())?
        .or_else(|| fallback_ui.map(|ui| ui.background))
        .unwrap_or(UiColorRef::Default);
    let foreground = resolve_ui_ref(id, palette, file.foreground.as_deref())?
        .or_else(|| fallback_ui.map(|ui| ui.foreground))
        .unwrap_or(UiColorRef::Color(Color::White));
    let idle = resolve_required_ui_color(id, palette, file.idle.as_deref())?
        .or_else(|| fallback_ui.map(|ui| ui.idle))
        .unwrap_or(Color::White);
    let status = resolve_ui_ref(id, palette, file.status.as_deref())?
        .or_else(|| fallback_ui.map(|ui| ui.status))
        .unwrap_or(UiColorRef::Color(Color::Gray));
    let border = resolve_ui_ref(id, palette, file.border.as_deref())?
        .or_else(|| fallback_ui.map(|ui| ui.border))
        .unwrap_or(UiColorRef::Color(Color::Cyan));
    let accent = resolve_ui_ref(id, palette, file.accent.as_deref())?
        .or_else(|| fallback_ui.map(|ui| ui.accent))
        .unwrap_or(UiColorRef::Color(Color::Blue));
    let report = resolve_ui_ref(id, palette, file.report.as_deref())?
        .or_else(|| fallback_ui.map(|ui| ui.report))
        .unwrap_or(UiColorRef::Color(Color::Magenta));
    let warning = resolve_ui_ref(id, palette, file.warning.as_deref())?
        .or_else(|| fallback_ui.map(|ui| ui.warning))
        .unwrap_or(UiColorRef::Color(Color::Yellow));
    let error = resolve_ui_ref(id, palette, file.error.as_deref())?
        .or_else(|| fallback_ui.map(|ui| ui.error))
        .unwrap_or(UiColorRef::Color(Color::Red));
    let success = resolve_ui_ref(id, palette, file.success.as_deref())?
        .or_else(|| fallback_ui.map(|ui| ui.success))
        .unwrap_or(UiColorRef::Color(Color::Rgb(0, 176, 80)));
    Ok(ThemeUi {
        background,
        foreground,
        idle,
        status,
        border,
        accent,
        report,
        warning,
        error,
        success,
    })
}

fn resolve_ui_ref(
    id: &str,
    palette: &BTreeMap<String, Color>,
    raw: Option<&str>,
) -> Result<Option<UiColorRef>, String> {
    let Some(raw) = raw else {
        return Ok(None);
    };
    if raw.eq_ignore_ascii_case("default") {
        return Ok(Some(UiColorRef::Default));
    }
    palette
        .get(raw)
        .copied()
        .map(UiColorRef::Color)
        .map(Some)
        .ok_or_else(|| format!("Theme '{id}' UI color '{raw}' is not defined in [palette]"))
}

fn resolve_required_ui_color(
    id: &str,
    palette: &BTreeMap<String, Color>,
    raw: Option<&str>,
) -> Result<Option<Color>, String> {
    let Some(raw) = raw else {
        return Ok(None);
    };
    if raw.eq_ignore_ascii_case("default") {
        return Err(format!(
            "Theme '{id}' uses 'default' for an RGB-required UI role; use a palette swatch"
        ));
    }
    palette
        .get(raw)
        .copied()
        .map(Some)
        .ok_or_else(|| format!("Theme '{id}' UI color '{raw}' is not defined in [palette]"))
}

fn parse_hex_color(theme_id: &str, key: &str, raw: &str) -> Result<Color, String> {
    let value = raw.strip_prefix('#').unwrap_or(raw);
    if value.len() != 6 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(format!(
            "Theme '{theme_id}' palette swatch '{key}' must be #RRGGBB"
        ));
    }
    let parsed = u32::from_str_radix(value, 16).map_err(|error| error.to_string())?;
    Ok(Color::Rgb(
        ((parsed >> 16) & 0xff) as u8,
        ((parsed >> 8) & 0xff) as u8,
        (parsed & 0xff) as u8,
    ))
}
