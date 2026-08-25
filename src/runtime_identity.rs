use chrono::{DateTime, SecondsFormat, Utc};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TransitionIdentity {
    pub operation_id: String,
    token: String,
}

impl TransitionIdentity {
    pub(crate) fn tui_active_stable_id(&self) -> String {
        format!("tui-active:{}", self.token)
    }
}

const FNV1A_128_OFFSET_BASIS: u128 = 0x6c62_272e_07bb_0142_62b8_2175_6295_c58d;
const FNV1A_128_PRIME: u128 = 0x0000_0000_0100_0000_0000_0000_0000_013b;

// Persisted protocol token. Length-prefixing makes field boundaries unambiguous; changing this
// framing or hash changes retry identities and therefore requires an explicit compatibility decision.
fn stable_transition_token(fields: &[&str]) -> String {
    let mut hash = FNV1A_128_OFFSET_BASIS;
    for field in fields {
        let bytes = field.as_bytes();
        let field_len = u64::try_from(bytes.len())
            .expect("transition identity field length exceeds the supported range");
        for byte in field_len
            .to_be_bytes()
            .into_iter()
            .chain(bytes.iter().copied())
        {
            hash ^= u128::from(byte);
            hash = hash.wrapping_mul(FNV1A_128_PRIME);
        }
    }
    format!("{hash:032x}")
}

pub(crate) fn transition_identity(
    kind: &str,
    expected_stable_id: &str,
    at_utc: DateTime<Utc>,
    discriminator: &str,
) -> TransitionIdentity {
    let timestamp = at_utc.to_rfc3339_opts(SecondsFormat::Nanos, true);
    let token = stable_transition_token(&[kind, expected_stable_id, &timestamp, discriminator]);
    TransitionIdentity {
        operation_id: format!("rt:{kind}:{token}"),
        token,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use chrono::{TimeZone, Utc};

    use super::transition_identity;

    fn timestamp() -> chrono::DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 8, 25, 20, 3, 32)
            .single()
            .unwrap()
            + chrono::Duration::nanoseconds(123_456_789)
    }

    #[test]
    fn transition_identity_has_a_frozen_stable_token_contract() {
        let identity = transition_identity("switch", "legacy-active", timestamp(), "2");

        assert_eq!(
            identity.operation_id,
            "rt:switch:344a26c4f1b8c0d93b71811e4a6177f1"
        );
        assert_eq!(
            identity.tui_active_stable_id(),
            "tui-active:344a26c4f1b8c0d93b71811e4a6177f1"
        );
    }

    #[test]
    fn recursive_predecessor_is_cut_to_bounded_identity_in_one_transition() {
        let recursive = format!("tui-active:{}seed", "switch:tui-active:".repeat(1_000));
        let identity = transition_identity("switch", &recursive, timestamp(), "2");

        assert_eq!(identity.operation_id.len(), 42);
        assert_eq!(identity.tui_active_stable_id().len(), 43);
        assert!(!identity.operation_id.contains(&recursive));
        assert!(!identity.tui_active_stable_id().contains(&recursive));
    }

    #[test]
    fn long_transition_chain_stays_bounded_unique_and_deterministic() {
        let base = timestamp();
        let mut predecessor = format!("tui-active:{}seed", "switch:tui-active:".repeat(1_000));
        let mut active_ids = HashSet::new();

        for index in 0..2_000i64 {
            let at = base + chrono::Duration::nanoseconds(index);
            let identity = transition_identity("switch", &predecessor, at, "2");
            let retry = transition_identity("switch", &predecessor, at, "2");

            assert_eq!(identity, retry);
            assert_eq!(identity.operation_id.len(), 42);
            assert_eq!(identity.tui_active_stable_id().len(), 43);
            assert!(active_ids.insert(identity.tui_active_stable_id()));
            predecessor = identity.tui_active_stable_id();
        }
    }

    #[test]
    fn transition_fields_are_part_of_identity() {
        let at = timestamp();
        let baseline = transition_identity("switch", "active-a", at, "2");

        assert_ne!(baseline, transition_identity("finish", "active-a", at, "2"));
        assert_ne!(baseline, transition_identity("switch", "active-b", at, "2"));
        assert_ne!(
            baseline,
            transition_identity(
                "switch",
                "active-a",
                at + chrono::Duration::nanoseconds(1),
                "2"
            )
        );
        assert_ne!(baseline, transition_identity("switch", "active-a", at, "3"));
    }
}
