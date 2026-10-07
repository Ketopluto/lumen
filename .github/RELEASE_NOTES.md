### Fixed

**Lumen now works behind a corporate network that inspects HTTPS.** On a managed machine —
Netskope, Zscaler, and similar — every search and download failed, because Lumen only trusted the
copy of Mozilla's root certificates compiled into it and ignored the ones installed on the
computer. An inspecting proxy re-signs every connection with its own certificate authority, which
lives in the system store, so nothing Lumen fetched could be verified. It now trusts both: the
system's certificates first, with the bundled list as a fallback for a machine whose store is
unreadable.

Two smaller things that came with it:

- A rejected certificate used to read `Network error: invalid peer certificate: UnknownIssuer`.
  It now says what is actually wrong and what fixes it.
- `lumen --netcheck` in a terminal reports each source as reachable or not, with the full reason
  and any proxy in use — enough to tell an inspected connection from a blocked one.

### Which file do I download?

| System | File |
|---|---|
| Windows 10/11 | `Lumen_4.0.1_x64-setup.exe` (or `..._portable.exe`, no installer) |
| macOS, Apple Silicon | `Lumen_4.0.1_aarch64.dmg` |
| macOS, Intel | `Lumen_4.0.1_x64.dmg` |
| Debian / Ubuntu / Mint | `Lumen_4.0.1_amd64.deb` |
| Fedora / openSUSE | `Lumen-4.0.1-1.x86_64.rpm` |
| Any other Linux | `Lumen_4.0.1_amd64.AppImage` |

Neither the Windows nor the macOS build is code-signed, so the first launch needs one extra
click — see [Installing](https://github.com/Ketopluto/lumen#install) in the README. On macOS in
particular you will need the `xattr` command shown there.

### Everything from 4.0.0

Lumen runs on **Windows, macOS and Linux**, live wallpapers included: any X11 desktop and the
wlroots Wayland compositors play video behind the icons, GNOME on Wayland cannot and says so, and
macOS is beta — CI confirms the surface lands at the desktop level on a real window server, but
nobody has looked at it on a Mac yet. Start-on-login works on all three. The Live tab leads with
about 2,600 animated anime wallpapers from Safebooru and Konachan, which are GIF loops and so need
no video codec anywhere.
