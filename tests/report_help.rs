#![cfg(target_os = "linux")]

use std::process::Command;

#[test]
fn report_help_describes_calendar_periods_not_rolling_windows() {
    let output = Command::new(env!("CARGO_BIN_EXE_strata"))
        .args(["report", "--help"])
        .output()
        .expect("Strata help process should run");
    assert!(output.status.success());
    let help = String::from_utf8(output.stdout).expect("help should be UTF-8");
    assert!(help.contains("Show the current operational week"), "{help}");
    assert!(help.contains("Show the current calendar month"), "{help}");
    assert!(!help.contains("last 7 days"), "{help}");
    assert!(!help.contains("last 30 days"), "{help}");
}
