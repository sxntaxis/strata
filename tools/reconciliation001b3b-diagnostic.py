from pathlib import Path

path = Path("src/app.rs")
content = path.read_text()
old = '''        if let Err(error) = self.settle_transition_boundary(finished_at_utc) {
            self.record_storage_result_for::<()>(
'''
new = '''        if let Err(error) = self.settle_transition_boundary(finished_at_utc) {
            eprintln!("transition settlement failure: {error}");
            self.record_storage_result_for::<()>(
'''
if old not in content:
    raise SystemExit("finish settlement diagnostic marker missing")
content = content.replace(old, new, 1)
old = '''    fn end_active_session_at(
        &mut self,
        observed_end_utc: DateTime<Utc>,
        clock_mode: SessionClockMode,
    ) -> Option<usize> {
        let interval = match self.reconciled_active_interval(observed_end_utc, clock_mode) {
            Ok(interval) => interval,
            Err(error) => {
                self.record_storage_result_for::<()>(
'''
new = '''    fn end_active_session_at(
        &mut self,
        observed_end_utc: DateTime<Utc>,
        clock_mode: SessionClockMode,
    ) -> Option<usize> {
        let interval = match self.reconciled_active_interval(observed_end_utc, clock_mode) {
            Ok(interval) => interval,
            Err(error) => {
                eprintln!(
                    "finish reconciliation failure: {error}; pending_mutations={}; simulation_time={}; observed_end={}; monotonic={:?}; started={:?}",
                    self.simulation.pending_mutations.len(),
                    self.simulation.simulation_time_utc,
                    observed_end_utc,
                    self.time_tracker.current_elapsed(),
                    self.session.active_session_started_at_utc,
                );
                self.record_storage_result_for::<()>(
'''
if old not in content:
    raise SystemExit("finish reconciliation diagnostic marker missing")
path.write_text(content.replace(old, new, 1))
