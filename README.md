# Lumen

A lightweight live wallpaper app for Windows. Browse 600,000+ wallpapers, anime art, and live video wallpapers, and set any of them in one click. Live wallpapers play behind your desktop icons.

## Download

1. Go to the [latest release](https://github.com/Ketopluto/lumen/releases/latest).
2. Download **`Lumen_3.0.0_x64-setup.exe`** and run it. Lumen installs for your user only (no admin needed) and adds Start menu and desktop shortcuts.
   - Prefer not to install? Download **`Lumen_3.0.0_x64_portable.exe`** and run it directly.
3. If Windows shows "Windows protected your PC", click **More info**, then **Run anyway**. Lumen isn't code-signed yet, so SmartScreen doesn't recognize it.

Requires 64-bit Windows 10 or 11. Lumen uses Microsoft Edge WebView2, which Windows 11 already includes; on older systems the installer adds it if it's missing.

## Features

- **Huge free library, no keys needed**: Wallhaven (600k+ wallpapers, sorted by top, trending, newest, random, or most favorited), 110k+ anime wallpapers plus Konachan, Wikimedia Commons featured photos, NASA space imagery, and Bing's daily photos
- **Live wallpapers**: videos and GIFs play behind your desktop icons, including thousands of free Wikimedia time-lapses. Works on Windows 11 24H2+ as well as older builds.
- **Stays on**: starts with Windows, lives in the tray, and brings the live wallpaper back by itself if Explorer restarts or your display setup changes. It only pauses while a fullscreen game or app is running.
- **Your own files**: browse local folders with cached thumbnails, or open any image or video with `lumen.exe <file>`
- **Favorites and collections**, **history**, and **slideshows** from favorites, a collection, or a folder
- **Optional sources**: add a free Unsplash or Pexels key in Settings for their photos, plus Pexels videos
- Closing the window unloads the interface; only the tray icon and the wallpaper keep running

## Develop

Prerequisites: Node 20+, Rust (stable), and WebView2 (preinstalled on Windows 10/11).

```bash
npm install
npm run tauri dev
```

To test without touching your real favorites and history, set `LUMEN_DATA_DIR` to a scratch folder first.

## Build

```bash
npm run tauri build
```

The installer is written to `src-tauri/target/release/bundle/nsis/`.

## Tests

```bash
cd src-tauri && cargo test
```

## License

MIT
