from pathlib import Path

path = Path("src/sqlite/fault_certification.rs")
text = path.read_text()

text = text.replace(
    "repository::{NewCategoryRecord, SandStateRecord},",
    "repository::{CheckpointStatus, NewCategoryRecord, SandStateRecord},",
)
text = text.replace(
    "fn sand_state(frame_count: u64) -> SandState {",
    "fn sand_state(frame_count: usize) -> SandState {",
)
text = text.replace(
    "        rng_state: frame_count,",
    "        rng_state: u64::try_from(frame_count).unwrap(),",
)

replacements = {
    '''assert_eq!(
            SqliteRepository::open(path)
                .unwrap()
                .checkpoint()
                .unwrap()
                .unwrap()
                .status
                .as_str(),
            "pending"
        );''': '''assert_eq!(
            SqliteRepository::open(path)
                .unwrap()
                .checkpoint()
                .unwrap()
                .unwrap()
                .status,
            CheckpointStatus::Pending
        );''',
    '''assert_eq!(
            SqliteRepository::open(path)
                .unwrap()
                .checkpoint()
                .unwrap()
                .unwrap()
                .status
                .as_str(),
            "recovering"
        );''': '''assert_eq!(
            SqliteRepository::open(path)
                .unwrap()
                .checkpoint()
                .unwrap()
                .unwrap()
                .status,
            CheckpointStatus::Recovering
        );''',
    '''assert_eq!(
            SqliteRepository::open(path)
                .unwrap()
                .checkpoint()
                .unwrap()
                .unwrap()
                .status
                .as_str(),
            "committed"
        );''': '''assert_eq!(
            SqliteRepository::open(path)
                .unwrap()
                .checkpoint()
                .unwrap()
                .unwrap()
                .status,
            CheckpointStatus::Committed
        );''',
}

for old, new in replacements.items():
    count = text.count(old)
    if old.endswith('"recovering"\n        );'):
        if count != 2:
            raise SystemExit(f"expected two recovering assertions, found {count}")
        text = text.replace(old, new)
    else:
        if count != 1:
            raise SystemExit(f"expected one assertion replacement, found {count}")
        text = text.replace(old, new, 1)

path.write_text(text)
