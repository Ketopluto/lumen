# Lumen

A lightweight live wallpaper app for **Windows, macOS and Linux**. Browse 600,000+ wallpapers,
anime art, and live video wallpapers, and set any of them in one click. Live wallpapers play
behind your desktop icons.

## Install

Download from the [latest release](https://github.com/Ketopluto/lumen/releases/latest).

**Windows 10/11 (64-bit)** — run `Lumen_4.0.0_x64-setup.exe`. It installs for your user only, no
admin needed, and adds Start menu and desktop shortcuts. Prefer no installer? Take
`Lumen_4.0.0_x64_portable.exe` and run it directly. If Windows says "Windows protected your PC",
click **More info** → **Run anyway**: Lumen isn't code-signed, so SmartScreen doesn't recognise
it. WebView2 is already part of Windows 11; on older builds the installer adds it.

**macOS 11 or newer** — take `Lumen_4.0.0_aarch64.dmg` on Apple Silicon or `Lumen_4.0.0_x64.dmg`
on Intel, and drag Lumen to Applications. The build isn't notarised, so macOS will call it
damaged. Clear the quarantine flag once, in Terminal:

```bash
xattr -dr com.apple.quarantine /Applications/Lumen.app
```

**Linux** — `.deb` for Debian, Ubuntu and Mint, `.rpm` for Fedora and openSUSE, or the AppImage
anywhere else (`chmod +x` it first). Live wallpapers need GStreamer codecs; the AppImage carries
its own, and for the packages install `gstreamer1.0-plugins-good` and `gstreamer1.0-libav`
(`gstreamer1-plugins-good`, `gstreamer1-libav` on Fedora). On Wayland, live wallpapers also need
`gtk-layer-shell` — Lumen tells you if it's missing.

## What works where

| Platform | Live wallpapers | Auto-pause | Start on login |
|---|---|---|---|
| Windows 10 / 11 | yes, all monitors | fullscreen + battery | yes |
| Linux, any X11 desktop | yes, all monitors | fullscreen + battery | yes |
| Wayland: sway, Hyprland, KDE Plasma | yes, one per output | battery only | yes |
| Wayland: GNOME | **no** — still images only | battery only | yes |
| macOS 11+ (Apple Silicon / Intel) | **beta**, one per display | fullscreen + battery | yes |

Two limits worth knowing before you download:

- **GNOME on Wayland can't do live wallpapers.** Its compositor deliberately offers no way for an
  app to draw on the desktop, and there's no workaround. Still wallpapers work normally. If you
  want live ones on GNOME, log out and pick **GNOME on Xorg** at the login screen.
- **macOS support is beta.** It's written and it compiles, but it has never run on a real Mac —
  I don't own one. Please open an issue with what you see, working or not.

Fullscreen auto-pause needs to inspect other windows, which Wayland has no protocol for, so that
toggle is disabled there. Lumen shows the reason for anything it can't do on your system rather
than offering a switch that does nothing.

## Features

- **Huge free library, no keys needed**: Wallhaven (600k+ wallpapers, sorted by top, trending,
  newest, random, or most favorited), 110k+ anime wallpapers plus Konachan, Wikimedia Commons
  featured photos, NASA space imagery, and Bing's daily photos
- **Live wallpapers**: videos and GIFs play behind your desktop icons, including thousands of free
  Wikimedia time-lapses
- **Stays on**: starts with your session, lives in the tray (menu bar on macOS), and brings the
  live wallpaper back by itself if the desktop restarts or your display setup changes. It pauses
  while a fullscreen game is running, and on battery if you ask it to.
- **Your own files**: browse local folders with cached thumbnails, or open any image or video with
  `lumen <file>`
- **Favorites and collections**, **history**, and **slideshows** from favorites, a collection, or
  a folder
- **Optional sources**: add a free Unsplash or Pexels key in Settings for their photos, plus
  Pexels videos
- Sources you can't play are hidden: Lumen asks the webview which codecs it has instead of
  handing you a black desktop
- Closing the window unloads the interface; only the tray icon and the wallpaper keep running

## Develop

Prerequisites: Node 20+ and stable Rust, plus the [Tauri 2 system
dependencies](https://tauri.app/start/prerequisites/) for your OS — WebView2 on Windows (already
there on 10/11), Xcode command line tools on macOS, and on Debian/Ubuntu:

```bash
sudo apt install libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev patchelf build-essential curl wget file libxdo-dev libssl-dev
```

```bash
npm install
npm run tauri dev
```

To test without touching your real favorites and history, set `LUMEN_DATA_DIR` to a scratch folder
first.

Everything platform-specific lives in `src-tauri/src/services/desktop/`: one module per backend,
selected at compile time, with the shared contract and the two rules that keep it honest
documented in `mod.rs`. The rest of the app never uses `#[cfg]` — it asks `capabilities()`.

## Build

```bash
npm run tauri build
```

Bundles land in `src-tauri/target/release/bundle/`. Each OS builds its own targets: NSIS on
Windows, `.app` and `.dmg` on macOS, `.deb`, `.rpm` and AppImage on Linux — configured in
`src-tauri/tauri.{windows,macos,linux}.conf.json`. Releases are built by
[`.github/workflows/release.yml`](.github/workflows/release.yml) when a `v*` tag is pushed.

## Tests

```bash
cd src-tauri && cargo test
```

CI compiles every platform backend on Windows, macOS and Linux with `clippy -D warnings` on each
push — that's what keeps the two backends I can't run from rotting.

## License

MIT
