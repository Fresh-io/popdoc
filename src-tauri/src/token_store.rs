// Secure storage for the Google OAuth tokens (access + refresh).
//
// macOS: the login Keychain. This is safe now that release builds are
// Developer-ID signed + notarized — the code signature is stable across
// rebuilds, so the Keychain ACL no longer re-prompts on every dev build
// (the reason we had previously moved to a plaintext 0600 file).
//
// Other platforms: fall back to the same 0600 file in Application Support.
//
// The store deals in opaque strings; `oauth.rs` owns the JSON shape and the
// one-time migration of any legacy plaintext file.

#[cfg(target_os = "macos")]
pub use macos::{clear, load, save};

#[cfg(not(target_os = "macos"))]
pub use fallback::{clear, load, save};

#[cfg(target_os = "macos")]
mod macos {
    use anyhow::{anyhow, Result};
    use keyring_core::{Entry, Error};
    use std::sync::OnceLock;

    // User-facing: this string is what the macOS Keychain authorization dialog
    // shows ("Popdoc wants to use confidential information stored in «Popdoc»").
    // Deliberately NOT the bundle id — no "myli" must appear in any user-visible
    // surface. Safe to choose freely: only our own code references this entry.
    const SERVICE: &str = "Popdoc";
    const ACCOUNT: &str = "Google Drive";

    // keyring 4 split the crate into `keyring-core` (the `Entry` API) plus a
    // pluggable backend that must be registered as the process-wide default
    // store before any `Entry` is used. We register the Apple native Keychain
    // store — the same `security_framework` generic-password mechanism the
    // keyring 3 `apple-native` backend used, so items stay byte-compatible:
    // tokens written by older builds keep loading, same Keychain ACL / dialog.
    fn ensure_store() -> Result<()> {
        static STORE: OnceLock<std::result::Result<(), String>> = OnceLock::new();
        STORE
            .get_or_init(|| {
                let store = apple_native_keyring_store::keychain::Store::new()
                    .map_err(|e| format!("init keychain store: {e}"))?;
                keyring_core::set_default_store(store);
                Ok(())
            })
            .clone()
            .map_err(|e| anyhow!(e))
    }

    fn entry() -> Result<Entry> {
        ensure_store()?;
        Entry::new(SERVICE, ACCOUNT).map_err(|e| anyhow!("keychain entry: {e}"))
    }

    pub fn load() -> Result<Option<String>> {
        match entry()?.get_password() {
            Ok(s) => Ok(Some(s)),
            Err(Error::NoEntry) => Ok(None),
            Err(e) => Err(anyhow!("keychain read: {e}")),
        }
    }

    pub fn save(secret: &str) -> Result<()> {
        entry()?
            .set_password(secret)
            .map_err(|e| anyhow!("keychain write: {e}"))
    }

    pub fn clear() -> Result<()> {
        match entry()?.delete_credential() {
            Ok(()) | Err(Error::NoEntry) => Ok(()),
            Err(e) => Err(anyhow!("keychain delete: {e}")),
        }
    }
}

#[cfg(not(target_os = "macos"))]
mod fallback {
    use crate::config::{ensure_app_dir, tokens_path};
    use anyhow::{anyhow, Result};

    pub fn load() -> Result<Option<String>> {
        match std::fs::read_to_string(tokens_path()) {
            Ok(s) => Ok(Some(s)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(anyhow!("tokens read: {e}")),
        }
    }

    pub fn save(secret: &str) -> Result<()> {
        ensure_app_dir()?;
        let path = tokens_path();
        let tmp = path.with_extension("tmp");
        std::fs::write(&tmp, secret)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600));
        }
        std::fs::rename(&tmp, &path)?;
        Ok(())
    }

    pub fn clear() -> Result<()> {
        match std::fs::remove_file(tokens_path()) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(anyhow!("tokens delete: {e}")),
        }
    }
}
