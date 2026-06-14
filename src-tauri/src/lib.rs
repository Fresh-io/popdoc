mod config;
mod credentials;
mod dedup;
mod drive;
mod i18n;
mod mime_map;
mod oauth;
mod settings;
mod token_store;
mod updater;

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tauri::{AppHandle, Emitter, Manager, RunEvent, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
use tokio::sync::Mutex;

use crate::dedup::DedupSource;
use crate::i18n::{t, Key, Lang};
use crate::mime_map::{doc_type_from_google_mime, drive_url, mapping_for_file};

#[derive(Default)]
pub(crate) struct AppState {
    queue: Mutex<Vec<PathBuf>>,
    processing: Mutex<bool>,
    received_file: AtomicBool,
    /// An update downloaded in the background, parked for install-on-quit.
    pub(crate) pending_update: std::sync::Mutex<Option<updater::PendingUpdate>>,
    /// True while a background update download is in flight, so the quit path
    /// can wait briefly for it to finish before installing.
    pub(crate) update_downloading: AtomicBool,
}

fn notify(app: &AppHandle, title: &str, body: &str) {
    eprintln!("[GDL] {title}: {body}");
    // Native blocking dialog — works reliably without prior user permission.
    let _ = app
        .dialog()
        .message(body)
        .title(title)
        .kind(MessageDialogKind::Error)
        .blocking_show();
}

async fn ask_duplicate(app: &AppHandle, file_name: &str, lang: Lang) -> DuplicateChoice {
    let app = app.clone();
    let file_name = file_name.to_string();
    let result = tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .message(format!("{}{}", t(lang, Key::DupBody), file_name))
            .title(t(lang, Key::DupTitle))
            .kind(MessageDialogKind::Info)
            .buttons(MessageDialogButtons::OkCancelCustom(
                t(lang, Key::DupOpenExisting).to_string(),
                t(lang, Key::DupImportCopy).to_string(),
            ))
            .blocking_show()
    })
    .await
    .unwrap_or(false);

    if result {
        DuplicateChoice::OpenExisting
    } else {
        DuplicateChoice::ImportNew
    }
}

enum DuplicateChoice {
    OpenExisting,
    ImportNew,
}

/// Inform the user that the local file changed since it was imported, so a new
/// Google document will be created instead of reopening the stale one. Modal,
/// single acknowledge button — the import proceeds once it's dismissed.
async fn notify_modified(app: &AppHandle, file_name: &str, lang: Lang) {
    let app = app.clone();
    let file_name = file_name.to_string();
    let _ = tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .message(format!("{}{}", t(lang, Key::ModifiedBody), file_name))
            .title(t(lang, Key::ModifiedTitle))
            .kind(MessageDialogKind::Info)
            .blocking_show()
    })
    .await;
}

const PROGRESS_WIDTH: f64 = 320.0;
const PROGRESS_HEIGHT: f64 = 140.0;

fn open_progress_window(app: &AppHandle, file_name: &str, lang: Lang) {
    if app.get_webview_window("progress").is_some() {
        let _ = app.emit("progress:status", t(lang, Key::ImportInProgress).to_string());
        return;
    }

    let url = format!(
        "progress.html?name={}&lang={}",
        urlencoding::encode(file_name),
        lang.code(),
    );

    let monitor = app.primary_monitor().ok().flatten();
    let (mx, my, mw, mh) = if let Some(m) = monitor {
        let pos = m.position();
        let size = m.size();
        let scale = m.scale_factor();
        (
            pos.x as f64 / scale,
            pos.y as f64 / scale,
            size.width as f64 / scale,
            size.height as f64 / scale,
        )
    } else {
        (0.0, 0.0, 1440.0, 900.0)
    };
    let x = mx + (mw - PROGRESS_WIDTH) / 2.0;
    let y = my + mh - PROGRESS_HEIGHT - 60.0;

    let builder = WebviewWindowBuilder::new(app, "progress", WebviewUrl::App(url.into()))
        .title("Popdoc")
        .inner_size(PROGRESS_WIDTH, PROGRESS_HEIGHT)
        .position(x, y)
        .resizable(false)
        .minimizable(false)
        .maximizable(false)
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .skip_taskbar(true)
        .shadow(false)
        .focused(false)
        .visible(true);

    if let Err(e) = builder.build() {
        eprintln!("[GDL] failed to open progress window: {e}");
    }
}

fn close_progress_window(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("progress") {
        // Let the fade-out animation play, then close.
        let app_clone = app.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(900));
            if let Some(w) = app_clone.get_webview_window("progress") {
                let _ = w.close();
            }
        });
        let _ = w; // referenced
    }
}

fn close_progress_window_now(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("progress") {
        let _ = w.close();
    }
}

fn emit_status(app: &AppHandle, status: &str) {
    let _ = app.emit("progress:status", status.to_string());
}

fn emit_done(app: &AppHandle) {
    let _ = app.emit("progress:done", ());
}

fn emit_error(app: &AppHandle, message: &str) {
    let _ = app.emit("progress:error", message.to_string());
}

async fn process_file(app: AppHandle, client: reqwest::Client, file_path: PathBuf) {
    let lang = i18n::resolve();

    if !file_path.exists() {
        notify(&app, t(lang, Key::FileNotFound), &file_path.display().to_string());
        return;
    }

    let Some(mapping) = mapping_for_file(&file_path) else {
        notify(
            &app,
            t(lang, Key::UnsupportedFormat),
            &file_path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string(),
        );
        return;
    };

    let display_name = file_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string();

    open_progress_window(&app, &display_name, lang);
    // Give the webview a moment to mount its event listeners.
    tokio::time::sleep(std::time::Duration::from_millis(120)).await;
    emit_status(&app, t(lang, Key::ConnectingDrive));

    let access_token = match oauth::get_access_token(&client, lang).await {
        Ok(t) => t,
        Err(e) => {
            emit_error(&app, t(lang, Key::AuthFailed));
            close_progress_window(&app);
            notify(&app, t(lang, Key::AuthFailed), &e.to_string());
            return;
        }
    };

    emit_status(&app, t(lang, Key::CheckingFile));

    let hit = match dedup::lookup(&file_path) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("[GDL] dedup lookup failed: {e}");
            None
        }
    };

    if let Some(h) = hit {
        let still_exists = drive::check_exists(&client, &access_token, &h.drive_id).await;
        if !still_exists {
            let _ = dedup::remove_record(&h.drive_id);
        } else if h.modified {
            // The file was edited since it was imported — reopening the existing
            // Google doc would show stale content. Tell the user and fall
            // through to a fresh import (which records the new baseline).
            close_progress_window_now(&app);
            notify_modified(&app, &display_name, lang).await;
            open_progress_window(&app, &display_name, lang);
            tokio::time::sleep(std::time::Duration::from_millis(120)).await;
        } else {
            // Close progress immediately — the modal dialog must not be covered
            // by the always-on-top progress window, which would steal clicks.
            close_progress_window_now(&app);
            match ask_duplicate(&app, &display_name, lang).await {
                DuplicateChoice::OpenExisting => {
                    if matches!(h.source, DedupSource::Hash) {
                        dedup::backfill_xattr(&file_path, &h.drive_id, &h.google_mime);
                    }
                    let url = drive_url(doc_type_from_google_mime(&h.google_mime), &h.drive_id);
                    let _ = tauri_plugin_opener::open_url(&url, None::<&str>);
                    return;
                }
                DuplicateChoice::ImportNew => {
                    open_progress_window(&app, &display_name, lang);
                    tokio::time::sleep(std::time::Duration::from_millis(120)).await;
                }
            }
        }
    }

    emit_status(&app, t(lang, Key::PreparingFolder));

    let folder_id = match drive::find_or_create_folder(&client, &access_token).await {
        Ok(id) => id,
        Err(e) => {
            emit_error(&app, t(lang, Key::DriveError));
            close_progress_window(&app);
            notify(&app, t(lang, Key::DriveFolderError), &e.to_string());
            return;
        }
    };

    emit_status(&app, t(lang, Key::ImportInProgress));

    match drive::upload_and_convert(&client, &access_token, &file_path, &mapping, &folder_id).await
    {
        Ok(result) => {
            if let Err(e) = dedup::record(&file_path, &result.file_id, &result.google_mime) {
                eprintln!("dedup record failed: {e}");
            }
            let url = drive_url(mapping.doc_type, &result.file_id);
            emit_done(&app);
            let _ = tauri_plugin_opener::open_url(&url, None::<&str>);
            close_progress_window(&app);
        }
        Err(e) => {
            emit_error(&app, t(lang, Key::ImportFailed));
            close_progress_window(&app);
            notify(&app, t(lang, Key::ImportFailed), &e.to_string());
        }
    }
}

async fn drain_queue(app: AppHandle) {
    let state = app.state::<Arc<AppState>>();
    {
        let mut p = state.processing.lock().await;
        if *p {
            return;
        }
        *p = true;
    }
    let client = reqwest::Client::builder()
        .user_agent("Popdoc/0.2")
        .build()
        .expect("reqwest client");

    loop {
        let next = {
            let mut q = state.queue.lock().await;
            q.pop()
        };
        let Some(path) = next else { break };
        process_file(app.clone(), client.clone(), path).await;
    }

    {
        let mut p = state.processing.lock().await;
        *p = false;
    }

    // Exit if nothing more to do AND no visible windows (transient utility behaviour).
    // If the user opened the settings window, we stay alive until they close it.
    let q = state.queue.lock().await;
    if q.is_empty() {
        let app_clone = app.clone();
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            let has_visible_window = app_clone
                .webview_windows()
                .values()
                .any(|w| w.label() == "settings" && w.is_visible().unwrap_or(false));
            if !has_visible_window {
                updater::install_pending(&app_clone).await;
                app_clone.exit(0);
            }
        });
    }
}

fn enqueue_paths(app: &AppHandle, paths: Vec<PathBuf>) {
    if paths.is_empty() {
        return;
    }
    let state = app.state::<Arc<AppState>>().inner().clone();
    state.received_file.store(true, Ordering::SeqCst);
    let app_clone = app.clone();
    tauri::async_runtime::spawn(async move {
        {
            let mut q = state.queue.lock().await;
            for p in paths {
                q.push(p);
            }
        }
        drain_queue(app_clone).await;
    });
}

fn paths_from_argv(argv: &[String]) -> Vec<PathBuf> {
    argv.iter()
        .skip(1)
        .filter(|s| !s.starts_with('-'))
        .map(PathBuf::from)
        .filter(|p| p.exists() && mapping_for_file(p).is_some())
        .collect()
}

#[derive(serde::Serialize)]
struct SettingsPayload {
    drive_folder_name: String,
    version: String,
    /// Saved language preference: `"auto"`, `"fr"`, `"en"` or `"es"`.
    language: String,
    /// What `"auto"` currently resolves to, so the UI can preview the System
    /// option and translate itself live when it's picked.
    auto_lang: String,
}

#[tauri::command]
fn get_settings() -> SettingsPayload {
    SettingsPayload {
        drive_folder_name: settings::drive_folder_name(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        language: settings::language_pref().unwrap_or_else(|| "auto".to_string()),
        auto_lang: i18n::auto_lang().code().to_string(),
    }
}

#[tauri::command]
fn set_drive_folder(name: String) -> Result<(), String> {
    settings::set_drive_folder_name(&name).map_err(|e| e.to_string())
}

#[tauri::command]
fn set_language(code: String) -> Result<(), String> {
    settings::set_language(&code).map_err(|e| e.to_string())
}

#[tauri::command]
fn sign_out() -> Result<(), String> {
    oauth::clear_tokens().map_err(|e| e.to_string())
}

/// Manual "Check for updates" from the Settings window. Bypasses the throttle;
/// a found update is downloaded and applied when the app next quits.
#[tauri::command]
async fn check_for_updates(app: AppHandle) -> updater::CheckResult {
    updater::check(&app, true).await
}

fn open_settings_window(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("settings") {
        let _ = w.set_focus();
        return;
    }
    let lang = i18n::resolve();
    let url = format!("settings.html?lang={}", lang.code());
    let builder = WebviewWindowBuilder::new(app, "settings", WebviewUrl::App(url.into()))
        .title(t(lang, Key::SettingsWindowTitle))
        .inner_size(560.0, 480.0)
        .resizable(false)
        .minimizable(false)
        .maximizable(false)
        .center()
        .visible(true);
    if let Err(e) = builder.build() {
        eprintln!("[GDL] failed to open settings window: {e}");
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let state = Arc::new(AppState::default());

    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            let paths = paths_from_argv(&argv);
            if paths.is_empty() {
                open_settings_window(app);
            } else {
                enqueue_paths(app, paths);
            }
        }))
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            get_settings,
            set_drive_folder,
            set_language,
            sign_out,
            check_for_updates
        ])
        .setup(|app| {
            let handle = app.handle().clone();
            // Silent, throttled update check in parallel with file processing.
            // Never blocks opening the user's file; applies on quit.
            updater::spawn_launch_check(&handle);
            let argv: Vec<String> = std::env::args().collect();
            let initial = paths_from_argv(&argv);
            if !initial.is_empty() {
                enqueue_paths(&handle, initial);
            } else {
                // No file in argv. On macOS, "Open with…" delivers the file via
                // RunEvent::Opened *after* setup() runs, so we can't decide yet.
                // Wait briefly; if nothing got queued, treat this as a direct launch
                // and open the settings window.
                let handle_clone = handle.clone();
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
                    let state = handle_clone.state::<Arc<AppState>>().inner().clone();
                    let should_open = !state.received_file.load(Ordering::SeqCst);
                    if should_open {
                        let h = handle_clone.clone();
                        let _ = handle_clone.run_on_main_thread(move || {
                            open_settings_window(&h);
                        });
                    }
                });
            }
            Ok(())
        });

    let app = builder
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(move |app_handle, event| match event {
        RunEvent::ExitRequested { api, .. } => {
            // macOS triggers ExitRequested when the last window closes. If we're
            // mid-import (e.g. the progress window was closed to show the
            // duplicate dialog), prevent the exit until the queue drains.
            let state = app_handle.state::<Arc<AppState>>().inner().clone();
            let (q_empty, processing) = tauri::async_runtime::block_on(async {
                let q = state.queue.lock().await;
                let p = state.processing.lock().await;
                (q.is_empty(), *p)
            });
            if processing || !q_empty {
                api.prevent_exit();
            }
        }
        RunEvent::Opened { urls } => {
            let paths: Vec<PathBuf> = urls
                .into_iter()
                .filter_map(|u| u.to_file_path().ok())
                .filter(|p: &PathBuf| p.exists() && mapping_for_file(p).is_some())
                .collect();
            enqueue_paths(app_handle, paths);
        }
        RunEvent::WindowEvent {
            label,
            event: tauri::WindowEvent::Destroyed,
            ..
        } if label == "settings" => {
            let app_clone = app_handle.clone();
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_millis(200));
                let any_window_open = !app_clone.webview_windows().is_empty();
                let state = app_clone.state::<Arc<AppState>>().inner().clone();
                let h2 = app_clone.clone();
                tauri::async_runtime::spawn(async move {
                    let should_exit = {
                        let q = state.queue.lock().await;
                        let p = state.processing.lock().await;
                        q.is_empty() && !*p && !any_window_open
                    };
                    if should_exit {
                        updater::install_pending(&h2).await;
                        h2.exit(0);
                    }
                });
            });
        }
        _ => {}
    });
}
