# Desktop platforms and architectures

Ruf uses one frontend and phone core, with the existing OS-specific native
adapter. Architecture support means a separate SDK, native executable, inventory
and installer for each target. It does not establish hardware/provider acceptance.

| OS         | CPU                   | Rust target                 | Build runner       | Packages           |
| ---------- | --------------------- | --------------------------- | ------------------ | ------------------ |
| macOS 14+  | Apple Silicon / ARM64 | `aarch64-apple-darwin`      | `macos-15`         | App, DMG           |
| macOS 14+  | Intel / x86_64        | `x86_64-apple-darwin`       | `macos-15-intel`   | App, DMG           |
| Linux GNU  | x86_64 / amd64        | `x86_64-unknown-linux-gnu`  | `ubuntu-22.04`     | DEB, RPM, AppImage |
| Linux GNU  | ARM64 / aarch64       | `aarch64-unknown-linux-gnu` | `ubuntu-22.04-arm` | DEB, RPM, AppImage |
| Windows 11 | x86_64                | `x86_64-pc-windows-msvc`    | `windows-2022`     | NSIS               |

An additional experimental Windows ARM64 CI job uses `windows-11-arm` to build
`aarch64-pc-windows-msvc` and retain its NSIS installer. It remains separate from
this matrix's release selection and does not establish Windows ARM64 hardware,
audio or SIP acceptance.

## Observed coverage

The [16 November PR CI](verification/2026-11-16-ci-platform-fixes.md) passed native
Linux x64, macOS ARM64 and Windows x64 builds/tests and their installers. Intel
macOS and ARM64 Linux/Windows remain part of the full main/manual/release matrix;
that PR run did not execute them. Linux ARM64 additionally has
[Ubuntu/GNOME and Fedora/KDE VM acceptance](verification/2026-11-16-linux-audio-recovery.md).
Physical device/provider qualification remains separate for every architecture.
These development targets are not all present in the original
[published 0.1.1 assets](releases/0.1.1.md).

## Linux distributions

Linux binary builds use Ubuntu 22.04/glibc 2.35 as their compatibility baseline,
with WebKitGTK 4.1. Building on a newer distribution can raise the required glibc
version; host-built packages should not silently replace baseline release builds.

| Distribution family                                  | Delivery                                  | Requirements / boundary                                                                                                                           |
| ---------------------------------------------------- | ----------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------- |
| Ubuntu 22.04+ / Debian 12+                           | DEB or AppImage                           | GTK3, WebKitGTK 4.1, libpulse and a graphical user session.                                                                                       |
| Fedora / openSUSE                                    | RPM or AppImage                           | Compatible glibc (2.35+), GTK3 and WebKitGTK 4.1. RPM uses shared-library capability dependencies instead of distribution-specific package names. |
| Arch / derivatives and other compatible GNU desktops | AppImage or source build                  | Same ABI/session requirements; this is not an AUR or distribution-repository publication.                                                         |
| Raspberry Pi and other ARM64 desktops                | ARM64 package for the distribution family | A 64-bit GNU OS and WebKitGTK 4.1 are required; a 32-bit OS cannot run an ARM64 package.                                                          |

The system tray requires Ayatana AppIndicator; DEB/RPM declare its runtime
dependency, and AppImage includes its library. Package checks verify this even
though Tauri loads the library dynamically.

Audio remains in baresip's `pulse` module, using PulseAudio or pipewire-pulse.
Secret Service and freedesktop notifications use the user's session D-Bus.
No browser microphone, second audio stack or implicit permission fallback is
introduced. OS service/hardware support and distribution acceptance still need
observed execution, as described in [Linux acceptance](linux.md).

musl/Alpine, 32-bit x86/ARM, RISC-V, BSD, NixOS-specific bundles, Flatpak and Snap
are not additional prebuilt targets. Adding a container/sandbox format requires
its own audio, credential and desktop permission review. The SDK builder rejects
unconfigured host triples instead of staging a misleading GNU SDK.

## macOS

Both CPU families retain the macOS 14 minimum and the same CoreAudio,
Keychain/Contacts/UserNotifications integration and entitlements. OpenSSL has
separate native build/cache directories for ARM64 and x86_64; the dependency
probe verifies the archive's CPU and deployment minimum before staging it.

Build natively on the matching CPU with the pinned toolchains. Apple Silicon can
also execute an explicitly selected Intel toolchain under Rosetta for validation;
that is not an observation on physical Intel hardware. A universal application
is not manufactured from a single-CPU SDK. Each native DMG keeps its own SBOM
and executable/installer hashes.

## Build and release evidence

On a matching Linux GNU host:

```sh
bun run native:build
bun run desktop:build --features native --bundles deb,rpm,appimage
```

On a matching macOS host, build the pinned native OpenSSL as documented in
`native/README.md`, select its prefix, then:

```sh
bun run native:build
bun run desktop:build --features native --bundles app,dmg
```

The draft-release workflow collects both macOS CPUs, both Linux CPUs and existing
Windows x64. It requires every expected format, native release context, matching
version, SBOM/installer hashes and identical sidecars across Linux formats.
Missing, mixed, altered or colliding artifacts stop the draft before `gh release`.
No universal or portable build is accepted as a substitute for native evidence.

Runner availability: [GitHub's runner matrix](https://docs.github.com/en/actions/how-tos/write-workflows/choose-where-workflows-run/choose-the-runner-for-a-job).
Linux baseline rationale: [Tauri AppImage compatibility](https://v2.tauri.app/distribute/appimage/).
