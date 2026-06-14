use anyhow::{anyhow, Result};
use base64::Engine;
use rand::RngExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;

use crate::config::OAUTH_SCOPE;
use crate::credentials::{EMBEDDED_CLIENT_ID, EMBEDDED_CLIENT_SECRET};
use crate::token_store;
use crate::i18n::{self, Key, Lang};

const AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const TOKEN_URL: &str = "https://oauth2.googleapis.com/token";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StoredTokens {
    pub access_token: String,
    pub refresh_token: String,
    pub expiry_epoch_ms: u64,
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn base64url(bytes: &[u8]) -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

fn random_bytes(n: usize) -> Vec<u8> {
    let mut buf = vec![0u8; n];
    rand::rng().fill(&mut buf[..]);
    buf
}

// Tokens live in the secure store (`token_store`): the macOS Keychain on mac,
// a 0600 file elsewhere. Refresh tokens can be revoked at any time from
// myaccount.google.com if needed.

pub fn load_tokens() -> Result<Option<StoredTokens>> {
    if let Some(raw) = token_store::load()? {
        match serde_json::from_str::<StoredTokens>(&raw) {
            Ok(t) => return Ok(Some(t)),
            Err(_) => {
                // Corrupt entry — drop it and fall through to a fresh login.
                let _ = token_store::clear();
            }
        }
    }

    // One-time migration: pre-notarization builds kept tokens in a plaintext
    // 0600 file, under the old `GDocLauncher` data dir. Move it into the
    // Keychain, then delete the file on disk. Save must succeed before we
    // delete, otherwise we'd lose the only copy of the refresh token.
    #[cfg(target_os = "macos")]
    {
        let legacy = crate::config::legacy_app_support_dir().join("tokens.json");
        if let Ok(raw) = std::fs::read_to_string(&legacy) {
            if let Ok(t) = serde_json::from_str::<StoredTokens>(&raw) {
                if token_store::save(&raw).is_ok() {
                    let _ = std::fs::remove_file(&legacy);
                    return Ok(Some(t));
                }
            } else {
                let _ = std::fs::remove_file(&legacy);
            }
        }
    }

    Ok(None)
}

pub fn save_tokens(tokens: &StoredTokens) -> Result<()> {
    let raw = serde_json::to_string(tokens)?;
    token_store::save(&raw)?;
    // Remove any leftover legacy plaintext file once the secret is in the Keychain.
    #[cfg(target_os = "macos")]
    {
        let _ = std::fs::remove_file(crate::config::legacy_app_support_dir().join("tokens.json"));
    }
    Ok(())
}

pub fn clear_tokens() -> Result<()> {
    token_store::clear()?;
    // Belt-and-braces: also remove any legacy plaintext file.
    #[cfg(target_os = "macos")]
    {
        let _ = std::fs::remove_file(crate::config::legacy_app_support_dir().join("tokens.json"));
    }
    Ok(())
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: u64,
}

async fn exchange_code(
    client: &reqwest::Client,
    code: &str,
    verifier: &str,
    redirect_uri: &str,
) -> Result<StoredTokens> {
    let params = [
        ("code", code),
        ("client_id", EMBEDDED_CLIENT_ID),
        ("client_secret", EMBEDDED_CLIENT_SECRET),
        ("redirect_uri", redirect_uri),
        ("grant_type", "authorization_code"),
        ("code_verifier", verifier),
    ];
    let res = client.post(TOKEN_URL).form(&params).send().await?;
    if !res.status().is_success() {
        let s = res.status();
        let t = res.text().await.unwrap_or_default();
        return Err(anyhow!("token exchange {s}: {t}"));
    }
    let tr: TokenResponse = res.json().await?;
    let refresh = tr
        .refresh_token
        .ok_or_else(|| anyhow!("no refresh_token returned (was prompt=consent set?)"))?;
    Ok(StoredTokens {
        access_token: tr.access_token,
        refresh_token: refresh,
        expiry_epoch_ms: now_ms() + tr.expires_in * 1000,
    })
}

async fn refresh_access_token(
    client: &reqwest::Client,
    refresh_token: &str,
) -> Result<StoredTokens> {
    let params = [
        ("client_id", EMBEDDED_CLIENT_ID),
        ("client_secret", EMBEDDED_CLIENT_SECRET),
        ("refresh_token", refresh_token),
        ("grant_type", "refresh_token"),
    ];
    let res = client.post(TOKEN_URL).form(&params).send().await?;
    if !res.status().is_success() {
        let s = res.status();
        let t = res.text().await.unwrap_or_default();
        return Err(anyhow!("token refresh {s}: {t}"));
    }
    let tr: TokenResponse = res.json().await?;
    Ok(StoredTokens {
        access_token: tr.access_token,
        refresh_token: tr.refresh_token.unwrap_or_else(|| refresh_token.to_string()),
        expiry_epoch_ms: now_ms() + tr.expires_in * 1000,
    })
}

async fn run_loopback_flow(client: &reqwest::Client, lang: Lang) -> Result<StoredTokens> {
    if EMBEDDED_CLIENT_ID.is_empty() || EMBEDDED_CLIENT_SECRET.is_empty() {
        return Err(anyhow!(
            "OAuth credentials missing. Edit src-tauri/src/credentials.rs"
        ));
    }

    let verifier = base64url(&random_bytes(32));
    let challenge = base64url(&Sha256::digest(verifier.as_bytes()));
    let state = base64url(&random_bytes(16));

    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let port = listener.local_addr()?.port();
    let redirect_uri = format!("http://127.0.0.1:{}/callback", port);

    let auth_url = format!(
        "{}?client_id={}&redirect_uri={}&response_type=code&scope={}&state={}&code_challenge={}&code_challenge_method=S256&access_type=offline&prompt=consent",
        AUTH_URL,
        urlencoding::encode(EMBEDDED_CLIENT_ID),
        urlencoding::encode(&redirect_uri),
        urlencoding::encode(OAUTH_SCOPE),
        urlencoding::encode(&state),
        urlencoding::encode(&challenge),
    );

    tauri_plugin_opener::open_url(&auth_url, None::<&str>)
        .map_err(|e| anyhow!("failed to open browser: {e}"))?;

    let (mut socket, _) = tokio::time::timeout(
        std::time::Duration::from_secs(300),
        listener.accept(),
    )
    .await
    .map_err(|_| anyhow!("OAuth timed out"))??;

    let (read_half, mut write_half) = socket.split();
    let mut reader = BufReader::new(read_half.take(8192));
    let mut request_line = String::new();
    reader.read_line(&mut request_line).await?;

    let path = request_line
        .split_whitespace()
        .nth(1)
        .ok_or_else(|| anyhow!("bad request line"))?
        .to_string();

    // Drain headers
    let mut header = String::new();
    loop {
        header.clear();
        let n = reader.read_line(&mut header).await?;
        if n == 0 || header == "\r\n" {
            break;
        }
    }

    let full_url = format!("http://127.0.0.1{}", path);
    let parsed = url::Url::parse(&full_url)?;
    let mut code: Option<String> = None;
    let mut got_state: Option<String> = None;
    let mut got_error: Option<String> = None;
    for (k, v) in parsed.query_pairs() {
        match k.as_ref() {
            "code" => code = Some(v.into_owned()),
            "state" => got_state = Some(v.into_owned()),
            "error" => got_error = Some(v.into_owned()),
            _ => {}
        }
    }

    // Decide the outcome *before* rendering — the browser page must reflect
    // whether Google actually returned a usable code, not just that a request
    // arrived (the user can hit "Cancel", which redirects back with ?error=).
    let succeeded =
        got_error.is_none() && got_state.as_deref() == Some(state.as_str()) && code.is_some();
    let (title_key, body_key) = if succeeded {
        (Key::OAuthConnectedTitle, Key::OAuthConnectedBody)
    } else {
        (Key::OAuthFailedTitle, Key::OAuthFailedBody)
    };

    // Confirmation page in Popdoc colours (light/dark, joyful mascot).
    // The CSS uses `{`/`}`, so we keep the template static and substitute the
    // localised pieces via `replace` rather than `format!`.
    let response_body = r##"<!DOCTYPE html><html lang="__LANG__"><head><meta charset="utf-8"><title>Popdoc</title><style>
:root{color-scheme:light dark}
body{font-family:-apple-system,BlinkMacSystemFont,sans-serif;display:flex;align-items:center;justify-content:center;min-height:92vh;margin:0;background:light-dark(#FAFAF8,#14181D);color:light-dark(#14181D,#F4F6F5)}
.card{text-align:center;padding:48px 56px;border-radius:28px;background:light-dark(#FFFFFF,#1B2129);border:1px solid light-dark(rgba(20,24,29,.09),rgba(244,246,245,.1))}
svg{width:88px;height:88px;color:light-dark(#0E8C75,#2DD4BF);margin-bottom:18px}
h2{font-size:22px;letter-spacing:-.01em;margin:0 0 8px}
p{margin:0;color:light-dark(#4D5560,#A8B2BC)}
.wm{margin-top:26px;font-weight:600;font-size:14px;color:light-dark(#0E8C75,#2DD4BF)}
</style></head><body><div class="card">
<svg viewBox="0 0 1024 1024" aria-hidden="true"><defs><mask id="m"><rect width="1024" height="1024" fill="#fff"/><g stroke="#000" stroke-width="30" stroke-linecap="round" stroke-linejoin="round" fill="none"><path d="M 412 512 L 444 480 L 476 512"/><path d="M 548 512 L 580 480 L 612 512"/></g></mask></defs><g mask="url(#m)" fill="currentColor"><path d="M 336 200 L 572 200 L 572 332 Q 572 380 620 380 L 752 380 L 752 760 Q 752 824 688 824 L 336 824 Q 272 824 272 760 L 272 264 Q 272 200 336 200 Z"/><path d="M 600 200 L 752 352 L 624 352 Q 600 352 600 328 Z"/></g></svg>
<h2>__H2__</h2><p>__P__</p>
<div class="wm">Popdoc</div></div></body></html>"##
        .replace("__LANG__", lang.code())
        .replace("__H2__", i18n::t(lang, title_key))
        .replace("__P__", i18n::t(lang, body_key));
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        response_body.len(),
        response_body
    );
    let _ = write_half.write_all(response.as_bytes()).await;
    let _ = write_half.shutdown().await;

    if let Some(e) = got_error {
        return Err(anyhow!("OAuth error: {e}"));
    }
    if got_state.as_deref() != Some(state.as_str()) {
        return Err(anyhow!("OAuth state mismatch"));
    }
    let code = code.ok_or_else(|| anyhow!("missing code"))?;

    let tokens = exchange_code(client, &code, &verifier, &redirect_uri).await?;
    save_tokens(&tokens)?;
    Ok(tokens)
}

pub async fn get_access_token(client: &reqwest::Client, lang: Lang) -> Result<String> {
    let tokens = match load_tokens()? {
        Some(t) => t,
        None => run_loopback_flow(client, lang).await?,
    };

    if tokens.expiry_epoch_ms > now_ms() + 60_000 {
        return Ok(tokens.access_token);
    }

    match refresh_access_token(client, &tokens.refresh_token).await {
        Ok(refreshed) => {
            save_tokens(&refreshed)?;
            Ok(refreshed.access_token)
        }
        Err(_) => {
            let _ = clear_tokens();
            let fresh = run_loopback_flow(client, lang).await?;
            Ok(fresh.access_token)
        }
    }
}
