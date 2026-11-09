//! Keeping Lumen on the latest release.
//!
//! The release on GitHub is the source of truth: its tag is the version, and its installer is the
//! update. No signing key is involved, which means no secret to install before updates work — the
//! download comes from this project's own release over HTTPS, the same place and the same file a
//! person would download by hand.
//!
//! The one lesson this is built around: an installer exiting zero proves nothing. A silent failure
//! to overwrite a locked executable is how a September build survived three "successful" installs.
//! So the version expected after an update is written down first, and the next launch says plainly
//! whether it took.

use crate::services::api_client::ApiClient;
use crate::services::database::Database;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};

const LATEST_RELEASE: &str = "https://api.github.com/repos/Ketopluto/lumen/releases/latest";
/// What the next launch should be running, if an update was started.
const EXPECTED_KEY: &str = "update_expected_version";
/// A version whose installer ran and changed nothing. Never retried on its own: an update that
/// exits the app and fails takes the app down on every launch, which is worse than being behind.
const FAILED_KEY: &str = "update_failed_version";

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    #[serde(default)]
    assets: Vec<Asset>,
    #[serde(default)]
    html_url: String,
}

#[derive(Deserialize, Clone)]
struct Asset {
    name: String,
    browser_download_url: String,
    #[serde(default)]
    size: u64,
}

/// What the UI shows in Settings.
#[derive(Debug, Clone, Serialize)]
pub struct UpdateStatus {
    pub current: String,
    pub latest: Option<String>,
    /// A newer release exists.
    pub available: bool,
    /// Lumen can install it itself on this platform.
    pub installable: bool,
    pub release_url: Option<String>,
    /// `current`, `available`, `downloading`, `installing`, `failed`, or `unknown`.
    pub state: String,
    pub message: Option<String>,
}

impl UpdateStatus {
    fn current_version() -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }

    fn unknown(message: impl Into<String>) -> Self {
        Self {
            current: Self::current_version(),
            latest: None,
            available: false,
            installable: false,
            release_url: None,
            state: "unknown".into(),
            message: Some(message.into()),
        }
    }
}

/// `v4.1.0` or `4.1.0` to `(4, 1, 0)`. Anything unparsable sorts as oldest, so a release named
/// something unexpected never looks newer than what is installed.
fn parts(version: &str) -> (u32, u32, u32) {
    let mut numbers = version
        .trim()
        .trim_start_matches('v')
        .split(['.', '-', '+'])
        .filter_map(|part| part.parse::<u32>().ok());
    (
        numbers.next().unwrap_or(0),
        numbers.next().unwrap_or(0),
        numbers.next().unwrap_or(0),
    )
}

fn is_newer(latest: &str, current: &str) -> bool {
    parts(latest) > parts(current)
}

/// The installer for this platform, by exact name, so a portable build or another architecture is
/// never mistaken for the thing to run.
fn installer_for(version: &str, assets: &[Asset]) -> Option<Asset> {
    let wanted = if cfg!(target_os = "windows") {
        format!("Lumen_{}_x64-setup.exe", version)
    } else {
        return None; // macOS and Linux are told where to download; see `installable`.
    };
    assets.iter().find(|asset| asset.name == wanted).cloned()
}

/// Asks GitHub what the latest release is. Never fails loudly: a missing network or a rate limit
/// is not something to interrupt someone's desktop over.
pub async fn check(app: &AppHandle) -> UpdateStatus {
    let current = UpdateStatus::current_version();
    let release: Release = match ApiClient::get_json(LATEST_RELEASE, &[("Accept", "application/vnd.github+json")]).await
    {
        Ok(release) => release,
        Err(e) => return UpdateStatus::unknown(e),
    };
    let latest = release.tag_name.trim_start_matches('v').to_string();
    let available = is_newer(&latest, &current);
    let installer = installer_for(&latest, &release.assets);
    let status = UpdateStatus {
        current,
        latest: Some(latest),
        available,
        installable: available && installer.is_some(),
        release_url: Some(release.html_url),
        state: if available { "available" } else { "current" }.into(),
        message: match (available, installer.is_some()) {
            (true, false) if cfg!(target_os = "windows") => {
                Some("That release has no Windows installer attached.".into())
            }
            (true, false) => Some("Download it from the release page; Lumen cannot install it for you here.".into()),
            _ => None,
        },
    };
    let _ = app.emit("update-status", &status);
    status
}

/// Downloads the latest installer and hands over to it, which means this process has to go: an
/// installer cannot replace an executable that is still running — the mistake that left a
/// September build in place while three installs reported success.
pub async fn install(app: &AppHandle) -> Result<(), String> {
    let status = check(app).await;
    if !status.available {
        return Err("Lumen is already up to date".into());
    }
    let version = status.latest.clone().unwrap_or_default();

    let release: Release = ApiClient::get_json(LATEST_RELEASE, &[("Accept", "application/vnd.github+json")])
        .await
        .map_err(|e| e.to_string())?;
    let installer =
        installer_for(&version, &release.assets).ok_or("That release has no installer Lumen can run on this system")?;

    let dir = crate::utils::app_data_dir().join("updates");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let file = dir.join(&installer.name);
    log::info!("downloading {} ({} bytes)", installer.name, installer.size);
    ApiClient::download_file(app, "update", &installer.browser_download_url, &file).await?;

    // A truncated download would "install" and change nothing.
    let size = std::fs::metadata(&file).map(|m| m.len()).unwrap_or(0);
    if installer.size > 0 && size != installer.size {
        let _ = std::fs::remove_file(&file);
        return Err(format!(
            "the download is incomplete: {} of {} bytes",
            size, installer.size
        ));
    }

    // Written before handing over, so the next launch can tell whether this actually worked.
    let db = app.state::<Arc<Database>>();
    let _ = db.set_setting(EXPECTED_KEY, &version);
    log::info!("installing Lumen {} and restarting", version);
    launch_installer(&file, &version)?;
    // Leave, so the file being replaced is not held open by this process.
    app.exit(0);
    Ok(())
}

/// Runs the installer silently and starts the new build afterwards, through one detached shell so
/// it outlives this process.
#[cfg(target_os = "windows")]
fn launch_installer(installer: &std::path::Path, version: &str) -> Result<(), String> {
    use std::io::Write;
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let dir = crate::utils::app_data_dir().join("updates");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let script_path = dir.join("install.ps1");
    let transcript = dir.join("install.log");

    // The installer cannot replace an executable that is still running, and silently does nothing
    // when it tries — so the handover waits for this process to actually be gone, checks the
    // version afterwards rather than trusting an exit code, retries, and then starts what it
    // installed. A transcript is left behind because an update that fails is otherwise invisible.
    let script = format!(
        r#"$ErrorActionPreference = 'SilentlyContinue'
Start-Transcript -Path '{transcript}' -Force | Out-Null
$deadline = (Get-Date).AddSeconds(60)
while ((Get-Process -Name lumen -ErrorAction SilentlyContinue) -and (Get-Date) -lt $deadline) {{
  Start-Sleep -Milliseconds 200
}}
if (Get-Process -Name lumen -ErrorAction SilentlyContinue) {{
  Write-Output 'lumen is still running after 60s; not installing'
}} else {{
  Start-Sleep -Milliseconds 800
  foreach ($try in 1..3) {{
    Start-Process -FilePath '{installer}' -ArgumentList '/S' -Wait
    Start-Sleep -Milliseconds 600
    $installed = (Get-Item '{exe}').VersionInfo.ProductVersion
    Write-Output "attempt $try installed $installed"
    if ($installed -eq '{version}') {{ break }}
  }}
}}
Start-Process -FilePath '{exe}'
Stop-Transcript | Out-Null
"#,
        transcript = transcript.display(),
        installer = installer.display(),
        exe = exe.display(),
        version = version,
    );
    let mut file = std::fs::File::create(&script_path).map_err(|e| e.to_string())?;
    file.write_all(script.as_bytes()).map_err(|e| e.to_string())?;
    drop(file);

    // One flag only: DETACHED_PROCESS and CREATE_NO_WINDOW are mutually exclusive, and passing
    // both is why the previous handover never ran at all.
    std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-WindowStyle",
            "Hidden",
            "-File",
            &script_path.to_string_lossy(),
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("could not start the installer: {}", e))
}

#[cfg(not(target_os = "windows"))]
fn launch_installer(_installer: &std::path::Path, _version: &str) -> Result<(), String> {
    Err("Lumen can only install its own updates on Windows".into())
}

/// Says whether the last update took effect. An installer that silently did nothing is the
/// failure this exists to make visible.
pub fn confirm_previous(app: &AppHandle) {
    let db = app.state::<Arc<Database>>();
    let Ok(Some(expected)) = db.get_setting(EXPECTED_KEY) else {
        return;
    };
    let current = UpdateStatus::current_version();
    let _ = db.delete_setting(EXPECTED_KEY);
    if parts(&current) >= parts(&expected) {
        log::info!("update to {} took effect (running {})", expected, current);
        let _ = db.delete_setting(FAILED_KEY);
        return;
    }
    // Remembered so the next launch does not try the same thing again and exit the app for
    // nothing. Settings can still install it on request.
    let _ = db.set_setting(FAILED_KEY, &expected);
    log::warn!(
        "update to {} did not take effect: still running {}. The installer may have been unable to \
         replace the running executable.",
        expected,
        current
    );
    let mut status = UpdateStatus::unknown(format!(
        "The update to {} did not install — Lumen is still {}. Try the installer from the release \
         page.",
        expected, current
    ));
    status.state = "failed".into();
    let _ = app.emit("update-status", &status);
}

/// Checks once a few seconds after launch, and installs when there is something newer. Quiet by
/// design: nothing happens unless a release is actually newer than this build.
pub fn spawn_startup_check(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        confirm_previous(&app);
        let status = check(&app).await;
        let failed = app.state::<Arc<Database>>().get_setting(FAILED_KEY).ok().flatten();
        if status.available && failed.as_deref() == status.latest.as_deref() {
            log::warn!(
                "not installing {} again on its own: the last attempt changed nothing. Install it \
                 from Settings to try once more.",
                failed.unwrap_or_default()
            );
            return;
        }
        match (status.available, status.installable) {
            (false, _) => log::info!("Lumen {} is the latest release", status.current),
            (true, false) => log::info!(
                "Lumen {} is available; this platform cannot install it automatically",
                status.latest.unwrap_or_default()
            ),
            (true, true) => {
                if let Err(e) = install(&app).await {
                    log::warn!("automatic update failed: {}", e);
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_compare_by_number_not_by_text() {
        assert!(is_newer("4.10.0", "4.9.0"), "10 is after 9");
        assert!(is_newer("v4.2.0", "4.1.9"));
        assert!(!is_newer("4.1.0", "4.1.0"));
        assert!(!is_newer("4.0.9", "4.1.0"));
        // A release named something unexpected must never look newer than what is installed.
        assert!(!is_newer("nightly", "4.1.0"));
    }

    #[test]
    fn only_the_exact_installer_is_offered() {
        let assets = vec![
            Asset {
                name: "Lumen_4.2.0_x64_portable.exe".into(),
                browser_download_url: "https://example.invalid/portable".into(),
                size: 10,
            },
            Asset {
                name: "Lumen_4.2.0_x64-setup.exe".into(),
                browser_download_url: "https://example.invalid/setup".into(),
                size: 20,
            },
            Asset {
                name: "Lumen_4.2.0_aarch64.dmg".into(),
                browser_download_url: "https://example.invalid/dmg".into(),
                size: 30,
            },
        ];
        let chosen = installer_for("4.2.0", &assets);
        if cfg!(target_os = "windows") {
            assert_eq!(chosen.unwrap().name, "Lumen_4.2.0_x64-setup.exe");
        } else {
            assert!(chosen.is_none(), "only Windows installs its own updates");
        }
        // A version with no matching asset offers nothing rather than the wrong file.
        assert!(installer_for("9.9.9", &assets).is_none());
    }
}
