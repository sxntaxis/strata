from pathlib import Path


app = Path("src/app.rs")
text = app.read_text()
old = """        self.checkpoint_recovery_active = false;
        self.checkpoint_recovery_payload = None;
    }
"""
new = """        self.checkpoint_recovery_active = false;
        self.checkpoint_recovery_payload = None;
        self.reconcile_all_daily_contributions();
    }
"""
if text.count(old) != 1:
    raise SystemExit("checkpoint recovery completion block was not found")
app.write_text(text.replace(old, new, 1))

for temporary in [
    ".github/workflows/sediment001d2-recovery-days.yml",
    "tools/sediment001d2-recovery-days.py",
    "tools/sediment001d2-recovery-days.trigger",
]:
    Path(temporary).unlink(missing_ok=True)
