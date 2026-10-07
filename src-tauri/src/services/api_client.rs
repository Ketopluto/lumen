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

/// Everything reqwest knows, not just the summary: the cause of a TLS failure is always a few
/// layers further down.
fn error_chain(e: &reqwest::Error) -> String {
    let mut chain = e.to_string();
    let mut source = std::error::Error::source(e);
    while let Some(cause) = source {
        chain.push_str(&format!("\n  caused by: {}", cause));
        source = cause.source();
    }
    chain
}

/// A rejected certificate is worth explaining. On a managed machine it is almost never a broken
/// website: something is decrypting HTTPS and signing it with a CA this computer is supposed to
/// trust. Lumen reads the system certificate store, so if that CA is installed properly this
/// cannot happen - and if it is not, no amount of retrying will help.
pub fn explain(e: &reqwest::Error) -> String {
    let chain = error_chain(e);
    let lowercase = chain.to_lowercase();
    let certificate_problem = lowercase.contains("certificate")
        || lowercase.contains("unknownissuer")
        || lowercase.contains("invalid peer")
        || lowercase.contains("tls");
    if certificate_problem {
        return "Could not verify a secure connection to this source. Something on this network is \
                inspecting HTTPS traffic (Netskope, Zscaler and similar corporate proxies do this) \
                and Lumen does not trust the certificate it presented. Ask IT to install that \
                proxy's certificate in this computer's trusted root store, then restart Lumen. \
                Run `lumen --netcheck` in a terminal for the details."
            .to_string();
    }
    format!("Network error: {}", e)
}

/// One request, with the HTTP failures every source shares turned into something a person can read.
async fn get(url: &str, headers: &[(&str, &str)]) -> Result<reqwest::Response, String> {
    let mut req = api_client().get(url);
    for (k, v) in headers {
        req = req.header(*k, *v);
    }
    let resp = req.send().await.map_err(|e| explain(&e))?;
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

    /// Diagnostic for `--netcheck`: the status code, or the whole error chain rather than the
    /// summary a person is shown.
    pub async fn probe(url: &str) -> Result<u16, String> {
        match api_client().get(url).send().await {
            Ok(resp) => Ok(resp.status().as_u16()),
            Err(e) => Err(error_chain(&e)),
        }
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
            .map_err(|e| format!("Download failed: {}", explain(&e)))?;
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

#[cfg(test)]
mod tls {
    /// Proves Lumen reads the certificates this machine trusts, rather than only the copy of
    /// Mozilla's list baked into the binary. Without that, a network that inspects HTTPS —
    /// Netskope, Zscaler, any corporate proxy — fails every fetch at the handshake, because the
    /// certificate it presents is signed by a CA that only the operating system knows about.
    ///
    /// The bundled roots are switched off here, so success can only have come from the OS store.
    #[test]
    #[ignore]
    fn the_operating_systems_certificates_are_trusted() {
        let client = reqwest::Client::builder()
            .tls_built_in_webpki_certs(false)
            .build()
            .expect("client with only the system's certificates");
        let status = tauri::async_runtime::block_on(async {
            client
                .get("https://safebooru.org/index.php?page=dapi&s=post&q=index&json=1&limit=1&tags=animated")
                .send()
                .await
                .map(|r| r.status())
        });
        println!("system-certificate-only request: {:?}", status);
        assert!(
            status.is_ok(),
            "the system certificate store did not verify a public site"
        );
    }
}
