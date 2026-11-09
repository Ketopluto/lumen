### Fixed: the update it downloaded never installed

4.2.0 would fetch a newer release, run its installer, and still be the old version afterwards.
Two mistakes, both mine:

- The handover was started with two mutually exclusive Windows process flags
  (`DETACHED_PROCESS` and `CREATE_NO_WINDOW`), so it never ran at all — which is also why Lumen
  did not come back after exiting.
- It ran the installer without waiting for Lumen to exit. An installer cannot replace a running
  executable and says nothing when it fails, which is the same trap that once left a September
  build in place through three "successful" installs.

The handover is now a script that waits for Lumen to actually be gone, installs, **checks the
installed version rather than the exit code**, retries up to three times, starts Lumen again, and
leaves a transcript in `updates\install.log` so a failure is never invisible.

An update that fails is also no longer retried on its own. Installing it exits the app, so
retrying every launch would take the app down repeatedly — worse than being a version behind. The
failed version is remembered, and Settings can still install it on request.

What already worked, and still does: Lumen noticed the new release on its own, downloaded the
right installer, checked it against the size GitHub reported, and reported plainly that the update
had not taken effect.

### Which file do I download?

| System | File |
|---|---|
| Windows 10/11 | `Lumen_4.2.2_x64-setup.exe` (or `..._portable.exe`, no installer) |
| macOS, Apple Silicon | `Lumen_4.2.2_aarch64.dmg` |
| macOS, Intel | `Lumen_4.2.2_x64.dmg` |
| Debian / Ubuntu / Mint | `Lumen_4.2.2_amd64.deb` |
| Fedora / openSUSE | `Lumen-4.2.2-1.x86_64.rpm` |
| Any other Linux | `Lumen_4.2.2_amd64.AppImage` |

4.2.0 and 4.2.1 cannot install this themselves — that is the bug being fixed. Run the installer
once by hand; updates after this one are automatic.
