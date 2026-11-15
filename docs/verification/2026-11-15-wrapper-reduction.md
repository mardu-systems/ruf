# Wrapper reduction — 15 November 2026

The previous adapter changes were committed first as `13b5c93`, as requested.
This records the subsequent working-tree reduction and local macOS arm64 checks.

## Measured scope

Physical production lines in the affected integration files, including comments
and blank lines, excluding native test files and Rust test-module suffixes:

| Revision                        | Production lines |
| ------------------------------- | ---------------: |
| Original `99375a6`              |             4304 |
| Previous refactoring, `13b5c93` |             4534 |
| This working tree               |             4373 |

This change removes **161 production lines** relative to the committed baseline
(3.6% of these files). The combined changes still add **69 production lines**
relative to the original version. No claim of a net reduction relative to the
original version is made. Tests and documentation are counted separately.

The same file set is counted for all revisions: adapter build.rs; native
bridge.c, bridge.h, call_controls.h and protocol.def; Rust dialogs.rs, engine.rs,
ffi.rs and lib.rs. Files absent at a revision count as zero. Rust test code starts
at the first top-level cfg(test) or cfg(all(test, ...)) section in these files.

## Removed structure

- Five C request entrypoints become one named sip_request. Separate account and
  sound FFI payload types disappear; device and flag fields retain their names.
- The C queue copies each request and all populated strings into one allocation.
  Per-field allocation/free handling disappears. Password wiping, generation
  checks, the sixteen-request bound and allocation-free stop remain.
- NativeEngine owns the callback context as a normal Box. Startup errors use the
  existing Drop path instead of repeating raw Box reconstruction, handle cleanup
  and lease release. The additional unsafe Send implementation is unnecessary.
- The destination map disappears. Pending and live CallSnapshot values already
  own the destination; no parallel index needs maintenance.
- The preview pass-through and unreachable queued-stop branch disappear. Existing
  validation proves the removed consultation/answer branches redundant. Call
  updates reuse the already validated reference/index under the existing lock.
- Account, sound and removal waits share one condition-variable loop. Existing
  SIP error classification is reused with the original fallback messages.

All public Rust APIs, desktop commands and DTOs remain compatible, including
configure_account. No dependencies, upstream patches, audio pipeline, permission
or credential policy changed. The revised private request contract is documented
in docs/api/native-adapter.md.

## Changed files

- Rust: crates/baresip-adapter/src/engine.rs, ffi.rs and dialogs.rs.
- Native: crates/baresip-adapter/native/bridge.c, bridge.h, call_controls.h and
  tests/call_actions.c. The existing test assertions exercise the new request
  entrypoint and internal ownership layout.
- Docs: docs/api/native-adapter.md, docs/architecture.md,
  crates/baresip-adapter/native/README.md and this record.

## Observed checks

| Check                                                                | Result                                                                                                                      |
| -------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------- |
| Native SDK build                                                     | Passed with the existing macOS-14-compatible OpenSSL install.                                                               |
| Formatting and git diff checks                                       | Passed.                                                                                                                     |
| Frontend typecheck and build                                         | Passed; zero Svelte errors or warnings.                                                                                     |
| Version check                                                        | Passed.                                                                                                                     |
| Type generation                                                      | Passed; generated TypeScript hashes unchanged.                                                                              |
| UI tests                                                             | 143 passed, zero failures.                                                                                                  |
| Portable workspace tests                                             | 157 passed, zero failures, one explicitly ignored test.                                                                     |
| Portable workspace Clippy, all targets, warnings denied              | Passed.                                                                                                                     |
| Native workspace Clippy including native test hooks, warnings denied | Passed.                                                                                                                     |
| Complete native runner                                               | Passed: NATIVE_CHECKS_OK.                                                                                                   |
| Strict C/network harness                                             | Passed, including payload ownership, saturated queue stop, call/audio/BLF cases, WASAPI injection and 65,536 DNS refreshes. |
| Asterisk                                                             | Passed: RTP echo, 17 lab tests, 14 security/lab cases and one desktop startup test.                                         |
| Native desktop build                                                 | Passed; local app, DMG, SBOM and build hashes produced.                                                                     |

The existing tests cover copied password/device buffers modified immediately
after submission, answer rollback, callback lifetimes, consultation ownership,
late transfer results, registration, RTP, DTMF, mute/hold and verified TLS with
required SDES-SRTP. Independent static review found no blocking issue.

These are local executions with Rust 1.98.1. Windows runtime, physical audio
devices, arbitrary providers and hosted CI were not qualified. CPU/RAM or
throughput savings were not benchmarked; the line counts and reduced allocation
structure do not establish a measured runtime speedup.
