use std::fs;
use std::process::Command;

fn main() {
    let git_commit = Command::new("git")
        .args(["rev-parse", "--short=12", "HEAD"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "unknown".to_string());
    let package_version = std::env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "unknown".into());
    println!("cargo:rustc-env=STRATA_BUILD_VERSION={package_version} ({git_commit})");

    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/index");
    if let Ok(head) = fs::read_to_string(".git/HEAD") {
        if let Some(reference) = head.trim().strip_prefix("ref: ") {
            println!("cargo:rerun-if-changed=.git/{reference}");
        }
    }
}
