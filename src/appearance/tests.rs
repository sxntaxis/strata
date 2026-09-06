use std::cmp::Ordering;
use std::path::PathBuf;

use ratatui::style::Color;

use super::*;
use super::color::{ThemeSwatch, nearest_swatch};
use super::theme::{Theme, UiColorRef, load_themes_from};

const SAMPLE: &str = r##"
schema = 1
[theme]
name = "Odd Palette"
appearance = "any"
[palette]
volcano = "#ff3000"
moss = "#40a040"
ocean = "#2090ff"
bruise = "#a040d0"
text = "#eeeeee"
[sand]
colors = ["bruise", "ocean", "volcano", "moss"]
[ui]
background = "default"
foreground = "text"
idle = "text"
"##;

#[test]
fn theme_supports_arbitrary_named_palette_and_sand_subset() {
    let theme = Theme::parse("odd", SAMPLE).unwrap();
    assert_eq!(theme.name, "Odd Palette");
    assert_eq!(theme.sand_color_count(), 4);
    assert_eq!(theme.ui.background, UiColorRef::Default);
}

#[test]
fn omitted_sand_uses_all_palette_swatches() {
    let source = r##"
schema = 1
[theme]
name = "Tiny"
[palette]
a = "#ff0000"
b = "#00ff00"
c = "#0000ff"
"##;
    let theme = Theme::parse("tiny", source).unwrap();
    assert_eq!(theme.sand_color_count(), 3);
}

#[test]
fn hue_wheel_is_derived_not_declaration_order() {
    let theme = Theme::parse("odd", SAMPLE).unwrap();
    let mut swatches = vec![
        ThemeSwatch::new("bruise".into(), Color::Rgb(160, 64, 208)).unwrap(),
        ThemeSwatch::new("ocean".into(), Color::Rgb(32, 144, 255)).unwrap(),
        ThemeSwatch::new("volcano".into(), Color::Rgb(255, 48, 0)).unwrap(),
        ThemeSwatch::new("moss".into(), Color::Rgb(64, 160, 64)).unwrap(),
    ];
    let declaration = swatches
        .iter()
        .map(|swatch| swatch.key.clone())
        .collect::<Vec<_>>();
    swatches.sort_by(ThemeSwatch::hue_order);
    let derived = swatches
        .iter()
        .map(|swatch| swatch.key.clone())
        .collect::<Vec<_>>();
    assert_ne!(derived, declaration);
    for pair in swatches.windows(2) {
        assert_ne!(ThemeSwatch::hue_order(&pair[0], &pair[1]), Ordering::Greater);
    }
    assert_eq!(theme.sand_color_count(), 4);
}

#[test]
fn theme_change_resolves_nearest_perceptual_swatch() {
    let theme = Theme::parse("odd", SAMPLE).unwrap();
    let swatches = [
        ThemeSwatch::new("ocean".into(), Color::Rgb(32, 144, 255)).unwrap(),
        ThemeSwatch::new("moss".into(), Color::Rgb(64, 160, 64)).unwrap(),
    ];
    assert_eq!(
        nearest_swatch(&swatches, Color::Rgb(30, 145, 250))
            .unwrap()
            .key,
        "ocean"
    );
    assert_eq!(
        theme.resolve_category_color(Color::Rgb(30, 145, 250)),
        Color::Rgb(32, 144, 255)
    );
}

#[test]
fn tagged_rgb_anchor_round_trips_and_legacy_indices_still_load() {
    let color = Color::Rgb(137, 82, 219);
    let encoded = encode_color_anchor(color).unwrap();
    assert_eq!(decode_color_anchor(encoded).unwrap(), color);
    assert_eq!(decode_color_anchor(6).unwrap(), COLORS[6]);
    assert_eq!(decode_color_anchor(18).unwrap(), COLORS[6]);
}

#[test]
fn built_in_theme_preserves_every_legacy_sand_rgb_without_fixed_length_contract() {
    let theme = Theme::parse(BUILTIN_THEME_ID, BUILTIN_THEME_SOURCE).unwrap();
    assert_eq!(theme.sand_color_count(), COLORS.len());
    for legacy in COLORS {
        assert_eq!(theme.resolve_category_color(legacy), legacy);
    }
}

#[test]
fn built_in_ui_palette_preserves_pre_theme_runtime_colors() {
    let theme = Theme::parse(BUILTIN_THEME_ID, BUILTIN_THEME_SOURCE).unwrap();
    assert_eq!(theme.ui.background, UiColorRef::Default);
    assert_eq!(theme.ui.foreground, UiColorRef::Color(Color::White));
    assert_eq!(theme.ui.idle, Color::White);
    assert_eq!(theme.ui.status, UiColorRef::Color(Color::Gray));
    assert_eq!(theme.ui.border, UiColorRef::Color(Color::Cyan));
    assert_eq!(theme.ui.accent, UiColorRef::Color(Color::Blue));
    assert_eq!(theme.ui.report, UiColorRef::Color(Color::Magenta));
    assert_eq!(theme.ui.warning, UiColorRef::Color(Color::Yellow));
    assert_eq!(theme.ui.error, UiColorRef::Color(Color::Red));
    assert_eq!(theme.ui.success, UiColorRef::Color(Color::Rgb(0, 176, 80)));
}

#[test]
fn built_in_hue_wheel_runs_red_through_violet_perceptually() {
    let theme = Theme::parse(BUILTIN_THEME_ID, BUILTIN_THEME_SOURCE).unwrap();
    let expected = [
        Color::Rgb(255, 0, 0),
        Color::Rgb(255, 51, 0),
        Color::Rgb(255, 153, 0),
        Color::Rgb(255, 204, 0),
        Color::Rgb(255, 255, 0),
        Color::Rgb(128, 255, 0),
        Color::Rgb(0, 176, 80),
        Color::Rgb(0, 255, 255),
        Color::Rgb(0, 153, 255),
        Color::Rgb(0, 0, 255),
        Color::Rgb(102, 51, 255),
        Color::Rgb(153, 0, 255),
    ];
    for (index, color) in expected.into_iter().enumerate() {
        assert_eq!(theme.sand_color_at(index), color);
    }
}

#[test]
fn theme_can_offer_more_than_the_historical_twelve_sand_colors() {
    let mut source = String::from("schema = 1\n[theme]\nname = \"Wide\"\n[palette]\n");
    for index in 0..17_u8 {
        source.push_str(&format!(
            "c{index} = \"#{:02x}{:02x}{:02x}\"\n",
            index.saturating_mul(13),
            255_u8.saturating_sub(index.saturating_mul(11)),
            index.saturating_mul(7),
        ));
    }
    let theme = Theme::parse("wide", &source).unwrap();
    assert_eq!(theme.sand_color_count(), 17);
}

#[test]
fn default_is_allowed_for_non_sand_ui_roles() {
    let source = r##"
schema = 1
[theme]
name = "External UI"
[palette]
sand = "#cc5500"
text = "#eeeeee"
[sand]
colors = ["sand"]
[ui]
background = "default"
foreground = "default"
status = "default"
idle = "text"
"##;
    let theme = Theme::parse("external", source).unwrap();
    assert_eq!(theme.ui.background, UiColorRef::Default);
    assert_eq!(theme.ui.foreground, UiColorRef::Default);
    assert_eq!(theme.ui.status, UiColorRef::Default);
}

#[test]
fn idle_rejects_default_until_terminal_foreground_resolution_exists() {
    let source = r##"
schema = 1
[theme]
name = "Bad Idle"
[palette]
sand = "#cc5500"
[sand]
colors = ["sand"]
[ui]
idle = "default"
"##;
    assert!(Theme::parse("bad-idle", source).is_err());
}

#[test]
fn default_is_a_ui_sentinel_not_a_palette_key_case_insensitively() {
    for key in ["default", "Default", "DEFAULT"] {
        let invalid = format!(
            "schema = 1\n[theme]\nname = \"Bad\"\n[palette]\n{key} = \"#ffffff\"\n"
        );
        assert!(Theme::parse("bad", &invalid).is_err());
    }
}

fn temp_theme_dir(label: &str) -> PathBuf {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "strata-appearance-{label}-{}-{nonce}",
        std::process::id()
    ))
}

#[test]
fn custom_theme_discovery_and_ignore_config_are_profile_local_and_deterministic() {
    let root = temp_theme_dir("profile");
    let themes = root.join("themes");
    std::fs::create_dir_all(&themes).unwrap();
    std::fs::write(themes.join("odd.toml"), SAMPLE).unwrap();
    let config_path = root.join("appearance.toml");
    std::fs::write(&config_path, "schema = 1\ntheme = \"odd\"\n").unwrap();

    let loaded = AppearanceState::load_from_paths(false, config_path.clone(), &themes).unwrap();
    assert_eq!(loaded.active_theme_id(), "odd");
    assert_eq!(loaded.theme_descriptors().len(), 2);

    let ignored = AppearanceState::load_from_paths(true, config_path, &themes).unwrap();
    assert_eq!(ignored.active_theme_id(), BUILTIN_THEME_ID);
    assert_eq!(ignored.theme_descriptors().len(), 1);

    std::fs::remove_dir_all(root).ok();
}

#[test]
fn theme_selection_persists_without_mutating_category_anchor() {
    let root = temp_theme_dir("selection");
    let themes = root.join("themes");
    std::fs::create_dir_all(&themes).unwrap();
    std::fs::write(themes.join("odd.toml"), SAMPLE).unwrap();
    let config_path = root.join("appearance.toml");

    let mut loaded =
        AppearanceState::load_from_paths(false, config_path.clone(), &themes).unwrap();
    let anchor = Color::Rgb(255, 0, 0);
    assert_eq!(loaded.active_theme_id(), BUILTIN_THEME_ID);
    loaded.select_theme("odd").unwrap();
    assert_eq!(loaded.active_theme_id(), "odd");
    assert_eq!(anchor, Color::Rgb(255, 0, 0));
    assert_ne!(loaded.resolve_category_color(anchor), Color::Reset);

    let persisted = load_config(&config_path).unwrap();
    assert_eq!(persisted.theme, "odd");
    std::fs::remove_dir_all(root).ok();
}

#[test]
fn sand_cycle_wraps_across_arbitrary_theme_length() {
    let theme = Theme::parse("odd", SAMPLE).unwrap();
    let original = theme.sand_color_at(0);
    let mut anchor = original;
    for _ in 0..theme.sand_color_count() {
        anchor = theme.cycle_category_anchor(anchor, 1);
    }
    assert_eq!(anchor, original);

    anchor = original;
    for _ in 0..theme.sand_color_count() {
        anchor = theme.cycle_category_anchor(anchor, -1);
    }
    assert_eq!(anchor, original);
}

#[test]
fn custom_theme_cannot_shadow_default_case_insensitively() {
    let root = temp_theme_dir("reserved");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("DEFAULT.toml"), SAMPLE).unwrap();
    let error = load_themes_from(&root).unwrap_err();
    assert!(error.contains("reserved built-in theme id 'default'"));
    std::fs::remove_dir_all(root).ok();
}

#[test]
fn malformed_custom_theme_fails_closed_instead_of_being_skipped() {
    let root = temp_theme_dir("malformed");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("broken.toml"), "this is not valid toml = [").unwrap();
    let error = load_themes_from(&root).unwrap_err();
    assert!(error.contains("broken"));
    std::fs::remove_dir_all(root).ok();
}
