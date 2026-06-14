use anyhow::{anyhow, Result};
use serde::Deserialize;
use serde_json::json;
use std::path::Path;
use tokio::io::AsyncReadExt as _;

use crate::mime_map::Mapping;
use crate::settings;

const DRIVE_API: &str = "https://www.googleapis.com/drive/v3";
const DRIVE_UPLOAD: &str = "https://www.googleapis.com/upload/drive/v3/files";

#[derive(Deserialize)]
struct FileMeta {
    id: Option<String>,
    #[serde(rename = "mimeType")]
    mime_type: Option<String>,
    #[serde(default)]
    trashed: bool,
    #[serde(default)]
    name: Option<String>,
}

#[derive(Deserialize)]
struct ListResp {
    files: Vec<FileMeta>,
}

pub async fn find_or_create_folder(
    client: &reqwest::Client,
    access_token: &str,
) -> Result<String> {
    let folder_name = settings::drive_folder_name();

    if let Some(id) = settings::cached_folder_id() {
        let url = format!("{}/files/{}?fields=id,trashed,name", DRIVE_API, id);
        let res = client.get(&url).bearer_auth(access_token).send().await?;
        if res.status().is_success() {
            let meta: FileMeta = res.json().await?;
            // Only reuse cache if the folder still exists AND still matches the configured name
            if meta.id.is_some() && !meta.trashed && meta.name.as_deref() == Some(folder_name.as_str()) {
                return Ok(id.clone());
            }
        }
    }

    let q = format!(
        "name='{}' and mimeType='application/vnd.google-apps.folder' and trashed=false and 'root' in parents",
        folder_name.replace('\'', "\\'")
    );
    let list_url = format!(
        "{}/files?q={}&fields=files(id,mimeType,trashed,name)&pageSize=1&spaces=drive",
        DRIVE_API,
        urlencoding::encode(&q)
    );
    let list_res = client
        .get(&list_url)
        .bearer_auth(access_token)
        .send()
        .await?;
    if !list_res.status().is_success() {
        let s = list_res.status();
        let t = list_res.text().await.unwrap_or_default();
        return Err(anyhow!("drive list {s}: {t}"));
    }
    let list: ListResp = list_res.json().await?;
    if let Some(f) = list.files.into_iter().next() {
        if let Some(id) = f.id {
            settings::set_cached_folder_id(&id)?;
            return Ok(id);
        }
    }

    let create_url = format!("{}/files?fields=id", DRIVE_API);
    let create_res = client
        .post(&create_url)
        .bearer_auth(access_token)
        .json(&json!({
            "name": folder_name,
            "mimeType": "application/vnd.google-apps.folder",
            "parents": ["root"],
        }))
        .send()
        .await?;
    if !create_res.status().is_success() {
        let s = create_res.status();
        let t = create_res.text().await.unwrap_or_default();
        return Err(anyhow!("drive create folder {s}: {t}"));
    }
    let created: FileMeta = create_res.json().await?;
    let id = created
        .id
        .ok_or_else(|| anyhow!("folder create returned no id"))?;
    settings::set_cached_folder_id(&id)?;
    Ok(id)
}

pub struct UploadResult {
    pub file_id: String,
    pub google_mime: String,
}

pub async fn upload_and_convert(
    client: &reqwest::Client,
    access_token: &str,
    file_path: &Path,
    mapping: &Mapping,
    folder_id: &str,
) -> Result<UploadResult> {
    let name = file_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Untitled")
        .to_string();

    let metadata = json!({
        "name": name,
        "mimeType": mapping.google_mime,
        "parents": [folder_id],
    });

    let boundary = format!("----GDL{}", rand::random::<u64>());

    let head = format!(
        "--{boundary}\r\nContent-Type: application/json; charset=UTF-8\r\n\r\n{}\r\n--{boundary}\r\nContent-Type: {}\r\n\r\n",
        metadata,
        mapping.source_mime
    );
    let tail = format!("\r\n--{boundary}--\r\n");

    let file_size = tokio::fs::metadata(file_path).await?.len();
    let content_length = head.len() as u64 + file_size + tail.len() as u64;

    let file = tokio::fs::File::open(file_path).await?;
    let reader = std::io::Cursor::new(head.into_bytes())
        .chain(file)
        .chain(std::io::Cursor::new(tail.into_bytes()));
    let stream = tokio_util::io::ReaderStream::new(reader);

    let url = format!("{}?uploadType=multipart&fields=id,mimeType", DRIVE_UPLOAD);
    let res = client
        .post(&url)
        .bearer_auth(access_token)
        .header("Content-Type", format!("multipart/related; boundary={boundary}"))
        .header("Content-Length", content_length)
        .body(reqwest::Body::wrap_stream(stream))
        .send()
        .await?;

    if !res.status().is_success() {
        let s = res.status();
        let t = res.text().await.unwrap_or_default();
        return Err(anyhow!("drive upload {s}: {t}"));
    }
    let meta: FileMeta = res.json().await?;
    let file_id = meta.id.ok_or_else(|| anyhow!("upload returned no id"))?;
    let google_mime = meta
        .mime_type
        .unwrap_or_else(|| mapping.google_mime.to_string());
    Ok(UploadResult { file_id, google_mime })
}

pub async fn check_exists(
    client: &reqwest::Client,
    access_token: &str,
    file_id: &str,
) -> bool {
    let url = format!("{}/files/{}?fields=id,trashed", DRIVE_API, file_id);
    match client.get(&url).bearer_auth(access_token).send().await {
        Ok(res) if res.status().is_success() => match res.json::<FileMeta>().await {
            Ok(m) => m.id.is_some() && !m.trashed,
            Err(_) => false,
        },
        _ => false,
    }
}
