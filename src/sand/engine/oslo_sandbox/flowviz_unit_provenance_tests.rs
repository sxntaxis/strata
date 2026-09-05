use super::*;

const TEST_SEED: u64 = 0x51A7_0157_D1A6_0001;

fn grain(id: u64) -> CategoryId {
    CategoryId::new(id)
}

#[derive(Debug)]
struct RainbowMotionTrace {
    category_id: CategoryId,
    start_site: usize,
    last_site: usize,
    seen_sequences: std::collections::BTreeSet<u64>,
    path: Vec<(u64, usize, usize)>,
    settled_site: Option<usize>,
    settled_depth: Option<usize>,
    discharged: bool,
}

fn capture_rainbow_motion_trace(
    engine: &OsloSandboxEngine,
    traces: &mut std::collections::BTreeMap<flowviz_unit::FlowVizMotionId, RainbowMotionTrace>,
) {
    for (id, carrier) in &engine.flowviz_unit_carriers {
        let first_segment = carrier
            .active
            .map(|active| active.segment)
            .or_else(|| carrier.queued.front().copied());
        let start_site = first_segment
            .map(|segment| segment.source)
            .unwrap_or_else(|| carrier.x.max(0.0).round() as usize);
        let trace = traces.entry(*id).or_insert_with(|| RainbowMotionTrace {
            category_id: carrier.category_id,
            start_site,
            last_site: start_site,
            seen_sequences: std::collections::BTreeSet::new(),
            path: Vec::new(),
            settled_site: None,
            settled_depth: None,
            discharged: false,
        });

        let segments = carrier
            .active
            .into_iter()
            .map(|active| active.segment)
            .chain(carrier.queued.iter().copied());
        for segment in segments {
            if !trace.seen_sequences.insert(segment.sequence) {
                continue;
            }
            if let Some(destination) = segment.destination {
                assert_eq!(
                    segment.source.abs_diff(destination),
                    1,
                    "diagnostic trace observed a non-adjacent authoritative hop"
                );
                trace.last_site = destination;
                trace
                    .path
                    .push((segment.sequence, segment.source, destination));
            }
        }

        match carrier.physical {
            flowviz_unit::UnitPhysicalState::Rolling => {}
            flowviz_unit::UnitPhysicalState::Settled(site) => {
                trace.settled_site = Some(site);
                trace.last_site = site;
                if let Some(depth) = engine
                    .flowviz_unit_custody
                    .get(site)
                    .and_then(|custody| custody.iter().position(|value| *value == Some(*id)))
                {
                    trace.settled_depth = Some(depth);
                }
            }
            flowviz_unit::UnitPhysicalState::Discharged => {
                trace.discharged = true;
            }
        }
    }
}

fn category_column_counts(
    columns: &[Vec<CategoryId>],
) -> std::collections::HashMap<CategoryId, usize> {
    let mut counts = std::collections::HashMap::new();
    for category_id in columns.iter().flatten().copied() {
        *counts.entry(category_id).or_insert(0) += 1;
    }
    counts
}

/// Diagnostic only: this does not alter runtime physics or presentation. It
/// observes the already-certified 015G MotionId/adjacent-edge facts while the
/// deterministic rainbow wall collapses, so owner-visible cyan/green behavior
/// can be classified as authoritative transport versus presentation artifact.
#[test]
#[ignore = "diagnostic rainbow provenance trace for owner review"]
fn unit_visual_support_rainbow_provenance_diagnostic() {
    let categories = [
        grain(101),
        grain(102),
        grain(103),
        grain(104),
        grain(105),
        grain(106),
    ];
    let labels = ["green", "yellow", "red", "purple", "blue", "cyan"];
    let mut engine =
        OsloSandboxEngine::new_front_unit_visual_support_flowviz_vessel(40, 16, TEST_SEED);
    engine.debug_fill_rainbow_80(&categories).unwrap();

    let initial_columns = engine.columns.clone();
    let initial_counts = category_column_counts(&initial_columns);
    let initial_sites = initial_columns
        .iter()
        .enumerate()
        .filter_map(|(site, column)| (!column.is_empty()).then_some(site))
        .collect::<std::collections::BTreeSet<_>>();
    let green_band_height = initial_columns
        .iter()
        .find(|column| !column.is_empty())
        .map(|column| {
            column
                .iter()
                .take_while(|category_id| **category_id == categories[0])
                .count()
        })
        .unwrap_or(0);

    let mut traces = std::collections::BTreeMap::new();
    let mut previous_hops = std::collections::HashMap::<CategoryId, usize>::new();
    let mut physical_batches = 0usize;
    let mut guard = 0usize;

    loop {
        guard = guard.saturating_add(1);
        assert!(guard < 2_000_000, "rainbow diagnostic failed to quiesce");
        capture_rainbow_motion_trace(&engine, &mut traces);

        let batch_pending = engine.flowviz_unit_coherent_batch_pending();
        let visible_flow = engine.visible_flow_active();
        let visual_motion = engine.rolling_visual_motion_active();
        let changed = if batch_pending {
            engine.advance_testing_truthful_visual_frame()
        } else if visible_flow {
            physical_batches = physical_batches.saturating_add(1);
            engine.advance_testing_coherent_flow_frame()
        } else if visual_motion {
            engine.advance_testing_truthful_visual_frame()
        } else {
            false
        };

        capture_rainbow_motion_trace(&engine, &mut traces);

        if !batch_pending && visible_flow && physical_batches <= 16 {
            let mut totals = std::collections::HashMap::<CategoryId, usize>::new();
            for trace in traces.values() {
                *totals.entry(trace.category_id).or_insert(0) += trace.path.len();
            }
            let deltas = categories.map(|category_id| {
                let total = totals.get(&category_id).copied().unwrap_or(0);
                let previous = previous_hops.insert(category_id, total).unwrap_or(0);
                total.saturating_sub(previous)
            });
            eprintln!(
                "RAINBOW_BATCH batch={} green_hops={} yellow_hops={} red_hops={} purple_hops={} blue_hops={} cyan_hops={}",
                physical_batches, deltas[0], deltas[1], deltas[2], deltas[3], deltas[4], deltas[5]
            );
        }

        if !changed
            && engine.active_sites.is_empty()
            && !engine.explicit_flow_active()
            && !engine.rolling_visual_motion_active()
        {
            break;
        }
    }

    capture_rainbow_motion_trace(&engine, &mut traces);
    let final_counts = category_column_counts(&engine.columns);

    for (category_id, label) in categories.into_iter().zip(labels) {
        let category_traces = traces
            .values()
            .filter(|trace| trace.category_id == category_id)
            .collect::<Vec<_>>();
        let movement_entries = category_traces.len();
        let hops = category_traces
            .iter()
            .map(|trace| trace.path.len())
            .sum::<usize>();
        let max_hops = category_traces
            .iter()
            .map(|trace| trace.path.len())
            .max()
            .unwrap_or(0);
        let max_abs_dx = category_traces
            .iter()
            .map(|trace| trace.start_site.abs_diff(trace.last_site))
            .max()
            .unwrap_or(0);
        let discharged = category_traces
            .iter()
            .filter(|trace| trace.discharged)
            .count();
        let initial = initial_counts.get(&category_id).copied().unwrap_or(0);
        let final_count = final_counts.get(&category_id).copied().unwrap_or(0);
        assert_eq!(
            initial,
            final_count.saturating_add(discharged),
            "diagnostic category mass must reconcile"
        );
        eprintln!(
            "RAINBOW_TRACE layer={label} category={} initial={} movement_entries={} hops={} max_hops={} max_abs_dx={} discharged={} final={}",
            category_id.0,
            initial,
            movement_entries,
            hops,
            max_hops,
            max_abs_dx,
            discharged,
            final_count
        );
    }

    let green_final_inside_original = engine
        .columns
        .iter()
        .enumerate()
        .filter(|(site, _)| initial_sites.contains(site))
        .flat_map(|(_, column)| column.iter())
        .filter(|category_id| **category_id == categories[0])
        .count();
    let green_final_outside_original = final_counts
        .get(&categories[0])
        .copied()
        .unwrap_or(0)
        .saturating_sub(green_final_inside_original);
    eprintln!(
        "RAINBOW_GREEN_MASS initial={} final_inside_original_footprint={} final_outside_original_footprint={} movement_entries={} note=movement_entries_are_motion_episodes_not_persistent_grain_identity",
        initial_counts.get(&categories[0]).copied().unwrap_or(0),
        green_final_inside_original,
        green_final_outside_original,
        traces
            .values()
            .filter(|trace| trace.category_id == categories[0])
            .count()
    );

    let cyan_low_final = engine
        .columns
        .iter()
        .flat_map(|column| column.iter().enumerate())
        .filter(|(depth, category_id)| *depth < green_band_height && **category_id == categories[5])
        .count();
    let mut cyan_low_traces = traces
        .iter()
        .filter(|(_, trace)| {
            trace.category_id == categories[5]
                && trace
                    .settled_depth
                    .is_some_and(|depth| depth < green_band_height)
        })
        .collect::<Vec<_>>();
    cyan_low_traces.sort_by_key(|(_, trace)| std::cmp::Reverse(trace.path.len()));
    eprintln!(
        "RAINBOW_CYAN_LOW final_units_below_initial_green_band={} traced_settlement_episodes_below_band={} initial_green_band_height={}",
        cyan_low_final,
        cyan_low_traces.len(),
        green_band_height
    );
    for (id, trace) in cyan_low_traces.into_iter().take(12) {
        let mut path = trace.path.clone();
        path.sort_by_key(|(sequence, _, _)| *sequence);
        let compact_path = path
            .iter()
            .map(|(_, source, destination)| format!("{source}->{destination}"))
            .collect::<Vec<_>>()
            .join(",");
        eprintln!(
            "RAINBOW_CYAN_LOW_PATH motion={} start={} final={} depth={} hops={} path={}",
            id.0,
            trace.start_site,
            trace.last_site,
            trace.settled_depth.unwrap_or(usize::MAX),
            trace.path.len(),
            compact_path
        );
    }

    assert_eq!(engine.flowviz_shadow_columns, engine.columns);
    assert_eq!(engine.flowviz_unit_misses(), 0);
}
