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
    high.local_repose[x] = CLASSIC_REPOSE_HIGH;

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
    baseline.force_uniform_repose = true;
    baseline.local_repose.fill(CLASSIC_REPOSE_LOW);

    let mut heterogeneous = ClassicSandboxEngine::new(30, 10, 7, ClassicRainMode::Uniform);
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
fn repose_percentage_tuning_is_nonretroactive_and_fill_uses_the_new_value() {
    let mut engine = ClassicSandboxEngine::new(20, 8, 29, ClassicRainMode::Uniform);
    assert_eq!(
        engine.high_repose_percent(),
        ClassicSandboxEngine::default_high_repose_percent()
    );

    let before = engine.local_repose.clone();
    engine
        .set_high_repose_percent(100)
        .expect("100 percent repose tuning");
    assert_eq!(engine.local_repose, before, "tuning must not rewrite live columns");
    let categories = [CategoryId(1), CategoryId(2)];
    engine
        .debug_fill_rainbow_80(&categories)
        .expect("fill after 100 percent repose tuning");
    assert!(
        engine
            .local_repose
            .iter()
            .all(|repose| *repose == CLASSIC_REPOSE_HIGH)
    );

    engine
        .set_high_repose_percent(0)
        .expect("zero percent repose tuning");
    engine
        .debug_fill_rainbow_80(&categories)
        .expect("fill after zero percent repose tuning");
    assert!(
        engine
            .local_repose
            .iter()
            .all(|repose| *repose == CLASSIC_REPOSE_LOW)
    );

    engine.reset_high_repose_percent();
    assert_eq!(
        engine.high_repose_percent(),
        ClassicSandboxEngine::default_high_repose_percent()
    );
    assert!(engine.set_high_repose_percent(101).is_err());
}

#[test]
fn tuned_repose_percentage_remains_deterministic_for_a_fixed_seed() {
    let mut first = ClassicSandboxEngine::new(30, 10, 71, ClassicRainMode::Uniform);
    let mut second = ClassicSandboxEngine::new(30, 10, 71, ClassicRainMode::Uniform);
    first.set_high_repose_percent(20).unwrap();
    second.set_high_repose_percent(20).unwrap();
    first.clear();
    second.clear();
    assert_eq!(first.local_repose, second.local_repose);
    assert_eq!(first.repose_rng_state, second.repose_rng_state);
}
