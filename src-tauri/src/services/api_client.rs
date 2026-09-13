use reqwest::Client;
use std::path::Path;
use std::sync::OnceLock;
use std::time::Duration;
use tauri::Emitter;

const USER_AGENT: &str = concat!(
    "Lumen/",
    env!("CARGO_PKG_VERSION"),
    " (https://github.com/Ketopluto/lumen)"
);

/// Shared HTTP clients (connection pools are reused across calls).
pub struct ApiClient;

fn api_client() -> &'static Client {
    static CLIENT: OnceLock<Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        Client::builder()
            .timeout(Duration::from_secs(20))
            .user_agent(USER_AGENT)
            .build()
            .expect("failed to build HTTP client")
    })
}

/// Downloads can be large videos, so only stalls (not total time) time out.
fn download_client() -> &'static Client {
    static CLIENT: OnceLock<Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        Client::builder()
            .connect_timeout(Duration::from_secs(15))
            .read_timeout(Duration::from_secs(30))
            .user_agent(USER_AGENT)
            .build()
            .expect("failed to build HTTP client")
    })
}

#[derive(Clone, serde::Serialize)]
struct DownloadProgress<'a> {
    id: &'a str,
    received: u64,
    total: Option<u64>,
}

/// One request, with the HTTP failures every source shares turned into something a person can read.
async fn get(url: &str, headers: &[(&str, &str)]) -> Result<reqwest::Response, String> {
    let mut req = api_client().get(url);
    for (k, v) in headers {
        req = req.header(*k, *v);
    }
    let resp = req.send().await.map_err(|e| format!("Network error: {}", e))?;
    let status = resp.status();
    if !status.is_success() {
        return Err(match status.as_u16() {
            401 | 403 => "The source rejected the request (check your API key in Settings)".to_string(),
            429 => "Rate limited by the source — wait a minute and try again".to_string(),
            _ => format!("HTTP {}", status),
        });
    }
    Ok(resp)
}

impl ApiClient {
    /// GET request returning JSON deserialized into T.
    pub async fn get_json<T: serde::de::DeserializeOwned>(url: &str, headers: &[(&str, &str)]) -> Result<T, String> {
        get(url, headers)
            .await?
            .json::<T>()
            .await
            .map_err(|e| format!("Unexpected response: {}", e))
    }

    /// GET returning the raw body, for a source that answers with something other than JSON: an
    /// empty search on Safebooru comes back as an empty body rather than as `[]`.
    pub async fn get_text(url: &str, headers: &[(&str, &str)]) -> Result<String, String> {
        get(url, headers)
            .await?
            .text()
            .await
            .map_err(|e| format!("Unexpected response: {}", e))
    }

    /// Stream a file to disk, emitting `download-progress` events.
    /// Writes to a `.part` file first so interrupted downloads are never mistaken for complete ones.
    pub async fn download_file(app: &tauri::AppHandle, id: &str, url: &str, dest: &Path) -> Result<(), String> {
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let mut resp = download_client()
            .get(url)
            .send()
            .await
            .map_err(|e| format!("Download failed: {}", e))?;
        if !resp.status().is_success() {
            return Err(format!("Download failed: HTTP {}", resp.status()));
        }
        let content_type = resp
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();
        if content_type.starts_with("text/") {
            return Err("Download failed: the server returned a web page instead of a file".into());
        }

        let total = resp.content_length();
        let part = dest.with_extension("part");
        let mut file = std::fs::File::create(&part).map_err(|e| e.to_string())?;
        let mut received: u64 = 0;
        let mut last_emit: u64 = 0;

        let result: Result<(), String> = async {
            use std::io::Write;
            while let Some(chunk) = resp.chunk().await.map_err(|e| format!("Download interrupted: {}", e))? {
                file.write_all(&chunk).map_err(|e| e.to_string())?;
                received += chunk.len() as u64;
                if received - last_emit > 512 * 1024 {
                    last_emit = received;
                    let _ = app.emit("download-progress", DownloadProgress { id, received, total });
                }
            }
            file.flush().map_err(|e| e.to_string())
        }
        .await;
        drop(file);

        if let Err(e) = result {
            let _ = std::fs::remove_file(&part);
            return Err(e);
        }
        std::fs::rename(&part, dest).map_err(|e| e.to_string())?;
        let _ = app.emit(
            "download-progress",
            DownloadProgress {
                id,
                received,
                total: Some(received),
            },
        );
        Ok(())
    }
}
