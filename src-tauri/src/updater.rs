//! Auto-update via `tauri-plugin-updater` (minisign-signed archives hosted on
//! GitHub Releases — see `tauri.conf.json` → `plugins.updater`).
//!
//! Design (per CLAUDE.md "Auto-update"): the app is a transient utility that
//! exits ~500 ms after the import queue drains, so we never block opening the
//! user's file on an update. Instead:
//!
//!   1. A silent check runs in parallel with file processing at launch
//!      (`spawn_launch_check`), throttled to once every [`CHECK_INTERVAL`].
//!   2. The manual "Check for updates" button in Settings calls [`check`] with
//!      `force = true`, bypassing the throttle.
//!   3. When a newer version is found it is downloaded in the background and
//!      *parked* in `AppState::pending_update`. It is swapped into place only at
//!      quit, when the app was about to exit anyway ([`install_pending`]). We do
//!      NOT relaunch — the running process keeps its in-memory code and the next
//!      double-click picks up the new version.

use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tauri::{AppHandle, Manager};
use tauri_plugin_updater::{Update, UpdaterExt};

use crate::AppState;

/// Minimum gap between silent (background) update checks.
const CHECK_INTERVAL: Duration = Duration::from_secs(6 * 60 * 60); // 6 h

/// How long the quit path will wait for an in-flight background download to
/// finish before giving up (the update then lands on a later run instead).
const DRAIN_WAIT: Duration = Duration::from_secs(20);

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// A downloaded-but-not-yet-installed update, parked until the app quits.
pub struct PendingUpdate {
    pub version: String,
    update: Update,
    bytes: Vec<u8>,
}

impl PendingUpdate {
    /// Swap the new bundle into place. Synchronous; does not relaunch.
    fn install(self) -> tauri_plugin_updater::Result<()> {
        self.update.install(self.bytes)
    }
}

/// Outcome of a check, surfaced to the Settings UI by the Tauri command.
#[derive(serde::Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum CheckResult {
    /// Already on the latest version.
    UpToDate,
    /// A newer version was found and downloaded; it installs on quit.
    Downloaded { version: String },
    /// The check was throttled (background only) and didn't run.
    Skipped,
    /// Network / signature / config error — message is for logs, not the user.
    Error { message: String },
}

/// Check GitHub for a newer release. When found, download it and park it in
/// `AppState::pending_update` for install-on-quit. `force` bypasses the throttle
/// (used by the manual button); the background check passes `false`.
pub async fn check(app: &AppHandle, force: bool) -> CheckResult {
    if !force {
        let last = crate::settings::last_update_check().unwrap_or(0);
        if now_secs().saturating_sub(last) < CHECK_INTERVAL.as_secs() {
            return CheckResult::Skipped;
        }
    }
    // Record the attempt up front so a flapping network doesn't hammer GitHub.
    let _ = crate::settings::set_last_update_check(now_secs());

    let updater = match app.updater() {
        Ok(u) => u,
        Err(e) => return CheckResult::Error { message: e.to_string() },
    };

    let update = match updater.check().await {
        Ok(Some(u)) => u,
        Ok(None) => return CheckResult::UpToDate,
        Err(e) => return CheckResult::Error { message: e.to_string() },
    };

    let version = update.version.clone();
    let state = app.state::<Arc<AppState>>().inner().clone();
    state.update_downloading.store(true, Ordering::SeqCst);

    let bytes = update.download(|_, _| {}, || {}).await;
    state.update_downloading.store(false, Ordering::SeqCst);

    match bytes {
        Ok(bytes) => {
            eprintln!("[GDL] update {version} downloaded, will install on quit");
            *state.pending_update.lock().unwrap() = Some(PendingUpdate {
                version: version.clone(),
                update,
                bytes,
            });
            CheckResult::Downloaded { version }
        }
        Err(e) => CheckResult::Error { message: e.to_string() },
    }
}

/// Fire the silent launch check in the background. Returns immediately — it must
/// never delay opening the user's file.
pub fn spawn_launch_check(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        if let CheckResult::Error { message } = check(&app, false).await {
            eprintln!("[GDL] update check failed: {message}");
        }
    });
}

/// Install a parked update if one is ready. Called from the quit path, just
/// before `app.exit(0)`. If a background download is still running, wait briefly
/// for it (the user's file already opened, so this is invisible) then install.
/// Best-effort: any failure just leaves the current version in place.
pub async fn install_pending(app: &AppHandle) {
    let state = app.state::<Arc<AppState>>().inner().clone();

    if state.update_downloading.load(Ordering::SeqCst) {
        let deadline = now_secs() + DRAIN_WAIT.as_secs();
        while state.update_downloading.load(Ordering::SeqCst) && now_secs() < deadline {
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
    }

    let pending = state.pending_update.lock().unwrap().take();
    if let Some(p) = pending {
        let version = p.version.clone();
        match p.install() {
            Ok(()) => eprintln!("[GDL] installed update {version} (active on next launch)"),
            Err(e) => eprintln!("[GDL] update install failed: {e}"),
        }
    }
}
