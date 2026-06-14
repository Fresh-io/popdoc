use std::path::PathBuf;

pub const OAUTH_SCOPE: &str = "https://www.googleapis.com/auth/drive.file";
pub const XATTR_DRIVE_ID: &str = "com.gdoclauncher.drive_id";
pub const XATTR_MIME_TYPE: &str = "com.gdoclauncher.mime_type";
/// SHA-256 of the file content at import time — the baseline used to detect
/// later local edits (kept under the legacy `com.gdoclauncher.*` namespace, see
/// CLAUDE.md "Dedup strategy", so it survives the data-dir rename).
pub const XATTR_SHA256: &str = "com.gdoclauncher.sha256";

fn data_base() -> PathBuf {
    dirs::data_dir().unwrap_or_else(|| {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".local/share")
    })
}

pub fn app_support_dir() -> PathBuf {
    data_base().join("Popdoc")
}

/// Pre-rename data directory (`GDocLauncher`). Used only by the one-time token
/// migration in `oauth.rs` so an upgrading install doesn't get logged out.
pub fn legacy_app_support_dir() -> PathBuf {
    data_base().join("GDocLauncher")
}

pub fn imports_db_path() -> PathBuf {
    app_support_dir().join("imports.json")
}

pub fn config_path() -> PathBuf {
    app_support_dir().join("config.json")
}

// Only the non-macOS token store writes a plaintext file; on macOS tokens live
// in the Keychain, so this path is unused there.
#[cfg(not(target_os = "macos"))]
pub fn tokens_path() -> PathBuf {
    app_support_dir().join("tokens.json")
}

pub fn ensure_app_dir() -> std::io::Result<()> {
    std::fs::create_dir_all(app_support_dir())
}
