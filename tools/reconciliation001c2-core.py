from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    file = Path(path)
    content = file.read_text()
    if old not in content:
        raise SystemExit(f"marker missing in {path}: {old[:180]!r}")
    file.write_text(content.replace(old, new, 1))


replace_once(
    "src/lib.rs",
    "mod keybindings;\nmod legacy_transition;\n",
    "mod keybindings;\n#[allow(dead_code)]\nmod legacy_category_lifecycle;\nmod legacy_transition;\n",
)
replace_once(
    "src/storage.rs",
    '''pub fn get_category_tags_path() -> PathBuf {
    get_state_dir().join("category_tags.json")
}

pub fn get_keymap_path() -> PathBuf {
''',
    '''pub fn get_category_tags_path() -> PathBuf {
    get_state_dir().join("category_tags.json")
}

#[allow(dead_code)]
pub fn get_category_lifecycle_prepared_path() -> PathBuf {
    get_state_dir().join("category_lifecycle_prepared.json")
}

#[allow(dead_code)]
pub fn get_category_lifecycle_ledger_path() -> PathBuf {
    get_state_dir().join("category_lifecycle_ledger.json")
}

pub fn get_keymap_path() -> PathBuf {
''',
)
path = Path("src/legacy_category_lifecycle.rs")
content = path.read_text()
content = content.replace("storage::write_private_json_atomic", "storage::write_json_atomic")
content = content.replace("collections::{BTreeMap, BTreeSet}", "collections::BTreeSet")
content = content.replace("use ratatui::style::Color;\n", "")
content = content.replace(
    "    daily_contribution_from_slices(operational_day, width, height, &slices)\n",
    "    Ok(daily_contribution_from_slices(operational_day, width, height, &slices))\n",
)
old_faults = '''#[cfg(test)]
fn test_fault_cell() -> &'static std::sync::RwLock<Option<String>> {
    static CELL: std::sync::OnceLock<std::sync::RwLock<Option<String>>> =
        std::sync::OnceLock::new();
    CELL.get_or_init(|| std::sync::RwLock::new(None))
}

fn maybe_inject_test_fault(phase: &str) -> Result<(), String> {
    #[cfg(test)]
    if test_fault_cell()
        .read()
        .ok()
        .and_then(|guard| guard.clone())
        .as_deref()
        == Some(phase)
    {
        return Err(format!("injected legacy lifecycle failure at {phase}"));
    }
    let _ = phase;
    Ok(())
}

#[cfg(test)]
fn with_test_fault<T>(phase: &str, operation: impl FnOnce() -> T) -> T {
    if let Ok(mut guard) = test_fault_cell().write() {
        *guard = Some(phase.to_string());
    }
    let result = operation();
    if let Ok(mut guard) = test_fault_cell().write() {
        *guard = None;
    }
    result
}
'''
new_faults = '''#[cfg(test)]
thread_local! {
    static TEST_FAULT: std::cell::RefCell<Option<String>> =
        const { std::cell::RefCell::new(None) };
}

fn maybe_inject_test_fault(phase: &str) -> Result<(), String> {
    #[cfg(test)]
    if TEST_FAULT.with(|fault| fault.borrow().as_deref() == Some(phase)) {
        return Err(format!("injected legacy lifecycle failure at {phase}"));
    }
    let _ = phase;
    Ok(())
}

#[cfg(test)]
struct TestFaultReset;

#[cfg(test)]
impl Drop for TestFaultReset {
    fn drop(&mut self) {
        TEST_FAULT.with(|fault| *fault.borrow_mut() = None);
    }
}

#[cfg(test)]
fn with_test_fault<T>(phase: &str, operation: impl FnOnce() -> T) -> T {
    TEST_FAULT.with(|fault| *fault.borrow_mut() = Some(phase.to_string()));
    let _reset = TestFaultReset;
    operation()
}
'''
if old_faults not in content:
    raise SystemExit("legacy lifecycle fault injection marker missing")
content = content.replace(old_faults, new_faults, 1)
path.write_text(content)
