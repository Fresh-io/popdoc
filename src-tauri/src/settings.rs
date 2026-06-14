use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::config::{config_path, ensure_app_dir};

pub const DEFAULT_DRIVE_FOLDER: &str = "Imports Office";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Settings {
    pub drive_folder_name: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub folder_id: Option<String>,
    /// UI language override: `"fr"`, `"en"`, `"es"`. Absent means "follow the
    /// system locale" (the default).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub language: Option<String>,
    /// Unix timestamp (seconds) of the last *background* update check. Used to
    /// throttle the silent launch check; the manual button ignores it.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub last_update_check: Option<u64>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            drive_folder_name: DEFAULT_DRIVE_FOLDER.to_string(),
            folder_id: None,
            language: None,
            last_update_check: None,
        }
    }
}

pub fn load() -> Settings {
    std::fs::read_to_string(config_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save(settings: &Settings) -> Result<()> {
    ensure_app_dir()?;
    let path = config_path();
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, serde_json::to_vec_pretty(settings)?)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600));
    }
    std::fs::rename(tmp, path)?;
    Ok(())
}

pub fn drive_folder_name() -> String {
    let n = load().drive_folder_name;
    if n.trim().is_empty() {
        DEFAULT_DRIVE_FOLDER.to_string()
    } else {
        n
    }
}

pub fn set_drive_folder_name(new_name: &str) -> Result<()> {
    let mut s = load();
    let trimmed = new_name.trim();
    let final_name = if trimmed.is_empty() {
        DEFAULT_DRIVE_FOLDER.to_string()
    } else {
        trimmed.to_string()
    };
    if s.drive_folder_name != final_name {
        s.drive_folder_name = final_name;
        s.folder_id = None; // invalidate cache, will be looked up / created next upload
    }
    save(&s)
}

/// The saved language preference, or `None` to follow the system locale.
/// A stored `"auto"` (or any unrecognised value) is treated as `None`.
pub fn language_pref() -> Option<String> {
    load()
        .language
        .filter(|c| !c.trim().is_empty() && c != "auto")
}

/// Persist the language preference. Pass `"auto"` (or an empty string) to clear
/// the override and follow the system locale.
pub fn set_language(code: &str) -> Result<()> {
    let mut s = load();
    let trimmed = code.trim();
    s.language = if trimmed.is_empty() || trimmed == "auto" {
        None
    } else {
        Some(trimmed.to_string())
    };
    save(&s)
}

/// Unix timestamp of the last background update check (0 if never).
pub fn last_update_check() -> Option<u64> {
    load().last_update_check
}

/// Record that a background update check just ran.
pub fn set_last_update_check(ts: u64) -> Result<()> {
    let mut s = load();
    s.last_update_check = Some(ts);
    save(&s)
}

pub fn cached_folder_id() -> Option<String> {
    load().folder_id
}

pub fn set_cached_folder_id(id: &str) -> Result<()> {
    let mut s = load();
    s.folder_id = Some(id.to_string());
    save(&s)
}
