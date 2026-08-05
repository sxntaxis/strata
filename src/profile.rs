use std::{
    fs::{self, OpenOptions},
    io::{ErrorKind, Write},
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
        resolve_context(None)
            .unwrap_or_else(|error| panic!("cannot initialize Strata profile: {error}"))
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
    let root = if let Some(explicit_root) = explicit_root {
        Some(explicit_root)
    } else {
        nonempty_env("STRATA_PROFILE")
    };

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
        return Err(format!(
            "profile root {} is not a directory",
            path.display()
        ));
    }
    fs::create_dir_all(path)
        .map_err(|error| format!("cannot create profile root {}: {error}", path.display()))?;
    fs::canonicalize(path)
        .map_err(|error| format!("cannot resolve profile root {}: {error}", path.display()))
}

fn ensure_directory(path: &Path) -> Result<(), String> {
    if path.exists() && !path.is_dir() {
        return Err(format!(
            "profile path {} is not a directory",
            path.display()
        ));
    }
    fs::create_dir_all(path).map_err(|error| {
        format!(
            "cannot create profile directory {}: {error}",
            path.display()
        )
    })
}

fn load_or_create_manifest(path: &Path) -> Result<String, String> {
    if path.exists() {
        return read_manifest(path);
    }

    let manifest = ProfileManifest {
        schema_version: PROFILE_SCHEMA_VERSION,
        profile_id: new_uuid_v4(),
    };
    if publish_manifest_if_absent(path, &manifest)? {
        Ok(manifest.profile_id)
    } else {
        read_manifest(path)
    }
}

fn read_manifest(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path)
        .map_err(|error| format!("cannot read profile manifest {}: {error}", path.display()))?;
    let manifest: ProfileManifest = serde_json::from_slice(&bytes)
        .map_err(|error| format!("invalid profile manifest {}: {error}", path.display()))?;
    validate_manifest(&manifest, path)?;
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

fn publish_manifest_if_absent(path: &Path, manifest: &ProfileManifest) -> Result<bool, String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("profile manifest {} has no parent", path.display()))?;
    ensure_directory(parent)?;
    let temporary = parent.join(format!(
        ".profile.json.tmp-{}-{}",
        std::process::id(),
        new_uuid_v4()
    ));
    let bytes = serde_json::to_vec_pretty(manifest).map_err(|error| error.to_string())?;

    let write_result = (|| -> Result<(), std::io::Error> {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        file.write_all(&bytes)?;
        file.sync_all()
    })();
    if let Err(error) = write_result {
        let _ = fs::remove_file(&temporary);
        return Err(format!(
            "cannot write profile manifest temporary file {}: {error}",
            temporary.display()
        ));
    }

    let published = match fs::hard_link(&temporary, path) {
        Ok(()) => true,
        Err(error) if error.kind() == ErrorKind::AlreadyExists => false,
        Err(error) => {
            let _ = fs::remove_file(&temporary);
            return Err(format!(
                "cannot publish profile manifest {}: {error}",
                path.display()
            ));
        }
    };
    fs::remove_file(&temporary).map_err(|error| {
        format!(
            "cannot remove profile manifest temporary file {}: {error}",
            temporary.display()
        )
    })?;
    Ok(published)
}

fn new_uuid_v4() -> String {
    let mut bytes = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut bytes);
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        bytes[0],
        bytes[1],
        bytes[2],
        bytes[3],
        bytes[4],
        bytes[5],
        bytes[6],
        bytes[7],
        bytes[8],
        bytes[9],
        bytes[10],
        bytes[11],
        bytes[12],
        bytes[13],
        bytes[14],
        bytes[15]
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
