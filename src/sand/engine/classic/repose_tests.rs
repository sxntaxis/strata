use super::*;
use crate::domain::CategoryId;

fn install_centered_compact_wall(
    engine: &mut ClassicSandboxEngine,
    width_dots: usize,
    height_dots: usize,
) {
    engine.clear();
    let bounds = engine.surface.viewport_bounds().expect("visible basin");
    assert!(width_dots < bounds.x_end - bounds.x_start);
    assert!(height_dots < bounds.y_end - bounds.y_start);

    let visible_width = bounds.x_end - bounds.x_start;
    let x_start = bounds.x_start + (visible_width - width_dots) / 2;
    let y_start = bounds.y_end - height_dots;
    for y in y_start..bounds.y_end {
        for x in x_start..x_start + width_dots {
            engine.surface.grid[y][x] = Some(CategoryId(1));
        }
    }
    engine.total_generated = engine.surface.physical_grain_count();
    engine.sync_surface_metadata();
}

fn relax_fixed(engine: &mut ClassicSandboxEngine, passes: usize) {
    for _ in 0..passes {
        engine.apply_gravity();
    }
}

fn supported_height_profile(engine: &ClassicSandboxEngine) -> Vec<usize> {
    let bounds = engine.surface.viewport_bounds().expect("visible basin");
    (bounds.x_start..bounds.x_end)
        .map(|x| {
            let mut height = 0usize;
            for y in (bounds.y_start..bounds.y_end).rev() {
                if engine.surface.grid[y][x].is_none() {
                    break;
                }
                height += 1;
            }
            height
        })
        .collect()
}

fn occupied_profile(profile: &[usize]) -> &[usize] {
    let first = profile.iter().position(|height| *height > 0).unwrap();
    let last = profile.iter().rposition(|height| *height > 0).unwrap();
    &profile[first..=last]
}

fn second_difference_roughness(profile: &[usize]) -> usize {
    occupied_profile(profile)
        .windows(3)
        .map(|window| {
            let left = window[0] as isize;
            let center = window[1] as isize;
            let right = window[2] as isize;
            (right - 2 * center + left).unsigned_abs()
        })
        .sum()
}

fn footprint_width(profile: &[usize]) -> usize {
    occupied_profile(profile).len()
}

fn apex_height(profile: &[usize]) -> usize {
    profile.iter().copied().max().unwrap_or(0)
}

#[test]
fn local_repose_two_blocks_only_the_extra_unit_of_relief() {
    let mut low = ClassicSandboxEngine::new(8, 6, 23, ClassicRainMode::Uniform);
    let mut high = ClassicSandboxEngine::new(8, 6, 23, ClassicRainMode::Uniform);
    let bounds = low.surface.viewport_bounds().expect("visible basin");
    let x = bounds.x_start + (bounds.x_end - bounds.x_start) / 2;
    let floor = bounds.y_end - 1;
    let category = CategoryId(1);

    for engine in [&mut low, &mut high] {
        engine.surface.grid[floor][x] = Some(category);
        engine.surface.grid[floor - 1][x] = Some(category);
        engine.surface.grid[floor - 2][x] = Some(category);
        engine.surface.grid[floor][x - 1] = Some(category);
        engine.surface.grid[floor][x + 1] = Some(category);
    }
    low.local_repose[x] = CLASSIC_REPOSE_LOW;
    high.local_repose[x] = CLASSIC_REPOSE_MID;

    low.move_grain_once(bounds, x, floor - 2);
    high.move_grain_once(bounds, x, floor - 2);

    assert_eq!(low.surface.grid[floor - 2][x], None);
    assert_eq!(high.surface.grid[floor - 2][x], Some(category));
}

#[test]
fn heterogeneous_classic_is_deterministic_for_a_fixed_seed() {
    let mut first = ClassicSandboxEngine::new(30, 10, 41, ClassicRainMode::Uniform);
    let mut second = ClassicSandboxEngine::new(30, 10, 41, ClassicRainMode::Uniform);
    install_centered_compact_wall(&mut first, 30, 24);
    install_centered_compact_wall(&mut second, 30, 24);

    relax_fixed(&mut first, 800);
    relax_fixed(&mut second, 800);

    assert_eq!(first.surface.grid, second.surface.grid);
    assert_eq!(first.local_repose, second.local_repose);
    assert_eq!(first.physics_rng_state, second.physics_rng_state);
    assert_eq!(first.repose_rng_state, second.repose_rng_state);
    assert_eq!(first.movement_counts(), second.movement_counts());
}

#[test]
fn weak_local_repose_increases_surface_roughness_without_rewriting_macroform() {
    let mut baseline = ClassicSandboxEngine::new(30, 10, 7, ClassicRainMode::Uniform);
    baseline.set_experiment_profile_name("rugged").unwrap();
    baseline.force_uniform_repose = true;
    baseline.local_repose.fill(CLASSIC_REPOSE_LOW);

    let mut heterogeneous = ClassicSandboxEngine::new(30, 10, 7, ClassicRainMode::Uniform);
    heterogeneous.set_experiment_profile_name("rugged").unwrap();
    install_centered_compact_wall(&mut baseline, 30, 24);
    install_centered_compact_wall(&mut heterogeneous, 30, 24);
    baseline.force_uniform_repose = true;
    baseline.local_repose.fill(CLASSIC_REPOSE_LOW);

    let expected_mass = baseline.surface.physical_grain_count();
    assert_eq!(heterogeneous.surface.physical_grain_count(), expected_mass);

    relax_fixed(&mut baseline, 800);
    relax_fixed(&mut heterogeneous, 800);

    assert_eq!(baseline.surface.physical_grain_count(), expected_mass);
    assert_eq!(heterogeneous.surface.physical_grain_count(), expected_mass);

    let baseline_profile = supported_height_profile(&baseline);
    let heterogeneous_profile = supported_height_profile(&heterogeneous);
    let baseline_roughness = second_difference_roughness(&baseline_profile);
    let heterogeneous_roughness = second_difference_roughness(&heterogeneous_profile);
    assert!(baseline_roughness > 0);
    assert!(
        heterogeneous_roughness.saturating_mul(10) >= baseline_roughness.saturating_mul(11),
        "weak repose heterogeneity did not increase roughness enough: baseline={baseline_roughness}, heterogeneous={heterogeneous_roughness}"
    );

    let baseline_apex = apex_height(&baseline_profile);
    let heterogeneous_apex = apex_height(&heterogeneous_profile);
    assert!(baseline_apex.abs_diff(heterogeneous_apex) <= 1);

    let baseline_width = footprint_width(&baseline_profile);
    let heterogeneous_width = footprint_width(&heterogeneous_profile);
    eprintln!(
        "CLASSIC_002_METRICS baseline_roughness={baseline_roughness} heterogeneous_roughness={heterogeneous_roughness} baseline_apex={baseline_apex} heterogeneous_apex={heterogeneous_apex} baseline_width={baseline_width} heterogeneous_width={heterogeneous_width}"
    );
    let width_tolerance = baseline_width.div_ceil(10).max(1);
    assert!(
        baseline_width.abs_diff(heterogeneous_width) <= width_tolerance,
        "weak repose heterogeneity changed runout too much: baseline={baseline_width}, heterogeneous={heterogeneous_width}"
    );
}

#[test]
fn resize_preserves_existing_local_repose_with_the_shifted_canonical_terrain() {
    let mut engine = ClassicSandboxEngine::new(12, 8, 13, ClassicRainMode::Uniform);
    let old_width = engine.surface.grid_width_dots;
    let old_repose = engine.local_repose.clone();

    engine.resize(20, 8);

    let horizontal_offset = (engine.surface.grid_width_dots - old_width) / 2;
    assert!(horizontal_offset > 0);
    assert_eq!(
        &engine.local_repose[horizontal_offset..horizontal_offset + old_width],
        old_repose.as_slice()
    );
}

#[test]
fn classic_and_hybrid_still_share_the_same_repose_and_gravity_law() {
    let mut classic = ClassicSandboxEngine::new(30, 10, 19, ClassicRainMode::Uniform);
    let mut hybrid = ClassicSandboxEngine::new(30, 10, 19, ClassicRainMode::WanderingFocus);
    install_centered_compact_wall(&mut classic, 30, 24);
    install_centered_compact_wall(&mut hybrid, 30, 24);

    relax_fixed(&mut classic, 500);
    relax_fixed(&mut hybrid, 500);

    assert_eq!(classic.surface.grid, hybrid.surface.grid);
    assert_eq!(classic.local_repose, hybrid.local_repose);
    assert_eq!(classic.physics_rng_state, hybrid.physics_rng_state);
    assert_eq!(classic.repose_rng_state, hybrid.repose_rng_state);
}

#[test]
fn local_repose_three_requires_one_more_unit_of_relief_than_repose_two() {
    let mut mid = ClassicSandboxEngine::new(8, 7, 23, ClassicRainMode::Uniform);
    let mut high = ClassicSandboxEngine::new(8, 7, 23, ClassicRainMode::Uniform);
    let bounds = mid.surface.viewport_bounds().expect("visible basin");
    let x = bounds.x_start + (bounds.x_end - bounds.x_start) / 2;
    let floor = bounds.y_end - 1;
    let category = CategoryId(1);

    for engine in [&mut mid, &mut high] {
        for y in floor - 3..=floor {
            engine.surface.grid[y][x] = Some(category);
        }
        // Both diagonals have two empty cells below the source top, but the
        // third cell is occupied. Repose=2 may release to either side; repose=3
        // must wait regardless of the random left/right choice.
        engine.surface.grid[floor][x - 1] = Some(category);
        engine.surface.grid[floor][x + 1] = Some(category);
    }
    mid.local_repose[x] = CLASSIC_REPOSE_MID;
    high.local_repose[x] = CLASSIC_REPOSE_HIGH;

    mid.move_grain_once(bounds, x, floor - 3);
    high.move_grain_once(bounds, x, floor - 3);

    assert_eq!(mid.surface.grid[floor - 3][x], None);
    assert_eq!(high.surface.grid[floor - 3][x], Some(category));
}

#[test]
fn baseline_profile_preserves_classic_002_exact_repose_mapping() {
    let mut engine = ClassicSandboxEngine::new(30, 10, 71, ClassicRainMode::Uniform);
    engine.set_experiment_profile_name("rugged").unwrap();
    engine.set_texture_profile_name("baseline").unwrap();
    engine.clear();
    assert_eq!(engine.texture_profile_name(), "baseline");
    let high_sites = engine
        .local_repose
        .iter()
        .enumerate()
        .filter_map(|(x, repose)| (*repose == CLASSIC_REPOSE_MID).then_some(x))
        .collect::<Vec<_>>();
    assert_eq!(high_sites, vec![37]);
    assert!(
        engine
            .local_repose
            .iter()
            .all(|repose| *repose <= CLASSIC_REPOSE_MID)
    );
}

#[test]
fn texture_profile_selection_is_nonretroactive_and_fill_resamples_cleanly() {
    let mut engine = ClassicSandboxEngine::new(100, 8, 29, ClassicRainMode::Uniform);
    engine.set_experiment_profile_name("rugged").unwrap();
    assert_eq!(engine.texture_profile_name(), "rugged");
    let before = engine.local_repose.clone();

    engine
        .set_texture_profile_name("textured")
        .expect("textured profile");
    assert_eq!(
        engine.local_repose, before,
        "profile selection must not rewrite live columns"
    );
    engine
        .debug_fill_rainbow_80(&[CategoryId(1), CategoryId(2)])
        .expect("textured fill");
    assert_eq!(engine.texture_profile_name(), "textured");
    assert!(engine.local_repose.contains(&CLASSIC_REPOSE_HIGH));

    engine
        .set_texture_profile_name("rugged")
        .expect("rugged profile");
    engine
        .debug_fill_rainbow_80(&[CategoryId(1), CategoryId(2)])
        .expect("rugged fill");
    assert_eq!(engine.texture_profile_name(), "rugged");

    engine
        .set_texture_profile_name("terraced")
        .expect("terraced profile");
    engine
        .debug_fill_rainbow_80(&[CategoryId(1), CategoryId(2)])
        .expect("terraced fill");
    assert_eq!(engine.texture_profile_name(), "terraced");
    assert!(engine.set_texture_profile_name("custom").is_err());
}

#[test]
fn texture_profiles_remain_deterministic_for_a_fixed_seed() {
    for profile in ["baseline", "textured", "rugged", "terraced"] {
        let mut first = ClassicSandboxEngine::new(40, 10, 71, ClassicRainMode::Uniform);
        let mut second = ClassicSandboxEngine::new(40, 10, 71, ClassicRainMode::Uniform);
        first.set_experiment_profile_name("rugged").unwrap();
        second.set_experiment_profile_name("rugged").unwrap();
        first.set_texture_profile_name(profile).unwrap();
        second.set_texture_profile_name(profile).unwrap();
        first.clear();
        second.clear();
        assert_eq!(first.local_repose, second.local_repose, "profile={profile}");
        assert_eq!(
            first.repose_rng_state, second.repose_rng_state,
            "profile={profile}"
        );
    }
}

fn profile_metrics(profile: &str) -> (usize, usize, usize, usize) {
    let mut engine = ClassicSandboxEngine::new(30, 10, 7, ClassicRainMode::Uniform);
    engine.set_experiment_profile_name("rugged").unwrap();
    engine.set_texture_profile_name(profile).unwrap();
    install_centered_compact_wall(&mut engine, 30, 24);
    let expected_mass = engine.surface.physical_grain_count();
    relax_fixed(&mut engine, 800);
    assert_eq!(engine.surface.physical_grain_count(), expected_mass);
    let profile_heights = supported_height_profile(&engine);
    (
        second_difference_roughness(&profile_heights),
        apex_height(&profile_heights),
        footprint_width(&profile_heights),
        expected_mass,
    )
}

#[test]
fn texture_profile_sweep_preserves_classic_macroform_and_offers_stronger_relief() {
    let baseline = profile_metrics("baseline");
    let textured = profile_metrics("textured");
    let rugged = profile_metrics("rugged");
    let terraced = profile_metrics("terraced");

    for (name, metrics) in [
        ("baseline", baseline),
        ("textured", textured),
        ("rugged", rugged),
        ("terraced", terraced),
    ] {
        eprintln!(
            "CLASSIC_004_METRICS profile={name} roughness={} apex={} width={} mass={}",
            metrics.0, metrics.1, metrics.2, metrics.3
        );
        assert_eq!(metrics.3, baseline.3, "profile={name}");
        assert!(baseline.1.abs_diff(metrics.1) <= 3, "profile={name}");
        let width_tolerance = baseline.2.saturating_mul(15).div_ceil(100).max(1);
        assert!(
            baseline.2.abs_diff(metrics.2) <= width_tolerance,
            "profile={name}: baseline_width={} width={} tolerance={width_tolerance}",
            baseline.2,
            metrics.2
        );
    }

    let strongest = textured.0.max(rugged.0).max(terraced.0);
    assert!(
        strongest.saturating_mul(10) >= baseline.0.saturating_mul(11),
        "no preset increased relief enough: baseline={} textured={} rugged={} terraced={}",
        baseline.0,
        textured.0,
        rugged.0,
        terraced.0
    );
}

#[test]
fn classic_experiment_selection_is_nonretroactive_and_uses_rugged_texture_base() {
    let mut engine = ClassicSandboxEngine::new(80, 10, 101, ClassicRainMode::Uniform);
    let before = engine.local_repose.clone();

    for profile in [
        "rugged",
        "memory",
        "slope",
        "memory-slope",
        "anchored",
        "momentum",
        "momentum-repose",
        "momentum-tangent",
        "momentum-soft",
        "momentum-contact",
        "momentum-repose-contact",
    ] {
        engine.set_experiment_profile_name(profile).unwrap();
        assert_eq!(engine.experiment_profile_name(), profile);
        assert_eq!(engine.texture_profile_name(), "rugged");
        assert_eq!(
            engine.local_repose, before,
            "experiment selection must not rewrite live columns"
        );
    }
    assert!(engine.set_experiment_profile_name("custom").is_err());
}

#[test]
fn anchored_is_the_owner_selected_default_classic_experiment() {
    let engine = ClassicSandboxEngine::new(80, 10, 102, ClassicRainMode::Uniform);
    assert_eq!(engine.texture_profile_name(), "rugged");
    assert_eq!(engine.experiment_profile_name(), "anchored");
}

#[test]
fn memory_profile_holds_local_repose_for_three_surface_refreshes() {
    let mut engine = ClassicSandboxEngine::new(30, 10, 103, ClassicRainMode::Uniform);
    engine.set_experiment_profile_name("memory").unwrap();
    engine.clear();
    let x = engine.local_repose.len() / 2;
    engine.local_repose[x] = CLASSIC_REPOSE_HIGH;
    engine.repose_memory_remaining[x] = 3;
    let rng_before = engine.repose_rng_state;

    for expected_remaining in [2, 1, 0] {
        engine.refresh_local_repose(x);
        assert_eq!(engine.local_repose[x], CLASSIC_REPOSE_HIGH);
        assert_eq!(engine.repose_memory_remaining[x], expected_remaining);
        assert_eq!(engine.repose_rng_state, rng_before);
    }

    engine.refresh_local_repose(x);
    assert_ne!(engine.repose_rng_state, rng_before);
    assert_eq!(engine.repose_memory_remaining[x], 3);
}

#[test]
fn slope_profile_mildly_prefers_the_side_with_more_open_relief() {
    let mut engine = ClassicSandboxEngine::new(24, 12, 107, ClassicRainMode::Uniform);
    engine.set_experiment_profile_name("slope").unwrap();
    let bounds = engine.surface.viewport_bounds().expect("visible basin");
    let x = bounds.x_start + (bounds.x_end - bounds.x_start) / 2;
    let y = bounds.y_start + 2;
    let left = x - 1;
    let right = x + 1;

    // Left has deep open relief. Right is open for the ordinary diagonal but
    // becomes supported immediately after it, making left the steeper side.
    engine.surface.grid[y + 2][right] = Some(CategoryId(1));
    engine.local_repose[x] = CLASSIC_REPOSE_LOW;

    let mut left_choices = 0usize;
    let samples = 128usize;
    for _ in 0..samples {
        if engine.choose_diagonal_step(bounds, x, y) == -1 {
            left_choices += 1;
        }
    }
    assert!(
        left_choices >= 80,
        "3:1 steep-side preference was not visible: left={left_choices}/{samples}, left_drop={}, right_drop={}",
        engine.diagonal_drop_depth(bounds, left, y),
        engine.diagonal_drop_depth(bounds, right, y)
    );
}

#[test]
fn anchored_profile_can_generate_rare_repose_four_sites() {
    let mut engine = ClassicSandboxEngine::new(20, 10, 109, ClassicRainMode::Uniform);
    engine.set_experiment_profile_name("anchored").unwrap();
    let mut saw_anchor = false;
    for _ in 0..10_000 {
        if engine.sample_base_local_repose() == CLASSIC_REPOSE_ANCHOR {
            saw_anchor = true;
            break;
        }
    }
    assert!(saw_anchor, "anchored profile never produced repose=4");
}

#[test]
fn momentum_profile_allows_at_most_one_bonus_same_direction_diagonal() {
    let mut engine = ClassicSandboxEngine::new(18, 12, 113, ClassicRainMode::Uniform);
    engine.set_experiment_profile_name("momentum").unwrap();
    engine.local_repose.fill(CLASSIC_REPOSE_LOW);
    let bounds = engine.surface.viewport_bounds().expect("visible basin");
    let x = bounds.x_start + (bounds.x_end - bounds.x_start) / 2;
    let y = bounds.y_start + 2;
    let category = CategoryId(1);

    // Build a receiving surface two dots below the prospective bonus target:
    // deep enough for momentum, but still in the local slope-like band.
    engine.surface.grid[y][x] = Some(category);
    engine.surface.grid[y + 1][x] = Some(category);
    engine.surface.grid[y + 4][x - 2] = Some(category);
    engine.surface.grid[y + 4][x + 2] = Some(category);
    engine.move_grain_once(bounds, x, y);

    let occupied = [(x - 2, y + 2), (x + 2, y + 2)]
        .into_iter()
        .filter(|(px, py)| engine.surface.grid[*py][*px] == Some(category))
        .count();
    assert_eq!(
        occupied, 1,
        "momentum must add exactly one same-direction bonus hop on slope-like relief"
    );
    assert_eq!(engine.diagonal_moves, 2);
    assert_eq!(engine.surface.physical_grain_count(), 4);
}

#[test]
fn momentum_profile_keeps_bonus_at_upper_slope_band_boundary() {
    let mut engine = ClassicSandboxEngine::new(18, 14, 115, ClassicRainMode::Uniform);
    engine.set_experiment_profile_name("momentum").unwrap();
    engine.local_repose.fill(CLASSIC_REPOSE_LOW);
    let bounds = engine.surface.viewport_bounds().expect("visible basin");
    let x = bounds.x_start + (bounds.x_end - bounds.x_start) / 2;
    let y = bounds.y_start + 2;
    let category = CategoryId(1);

    // Support at y+5 gives the prospective bonus column drop depth 3, the
    // upper edge of the accepted local slope band.
    engine.surface.grid[y][x] = Some(category);
    engine.surface.grid[y + 1][x] = Some(category);
    engine.surface.grid[y + 5][x - 2] = Some(category);
    engine.surface.grid[y + 5][x + 2] = Some(category);
    engine.move_grain_once(bounds, x, y);

    let bonus_targets = [(x - 2, y + 2), (x + 2, y + 2)]
        .into_iter()
        .filter(|(px, py)| engine.surface.grid[*py][*px] == Some(category))
        .count();
    assert_eq!(bonus_targets, 1, "drop depth 3 must retain momentum");
    assert_eq!(engine.diagonal_moves, 2);
    assert_eq!(engine.surface.physical_grain_count(), 4);
}

#[test]
fn momentum_profile_suppresses_bonus_hop_on_cliff_like_relief() {
    let mut engine = ClassicSandboxEngine::new(18, 16, 117, ClassicRainMode::Uniform);
    engine.set_experiment_profile_name("momentum").unwrap();
    engine.local_repose.fill(CLASSIC_REPOSE_LOW);
    let bounds = engine.surface.viewport_bounds().expect("visible basin");
    let x = bounds.x_start + (bounds.x_end - bounds.x_start) / 2;
    let y = bounds.y_start + 2;
    let category = CategoryId(1);

    // The ordinary Classic diagonal is valid, but both possible continuation
    // columns are deep open cliffs. Momentum must stop after that first hop.
    engine.surface.grid[y][x] = Some(category);
    engine.surface.grid[y + 1][x] = Some(category);
    engine.move_grain_once(bounds, x, y);

    let ordinary_targets = [(x - 1, y + 1), (x + 1, y + 1)]
        .into_iter()
        .filter(|(px, py)| engine.surface.grid[*py][*px] == Some(category))
        .count();
    let bonus_targets = [(x - 2, y + 2), (x + 2, y + 2)]
        .into_iter()
        .filter(|(px, py)| engine.surface.grid[*py][*px] == Some(category))
        .count();

    assert_eq!(ordinary_targets, 1, "ordinary Classic diagonal must remain");
    assert_eq!(bonus_targets, 0, "cliff-like relief must suppress momentum");
    assert_eq!(engine.diagonal_moves, 1);
    assert_eq!(engine.surface.physical_grain_count(), 2);
}

#[test]
fn momentum_repose_profile_tracks_the_local_repose_band() {
    let mut engine = ClassicSandboxEngine::new(18, 14, 119, ClassicRainMode::Uniform);
    engine
        .set_experiment_profile_name("momentum-repose")
        .unwrap();
    let bounds = engine.surface.viewport_bounds().expect("visible basin");
    let x = bounds.x_start + (bounds.x_end - bounds.x_start) / 2;
    let target_x = x + 1;
    let y = bounds.y_start + 2;

    engine.local_repose[x] = CLASSIC_REPOSE_LOW;
    assert!(engine.momentum_bonus_is_eligible(bounds, x, target_x, y, 1, 1));
    assert!(engine.momentum_bonus_is_eligible(bounds, x, target_x, y, 1, 2));
    assert!(!engine.momentum_bonus_is_eligible(bounds, x, target_x, y, 1, 3));

    engine.local_repose[x] = CLASSIC_REPOSE_HIGH;
    assert!(!engine.momentum_bonus_is_eligible(bounds, x, target_x, y, 1, 2));
    assert!(engine.momentum_bonus_is_eligible(bounds, x, target_x, y, 1, 3));
    assert!(engine.momentum_bonus_is_eligible(bounds, x, target_x, y, 1, 4));
    assert!(!engine.momentum_bonus_is_eligible(bounds, x, target_x, y, 1, 5));
}

#[test]
fn momentum_tangent_profile_requires_bounded_forward_surface_continuity() {
    let mut engine = ClassicSandboxEngine::new(18, 16, 121, ClassicRainMode::Uniform);
    engine
        .set_experiment_profile_name("momentum-tangent")
        .unwrap();
    let bounds = engine.surface.viewport_bounds().expect("visible basin");
    let x = bounds.x_start + (bounds.x_end - bounds.x_start) / 2;
    let target_x = x + 1;
    let forward_x = x + 2;
    let y = bounds.y_start + 2;
    let category = CategoryId(1);

    // A coherent diagonal keeps the prospective drop roughly constant after
    // advancing one row and one column: 1 now, 1 at the forward sample.
    engine.surface.grid[y + 2][target_x] = Some(category);
    engine.surface.grid[y + 3][forward_x] = Some(category);
    assert!(engine.momentum_bonus_is_eligible(bounds, x, target_x, y, 1, 1));

    // A sharp kink breaks local tangent continuity even though the first
    // receiving column alone is still close enough to the grain.
    engine.surface.grid[y + 3][forward_x] = None;
    engine.surface.grid[y + 5][forward_x] = Some(category);
    assert!(!engine.momentum_bonus_is_eligible(bounds, x, target_x, y, 1, 1));

    // Deep open walls remain ineligible even if distant surfaces happen to be
    // mutually parallel; tangent mode must not restore lateral-looking cliffs.
    engine.surface.grid[y + 2][target_x] = None;
    engine.surface.grid[y + 5][forward_x] = None;
    assert!(!engine.momentum_bonus_is_eligible(bounds, x, target_x, y, 1, 8));
}

#[test]
fn momentum_soft_profile_has_exact_repose_certain_one_extra_dot_soft_and_cliff_off() {
    let mut engine = ClassicSandboxEngine::new(18, 14, 123, ClassicRainMode::Uniform);
    engine.set_experiment_profile_name("momentum-soft").unwrap();
    let bounds = engine.surface.viewport_bounds().expect("visible basin");
    let x = bounds.x_start + (bounds.x_end - bounds.x_start) / 2;
    let target_x = x + 1;
    let y = bounds.y_start + 2;

    engine.local_repose[x] = CLASSIC_REPOSE_MID;
    assert!(engine.momentum_bonus_is_eligible(bounds, x, target_x, y, 1, 2));
    assert!(!engine.momentum_bonus_is_eligible(bounds, x, target_x, y, 1, 4));

    let mut accepted = 0usize;
    let samples = 100usize;
    for _ in 0..samples {
        if engine.momentum_bonus_is_eligible(bounds, x, target_x, y, 1, 3) {
            accepted += 1;
        }
    }
    assert!(
        (40..=80).contains(&accepted),
        "60% soft edge was not observable: accepted={accepted}/{samples}"
    );
}

#[test]
fn momentum_control_can_amplify_an_airborne_blocker_into_a_bonus_lane() {
    let mut engine = ClassicSandboxEngine::new(18, 14, 129, ClassicRainMode::Uniform);
    engine.set_experiment_profile_name("momentum").unwrap();
    engine.local_repose.fill(CLASSIC_REPOSE_LOW);
    let bounds = engine.surface.viewport_bounds().expect("visible basin");
    let x = bounds.x_start + (bounds.x_end - bounds.x_start) / 2;
    let y = bounds.y_start + 2;
    let category = CategoryId(1);

    engine.surface.grid[y][x] = Some(category);
    engine.surface.grid[y + 1][x] = Some(category);
    engine.surface.grid[y + 4][x - 2] = Some(category);
    engine.surface.grid[y + 4][x + 2] = Some(category);
    assert!(!engine.direct_blocker_is_grounded(bounds, x, y + 1));

    engine.move_grain_once(bounds, x, y);
    let bonus_targets = [(x - 2, y + 2), (x + 2, y + 2)]
        .into_iter()
        .filter(|(px, py)| engine.surface.grid[*py][*px] == Some(category))
        .count();
    assert_eq!(
        bonus_targets, 1,
        "CLASSIC-007 control should expose the airborne-collision amplification under test"
    );
    assert_eq!(engine.diagonal_moves, 2);
}

#[test]
fn momentum_contact_suppresses_only_the_bonus_after_an_airborne_blocker() {
    let mut engine = ClassicSandboxEngine::new(18, 14, 129, ClassicRainMode::Uniform);
    engine
        .set_experiment_profile_name("momentum-contact")
        .unwrap();
    engine.local_repose.fill(CLASSIC_REPOSE_LOW);
    let bounds = engine.surface.viewport_bounds().expect("visible basin");
    let x = bounds.x_start + (bounds.x_end - bounds.x_start) / 2;
    let y = bounds.y_start + 2;
    let category = CategoryId(1);

    // The direct blocker has air beneath it, so it is another falling grain,
    // not the supported pile. The ordinary Classic diagonal remains legal and
    // the receiving columns are deliberately in the fixed momentum depth band.
    engine.surface.grid[y][x] = Some(category);
    engine.surface.grid[y + 1][x] = Some(category);
    engine.surface.grid[y + 4][x - 2] = Some(category);
    engine.surface.grid[y + 4][x + 2] = Some(category);
    let expected_mass = engine.surface.physical_grain_count();

    assert!(!engine.direct_blocker_is_grounded(bounds, x, y + 1));
    engine.move_grain_once(bounds, x, y);

    let ordinary_targets = [(x - 1, y + 1), (x + 1, y + 1)]
        .into_iter()
        .filter(|(px, py)| engine.surface.grid[*py][*px] == Some(category))
        .count();
    let bonus_targets = [(x - 2, y + 2), (x + 2, y + 2)]
        .into_iter()
        .filter(|(px, py)| engine.surface.grid[*py][*px] == Some(category))
        .count();

    assert_eq!(ordinary_targets, 1, "ordinary Classic diagonal must remain");
    assert_eq!(bonus_targets, 0, "airborne blockers must not seed a momentum lane");
    assert_eq!(engine.diagonal_moves, 1);
    assert_eq!(engine.surface.physical_grain_count(), expected_mass);
}

#[test]
fn momentum_contact_keeps_the_bonus_when_the_blocker_is_supported_pile() {
    let mut engine = ClassicSandboxEngine::new(18, 18, 131, ClassicRainMode::Uniform);
    engine
        .set_experiment_profile_name("momentum-contact")
        .unwrap();
    engine.local_repose.fill(CLASSIC_REPOSE_LOW);
    let bounds = engine.surface.viewport_bounds().expect("visible basin");
    let x = bounds.x_start + (bounds.x_end - bounds.x_start) / 2;
    let floor = bounds.y_end - 1;
    let y = floor - 5;
    let category = CategoryId(1);

    engine.surface.grid[y][x] = Some(category);
    for row in y + 1..=floor {
        engine.surface.grid[row][x] = Some(category);
    }
    // Both possible bonus receiving columns have drop depth exactly two and
    // are themselves ordinary bottom-connected pile columns.
    for target_x in [x - 2, x + 2] {
        for row in y + 4..=floor {
            engine.surface.grid[row][target_x] = Some(category);
        }
    }
    let expected_mass = engine.surface.physical_grain_count();

    assert!(engine.direct_blocker_is_grounded(bounds, x, y + 1));
    engine.move_grain_once(bounds, x, y);

    let bonus_targets = [(x - 2, y + 2), (x + 2, y + 2)]
        .into_iter()
        .filter(|(px, py)| engine.surface.grid[*py][*px] == Some(category))
        .count();
    assert_eq!(bonus_targets, 1, "supported surface contact must retain momentum");
    assert_eq!(engine.diagonal_moves, 2);
    assert_eq!(engine.surface.physical_grain_count(), expected_mass);
}

#[test]
fn momentum_repose_contact_keeps_repose_gate_but_rejects_airborne_collision() {
    let mut engine = ClassicSandboxEngine::new(18, 14, 133, ClassicRainMode::Uniform);
    engine
        .set_experiment_profile_name("momentum-repose-contact")
        .unwrap();
    engine.local_repose.fill(CLASSIC_REPOSE_LOW);
    let bounds = engine.surface.viewport_bounds().expect("visible basin");
    let x = bounds.x_start + (bounds.x_end - bounds.x_start) / 2;
    let target_x = x + 1;
    let y = bounds.y_start + 2;
    let category = CategoryId(1);

    // The profile retains A's exact repose-relative gate.
    assert!(engine.momentum_bonus_is_eligible(bounds, x, target_x, y, 1, 1));
    assert!(engine.momentum_bonus_is_eligible(bounds, x, target_x, y, 1, 2));
    assert!(!engine.momentum_bonus_is_eligible(bounds, x, target_x, y, 1, 3));

    // But an airborne direct blocker suppresses the bonus even when the
    // receiving relief itself would pass that repose-relative gate.
    engine.surface.grid[y][x] = Some(category);
    engine.surface.grid[y + 1][x] = Some(category);
    engine.surface.grid[y + 4][x - 2] = Some(category);
    engine.surface.grid[y + 4][x + 2] = Some(category);
    engine.move_grain_once(bounds, x, y);
    let bonus_targets = [(x - 2, y + 2), (x + 2, y + 2)]
        .into_iter()
        .filter(|(px, py)| engine.surface.grid[*py][*px] == Some(category))
        .count();
    assert_eq!(bonus_targets, 0);
    assert_eq!(engine.diagonal_moves, 1);
}

#[test]
fn classic_009_contact_profiles_remain_mass_conserving_and_observable() {
    for profile in ["momentum-contact", "momentum-repose-contact"] {
        let metrics = experiment_metrics(profile);
        eprintln!(
            "CLASSIC_009_METRICS profile={profile} roughness={} apex={} width={} diagonal_moves={}",
            metrics.0, metrics.1, metrics.2, metrics.3
        );
        assert!(metrics.1 > 0, "profile={profile}");
        assert!(metrics.2 > 0, "profile={profile}");
        assert!(metrics.3 > 0, "profile={profile}");
    }
}

#[test]
fn classic_008_momentum_profiles_remain_mass_conserving_and_observable() {
    for profile in [
        "momentum",
        "momentum-repose",
        "momentum-tangent",
        "momentum-soft",
    ] {
        let metrics = experiment_metrics(profile);
        eprintln!(
            "CLASSIC_008_METRICS profile={profile} roughness={} apex={} width={} diagonal_moves={}",
            metrics.0, metrics.1, metrics.2, metrics.3
        );
        assert!(metrics.1 > 0, "profile={profile}");
        assert!(metrics.2 > 0, "profile={profile}");
        assert!(metrics.3 > 0, "profile={profile}");
    }
}

fn experiment_metrics(profile: &str) -> (usize, usize, usize, usize) {
    let mut engine = ClassicSandboxEngine::new(30, 10, 127, ClassicRainMode::Uniform);
    engine.set_experiment_profile_name(profile).unwrap();
    install_centered_compact_wall(&mut engine, 30, 24);
    let expected_mass = engine.surface.physical_grain_count();
    relax_fixed(&mut engine, 800);
    assert_eq!(engine.surface.physical_grain_count(), expected_mass);
    let heights = supported_height_profile(&engine);
    (
        second_difference_roughness(&heights),
        apex_height(&heights),
        footprint_width(&heights),
        engine.diagonal_moves,
    )
}

#[test]
fn classic_experiment_ladder_remains_mass_conserving_and_observable() {
    for profile in [
        "rugged",
        "memory",
        "slope",
        "memory-slope",
        "anchored",
        "momentum",
    ] {
        let metrics = experiment_metrics(profile);
        eprintln!(
            "CLASSIC_005_METRICS profile={profile} roughness={} apex={} width={} diagonal_moves={}",
            metrics.0, metrics.1, metrics.2, metrics.3
        );
        assert!(metrics.1 > 0, "profile={profile}");
        assert!(metrics.2 > 0, "profile={profile}");
        assert!(metrics.3 > 0, "profile={profile}");
    }
}
