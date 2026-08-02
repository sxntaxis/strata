from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    target = Path(path)
    text = target.read_text()
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"expected one match in {path}, found {count}: {old[:160]!r}")
    target.write_text(text.replace(old, new, 1))


replace_once(
    "src/legacy_transition.rs",
    '''impl LegacyTransitionReceipt {
    pub(crate) const VERSION: u8 = 1;
}
''',
    '''impl LegacyTransitionReceipt {
    pub(crate) const VERSION: u8 = 1;

    pub(crate) fn validate_switch_boundaries(&self) -> Result<(), String> {
        if self.version != Self::VERSION {
            return Err(format!(
                "unsupported legacy transition receipt version {}",
                self.version
            ));
        }
        if self.kind != LegacyTransitionKind::Switch {
            return Err("unsupported legacy transition kind".to_string());
        }
        if self.resulting_active.started_at_utc != self.transition_at_utc {
            return Err(format!(
                "legacy switch receipt {} has inconsistent resulting start time",
                self.operation_id
            ));
        }
        if let Some(completed) = &self.completed_session {
            if completed.category_id != self.expected_previous_category_id {
                return Err(format!(
                    "legacy switch receipt {} completed the wrong category",
                    self.operation_id
                ));
            }
            if completed.started_at_utc != Some(self.expected_previous_started_at_utc) {
                return Err(format!(
                    "legacy switch receipt {} has inconsistent previous start time",
                    self.operation_id
                ));
            }
            if completed.ended_at_utc != Some(self.transition_at_utc) {
                return Err(format!(
                    "legacy switch receipt {} has inconsistent completion time",
                    self.operation_id
                ));
            }
            if completed.elapsed_seconds == 0 {
                return Err(format!(
                    "legacy switch receipt {} stores a zero-work completed row",
                    self.operation_id
                ));
            }
        }
        Ok(())
    }
}
''',
)

path = Path("src/legacy_transition.rs")
text = path.read_text()
marker = '''    #[test]
    fn absent_receipt_session_is_appended_once() {
'''
proof = r'''    fn switch_receipt(completed_session: Option<LegacySessionReceipt>) -> LegacyTransitionReceipt {
        LegacyTransitionReceipt {
            version: LegacyTransitionReceipt::VERSION,
            operation_id: "legacy-switch:test".to_string(),
            kind: LegacyTransitionKind::Switch,
            expected_previous_category_id: 4,
            expected_previous_started_at_utc: Utc
                .with_ymd_and_hms(2026, 8, 2, 16, 0, 0)
                .unwrap(),
            transition_at_utc: Utc.with_ymd_and_hms(2026, 8, 2, 17, 0, 0).unwrap(),
            completed_session,
            resulting_active: LegacyActiveReceipt {
                category_id: 5,
                description: String::new(),
                started_at_utc: Utc.with_ymd_and_hms(2026, 8, 2, 17, 0, 0).unwrap(),
            },
        }
    }

    #[test]
    fn switch_receipt_validates_all_temporal_and_category_boundaries() {
        let completed = LegacySessionReceipt::from_session(&session(7, "work"));
        switch_receipt(Some(completed.clone()))
            .validate_switch_boundaries()
            .unwrap();

        let mut wrong_category = switch_receipt(Some(completed.clone()));
        wrong_category.expected_previous_category_id = 99;
        assert!(wrong_category.validate_switch_boundaries().is_err());

        let mut wrong_transition = switch_receipt(Some(completed));
        wrong_transition.resulting_active.started_at_utc = Utc
            .with_ymd_and_hms(2026, 8, 2, 17, 0, 1)
            .unwrap();
        assert!(wrong_transition.validate_switch_boundaries().is_err());
    }

    #[test]
    fn absent_receipt_session_is_appended_once() {
'''
if marker not in text:
    raise SystemExit("legacy transition proof insertion marker not found")
path.write_text(text.replace(marker, proof, 1))

path = Path("src/app.rs")
text = path.read_text()
old = '''        if receipt.version != LegacyTransitionReceipt::VERSION {
            return Err(format!(
                "unsupported legacy transition receipt version {}",
                receipt.version
            ));
        }
        if receipt.kind != LegacyTransitionKind::Switch {
            return Err("unsupported legacy transition kind; evidence retained".to_string());
        }
'''
new = '''        if checkpoint.schema_version != DetachedRuntimeCheckpoint::VERSION {
            return Err(format!(
                "legacy transition receipt requires checkpoint schema {}, found {}; evidence retained",
                DetachedRuntimeCheckpoint::VERSION,
                checkpoint.schema_version
            ));
        }
        receipt.validate_switch_boundaries()?;
        let expected_identity = format!(
            "legacy:{}:{}",
            receipt.expected_previous_category_id,
            receipt
                .expected_previous_started_at_utc
                .to_rfc3339_opts(SecondsFormat::Nanos, true)
        );
        let expected_operation_id = self.transition_operation_id(
            "legacy-switch",
            &expected_identity,
            receipt.transition_at_utc,
            &receipt.resulting_active.category_id.to_string(),
        );
        if receipt.operation_id != expected_operation_id {
            return Err(format!(
                "legacy switch receipt operation ID {} is inconsistent; evidence retained",
                receipt.operation_id
            ));
        }
'''
if text.count(old) != 1:
    raise SystemExit("legacy receipt validation block not found")
path.write_text(text.replace(old, new, 1))

for temporary in [
    ".github/workflows/reconciliation001b2a-fixup.yml",
    "tools/reconciliation001b2a-fixup.py",
]:
    Path(temporary).unlink(missing_ok=True)
