# Windows audio hardening — 15 November 2026

This records local macOS arm64 execution for the Windows-audio working-tree
change based on `4bbebd4ba2b1ed4eff315198b88b4cfa82beca04`. It is not a Windows
runtime or hosted-CI qualification. The native SDK remains pinned to
baresip/libre 4.13.0; the ninth ordered SDK patch is
`native/patches/baresip-wasapi.patch`, SHA-256
`36ed8728aa17dd621cf8ef7c8f17a64b0b6f09cba0ec90f1c9410bb325390b90`.

## Behavior and regressions

- Actual WASAPI worker startup errors reject allocation. A fault-injected denial
  initially reproduced the previous successful-allocation bug. Success and timeout
  now race through CAS; a deterministic timeout-boundary regression failed before
  the fix and passed afterward.
- Capture honors SILENT packets with an unreadable/null OS sample pointer.
  Source errors reach the existing handler on the native loop, allowing that
  handler to synchronously destroy the source without a worker self-join.
- Default streams follow the communications-role endpoint. Fixed selections
  reopen only their selected ID. Recovery has a five-second window and 100 ms
  retry interval. Independent review reproduced indefinite retry when capture
  `ReleaseBuffer` failed after a packet; moving deadline reset after successful
  release made the new persistent-invalidation test pass.
- Optional backend status getters report playback errors per owned stream.
  Native regressions cover speaker-test cleanup, ringtone failure and failure
  of a background consultation without attributing it to the selected dialog.
- Account-free microphone/speaker diagnostics preserve saved backend/routing and
  metadata. Native synthetic tests forbid credential access. The first complete
  native run exposed a test-lease conflict with credential-recovery tests; the
  diagnostic test now takes their existing shared native-test mutex.
- Windows UI explains the unsupported permission query and provides the existing
  fixed privacy-settings action. This does not introduce a Windows authorization
  query. The communications-default behavior is explicit in Sound settings.

## Observed checks

All Rust commands used the installed project-pinned Rust 1.98.1 toolchain rather
than the broken Homebrew compiler selected by the initial shell PATH. The SDK
was rebuilt with the existing macOS-14-compatible OpenSSL prefix. The default
Homebrew OpenSSL was correctly rejected because its archive targets macOS 27.

| Check                                                              | Observed result                                                                                                                                                                                                                |
| ------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `bun run native:build`                                             | Passed with compatible existing OpenSSL; exact patch content staged in the SDK manifest.                                                                                                                                       |
| `bun run test:network-roam`                                        | Passed: production queue tests, optional-player status, fault-injected production WASAPI workers, latency probes and 65,536 DNS refreshes.                                                                                     |
| Windows API syntax                                                 | Final patched `src.c`, `play.c`, `util.c`, `wasapi.c` passed MinGW x64 `-std=c11 -Wall -Wextra -Werror -fsyntax-only` against Windows headers in an ephemeral Debian container. This is not MSVC linkage or Windows execution. |
| `cargo test --locked --workspace`                                  | 157 passed, zero failures, one explicitly ignored desktop test.                                                                                                                                                                |
| `bun run test:native`                                              | Passed: `NATIVE_CHECKS_OK`, all selected Cargo partitions; desktop native library 104 passed, zero failures, five explicitly ignored cases.                                                                                    |
| `bun run test:asterisk`                                            | Passed: real synthetic 440 Hz RTP echo plus 32 adapter/desktop integration tests, including transfers, DTMF, hold/resume, mute, first incoming call, TLS/SRTP and plaintext refusal.                                           |
| Portable and native workspace Clippy, all targets, warnings denied | Passed.                                                                                                                                                                                                                        |
| `bun run test:ui`                                                  | 143 passed, zero failures.                                                                                                                                                                                                     |
| German settings browser flow                                       | `UI_FLOW_OK de settings`, including simulated Windows unsupported authorization and explicit capture/settings actions.                                                                                                         |
| `bun run check`, `bun run check:versions`                          | Passed; zero Svelte errors/warnings and versions remain 0.1.1.                                                                                                                                                                 |
| `bun run types:generate`                                           | Passed; generated TypeScript contracts unchanged.                                                                                                                                                                              |
| Frontend build and `bun run desktop:build --features native`       | Passed; native macOS app/DMG plus inventory/hash manifests produced.                                                                                                                                                           |

The local DMG `Ruf_0.1.1_aarch64.dmg` has SHA-256
`8bdcca990e68a084050bfcea075ef01420282fd72c2d9ed3cff9565d1163e809`.
It uses ad-hoc signing and was not notarized. Packaging emitted an environment
warning from `rust-objcopy` about missing `libLLVM.dylib`; packaging and final
hash/SBOM generation nevertheless completed successfully.

## Remaining qualification

No physical microphone, speaker, USB/Bluetooth headset, Windows 11 installation,
Windows permission transition or real Windows device reconnection was exercised.
No Windows installer was built or installed in this session; no hosted CI run
was observed. OS/driver calls that block can delay joined startup cleanup even
though the handshake and retry windows are bounded. The
[Windows physical acceptance matrix](../windows-audio-acceptance.md) keeps all
hardware rows not tested until an actual result is recorded.
