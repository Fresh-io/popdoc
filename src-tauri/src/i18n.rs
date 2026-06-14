//! Lightweight internationalisation for the native (Rust) side of the app:
//! progress statuses, blocking dialogs, the OAuth confirmation page and window
//! titles. The web UIs (`progress.html`, `settings.html`) carry their own
//! mirror table in `src/i18n.js`; both sides resolve to the same `Lang` so the
//! floating progress card and the events Rust emits into it always agree.
//!
//! Resolution order: explicit user preference (config.json) → macOS system
//! locale (`sys-locale`) → English fallback. Only the three languages the
//! website ships in are recognised (FR / EN / ES).

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Lang {
    Fr,
    En,
    Es,
}

impl Lang {
    pub fn code(self) -> &'static str {
        match self {
            Lang::Fr => "fr",
            Lang::En => "en",
            Lang::Es => "es",
        }
    }

    /// Parse a BCP-47-ish tag (`"fr"`, `"fr-FR"`, `"es_ES"`, …). Returns `None`
    /// for anything we don't translate, so callers fall through to the next
    /// source. `"auto"` deliberately maps to `None`.
    pub fn from_code(code: &str) -> Option<Lang> {
        match code.get(0..2).unwrap_or("").to_ascii_lowercase().as_str() {
            "fr" => Some(Lang::Fr),
            "es" => Some(Lang::Es),
            "en" => Some(Lang::En),
            _ => None,
        }
    }
}

/// What the system locale alone would select, ignoring any saved preference.
/// Exposed to the settings UI so the "System" dropdown choice can preview the
/// language it resolves to.
pub fn auto_lang() -> Lang {
    sys_locale::get_locale()
        .and_then(|loc| Lang::from_code(&loc))
        .unwrap_or(Lang::En)
}

/// The effective language: saved preference wins, otherwise the system locale.
pub fn resolve() -> Lang {
    if let Some(pref) = crate::settings::language_pref() {
        if let Some(l) = Lang::from_code(&pref) {
            return l;
        }
    }
    auto_lang()
}

#[derive(Clone, Copy)]
pub enum Key {
    ImportInProgress,
    ConnectingDrive,
    AuthFailed,
    CheckingFile,
    PreparingFolder,
    DriveError,
    DriveFolderError,
    ImportFailed,
    FileNotFound,
    UnsupportedFormat,
    DupBody,
    DupTitle,
    DupOpenExisting,
    DupImportCopy,
    ModifiedTitle,
    ModifiedBody,
    SettingsWindowTitle,
    OAuthConnectedTitle,
    OAuthConnectedBody,
    OAuthFailedTitle,
    OAuthFailedBody,
}

pub fn t(lang: Lang, key: Key) -> &'static str {
    use Key::*;
    use Lang::*;
    match (key, lang) {
        (ImportInProgress, Fr) => "Import en cours…",
        (ImportInProgress, En) => "Importing…",
        (ImportInProgress, Es) => "Importando…",

        (ConnectingDrive, Fr) => "Connexion à Google Drive…",
        (ConnectingDrive, En) => "Connecting to Google Drive…",
        (ConnectingDrive, Es) => "Conectando con Google Drive…",

        (AuthFailed, Fr) => "Authentification échouée",
        (AuthFailed, En) => "Authentication failed",
        (AuthFailed, Es) => "Error de autenticación",

        (CheckingFile, Fr) => "Vérification du fichier…",
        (CheckingFile, En) => "Checking the file…",
        (CheckingFile, Es) => "Comprobando el archivo…",

        (PreparingFolder, Fr) => "Préparation du dossier Drive…",
        (PreparingFolder, En) => "Preparing the Drive folder…",
        (PreparingFolder, Es) => "Preparando la carpeta de Drive…",

        (DriveError, Fr) => "Erreur Drive",
        (DriveError, En) => "Drive error",
        (DriveError, Es) => "Error de Drive",

        (DriveFolderError, Fr) => "Erreur dossier Drive",
        (DriveFolderError, En) => "Drive folder error",
        (DriveFolderError, Es) => "Error de carpeta de Drive",

        (ImportFailed, Fr) => "Import échoué",
        (ImportFailed, En) => "Import failed",
        (ImportFailed, Es) => "Error de importación",

        (FileNotFound, Fr) => "Fichier introuvable",
        (FileNotFound, En) => "File not found",
        (FileNotFound, Es) => "Archivo no encontrado",

        (UnsupportedFormat, Fr) => "Format non supporté",
        (UnsupportedFormat, En) => "Unsupported format",
        (UnsupportedFormat, Es) => "Formato no compatible",

        // Prefix only — the file name is appended by the caller.
        (DupBody, Fr) => "Ce fichier a déjà été importé.\n\n",
        (DupBody, En) => "This file has already been imported.\n\n",
        (DupBody, Es) => "Este archivo ya se ha importado.\n\n",

        (DupTitle, Fr) => "Fichier déjà importé",
        (DupTitle, En) => "File already imported",
        (DupTitle, Es) => "Archivo ya importado",

        (DupOpenExisting, Fr) => "Ouvrir l'existant",
        (DupOpenExisting, En) => "Open existing",
        (DupOpenExisting, Es) => "Abrir el existente",

        (DupImportCopy, Fr) => "Importer une copie",
        (DupImportCopy, En) => "Import a copy",
        (DupImportCopy, Es) => "Importar una copia",

        (ModifiedTitle, Fr) => "Fichier modifié",
        (ModifiedTitle, En) => "File modified",
        (ModifiedTitle, Es) => "Archivo modificado",

        // Prefix only — the file name is appended by the caller.
        (ModifiedBody, Fr) => {
            "Ce fichier a été modifié depuis son dernier import.\nUn nouveau document Google va être créé pour :\n\n"
        }
        (ModifiedBody, En) => {
            "This file has changed since it was last imported.\nA new Google document will be created for:\n\n"
        }
        (ModifiedBody, Es) => {
            "Este archivo ha cambiado desde la última importación.\nSe creará un nuevo documento de Google para:\n\n"
        }

        (SettingsWindowTitle, Fr) => "Réglages — Popdoc",
        (SettingsWindowTitle, En) => "Settings — Popdoc",
        (SettingsWindowTitle, Es) => "Ajustes — Popdoc",

        (OAuthConnectedTitle, Fr) => "Connecté à Google Drive",
        (OAuthConnectedTitle, En) => "Connected to Google Drive",
        (OAuthConnectedTitle, Es) => "Conectado a Google Drive",

        (OAuthConnectedBody, Fr) => "Tu peux fermer cet onglet et retourner à ton document.",
        (OAuthConnectedBody, En) => "You can close this tab and return to your document.",
        (OAuthConnectedBody, Es) => "Puedes cerrar esta pestaña y volver a tu documento.",

        (OAuthFailedTitle, Fr) => "Connexion annulée",
        (OAuthFailedTitle, En) => "Sign-in cancelled",
        (OAuthFailedTitle, Es) => "Conexión cancelada",

        (OAuthFailedBody, Fr) => "Tu peux fermer cet onglet et réessayer depuis ton document.",
        (OAuthFailedBody, En) => "You can close this tab and try again from your document.",
        (OAuthFailedBody, Es) => "Puedes cerrar esta pestaña e intentarlo de nuevo desde tu documento.",
    }
}
