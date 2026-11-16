# Linux desktop foundation, 15 November 2026

Observed locally against base commit `71553c8711db2a89bee1e88eca74599ce4dc3843`
plus the Linux changes before their categorized commits on
`codex/linux-foundation`. Pre-existing Windows ARM64 and macOS DMG edits remain
outside the Linux commits. No push, hosted workflow or release was performed.

## Implementation

- `native/build.ts`, `native/cache-key.ts`, the adapter build script and
  `native/patches/baresip-pulse.patch`: native Linux PulseAudio modules/linkage,
  exact SDK/patch identity, bounded native acceptance and terminal status.
- `crates/baresip-adapter/native/audio_devices_linux.c`: bounded, capture-free
  source/sink enumeration through libpulse, excluding monitor sources.
- `apps/desktop/src-tauri/src/store/linux.rs`: Secret Service through encrypted
  sessions, typed errors, noninteractive background access and journaled cleanup.
- `crates/baresip-adapter/src/notifications/linux.rs` and `src/linux.rs`:
  actionable freedesktop notifications and GTK clipboard dispatch. GTK is an
  optional `desktop` feature; headless adapter builds do not acquire a GUI
  requirement. No new Cargo packages were resolved; existing GTK, zbus,
  secret-service and pkg-config packages are reused as direct dependencies.
- Desktop lifecycle/permission presentation, installed-path autostart, existing
  IPC/DTO documentation, Linux setup and acceptance instructions.
- `scripts/desktop-artifacts.ts`, desktop/release workflows and bundle config:
  DEB/AppImage discovery, installer hashes, SBOM sidecars, Ubuntu x64 CI and
  private-session desktop startup checks.

Existing `CredentialStore`, foreground-operation serialization, phone-session
revalidation, SIP event-thread ownership, notification token authorization and
per-window IPC capabilities remain the boundaries. No frontend SIP/audio,
password-bearing persisted DTO, generic command bridge or new media pipeline
was added. Generated TypeScript contracts remain unchanged.

## Observed checks

| Check                                                                              | Result                                                                                                                                            |
| ---------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------- |
| `bun run format:check`, `bun run check`, `bun run build`, `bun run check:versions` | Passed locally on macOS; typecheck has zero errors/warnings. Rustup was selected explicitly instead of the broken local Homebrew compiler.        |
| `bun run types:generate` and generated-file diff                                   | Passed; no generated changes.                                                                                                                     |
| `bun run test:ui`                                                                  | 145 passed, zero failures.                                                                                                                        |
| `bun run test:ui:flows`                                                            | 23 German/English/multiwindow browser scenario groups passed.                                                                                     |
| Portable workspace Rust tests                                                      | macOS: 157 passed/1 ignored; Linux: 147 passed/6 ignored.                                                                                         |
| Workspace Clippy, all targets, portable/native                                     | Passed on macOS and Linux with warnings denied.                                                                                                   |
| `bun run test:native`                                                              | Complete selected Cargo partitions passed on macOS and Linux.                                                                                     |
| `bun run test:asterisk`                                                            | 32 isolated lab cases passed, including desktop restart/first incoming-call coverage. Fresh returned PCM: 4.04 seconds, RMS 5811, 440.1 Hz.       |
| Linux SDK build and `check-native-sdk.ts`                                          | Passed; restored SDK validation and NAT regressions passed.                                                                                       |
| `bun run test:linux`                                                               | Six isolated service tests passed, plus the sound routing/preview smoke example.                                                                  |
| Linux `bun run test:network-roam`                                                  | Passed, including both new Pulse metadata callback harnesses and existing WASAPI/call/audio/latency regressions.                                  |
| Native desktop packages                                                            | macOS app and Linux ARM64 DEB/AppImage built successfully.                                                                                        |
| Final DEB/AppImage startup                                                         | Both produced a visible Ruf window and remained running under private Xvfb/D-Bus, as an unprivileged user. No application sandbox bypass was set. |
| Installer/SBOM evidence                                                            | Both package hashes and SBOM hashes independently matched both manifests.                                                                         |
| Workflow/shell validation                                                          | actionlint v1.7.12, `bash -n` and `git diff --check` passed.                                                                                      |

Linux was Ubuntu 24.04/aarch64 in a disposable container, using libpulse
`1:16.1+dfsg1-2ubuntu10.1` and WebKitGTK `2.52.6-0ubuntu0.24.04.1`.
AppImage packaging/startup used extraction mode because the container has no
FUSE device. A first packaging run exposed the missing `file` prerequisite;
the documented/CI prerequisite list and successful final build include it.

The explicit audio-server-loss test initially failed because capture remained
active. The PulseAudio patch now reports terminal state on the SIP event loop
and the same test passes. Independent review also identified negative-`eol`
metadata null dereferences and missing C-harness libpulse linkage; both were
fixed and covered by the final Linux harness runs. The final review reported
no outstanding blocker.

## Artifact binding

Locally generated packages are ignored under `artifacts/linux-arm64/`; raw Linux
logs are ignored under `native/build/linux-foundation/`.

| Artifact                     | SHA-256                                                            |
| ---------------------------- | ------------------------------------------------------------------ |
| `Ruf_0.1.1_arm64.deb`        | `8cb170427afef68946ab8889f1b917c254e589fad8321be913376afb3ed30ee4` |
| `Ruf_0.1.1_aarch64.AppImage` | `7827c27dd89d225a0e03c98f5b237e17300ebe82941f869accf17bc65b387a11` |
| Compiled CycloneDX SBOM      | `fa5f915c8d945c81dfd34f9c05cd6f3d991116d457bdb4bfa5922458ebfc3132` |

The manifest's executable hash describes the final build-output executable.
Tauri patches bundle-type information when packaging; an executable extracted
from another bundle format need not have identical bytes. Installer hashes bind
the complete delivered archives.

## Remaining qualification

Ubuntu x64 hosted CI is configured, not observed. Physical Linux audio/headsets,
Wayland, real GNOME/KDE integration and arbitrary provider compatibility remain
unqualified. Xvfb startup proves window creation/liveness, not complete rendering
or hardware behavior; its environment emitted portal/DRI/GTK diagnostics.
System contacts remain explicitly unsupported. Restart Ruf after terminal
audio-server/stream failure or a notification-daemon restart; automatic recovery
is not claimed. No Windows runtime test or hosted CI result is implied.
