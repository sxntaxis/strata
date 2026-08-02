from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    target = Path(path)
    text = target.read_text()
    if text.count(old) != 1:
        raise SystemExit(f"expected one match in {path}: {old[:120]!r}")
    target.write_text(text.replace(old, new, 1))


replace_once(
    "src/sand/mod.rs",
    "mod engine;\nmod recovery;\n\n#[allow(unused_imports)]\npub use engine::{PendingGrainRun, SandEngine, SandState, SandStateGrain};\npub(crate) use recovery::{RecoveryTiming, recover_detached_sediment};\n",
    "mod engine;\nmod recovery;\nmod snapshot;\n\n#[allow(unused_imports)]\npub use engine::{PendingGrainRun, SandEngine, SandState, SandStateGrain};\npub(crate) use recovery::{RecoveryTiming, recover_detached_sediment};\npub use snapshot::{\n    SedimentIdlePolicy, SedimentSnapshot, SedimentSnapshotKind, SedimentSnapshotProvenance,\n    stable_source_revision,\n};\npub(crate) use snapshot::select_daily_artifact;\n",
)

replace_once(
    "src/app.rs",
    "    sand::{RecoveryTiming, SandEngine, SandState, SandStateGrain, recover_detached_sediment},",
    "    sand::{\n        RecoveryTiming, SandEngine, SandState, SandStateGrain, SedimentSnapshot,\n        recover_detached_sediment,\n    },",
)
replace_once(
    "src/app.rs",
    "    report_snapshot_end_day: Option<String>,\n    report_snapshot_state: Option<crate::sand::SandState>,\n    report_snapshot_preview_key: Option<String>,\n    report_snapshot_preview_engine: Option<SandEngine>,",
    "    report_snapshot_end_day: Option<String>,\n    report_snapshot_artifact: Option<SedimentSnapshot>,\n    report_snapshot_preview_key: Option<String>,\n    report_snapshot_preview_lines: Option<Vec<ratatui::text::Line<'static>>>,",
)
replace_once(
    "src/app.rs",
    "            report_snapshot_end_day: None,\n            report_snapshot_state: None,\n            report_snapshot_preview_key: None,\n            report_snapshot_preview_engine: None,",
    "            report_snapshot_end_day: None,\n            report_snapshot_artifact: None,\n            report_snapshot_preview_key: None,\n            report_snapshot_preview_lines: None,",
)

report = Path("src/app/report_state.rs")
text = report.read_text()
text = text.replace(
    "use std::collections::HashSet;",
    "use std::fmt::Write as _;",
    1,
)
text = text.replace(
    "use crate::sand::{SandEngine, SandState, SandStateGrain};",
    "use crate::sand::{\n    SandState, SandStateGrain, SedimentSnapshot, select_daily_artifact,\n    stable_source_revision,\n};",
    1,
)
old_lines = '''    pub(super) fn report_snapshot_lines(
        &mut self,
        width: u16,
        height: u16,
        _categories: &[Category],
    ) -> Option<Vec<Line<'static>>> {
        self.refresh_report_snapshot_cache();
        let state = self.report_snapshot_state.clone()?;

        let categories = self.report_categories();
        let valid_category_ids: HashSet<CategoryId> =
            categories.iter().map(|category| category.id).collect();

        let cache_key = format!(
            "{}:{}:{}:{}",
            self.report_snapshot_end_day.as_deref().unwrap_or_default(),
            width,
            height,
            state.grains.len()
        );

        let should_rebuild_preview = self
            .report_snapshot_preview_key
            .as_deref()
            .map(|key| key != cache_key.as_str())
            .unwrap_or(true)
            || self.report_snapshot_preview_engine.is_none();

        if should_rebuild_preview {
            let mut preview_engine = SandEngine::new(width, height);
            preview_engine.restore_state(&state, &valid_category_ids);
            self.report_snapshot_preview_engine = Some(preview_engine);
            self.report_snapshot_preview_key = Some(cache_key);
        }

        let preview_engine = self.report_snapshot_preview_engine.as_mut()?;
        preview_engine.update();
        Some(preview_engine.render(&categories))
    }
'''
new_lines = '''    pub(super) fn report_snapshot_lines(
        &mut self,
        width: u16,
        height: u16,
        _categories: &[Category],
    ) -> Option<Vec<Line<'static>>> {
        self.refresh_report_snapshot_cache();
        let snapshot = self.report_snapshot_artifact.clone()?;
        let cache_key = snapshot.render_cache_key(width, height);

        let should_rebuild_preview = self
            .report_snapshot_preview_key
            .as_deref()
            .map(|key| key != cache_key.as_str())
            .unwrap_or(true)
            || self.report_snapshot_preview_lines.is_none();

        if should_rebuild_preview {
            let categories = self.report_categories();
            self.report_snapshot_preview_lines =
                Some(snapshot.render_immutable(width, height, &categories));
            self.report_snapshot_preview_key = Some(cache_key);
        }

        self.report_snapshot_preview_lines.clone()
    }

    pub(super) fn report_snapshot_status_label(&self) -> String {
        if !self.should_use_report_snapshot() {
            return "live sediment".to_string();
        }

        self.report_snapshot_artifact
            .as_ref()
            .map(SedimentSnapshot::display_label)
            .unwrap_or_else(|| "historical sediment unavailable".to_string())
    }
'''
if text.count(old_lines) != 1:
    raise SystemExit("report snapshot rendering function did not match")
text = text.replace(old_lines, new_lines, 1)
text = text.replace(
    "        self.report_snapshot_state = None;\n        self.report_snapshot_preview_key = None;\n        self.report_snapshot_preview_engine = None;",
    "        self.report_snapshot_artifact = None;\n        self.report_snapshot_preview_key = None;\n        self.report_snapshot_preview_lines = None;",
    1,
)
old_refresh = '''        self.report_snapshot_end_day = Some(key);
        self.report_snapshot_state = self
            .load_daily_sand_snapshot(end_day)
            .or_else(|| self.synthetic_snapshot_from_time_log(end_day));
        self.report_snapshot_preview_key = None;
        self.report_snapshot_preview_engine = None;
'''
new_refresh = '''        let persisted = self
            .load_daily_sand_snapshot(end_day)
            .map(|state| SedimentSnapshot::legacy_daily_payload(key.clone(), state));
        let derived = self.synthetic_snapshot_from_time_log(end_day);

        self.report_snapshot_end_day = Some(key.clone());
        self.report_snapshot_artifact = select_daily_artifact(&key, persisted, derived);
        self.report_snapshot_preview_key = None;
        self.report_snapshot_preview_lines = None;
'''
if text.count(old_refresh) != 1:
    raise SystemExit("report snapshot refresh block did not match")
text = text.replace(old_refresh, new_refresh, 1)
old_rebuild = '''        if let Some(state) = self.synthetic_snapshot_from_time_log(end_day) {
            self.save_daily_sand_snapshot(end_day, &state);
            self.report_snapshot_state = Some(state);
        } else {
            self.delete_daily_sand_snapshot(end_day);
            self.report_snapshot_state = None;
        }

        self.report_snapshot_end_day = Some(key);
        self.report_snapshot_preview_key = None;
        self.report_snapshot_preview_engine = None;
'''
new_rebuild = '''        self.report_snapshot_artifact = self.synthetic_snapshot_from_time_log(end_day);
        self.report_snapshot_end_day = Some(key);
        self.report_snapshot_preview_key = None;
        self.report_snapshot_preview_lines = None;
'''
if text.count(old_rebuild) != 1:
    raise SystemExit("report snapshot rebuild block did not match")
text = text.replace(old_rebuild, new_rebuild, 1)
text = text.replace(
    "    fn synthetic_snapshot_from_time_log(&self, day: NaiveDate) -> Option<SandState> {",
    "    fn synthetic_snapshot_from_time_log(&self, day: NaiveDate) -> Option<SedimentSnapshot> {",
    1,
)
old_sort = '''        day_sessions.sort_by(|a, b| a.2.cmp(&b.2).then(a.3.cmp(&b.3)).then(a.4.cmp(&b.4)));

        let grid_width = self.sand_engine.grid_width_dots;
'''
new_sort = '''        day_sessions.sort_by(|a, b| a.2.cmp(&b.2).then(a.3.cmp(&b.3)).then(a.4.cmp(&b.4)));

        let day_key = day.format("%Y-%m-%d").to_string();
        let mut revision_material = format!("day={day_key}|idle=included|");
        for (category_id, seconds, start, end, session_id) in &day_sessions {
            let _ = write!(
                revision_material,
                "{category_id}:{seconds}:{start}:{end}:{session_id}|"
            );
        }
        let source_revision = stable_source_revision(revision_material.as_bytes());

        let grid_width = self.sand_engine.grid_width_dots;
'''
if text.count(old_sort) != 1:
    raise SystemExit("synthetic snapshot sort block did not match")
text = text.replace(old_sort, new_sort, 1)
old_return = '''        Some(SandState {
            version: SandState::VERSION,
            grid_width,
            grid_height,
            grains,
            frame_count: 0,
            sweep_left_to_right: true,
            rng_state: 0,
            pending_grains: Vec::new(),
            pending_runs: Vec::new(),
        })
'''
new_return = '''        let state = SandState {
            version: SandState::VERSION,
            grid_width,
            grid_height,
            grains,
            frame_count: 0,
            sweep_left_to_right: true,
            rng_state: 0,
            pending_grains: Vec::new(),
            pending_runs: Vec::new(),
        };

        Some(SedimentSnapshot::derived_preview(
            day_key,
            source_revision,
            state,
        ))
'''
if text.count(old_return) != 1:
    raise SystemExit("synthetic snapshot return block did not match")
text = text.replace(old_return, new_return, 1)
report.write_text(text)

modal = Path("src/app/report_modal_view.rs")
text = modal.read_text()
old_period = '''        let period_bottom_title = Line::from(vec![
            view_style::report_period_label_span("day", self.report_period == ReportPeriod::Today),
            Span::styled("·", Style::default().fg(Color::DarkGray)),
            view_style::report_period_label_span("week", self.report_period == ReportPeriod::Week),
            Span::styled("·", Style::default().fg(Color::DarkGray)),
            view_style::report_period_label_span(
                "month",
                self.report_period == ReportPeriod::Month,
            ),
        ])
        .alignment(Alignment::Center);

        let frame_block = Block::default()
            .title(interval_title)
            .title(center_title)
            .title(total_title)
            .title_bottom(period_bottom_title)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(border_color));
'''
new_period = '''        let period_bottom_title = Line::from(vec![
            view_style::report_period_label_span("day", self.report_period == ReportPeriod::Today),
            Span::styled("·", Style::default().fg(Color::DarkGray)),
            view_style::report_period_label_span("week", self.report_period == ReportPeriod::Week),
            Span::styled("·", Style::default().fg(Color::DarkGray)),
            view_style::report_period_label_span(
                "month",
                self.report_period == ReportPeriod::Month,
            ),
        ])
        .alignment(Alignment::Center);
        let snapshot_bottom_title = Line::from(Span::styled(
            self.report_snapshot_status_label(),
            Style::default().fg(Color::DarkGray),
        ))
        .alignment(Alignment::Left);

        let frame_block = Block::default()
            .title(interval_title)
            .title(center_title)
            .title(total_title)
            .title_bottom(snapshot_bottom_title)
            .title_bottom(period_bottom_title)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(border_color));
'''
if text.count(old_period) != 1:
    raise SystemExit("report modal title block did not match")
modal.write_text(text.replace(old_period, new_period, 1))

for temporary in [
    ".github/workflows/sediment001d1-apply.yml",
    "tools/sediment001d1-apply.py",
    "tools/sediment001d1.trigger",
]:
    Path(temporary).unlink(missing_ok=True)
