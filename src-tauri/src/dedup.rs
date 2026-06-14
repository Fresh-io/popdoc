use anyhow::Result;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::io::Read;
use std::path::Path;

use crate::config::{
    ensure_app_dir, imports_db_path, XATTR_DRIVE_ID, XATTR_MIME_TYPE, XATTR_SHA256,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ImportEntry {
    pub drive_id: String,
    pub google_mime: String,
    pub original_name: String,
    pub imported_at_ms: u64,
}

type Db = HashMap<String, ImportEntry>;

#[derive(Clone, Debug)]
pub enum DedupSource {
    Xattr,
    Hash,
}

#[derive(Clone, Debug)]
pub struct DedupHit {
    pub drive_id: String,
    pub google_mime: String,
    pub source: DedupSource,
    /// The local file was edited since it was imported (its content no longer
    /// matches the baseline recorded at import time). Always `false` for a
    /// `Hash` hit, since that match is itself a content-identity check.
    pub modified: bool,
}

fn load_db() -> Db {
    std::fs::read_to_string(imports_db_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_db(db: &Db) -> Result<()> {
    ensure_app_dir()?;
    let tmp = imports_db_path().with_extension("tmp");
    std::fs::write(&tmp, serde_json::to_vec_pretty(db)?)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600));
    }
    std::fs::rename(tmp, imports_db_path())?;
    Ok(())
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn read_xattrs(path: &Path) -> Option<(String, String)> {
    let id = xattr::get(path, XATTR_DRIVE_ID).ok().flatten()?;
    let mime = xattr::get(path, XATTR_MIME_TYPE).ok().flatten()?;
    Some((
        String::from_utf8(id).ok()?,
        String::from_utf8(mime).ok()?,
    ))
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn read_xattrs(_path: &Path) -> Option<(String, String)> {
    None
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn read_hash_xattr(path: &Path) -> Option<String> {
    let h = xattr::get(path, XATTR_SHA256).ok().flatten()?;
    String::from_utf8(h).ok()
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn read_hash_xattr(_path: &Path) -> Option<String> {
    None
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn write_xattrs(path: &Path, drive_id: &str, mime: &str, hash: Option<&str>) {
    let _ = xattr::set(path, XATTR_DRIVE_ID, drive_id.as_bytes());
    let _ = xattr::set(path, XATTR_MIME_TYPE, mime.as_bytes());
    if let Some(h) = hash {
        let _ = xattr::set(path, XATTR_SHA256, h.as_bytes());
    }
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn write_xattrs(_path: &Path, _drive_id: &str, _mime: &str, _hash: Option<&str>) {}

/// Recover the import-time hash for a Drive file from the imports DB — the
/// fallback baseline for files imported before the hash xattr existed.
fn baseline_hash_for_drive_id(db: &Db, drive_id: &str) -> Option<String> {
    db.iter()
        .find(|(_, e)| e.drive_id == drive_id)
        .map(|(hash, _)| hash.clone())
}

pub fn sha256_file(path: &Path) -> Result<String> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex::encode(hasher.finalize()))
}

pub fn lookup(path: &Path) -> Result<Option<DedupHit>> {
    if let Some((drive_id, google_mime)) = read_xattrs(path) {
        // Compare the current content against the baseline captured at import
        // time (preferring the xattr, then the imports DB). If neither baseline
        // is recoverable we can't tell, so we conservatively report unmodified.
        let current = sha256_file(path)?;
        let baseline =
            read_hash_xattr(path).or_else(|| baseline_hash_for_drive_id(&load_db(), &drive_id));
        let modified = baseline.is_some_and(|base| base != current);
        return Ok(Some(DedupHit {
            drive_id,
            google_mime,
            source: DedupSource::Xattr,
            modified,
        }));
    }
    let hash = sha256_file(path)?;
    let db = load_db();
    Ok(db.get(&hash).map(|e| DedupHit {
        drive_id: e.drive_id.clone(),
        google_mime: e.google_mime.clone(),
        source: DedupSource::Hash,
        modified: false,
    }))
}

pub fn record(path: &Path, drive_id: &str, google_mime: &str) -> Result<()> {
    let hash = sha256_file(path)?;
    let mut db = load_db();
    db.insert(
        hash.clone(),
        ImportEntry {
            drive_id: drive_id.to_string(),
            google_mime: google_mime.to_string(),
            original_name: path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string(),
            imported_at_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0),
        },
    );
    save_db(&db)?;
    write_xattrs(path, drive_id, google_mime, Some(&hash));
    Ok(())
}

pub fn backfill_xattr(path: &Path, drive_id: &str, google_mime: &str) {
    // The file matched by content hash, so its current digest *is* the baseline.
    let hash = sha256_file(path).ok();
    write_xattrs(path, drive_id, google_mime, hash.as_deref());
}

pub fn remove_record(drive_id: &str) -> Result<()> {
    let mut db = load_db();
    let before = db.len();
    db.retain(|_, e| e.drive_id != drive_id);
    if db.len() != before {
        save_db(&db)?;
    }
    Ok(())
}
