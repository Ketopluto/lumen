Lumen now runs on **Windows, macOS and Linux**, live wallpapers included.

### Which file do I download?

| System | File |
|---|---|
| Windows 10/11 | `Lumen_4.0.0_x64-setup.exe` (or `..._portable.exe`, no installer) |
| macOS, Apple Silicon | `Lumen_4.0.0_aarch64.dmg` |
| macOS, Intel | `Lumen_4.0.0_x64.dmg` |
| Debian / Ubuntu / Mint | `Lumen_4.0.0_amd64.deb` |
| Fedora / openSUSE | `Lumen-4.0.0-1.x86_64.rpm` |
| Any other Linux | `Lumen_4.0.0_amd64.AppImage` |

Neither the Windows nor the macOS build is code-signed, so the first launch needs one extra
click — see [Installing](https://github.com/Ketopluto/lumen#install) in the README. On macOS in
particular you will need the `xattr` command shown there.

### What's new

- **Live wallpapers on Linux.** Any X11 desktop (GNOME, KDE, XFCE, Cinnamon, MATE) plays video
  behind the icons, and so do the wlroots Wayland compositors — sway, Hyprland — and KDE Plasma
  on Wayland, through `wlr-layer-shell`. GNOME on Wayland cannot do this at all; Lumen says so
  and static wallpapers still work there.
- **Live wallpapers on macOS — beta.** The surface is placed at the desktop window level, below
  the icons, one per display. It compiles and is believed correct, but nobody has run it on a
  real Mac yet: please report what you see.
- Start-on-login now actually works everywhere: a Run key on Windows, a Launch Agent on macOS,
  an XDG autostart entry on Linux.
- Auto-pause for fullscreen apps on X11 and macOS, and pause-on-battery on laptops everywhere.
- The interface follows the platform: system fonts, macOS traffic lights, and only the settings
  the current desktop can honour — with a reason shown for the ones it cannot.
- Sources are filtered by what your system can actually decode, instead of leaving you with a
  black desktop when a codec is missing.

Windows behaviour is unchanged.
