### Fixed

- **The preview opened wherever the grid was scrolled to.** It is drawn into the page now rather
  than inside it: `position: fixed` is measured from the nearest ancestor with a transform or an
  animation, and the pages have both, so the overlay inherited the grid's scroll offset. Opening a
  wallpaper now centres it in the window, every time.
- **The close button sat partly outside the window.** It hung 44px to the right of the preview,
  which fell off the edge on a narrow window. It is inside the frame now, over the image.
- **A narrow window no longer squeezes the buttons off the edge.** The preview's details stack,
  the image takes whatever height is left instead of assuming the furniture is 170px tall, and
  below 620px it goes edge to edge.
- **Setting a new live wallpaper while one was playing failed**, so it had to be stopped first.
  Destroying a window is asynchronous — Tauri holds the label for a moment after the window is
  gone, and rebuilding with that label fails outright. The rebuild now waits for the labels to be
  free.
- **Local files showed as blank previews.** Browsing a folder listed its files without permitting
  them to be displayed, which filled the log with "asset protocol not configured to allow the
  path". Listing a folder now permits showing what is in it, and previewing a file permits that
  file.

### Which file do I download?

| System | File |
|---|---|
| Windows 10/11 | `Lumen_4.2.1_x64-setup.exe` (or `..._portable.exe`, no installer) |
| macOS, Apple Silicon | `Lumen_4.2.1_aarch64.dmg` |
| macOS, Intel | `Lumen_4.2.1_x64.dmg` |
| Debian / Ubuntu / Mint | `Lumen_4.2.1_amd64.deb` |
| Fedora / openSUSE | `Lumen-4.2.1-1.x86_64.rpm` |
| Any other Linux | `Lumen_4.2.1_amd64.AppImage` |

On Windows, Lumen installs this itself if you are already on 4.2.0.
