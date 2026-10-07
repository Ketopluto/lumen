//! `lumen --netcheck`: can this machine reach the wallpaper sources, and if not, why?
//!
//! Managed laptops run things that sit between an app and the internet — Netskope, Zscaler and
//! other inspecting proxies re-sign every HTTPS connection, and EDR agents can block a process
//! outright. Both look identical from the UI ("it won't load"), so this prints what actually
//! happened to each source: the status code, or the full error chain rather than the one-line
//! summary a person is shown.

use crate::services::api_client::ApiClient;
use std::io::Write;

/// One per source Lumen fetches from, so a proxy that allows some hosts and blocks others shows
/// up as exactly that.
const HOSTS: &[(&str, &str)] = &[
    ("Wallhaven", "https://wallhaven.cc/api/v1/search?q=nature&page=1"),
    ("Konachan", "https://konachan.net/post.json?limit=1"),
    (
        "Safebooru",
        "https://safebooru.org/index.php?page=dapi&s=post&q=index&json=1&limit=1&tags=animated",
    ),
    (
        "Wikimedia",
        "https://commons.wikimedia.org/w/api.php?action=query&format=json&titles=Main%20Page",
    ),
    ("Bing", "https://www.bing.com/HPImageArchive.aspx?format=js&idx=0&n=1"),
    ("NASA", "https://images-api.nasa.gov/search?q=galaxy&media_type=image"),
];

pub fn requested(args: &[String]) -> bool {
    args.iter().any(|arg| arg == "--netcheck")
}

/// Exit code: 0 everything reachable, 1 something is in the way.
pub fn run() -> i32 {
    line(&format!("Lumen {}", env!("CARGO_PKG_VERSION")));
    line("certificates: this system's store, plus the bundled Mozilla roots");
    for variable in ["HTTPS_PROXY", "HTTP_PROXY", "NO_PROXY", "https_proxy", "http_proxy"] {
        if let Ok(value) = std::env::var(variable) {
            line(&format!("proxy: {}={}", variable, value));
        }
    }

    let mut failures = 0;
    for (name, url) in HOSTS {
        match tauri::async_runtime::block_on(ApiClient::probe(url)) {
            Ok(status) if (200..400).contains(&status) => line(&format!("{:<10} ok ({})", name, status)),
            Ok(status) => {
                failures += 1;
                line(&format!("{:<10} HTTP {}", name, status));
            }
            Err(why) => {
                failures += 1;
                line(&format!(
                    "{:<10} FAILED\n           {}",
                    name,
                    why.replace('\n', "\n           ")
                ));
            }
        }
    }

    if failures == 0 {
        line("every source is reachable");
        return 0;
    }
    line(&format!(
        "{} of {} sources unreachable. A \"certificate\" or \"UnknownIssuer\" error above means \
         something is inspecting HTTPS and its certificate is not in this computer's trusted root \
         store — ask IT to install it there. A timeout or refused connection means the traffic is \
         being blocked outright rather than inspected.",
        failures,
        HOSTS.len()
    ));
    1
}

/// Straight to stdout and flushed: the log file is no use to someone running this in a terminal.
fn line(text: &str) {
    println!("[netcheck] {}", text);
    let _ = std::io::stdout().flush();
}
