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
path.write_text(content.replace(old, new, 1))
