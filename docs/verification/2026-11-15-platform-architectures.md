# Linux/macOS architecture expansion — 15 November 2026

Working-tree checks on `codex/linux-foundation`, based on `68d38db`. This record
adds architecture/build evidence to the earlier [Linux foundation](2026-11-15-linux-foundation.md).
It does not qualify every version-0.1.1 package or establish hosted CI acceptance.

## Scope and preserved boundaries

The build matrix now selects Linux GNU x86_64/ARM64 on Ubuntu 22.04 and macOS
ARM64/Intel on macOS 15 runners. Linux produces DEB/RPM/AppImage; each macOS
target produces its own app/DMG in CI. Native CPU, OpenSSL archives, SDK cache,
compiled inventory and release assets retain separate target identities.

No SIP/media implementation, IPC DTO, credential access policy, native ownership
or thread boundary changed. Existing Windows ARM64 and DMG artwork work remains
separate. Documentation is in [platforms](../platforms.md), [Linux](../linux.md),
[development](../development.md), [CI/release](../ci-cd.md) and the native README.

## Observed checks

- Bun 1.4.4 and Rustup Rust/Cargo 1.98.1; the host Homebrew Rust installation had
  a missing Z3 dylib. Repeating checks with Rustup resolved its style-check errors.
- 159 UI tests passed, including wrong-CPU OpenSSL archives, ARM/Intel recipe
  selection, mismatched Rust hosts and release validation failures. The new CPU
  and host tests were observed failing before their fixes.
- Frontend typecheck (zero errors/warnings), frontend build, versions, stable
  generated Rust-to-TypeScript contracts, formatting and workflow lint passed.
- macOS ARM64 portable workspace Clippy/tests and native workspace Clippy/tests
  passed. The existing process-isolated native suite completed successfully.
- Native macOS ARM64 and x86_64 apps built; executable architectures and ad-hoc
  signatures were verified. Intel SDK and app ran through explicit Intel Bun/Rust
  under Rosetta on Apple Silicon. The SDK's two retained C NAT test groups passed.
- The pinned Intel OpenSSL 3.5.9 recipe built successfully with the selected
  `test_ciphername`, `test_sslapi` and `test_tls13secrets` recipes. This local run
  did not execute the complete upstream OpenSSL suite; CI retains its full default.
- All 32 existing Asterisk registration, call/media and TLS/security checks passed;
  the disposable lab was stopped afterwards.
- On Ubuntu 22.04 ARM64 (glibc 2.35), the native SDK built and its retained C NAT
  test groups passed. Six isolated Linux service/failure tests passed, covering
  PulseAudio capture, GTK clipboard, notification actions/withdrawal, Secret
  Service storage/locking and missing-session credential rejection.
- Ubuntu 22.04 ARM64 native workspace Clippy passed with warnings denied.
  All three native installers built. Package architecture/dependency checks and
  extraction of the AppImage's actual tray library passed. The executable's
  highest referenced glibc symbol version was 2.34 (below the 2.35 baseline).
  The final DEB executable and AppImage each displayed a Ruf window and stayed
  running in a private Xvfb/D-Bus session as an unprivileged test user. AppImage
  used its extraction mode because the container has no FUSE device.

An independent review found a Linux tray packaging gap: Tauri CLI did not discover
the tray feature activated directly on a workspace dependency. A default feature
now exposes the same existing feature to the bundler, while the direct dependency
feature preserves builds with `--no-default-features`. Real-package checks in
`tests/interop/linux/bundles.sh` cover architecture, tray/audio dependencies and
the AppImage's extracted tray library. No additional concrete findings remained.

## Local artifact bindings

SHA-256 values from the checked working-tree outputs; Linux package sidecars
include their native build context, compiled SBOM and executable/installer hashes.
Local outputs remain ignored under `artifacts/platforms/linux-arm64` and `target`.

| Output                     | SHA-256                                                            |
| -------------------------- | ------------------------------------------------------------------ |
| Linux ARM64 DEB            | `d83c5915fd276da49b9854658ff7036808f24c2a31da8bbd98071164a643c5b8` |
| Linux ARM64 RPM            | `2a4e60be220e6d7bc92ea8167754dbc6909503e90e8f12007e2d37a662c3e2ea` |
| Linux ARM64 AppImage       | `e91e0efd18b4a15a6d8a454442d8a0ecbafdc5576a30ea3bbab6ad43faadcfb8` |
| macOS Intel app executable | `626eda9b0594d58cf37183fe8aba23e6083f98ab6ea9b531e54af185a74a7025` |
| macOS ARM64 app executable | `3b11018aaa24936297525d271c92e472bbde0e50ac804a6d60c0f966ea3ec738` |

## Qualification boundary

Linux x86_64 and hosted jobs were configured, not executed locally. Linux distro
families beyond the Ubuntu container, Wayland, physical audio/Bluetooth, physical
Intel hardware and arbitrary SIP providers remain unqualified. macOS DMG creation
was configured in CI; these local architecture checks built apps. No hosted run,
release publication, production signing or notarization was performed.

## Related open issues

Reviewed against the open issue checklists on 15 November 2026. Local commits
do not close GitHub issues; the notification issue is a closure candidate after
the Linux foundation and these follow-up commits are merged together.

| Issue                                                                             | Assessment                                                                                                                                                                                                                                                                                                  |
| --------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [#19 — call notifications](https://github.com/mardu-systems/ruf/issues/19)        | Closure candidate after merge: the existing native interface now has Linux delivery, capability handling, bound answer/reject actions and stale-banner withdrawal. Shared privacy/action regressions and isolated Linux D-Bus tests passed. Real desktop/provider acceptance remains documented separately. |
| [#17 — native audio](https://github.com/mardu-systems/ruf/issues/17)              | Keep open: backend, routing and synthetic capture are implemented/tested; physical two-way audio and device reconnection remain unverified.                                                                                                                                                                 |
| [#18 — credential storage](https://github.com/mardu-systems/ruf/issues/18)        | Keep open: Secret Service CRUD, locked-store and missing-service checks passed; installed-app restart/restore and selected desktop compatibility remain unverified.                                                                                                                                         |
| [#20 — tray/background operation](https://github.com/mardu-systems/ruf/issues/20) | Keep open: tray/menu implementation and package dependencies are present; real desktop Show/Quit and incoming calls with the window closed remain unverified.                                                                                                                                               |
| [#21 — login autostart](https://github.com/mardu-systems/ruf/issues/21)           | Keep open: installed-executable integration and launch policy exist; enable/disable, hidden login and duplicate-instance behavior need an actual Linux login-session test.                                                                                                                                  |
| [#22 — packages/CI](https://github.com/mardu-systems/ruf/issues/22)               | Keep open: distro/CPU/format matrix and CI are configured, ARM64 packages/startup passed locally; hosted CI plus clean installation/removal acceptance remain outstanding.                                                                                                                                  |

Other open issues concern unrelated functionality, signing or platform acceptance;
this change does not satisfy their checklists.
