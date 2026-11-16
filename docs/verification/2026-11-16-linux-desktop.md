# Linux desktop icons and ARM64 VM verification

The later [acceptance follow-up](2026-11-16-linux-acceptance.md) updates the real
keyring, package-removal, hosted CI and issue status described below.

Observed on 15–16 November 2026 on `codex/linux-foundation`, based on
`b77f779`. The runtime environments were Ubuntu 22.04.5/GNOME and Fedora 44/KDE,
both ARM64 guests in UTM on Apple Silicon. This supplements the
[foundation](2026-11-15-linux-foundation.md) and
[architecture record](2026-11-15-platform-architectures.md).

## Changes

Linux notifications previously sent `org.rufapp.desktop` as both their icon name
and desktop-entry hint, while packages installed `sip-desktop.png` and
`Ruf.desktop`. A private Wayland protocol observation confirmed GTK's application
id was `sip-desktop`. Real test banners appeared on GNOME and KDE with generic
icons; KDE also showed a generic window icon.

Notifications now use `sip-desktop` for both names. DEB/RPM install an additional
hidden `sip-desktop.desktop` alias, retaining the visible Ruf launcher. Tauri also
copies DEB files into AppImages, so the AppImage configuration replaces that alias
with a visible launcher containing freedesktop categories. The bundle identifier,
credential service, public IPC/DTO contracts, action tokens, daemon-owner binding,
timeouts and native thread/ownership boundaries are preserved. No dependency was
added. See [notification API](../api/notifications.md) and [Linux setup](../linux.md).

The startup test also exports temporary XDG paths to its private D-Bus activation
environment. Otherwise an activated portal could wait on the desktop's locked
keyring. Cleanup unmounts an owned document-portal mount before deleting the test
directory. The test does not change the real desktop's activation environment.

## Verification

Toolchains: Bun 1.4.4 and Rust 1.98.1. Host checks used the existing official
Rustup toolchain through a process-local PATH because Homebrew LLVM could not
load its Z3 dependency; no global toolchain repair was made.

| Check                                  | Observed result                                                                                                                                             |
| -------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Notification regression before the fix | Failed: D-Bus received `org.rufapp.desktop` instead of `sip-desktop`.                                                                                       |
| Old package regression                 | Failed: required desktop alias absent.                                                                                                                      |
| `bun run test:linux`                   | Six isolated service/failure tests passed on each distribution. Fedora executed the Ubuntu-built ARM64 test binaries, not a separate Fedora compiler build. |
| `bun run test:native`, Ubuntu          | 334 tests passed; all selected partitions completed.                                                                                                        |
| Host portable workspace tests          | 157 passed; opt-in platform/lab cases remained ignored.                                                                                                     |
| Frontend                               | 159 UI tests passed; format, type/Svelte checks, build and version consistency passed.                                                                      |
| Generated contracts                    | Type generation succeeded without generated-file changes.                                                                                                   |
| Clippy                                 | Host portable and Ubuntu native workspace/all-targets checks passed with warnings denied.                                                                   |
| Native Linux desktop build             | DEB, RPM and AppImage built successfully with SBOM and installer-hash manifests.                                                                            |
| Package checks                         | Architecture, tray/audio dependencies, installed icons, visible launchers, hidden DEB alias and RPM alias digest passed.                                    |
| Mutation check                         | A deliberately hidden visible launcher was rejected; the unchanged packages then passed.                                                                    |
| Installed DEB/RPM                      | GTK resolved the installed Ruf icon and hidden alias on both distributions; the visible launcher remained visible.                                          |
| Startup checks                         | Installed desktop and final AppImage each displayed a window and stayed running in private Xvfb/D-Bus sessions on both distributions.                       |

The new ARM64 package SHA-256 values are:

| Package  | SHA-256                                                            |
| -------- | ------------------------------------------------------------------ |
| DEB      | `ef8e6a814696373e1cf4efefce13c1e13da9e2120110317cfac09cd7515ed7a3` |
| RPM      | `26b92d0ce06aaf2c54f3dc3413327756970e87c454dca1c3dfa56a2e95bae271` |
| AppImage | `45c0f9e84ab9fc8efa028f80d4be5f76dccd991250154d1dd9c52db9bfb6dd8e` |

All three hashes matched the final build manifest. The packages and VM disks are
local test artifacts and are not committed.

## Real desktop observations and outstanding acceptance

Both Wayland user sessions were successfully opened. Existing GUI checks covered
settings navigation/drafts, validation, real test-banner delivery, tray Show/Quit,
close-to-background and second-launch restoration. These observations do not
prove that an incoming call works with the window closed.

On the actual desktop PulseAudio/PipeWire servers, the existing native sound-smoke
assertions passed with an explicit analog output. VM-only diagnostic copies
selected that output to avoid Fedora's additional AirPlay device. Guest output
volume was temporarily raised from zero and restored afterward. Human confirmation
of audible playback is still outstanding.

The existing native microphone diagnostic opened the analog input and stopped
automatically after ten seconds on both guests. Both measured peak 0.002, below
the diagnostic's 0.005 threshold: the speech-signal checks failed. Input mute and
volume were restored, with no remaining capture streams. No audio samples were
saved; host microphone authorization and physical audio remain unqualified.

The real user-keyring roundtrip did not pass: GNOME returned
`credentialAccessRequired`, and the KDE test was stopped after a 180-second wait
without unlocking. Earlier CLI unlock attempts had started duplicate test-owned
GNOME daemons. These were removed and the regular provider restored; the encrypted
login-keyring file was unchanged. Private synthetic keyring tests do not establish
successful real-user credential restoration.

The Mac and guest screens later locked again, preventing the final icon sight
check. Package contents, Wayland identity and GTK icon lookup were verified; a
post-fix graphical icon check is still open. Fedora's isolated startup sessions
also logged an accessibility-bus activation permission warning. SELinux remained
enforcing; complete accessibility acceptance is not claimed.

At this stage, the original Linux Asterisk runner selected ignored Linux service
tests without their private session. Earlier interop runs required a VM-only selection
adjustment; a separate TLS test showed a bounded-observation timing issue. These
are recorded limitations, not claims that the unmodified full lab runner passed.
The subsequent [Linux lab verification](2026-11-16-linux-lab.md) records the
repository fixes and a successful complete run of the regular runner.
No new x86_64 Linux execution, hosted CI, physical hardware qualification or
Windows runtime verification is established by this record.

## Issue assessment

The implementation checklist of [#19](https://github.com/mardu-systems/ruf/issues/19)
is satisfied by the native Linux notification adapter, shared privacy/action
regressions, private D-Bus action/withdrawal/capability tests and real desktop
test-banner delivery. The icon correction supplies the matching packaged identity.
The work is on the Linux feature branch; integration into the default branch is
a separate step.

Keep [#17](https://github.com/mardu-systems/ruf/issues/17) open for physical
two-way audio and device reconnection; [#18](https://github.com/mardu-systems/ruf/issues/18)
for real GNOME/KDE credential restoration; [#20](https://github.com/mardu-systems/ruf/issues/20)
for incoming calls with the window closed; [#21](https://github.com/mardu-systems/ruf/issues/21)
for actual login/autostart acceptance; and [#22](https://github.com/mardu-systems/ruf/issues/22)
for hosted CI and clean-system installation/removal acceptance.
