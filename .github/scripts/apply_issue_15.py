from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[2]

def load(path):
    return (ROOT / path).read_text()

def save(path, text):
    (ROOT / path).write_text(text)

def rep(text, old, new, label, count=1):
    actual = text.count(old)
    if actual != count:
        raise SystemExit(f"{label}: expected {count}, found {actual}")
    return text.replace(old, new)

def sub(text, pattern, repl, label, count=1, flags=0):
    result, actual = re.subn(pattern, repl, text, count=count, flags=flags)
    if actual != count:
        raise SystemExit(f"{label}: expected {count}, found {actual}")
    return result

profile_rs = r'''use std::{
    fs,
    path::{Path, PathBuf},
    sync::OnceLock,
};

use directories::ProjectDirs;
use rand::RngCore;
use serde::{Deserialize, Serialize};

const PROFILE_SCHEMA_VERSION: u8 = 1;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct ProfileManifest {
    schema_version: u8,
    profile_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ProfileContext {
    pub profile_id: String,
    pub root: Option<PathBuf>,
    pub data_dir: PathBuf,
    pub state_dir: PathBuf,
    pub config_dir: PathBuf,
    pub manifest_path: PathBuf,
    pub rooted: bool,
}

static PROFILE: OnceLock<ProfileContext> = OnceLock::new();

pub(crate) fn initialize(explicit_root: Option<PathBuf>) -> Result<(), String> {
    let requested = resolve_context(explicit_root)?;
    if let Some(existing) = PROFILE.get() {
        if existing != &requested {
            return Err(format!(
                "profile authority is already initialized as {}; live profile switching is forbidden",
                existing.profile_id
            ));
        }
        return Ok(());
    }
    PROFILE
        .set(requested)
        .map_err(|_| "profile authority initialization raced with another caller".to_string())
}

pub(crate) fn context() -> &'static ProfileContext {
    PROFILE.get_or_init(|| {
        resolve_context(None).unwrap_or_else(|error| panic!("cannot initialize Strata profile: {error}"))
    })
}

pub(crate) fn profile_id() -> String {
    context().profile_id.clone()
}

pub(crate) fn data_dir() -> PathBuf {
    context().data_dir.clone()
}

pub(crate) fn state_dir() -> PathBuf {
    context().state_dir.clone()
}

pub(crate) fn config_dir() -> PathBuf {
    context().config_dir.clone()
}

pub(crate) fn validate_artifact_profile(
    observed: Option<&str>,
    artifact: &str,
) -> Result<(), String> {
    let current = context();
    match observed {
        Some(value) if value == current.profile_id => Ok(()),
        Some(value) => Err(format!(
            "{artifact} belongs to profile {value}, but selected profile is {}; cross-profile state is refused",
            current.profile_id
        )),
        None if !current.rooted => Ok(()),
        None => Err(format!(
            "{artifact} has no profile identity, but rooted profile {} requires explicit binding",
            current.profile_id
        )),
    }
}

pub(crate) fn describe() -> serde_json::Value {
    let current = context();
    serde_json::json!({
        "schema_version": PROFILE_SCHEMA_VERSION,
        "profile_id": current.profile_id,
        "root": current.root.as_ref().map(|path| path.to_string_lossy().into_owned()),
        "data_dir": current.data_dir.to_string_lossy(),
        "state_dir": current.state_dir.to_string_lossy(),
        "config_dir": current.config_dir.to_string_lossy(),
        "manifest": current.manifest_path.to_string_lossy(),
        "switching": "process-bound; exit and invoke with --profile"
    })
}

fn resolve_context(explicit_root: Option<PathBuf>) -> Result<ProfileContext, String> {
    let env_profile = nonempty_env("STRATA_PROFILE");
    let legacy_alias = nonempty_env("STRATA_DATA_DIR");
    if let (Some(profile), Some(legacy)) = (&env_profile, &legacy_alias)
        && profile != legacy
    {
        return Err(format!(
            "STRATA_PROFILE ({}) conflicts with legacy STRATA_DATA_DIR ({}); select one complete profile",
            profile.display(),
            legacy.display()
        ));
    }
    let root = explicit_root.or(env_profile).or(legacy_alias);
    if let Some(root) = root {
        let root = absolute_directory(&root)?;
        let data_dir = root.join("data");
        let state_dir = root.join("state");
        let config_dir = root.join("config");
        ensure_directory(&data_dir)?;
        ensure_directory(&state_dir)?;
        ensure_directory(&config_dir)?;
        let manifest_path = root.join("profile.json");
        let profile_id = load_or_create_manifest(&manifest_path)?;
        return Ok(ProfileContext {
            profile_id,
            root: Some(root),
            data_dir,
            state_dir,
            config_dir,
            manifest_path,
            rooted: true,
        });
    }

    let project = ProjectDirs::from("com", "strata", "strata")
        .ok_or_else(|| "platform does not provide Strata profile directories".to_string())?;
    let data_dir = project.data_dir().to_path_buf();
    let config_dir = project.config_dir().to_path_buf();
    let state_dir = project
        .state_dir()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| data_dir.join("state"));
    ensure_directory(&data_dir)?;
    ensure_directory(&state_dir)?;
    ensure_directory(&config_dir)?;
    let manifest_path = data_dir.join("profile.json");
    let profile_id = load_or_create_manifest(&manifest_path)?;
    Ok(ProfileContext {
        profile_id,
        root: None,
        data_dir,
        state_dir,
        config_dir,
        manifest_path,
        rooted: false,
    })
}

fn nonempty_env(name: &str) -> Option<PathBuf> {
    std::env::var_os(name).and_then(|value| {
        let path = PathBuf::from(value);
        (!path.as_os_str().is_empty()).then_some(path)
    })
}

fn absolute_directory(path: &Path) -> Result<PathBuf, String> {
    if path.exists() && !path.is_dir() {
        return Err(format!("profile root {} is not a directory", path.display()));
    }
    fs::create_dir_all(path)
        .map_err(|error| format!("cannot create profile root {}: {error}", path.display()))?;
    fs::canonicalize(path)
        .map_err(|error| format!("cannot resolve profile root {}: {error}", path.display()))
}

fn ensure_directory(path: &Path) -> Result<(), String> {
    if path.exists() && !path.is_dir() {
        return Err(format!("profile path {} is not a directory", path.display()));
    }
    fs::create_dir_all(path)
        .map_err(|error| format!("cannot create profile directory {}: {error}", path.display()))
}

fn load_or_create_manifest(path: &Path) -> Result<String, String> {
    if path.exists() {
        let bytes = fs::read(path)
            .map_err(|error| format!("cannot read profile manifest {}: {error}", path.display()))?;
        let manifest: ProfileManifest = serde_json::from_slice(&bytes)
            .map_err(|error| format!("invalid profile manifest {}: {error}", path.display()))?;
        validate_manifest(&manifest, path)?;
        return Ok(manifest.profile_id);
    }
    let manifest = ProfileManifest {
        schema_version: PROFILE_SCHEMA_VERSION,
        profile_id: new_uuid_v4(),
    };
    write_manifest_atomic(path, &manifest)?;
    Ok(manifest.profile_id)
}

fn validate_manifest(manifest: &ProfileManifest, path: &Path) -> Result<(), String> {
    if manifest.schema_version != PROFILE_SCHEMA_VERSION {
        return Err(format!(
            "unsupported profile manifest schema {} in {}",
            manifest.schema_version,
            path.display()
        ));
    }
    if !is_uuid(&manifest.profile_id) {
        return Err(format!(
            "invalid profile UUID '{}' in {}",
            manifest.profile_id,
            path.display()
        ));
    }
    Ok(())
}

fn write_manifest_atomic(path: &Path, manifest: &ProfileManifest) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("profile manifest {} has no parent", path.display()))?;
    ensure_directory(parent)?;
    let temporary = parent.join(format!(".profile.json.tmp-{}", std::process::id()));
    let bytes = serde_json::to_vec_pretty(manifest).map_err(|error| error.to_string())?;
    fs::write(&temporary, bytes)
        .map_err(|error| format!("cannot write profile manifest temporary file: {error}"))?;
    fs::rename(&temporary, path)
        .map_err(|error| format!("cannot publish profile manifest {}: {error}", path.display()))
}

fn new_uuid_v4() -> String {
    let mut bytes = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut bytes);
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15]
    )
}

fn is_uuid(value: &str) -> bool {
    value.len() == 36
        && value
            .chars()
            .enumerate()
            .all(|(index, character)| match index {
                8 | 13 | 18 | 23 => character == '-',
                _ => character.is_ascii_hexdigit(),
            })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_profile_identity_is_uuid_v4_shaped() {
        let value = new_uuid_v4();
        assert!(is_uuid(&value));
        assert_eq!(&value[14..15], "4");
        assert!(matches!(&value[19..20], "8" | "9" | "a" | "b"));
    }
}
'''
save("src/profile.rs", profile_rs)

# lib: initialize profile before config and remove path override.
p = "src/lib.rs"
s = load(p)
s = rep(s, "mod legacy_transition;\n", "mod legacy_transition;\nmod profile;\n", "profile module")
s = sub(s, r"\nfn apply_startup_configuration\(loaded: &keybindings::LoadedKeybindings\) \{.*?\n\}\n", "\nfn apply_startup_configuration(loaded: &keybindings::LoadedKeybindings) {\n    domain::set_runtime_settings(loaded.runtime_settings);\n}\n", "startup config", flags=re.S)
s = rep(s,
"    let invocation = cli::parse_invocation();\n    let loaded = load_startup_configuration(invocation.ignore_config)?;",
"    let invocation = cli::parse_invocation();\n    profile::initialize(invocation.profile.clone()).map_err(|error| {\n        io::Error::new(io::ErrorKind::InvalidInput, format!(\"Profile error: {error}\"))\n    })?;\n    let loaded = load_startup_configuration(invocation.ignore_config)?;", "initialize profile")
save(p, s)

# CLI: explicit process-bound profile, profile inspection, active artifact binding.
p = "src/cli.rs"
s = load(p)
s = rep(s,
"    pub ignore_config: bool,\n\n    #[command(subcommand)]",
"    pub ignore_config: bool,\n\n    #[arg(\n        long,\n        global = true,\n        value_name = \"DIRECTORY\",\n        help = \"Select one complete process-lifetime profile root\"\n    )]\n    pub profile: Option<PathBuf>,\n\n    #[command(subcommand)]", "profile argument")
s = rep(s,
"pub enum Cli {\n    #[command(about = \"Start a new tracking session\")]",
"pub enum Cli {\n    #[command(about = \"Show the selected profile identity and complete authority paths\")]\n    Profile {\n        #[arg(long, help = \"Print the profile description as JSON\")]\n        json: bool,\n    },\n\n    #[command(about = \"Start a new tracking session\")]", "profile command")
s = rep(s,
"    sqlite, storage, temporal,\n};",
"    profile, sqlite, storage, temporal,\n};", "cli profile import")
s = rep(s,
"pub struct ActiveSession {\n    pub project:",
"pub struct ActiveSession {\n    #[serde(default)]\n    pub profile_id: Option<String>,\n    pub project:", "active profile field")
s = rep(s,
"    let session = ActiveSession {\n        project:",
"    let session = ActiveSession {\n        profile_id: Some(profile::profile_id()),\n        project:", "active profile write")
s = rep(s,
"    let active_session: ActiveSession = storage::read_json(&session_path)?;\n\n    let now_utc",
"    let active_session: ActiveSession = storage::read_json(&session_path)?;\n    profile::validate_artifact_profile(active_session.profile_id.as_deref(), \"active session\")?;\n\n    let now_utc", "stop profile validation")
s = rep(s,
"            let active: ActiveSession = storage::read_json(&active_path)?;\n            if let Some(provisional)",
"            let active: ActiveSession = storage::read_json(&active_path)?;\n            profile::validate_artifact_profile(active.profile_id.as_deref(), \"active session\")?;\n            if let Some(provisional)", "report profile validation")
# Add display function before parse_invocation.
anchor = "pub fn parse_invocation() -> Invocation {"
profile_fn = '''pub fn show_profile(json: bool) -> Result<(), String> {
    let description = profile::describe();
    if json {
        println!("{}", serde_json::to_string_pretty(&description).map_err(|error| error.to_string())?);
    } else {
        println!("Profile ID: {}", description["profile_id"].as_str().unwrap_or("unknown"));
        println!("Root: {}", description["root"].as_str().unwrap_or("XDG default"));
        println!("Data: {}", description["data_dir"].as_str().unwrap_or("unknown"));
        println!("State: {}", description["state_dir"].as_str().unwrap_or("unknown"));
        println!("Config: {}", description["config_dir"].as_str().unwrap_or("unknown"));
        println!("Switching: exit Strata and invoke again with --profile <directory>");
    }
    Ok(())
}

'''
if anchor not in s:
    raise SystemExit("parse invocation anchor missing")
s = s.replace(anchor, profile_fn + anchor, 1)
s = rep(s,
"    match cli {\n        Cli::Start {",
"    match cli {\n        Cli::Profile { json } => {\n            if let Err(error) = show_profile(json) {\n                eprintln!(\"Error: {error}\");\n                std::process::exit(1);\n            }\n        }\n        Cli::Start {", "profile run command")
save(p, s)

# Storage: every authority path comes from selected profile; remove partial path override.
p = "src/storage.rs"
s = load(p)
s = rep(s,
"    path::{Path, PathBuf},\n    sync::{OnceLock, RwLock},",
"    path::{Path, PathBuf},", "storage sync imports")
s = rep(s, "use directories::ProjectDirs;\n", "", "storage projectdirs import")
s = sub(s, r"\n#\[derive\(Debug, Clone, Default, PartialEq, Eq\)\]\npub struct RuntimeStorageSettings \{.*?\n\}\n\nfn runtime_storage_settings_cell\(\).*?\n\}\n", "\n", "runtime storage settings", flags=re.S)
s = sub(s, r"pub fn get_data_dir\(\) -> PathBuf \{.*?\n\}\n\npub fn get_config_dir\(\) -> PathBuf \{.*?\n\}\n\npub fn get_state_dir\(\) -> PathBuf \{.*?\n\}\n", "pub fn get_data_dir() -> PathBuf {\n    crate::profile::data_dir()\n}\n\npub fn get_config_dir() -> PathBuf {\n    crate::profile::config_dir()\n}\n\npub fn get_state_dir() -> PathBuf {\n    crate::profile::state_dir()\n}\n", "profile path getters", flags=re.S)
s = sub(s, r"pub fn get_time_log_path\(\) -> PathBuf \{.*?\n\}\n\npub fn normalize_time_log_path_input\(raw: &str\) -> Option<PathBuf> \{.*?\n\}\n", "pub fn get_time_log_path() -> PathBuf {\n    get_data_dir().join(\"time_log.csv\")\n}\n", "time log override removal", flags=re.S)
save(p, s)

# Config: reject legacy hot redirect and remove it from loaded runtime state/setters.
p = "src/keybindings.rs"
s = load(p)
s = rep(s,
"pub(crate) struct LoadedKeybindings {\n    pub keymap: Keymap,\n    pub runtime_settings: RuntimeSettings,\n    pub time_log_path: Option<PathBuf>,\n}",
"pub(crate) struct LoadedKeybindings {\n    pub keymap: Keymap,\n    pub runtime_settings: RuntimeSettings,\n}", "loaded config path")
s = rep(s,
"        runtime_settings: default_runtime_settings(),\n        time_log_path: None,",
"        runtime_settings: default_runtime_settings(),", "default loaded path")
s = sub(s, r"\nfn parse_time_log_path\(.*?\n\}\n\nfn load_config_or_default", "\nfn load_config_or_default", "parse time log removal", flags=re.S)
s = rep(s,
"    let runtime_settings = parse_runtime_settings(&config, path)?;\n    let time_log_path = parse_time_log_path(&config, path)?;",
"    let runtime_settings = parse_runtime_settings(&config, path)?;\n    if config.time_log_path.is_some() {\n        return Err(format!(\n            \"time_log_path in {} is no longer supported because it hot-redirected one file without switching complete profile authority; exit Strata and use --profile <directory>\",\n            path.display()\n        ));\n    }", "reject time log path")
s = rep(s,
"        keymap,\n        runtime_settings,\n        time_log_path,",
"        keymap,\n        runtime_settings,", "loaded return path")
s = sub(s, r"\npub\(crate\) fn set_time_log_path\(.*?\n\}\n", "\n", "remove set time log", flags=re.S)
save(p, s)

# App loaded config no longer carries a path; bind checkpoints and recovery statements.
p = "src/app.rs"
s = load(p)
s = rep(s,
"        let keybindings::LoadedKeybindings {\n            keymap,\n            runtime_settings,\n            time_log_path: _,\n        } = loaded;",
"        let keybindings::LoadedKeybindings {\n            keymap,\n            runtime_settings,\n        } = loaded;", "app loaded config")
s = rep(s,
"struct DetachedRuntimeCheckpoint {\n    schema_version: u8,",
"struct DetachedRuntimeCheckpoint {\n    schema_version: u8,\n    #[serde(default)]\n    profile_id: Option<String>,", "checkpoint profile field")
# Every typed checkpoint fixture/construction receives current binding.
s, inserted = re.subn(
    r"(DetachedRuntimeCheckpoint \{\n\s*schema_version: [^\n]+,)",
    r"\1\n            profile_id: Some(crate::profile::profile_id()),",
    s,
)
if inserted < 5:
    raise SystemExit(f"checkpoint constructors: expected many, found {inserted}")
s = rep(s,
"struct RecoveryStatement {\n    checkpoint_captured_at_utc:",
"struct RecoveryStatement {\n    profile_id: String,\n    checkpoint_captured_at_utc:", "recovery profile field")
s = rep(s,
"    Ok(RecoveryStatement {\n        checkpoint_captured_at_utc:",
"    Ok(RecoveryStatement {\n        profile_id: checkpoint\n            .profile_id\n            .clone()\n            .unwrap_or_else(crate::profile::profile_id),\n        checkpoint_captured_at_utc:", "recovery profile value")
# Validate before any replay and upgrade legacy default evidence after validation.
needle = "        if checkpoint.clear_all.is_some()"
insert = '''        if let Err(error) = crate::profile::validate_artifact_profile(
            checkpoint.profile_id.as_deref(),
            "detached runtime checkpoint",
        ) {
            if let Some(database_path) = self.sqlite_database_path.clone() {
                let _ = sqlite::quarantine_tui_checkpoint(&database_path);
            }
            self.record_storage_result::<()>(Err(error));
            return false;
        }
        checkpoint.profile_id = Some(crate::profile::profile_id());

'''
if s.count(needle) != 1:
    raise SystemExit("checkpoint validation anchor missing")
s = s.replace(needle, insert + needle, 1)
save(p, s)

# SQLite marker is profile-bound so copied absolute database pointers cannot cross profiles.
for p in ["src/sqlite/authority.rs", "src/sqlite/migration_command.rs"]:
    s = load(p)
    if p.endswith("authority.rs"):
        s = rep(s, "use crate::storage;", "use crate::{profile, storage};", "authority profile import")
    else:
        s = rep(s, "use crate::storage;", "use crate::{profile, storage};", "migration profile import")
    s = rep(s,
"struct StorageAuthorityMarker {\n    schema_version: u8,",
"struct StorageAuthorityMarker {\n    schema_version: u8,\n    #[serde(default)]\n    profile_id: Option<String>,", f"{p} marker profile")
    if p.endswith("authority.rs"):
        s = rep(s,
"fn validate_marker(marker: &StorageAuthorityMarker) -> Result<(), AuthorityError> {\n    if marker.schema_version",
"fn validate_marker(marker: &StorageAuthorityMarker) -> Result<(), AuthorityError> {\n    profile::validate_artifact_profile(marker.profile_id.as_deref(), \"SQLite authority marker\")\n        .map_err(AuthorityError::InvalidMarker)?;\n    if marker.schema_version", "authority validate profile")
    # Constructor(s), excluding struct definition, have constant/expression schema value.
    s, found = re.subn(
        r"(StorageAuthorityMarker \{\n\s*schema_version: (?!u8)[^\n]+,)",
        r"\1\n            profile_id: Some(profile::profile_id()),",
        s,
    )
    if p.endswith("migration_command.rs") and found < 1:
        raise SystemExit("migration marker constructor missing")
    save(p, s)

# Update config process test: old partial path is always rejected with migration guidance.
p = "tests/config_authority.rs"
s = load(p)
s = sub(s,
r"#\[test\]\nfn invalid_profile_path_blocks_mutation\(\) \{.*?\n\}\n\n#\[test\]\nfn ignore_config",
'''#[test]
fn time_log_path_hot_redirect_is_rejected() {
    let profile = TestProfile::new("profile-path");
    fs::write(
        profile.config_path(),
        serde_json::json!({"time_log_path": profile.root.join("elsewhere")}).to_string(),
    )
    .expect("write obsolete partial-path config");

    let output = profile.run(&["start", "unsafe", "--category", "Work"]);
    assert_config_failure(&profile, &output, "time_log_path");
    assert!(stderr(&output).contains("--profile"));
}

#[test]
fn ignore_config''', "config hot redirect proof", flags=re.S)
save(p, s)

# Dedicated process proof for two profiles, active binding, stale checkpoint, and profile inspection.
profile_test = r'''#![cfg(target_os = "linux")]

use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

fn root(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should follow epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("strata-profile-{label}-{}-{nonce}", std::process::id()))
}

fn seed(profile: &Path, description: &str) {
    fs::create_dir_all(profile.join("data")).unwrap();
    fs::write(
        profile.join("data/categories.csv"),
        format!("id,name,description,color_index,karma_effect\n1,Work,{description},0,1\n"),
    )
    .unwrap();
}

fn run(profile: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_strata"))
        .arg("--profile")
        .arg(profile)
        .args(args)
        .env_remove("STRATA_PROFILE")
        .env_remove("STRATA_DATA_DIR")
        .output()
        .expect("Strata process should run")
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn detach(profile: &Path) -> Output {
    let command_line = format!(
        "stty cols 100 rows 30; exec {} --profile {}",
        env!("CARGO_BIN_EXE_strata"),
        profile.display()
    );
    let mut child = Command::new("timeout")
        .args(["12s", "script", "-qefc", &command_line, "/dev/null"])
        .env_remove("STRATA_PROFILE")
        .env_remove("STRATA_DATA_DIR")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("TUI should start in a PTY");
    thread::sleep(Duration::from_millis(1200));
    let mut stdin = child.stdin.take().unwrap();
    stdin.write_all(b"d").unwrap();
    stdin.flush().unwrap();
    drop(stdin);
    child.wait_with_output().expect("TUI should finish")
}

#[test]
fn rooted_profiles_isolate_paths_and_bind_active_state() {
    let a = root("a");
    let b = root("b");
    seed(&a, "A metadata");
    seed(&b, "B metadata");

    let info_a = run(&a, &["profile", "--json"]);
    let info_b = run(&b, &["profile", "--json"]);
    assert!(info_a.status.success(), "{}", stderr(&info_a));
    assert!(info_b.status.success(), "{}", stderr(&info_b));
    let a_json: serde_json::Value = serde_json::from_slice(&info_a.stdout).unwrap();
    let b_json: serde_json::Value = serde_json::from_slice(&info_b.stdout).unwrap();
    assert_ne!(a_json["profile_id"], b_json["profile_id"]);
    assert_eq!(a_json["root"], a.to_string_lossy().as_ref());

    let started = run(&a, &["start", "A project", "--category", "Work"]);
    assert!(started.status.success(), "{}", stderr(&started));
    assert!(a.join("state/active_session.json").exists());
    assert!(!b.join("state/active_session.json").exists());

    let wrong_stop = run(&b, &["stop"]);
    assert!(!wrong_stop.status.success());
    assert!(stderr(&wrong_stop).contains("No active session"));

    fs::create_dir_all(b.join("state")).unwrap();
    fs::copy(
        a.join("state/active_session.json"),
        b.join("state/active_session.json"),
    )
    .unwrap();
    let copied_stop = run(&b, &["stop"]);
    assert!(!copied_stop.status.success());
    assert!(stderr(&copied_stop).contains("belongs to profile"));

    let stopped = run(&a, &["stop"]);
    assert!(stopped.status.success(), "{}", stderr(&stopped));
    assert!(a.join("data/time_log.csv").exists());
    assert!(!b.join("data/time_log.csv").exists());

    fs::remove_dir_all(a).ok();
    fs::remove_dir_all(b).ok();
}

#[test]
fn copied_detached_checkpoint_is_refused_by_target_profile() {
    let a = root("checkpoint-a");
    let b = root("checkpoint-b");
    seed(&a, "A metadata");
    seed(&b, "B metadata");

    let detached = detach(&a);
    assert!(detached.status.success(), "{}", stderr(&detached));
    let checkpoint = a.join("state/detached_runtime.json");
    assert!(checkpoint.exists());
    fs::create_dir_all(b.join("state")).unwrap();
    fs::copy(&checkpoint, b.join("state/detached_runtime.json")).unwrap();

    let target = Command::new("timeout")
        .args(["8s", env!("CARGO_BIN_EXE_strata"), "--profile"])
        .arg(&b)
        .output()
        .expect("target TUI should run");
    assert!(!target.status.success());
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&target.stdout),
        String::from_utf8_lossy(&target.stderr)
    );
    assert!(combined.contains("belongs to profile"), "{combined}");

    fs::remove_dir_all(a).ok();
    fs::remove_dir_all(b).ok();
}
'''
save("tests/profile_authority.rs", profile_test)

print("issue #15 transform applied")
