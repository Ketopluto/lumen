# Lumen

A lightweight live wallpaper manager for Windows, built with Tauri 2 + React.

## Features

- **Huge free library, no keys needed**: Wallhaven (600k+ wallpapers, sorted by top, hot, latest, random, or most liked), Wikimedia Commons featured and quality photos, NASA space imagery, and Bing daily images
- **Live wallpapers**: video and GIF wallpapers play behind your desktop icons, including thousands of free Wikimedia time-lapse and nature videos. Works on Windows 11 24H2+ as well as older builds.
- **Optional sources**: add a free Unsplash or Pexels key in Settings to unlock their photos, plus Pexels videos
- **Local folders**: browse your own images and videos with cached thumbnails
- **Favorites and collections**, **history**, and **slideshows** from favorites, a collection, or a folder
- **Light on resources**: live playback pauses automatically while a fullscreen app runs or when you're on battery, and the wallpaper window only loads a ~1 KB player
- Starts with Windows (optional), lives in the tray, and restores your last live wallpaper or slideshow on launch

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
