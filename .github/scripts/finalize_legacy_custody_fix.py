from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected one marker, found {count}")
    return text.replace(old, new, 1)


path = Path("src/app.rs")
text = path.read_text()
text = replace_once(
    text,
    '''            self.sand_engine
                .restore_state(&checkpoint.sand_state, &valid_category_ids);
            stage_clear_all_active_state(''',
    '''            self.sand_engine
                .restore_state(&checkpoint.sand_state, &valid_category_ids)?;
            stage_clear_all_active_state(''',
    "clear-all receipt restore",
)
rollback = '''self.sand_engine.restore_state(
                    &previous_sand,
                    &self
                        .time_tracker
                        .categories_for_storage()
                        .into_iter()
                        .chain(self.archived_categories.iter().cloned())
                        .map(|category| category.id)
                        .collect(),
                );'''
rollback_replacement = '''self.sand_engine
                    .restore_state(
                        &previous_sand,
                        &self
                            .time_tracker
                            .categories_for_storage()
                            .into_iter()
                            .chain(self.archived_categories.iter().cloned())
                            .map(|category| category.id)
                            .collect(),
                    )
                    .expect("captured rollback sediment must remain valid");'''
count = text.count(rollback)
if count != 3:
    raise SystemExit(f"rollback sediment restores: expected 3, found {count}")
text = text.replace(rollback, rollback_replacement)
rollback_less_indent = '''self.sand_engine.restore_state(
                &previous_sand,
                &self
                    .time_tracker
                    .categories_for_storage()
                    .into_iter()
                    .chain(self.archived_categories.iter().cloned())
                    .map(|category| category.id)
                    .collect(),
            );'''
rollback_less_replacement = '''self.sand_engine
                .restore_state(
                    &previous_sand,
                    &self
                        .time_tracker
                        .categories_for_storage()
                        .into_iter()
                        .chain(self.archived_categories.iter().cloned())
                        .map(|category| category.id)
                        .collect(),
                )
                .expect("captured rollback sediment must remain valid");'''
count = text.count(rollback_less_indent)
if count != 2:
    raise SystemExit(f"outer rollback sediment restores: expected 2, found {count}")
text = text.replace(rollback_less_indent, rollback_less_replacement)
text = replace_once(
    text,
    '''        self.sand_engine
            .restore_state(&settlement.state, &valid_category_ids);
        self.simulation.spawn_accumulator''',
    '''        self.sand_engine
            .restore_state(&settlement.state, &valid_category_ids)?;
        self.simulation.spawn_accumulator''',
    "transition settlement restore",
)
text = replace_once(
    text,
    '''            if let Some(engine) = self.simulation.catchup_visual_engine.as_mut() {
                engine.restore_state(&projected_state, &valid_category_ids);
            }
            self.simulation.catchup_visual_last_refresh''',
    '''            if let Some(engine) = self.simulation.catchup_visual_engine.as_mut() {
                engine
                    .restore_state(&projected_state, &valid_category_ids)
                    .ok()?;
            }
            self.simulation.catchup_visual_last_refresh''',
    "catch-up projection restore",
)
text = replace_once(
    text,
    '''            self.sand_engine
                .restore_state(&checkpoint.sand_state, &valid_category_ids);
            self.reconcile_all_daily_contributions();''',
    '''            self.sand_engine
                .restore_state(&checkpoint.sand_state, &valid_category_ids)?;
            self.reconcile_all_daily_contributions();''',
    "legacy finish replay restore",
)
text = replace_once(
    text,
    '''        self.sand_engine
            .restore_state(&recovered.state, &valid_category_ids);
        if !self''',
    '''        if let Err(error) = self
            .sand_engine
            .restore_state(&recovered.state, &valid_category_ids)
        {
            self.record_storage_result::<()>(Err(error));
            return false;
        }
        if !self''',
    "detached recovery restore",
)
path.write_text(text)

path = Path("src/sand/recovery.rs")
text = path.read_text()
text = replace_once(
    text,
    '        engine.restore_state(base_state, valid_category_ids);',
    '        engine.restore_state(base_state, valid_category_ids)?;',
    "bounded recovery base restore",
)
path.write_text(text)

path = Path("src/sand/snapshot.rs")
text = path.read_text()
text = replace_once(
    text,
    '        engine.restore_state(&self.state, &valid_category_ids);',
    '''        engine
            .restore_state(&self.state, &valid_category_ids)
            .expect("validated sediment snapshot must restore");''',
    "immutable snapshot restore",
)
path.write_text(text)

print("all sediment restore results handled")
