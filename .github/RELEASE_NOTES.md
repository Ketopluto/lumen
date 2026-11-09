### Lumen keeps itself up to date

Lumen now checks this page when it starts and installs a newer release on its own, so the version
you have is the version that is out. It verifies afterwards too: the version it expects is written
down before the installer runs, and if the installer did not take effect Lumen says so instead of
quietly carrying on — which is a real failure mode, not a hypothetical one.

Releases publish themselves now, so there is always a latest release here to update from.

### A wallpaper per monitor

Each screen can hold its own still image or live wallpaper, chosen from a picker in the sidebar
that draws your monitors to scale — screen 2 is wherever screen 2 actually is, with a dot on
whichever ones are playing something. The same wallpaper everywhere still costs a single video
decoding; only a genuine mix pays for one per screen. Spanning one video across every monitor is
still available as the Span fit mode.

Settings also reports the version Lumen is actually running, which it previously got wrong.

### Which file do I download?

| System | File |
|---|---|
| Windows 10/11 | `Lumen_4.2.0_x64-setup.exe` (or `..._portable.exe`, no installer) |
| macOS, Apple Silicon | `Lumen_4.2.0_aarch64.dmg` |
| macOS, Intel | `Lumen_4.2.0_x64.dmg` |
| Debian / Ubuntu / Mint | `Lumen_4.2.0_amd64.deb` |
| Fedora / openSUSE | `Lumen-4.2.0-1.x86_64.rpm` |
| Any other Linux | `Lumen_4.2.0_amd64.AppImage` |

Lumen installs its own updates on Windows. On macOS and Linux it tells you a release is out and
links here, because replacing a DMG or a system package from inside the app is not something it
should be doing quietly.

Neither the Windows nor the macOS build is code-signed, so the first launch needs one extra
click — see [Installing](https://github.com/Ketopluto/lumen#install) in the README.
