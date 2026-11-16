# Native adapter simplification — 15 November 2026

Local macOS arm64 verification of the adapter working-tree changes. This is not
a hosted CI run, Windows runtime qualification or physical audio-device test.

## Implementation

1. Control destinations reuse `contact_dial_target` directly, retaining the
   existing InvalidInput message without constructing a registered snapshot.
2. Each live CallSnapshot is authoritative. Native events update their owning
   dialog; selected-call compatibility fields are derived only on snapshot reads.
   Queued outgoing reservations retain immediate hangup tokens, and terminal
   events/errors remain observable after the final dialog closes. Unrelated
   command errors have an explicit projection lifecycle without becoming dialog
   errors. The old select/save/restore cycle and global hold/end copies are gone.
3. `bridge.h` separates account, call, sound and audio requests. Device IDs and
   mute/hold flags have named fields. C consumes `protocol.def`; Cargo generates
   private Rust constants from that same table. The existing native queue still
   owns its copied payloads and wipes passwords before releasing them. Stop still
   works without allocation or queue capacity. Condition notifications replace
   10-ms account/audio completion polling without abandoning delayed requests.

Public Engine methods, desktop IPC commands and generated DTOs are unchanged,
including configure_account. No dependencies or upstream SDK patches changed.
The private boundary is documented in `docs/api/native-adapter.md`.

Changed files:

- Rust adapter: `crates/baresip-adapter/src/{engine,dialogs,ffi,lib}.rs` and
  `crates/baresip-adapter/build.rs`.
- Native boundary: `crates/baresip-adapter/native/bridge.c`, `bridge.h`,
  `protocol.def`, `call_controls.h` and `tests/call_actions.c`.
- Documentation: `crates/baresip-adapter/native/README.md`,
  `docs/architecture.md`, `docs/api/README.md`, `docs/api/native-adapter.md`
  and this verification record.

## Regression and review evidence

- The selected-dialog read regression failed before projection was moved to the
  read boundary, then passed. Existing callback fixtures now inspect public
  snapshots for selected-call fields rather than their obsolete stored copies.
- Independent review found that an account-removal rejection during an incoming
  call could disappear from the new projection. A failing regression reproduced
  it; the correction preserves the command error, avoids attributing it to the
  call and clears it on the same lifecycle transitions as before. Re-review found
  no remaining actionable issue.
- The account/audio wakeup regressions cover success, rejection and shutdown. An
  initial predicate-based timeout test incorrectly accepted a missing notification
  after waiting to its deadline. The corrected wait test rejects an expired wait
  even if the predicate changed meanwhile. Suppressing notify_all then caused the
  account regression to fail; restoring it made the suite pass.
- The strict C harness checks borrowed-input copying (including a subsequently
  modified password/device buffer), named sound and control fields, overload at
  sixteen pending requests and shutdown while the queue is full.

## Observed final checks

| Check                                                                        | Result                                                                                                            |
| ---------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------- |
| `bun run native:build`                                                       | Passed with the existing macOS-14-compatible OpenSSL installation.                                                |
| `bun run format:check` / `git diff --check`                                  | Passed.                                                                                                           |
| `bun run check`                                                              | Passed; zero Svelte errors/warnings and TypeScript checks passed.                                                 |
| `bun run build` / `bun run check:versions`                                   | Passed.                                                                                                           |
| `bun run types:generate`                                                     | Passed; generated TypeScript file hashes unchanged.                                                               |
| `bun run test:ui`                                                            | 143 passed, zero failures.                                                                                        |
| `cargo test --locked --workspace`                                            | 157 passed, zero failures, one explicitly ignored test.                                                           |
| Portable workspace Clippy, all targets, warnings denied                      | Passed.                                                                                                           |
| Native workspace Clippy with native test hooks, all targets, warnings denied | Passed.                                                                                                           |
| `bun run test:native`                                                        | Final complete runner passed: NATIVE_CHECKS_OK.                                                                   |
| `bun run test:network-roam`                                                  | Passed: strict C queue/audio/BLF tests, portable WASAPI fault injection, latency checks and 65,536 DNS refreshes. |
| `bun run test:asterisk`                                                      | Final run passed: real RTP echo plus 17 lab tests, 14 security/lab cases and one desktop startup case.            |
| `bun run desktop:build --features native`                                    | Passed; local native app, DMG, SBOM and build hashes generated.                                                   |

The Asterisk run observed real registration, establishment, RTP tones, DTMF IVR,
mute/unmute, hold/resume, consultation/transfer and verified TLS with required
SDES-SRTP, including plaintext refusal. Hardware playback is not implied.

## Intermediate failure and limits

One native run failed only
`sip_event_tests::retired_register_timeout_preserves_replacement_registration`:
after 33 seconds it observed Registering instead of Registered. The same timing
failure is already recorded in `2026-11-15-blf.md`. An exact isolated repetition
passed, followed by a passing final complete native runner. No speculative timing
or production change was made for this observation; its underlying cause remains
unqualified by this refactoring.

Docker Desktop stopped between the first passing Asterisk run and its repetition.
That attempt failed before executing SIP tests because the daemon socket was
absent. Docker was restarted; the final complete Asterisk run passed.

Commands used the installed project-pinned Rust 1.98.1 through the Rustup PATH,
without changing global compiler settings. Physical macOS/Windows devices,
arbitrary providers and hosted release CI were not requalified. Desktop packaging
used the existing local signing setup; it does not certify release notarization.
