# Dependency refresh and repository review — 16 November 2026

Scope: the working tree on `codex/dependency-refresh-review`, based on
`819ad4a69281bfe9fb5a09c305605bd6f0f79189`. The user deferred release platform
selection, physical hardware acceptance and extended everyday testing. No release
number or IPC contract changes are included. The initial findings and checks below
are retained as historical evidence. See **Follow-up remediation** for the fixes
and fresh checks; native Windows qualification remains pending. This report is
not approval to publish a release.

## Dependency changes

| Group                              | Previous                | Updated                                    |
| ---------------------------------- | ----------------------- | ------------------------------------------ |
| Rust toolchain / workspace minimum | 1.98.1                  | 1.100.0                                    |
| Tauri Rust runtime                 | 2.12.5                  | 2.12.6, with compatible companion packages |
| Tauri JavaScript API / CLI         | 2.12.4 / 2.12.4         | 2.12.5 / 2.12.5                            |
| rusqlite / bundled SQLite bindings | 0.41.1 / 0.39.1         | 0.43.0 / 0.40.0 (SQLite 3.54.1)            |
| i18next                            | 27.1.1                  | 27.1.3                                     |
| Vite                               | 8.3.6                   | 8.3.8                                      |
| Playwright CLI                     | 0.1.23                  | 0.1.24                                     |
| Matching playwright-core types     | 1.65.0-alpha-2026-10-30 | 1.66.0-alpha-1794252756000                 |
| rust-cache action                  | 2.9.2                   | 2.9.3                                      |
| cache action                       | 5.1.0                   | 6.1.0                                      |
| upload / download artifact actions | 6 / 6                   | 7.0.2 / 8.0.2                              |

Cargo and Bun generated their lockfiles; compatible transitive patches were
included. Playwright's engine remains a prerelease because the stable CLI itself
depends on that exact engine; browser installation and TypeScript types match it.
SQLite notice filenames and the third-party inventory now match the resolved
versions; their upstream MIT license texts were verified unchanged.
The existing `native` feature now gates the adapter's `pkg-config` build dependency,
following its existing optional `serde_json` pattern. Rust 1.100 otherwise warns
about the unused dependency in portable builds.

The Asterisk 22 multiarchitecture image is pinned to OCI index
`sha256:b7ed63ba5d6b5b0515ac2d715fc3ba6df8b84857db639a64f8f7918132bb8af5`.
The vcpkg baseline is now `4df3365fcbb4795a44795883f8719bbb3b3bcd77`;
this alone does **not** resolve the OpenSSL finding below. Actions retain immutable
commit pins, existing permissions, fork-cache restrictions and the existing PR/full
platform matrix. Artifact uploads retain their default ZIP format; download v8's
digest verification is not disabled. No extra CI jobs were added.

Upstream change notes reviewed include [Tauri 2.12.6](https://github.com/tauri-apps/tauri/releases/tag/tauri-v2.12.6),
[rusqlite 0.42](https://github.com/rusqlite/rusqlite/releases/tag/v0.42.0)
and [0.43](https://github.com/rusqlite/rusqlite/releases/tag/v0.43.0),
[i18next 27.1.3](https://github.com/i18next/i18next/releases/tag/v27.1.3),
[Playwright CLI 0.1.24](https://github.com/microsoft/playwright-cli/releases/tag/v0.1.24)
and the Vite/API/CLI and Actions migration notes. SQLite ownership/error-path fixes,
locale fallback fixes and artifact format/digest behavior were relevant to this
project. No new runtime dependency or application abstraction was introduced.

## Compatibility holds

- Bun 1.4.4, Svelte 5.57.3 and baresip/libre 4.13.0 were already current.
- TypeScript stays at 6.0.3: current svelte-check 4.7.8 declares TypeScript 5/6
  support; TypeScript 7.0.2 is outside that peer range. No peer override is used.
- GTK stays at 0.18.2: current Tauri/tray dependencies share that generation's
  bindings. Forcing GTK 0.19 would not upgrade the complete compatible graph.
- zbus stays at 5.19: current secret-service 5.2 shares its connection types, and
  Tauri's single-instance plugin also uses that major. Moving only our direct
  dependency to zbus 6 would not remove the old transitive graph.
- windows-core stays at 0.62.2 with the current windows 0.62.2 generated bindings;
  independently upgrading it to 0.100 would mismatch those COM types.
- macOS OpenSSL stays at maintained 3.5.9 LTS rather than changing the supported
  TLS generation to 4.1. Distribution-owned Linux libraries remain OS-managed.

## Review findings

### P1 — Windows builds still select an unsupported OpenSSL series

`native/vcpkg.json:4` selects a registry baseline whose OpenSSL port is 3.6.5#2.
Both the previous and updated baselines select this version. At review time the
[OpenSSL source/support table](https://openssl-library.org/source/) no longer lists
3.6 among supported series. Consequently rebuilding the Windows installer at this
baseline still embeds a TLS dependency without upstream maintenance for its series.
This is a support gap; no specific reachable CVE is asserted here.

The [pinned registry](https://github.com/microsoft/vcpkg/blob/4df3365fcbb4795a44795883f8719bbb3b3bcd77/versions/o-/openssl.json)
does not yet offer OpenSSL 4.x or 3.5.9 (its latest 3.5 entry is 3.5.4).
Do not override to an unavailable version or downgrade to that old patch. The
concrete follow-up is a maintained 3.5.9 LTS port or a qualified supported 4.x build
through the existing native build inputs, followed by Windows x64/ARM64 build and
TLS/SRTP negative-path tests. Windows execution was unavailable in this local run.

### P2 — Linux dependency graph retains a GLib unsoundness advisory

`Cargo.lock` retains glib 0.18.5 through GTK/Tauri. RustSec
[RUSTSEC-2024-0429](https://rustsec.org/advisories/RUSTSEC-2024-0429.html)
reports unsound `VariantStrIter` behavior; fixed versions start at 0.20.
No Ruf call path to the affected iterator was established by this review, so this
is not a demonstrated remote exploit. It still prevents calling the dependency
audit clean. Upgrading the complete Tauri/GTK binding graph or evaluating a narrowly
maintained upstream patch is the follow-up; forcing a second GTK generation is not
a remedy.

Four additional unmaintained warnings remain: proc-macro-error 1.0.4
(RUSTSEC-2024-0370), zbus_names 4.3.5 (RUSTSEC-2026-0371), zvariant 5.15.1
(RUSTSEC-2026-0370), and zvariant_derive 5.15.1 (RUSTSEC-2026-0372).
They require compatible parent upgrades, not merely editing lockfile versions.
Cargo audit 0.23.0 reported zero entries in its vulnerability category and these
five warnings. Bun audit returned no advisories; neither result proves absence of
vulnerabilities in native or platform libraries.

### P2 — History searches discard database pagination

`apps/desktop/src-tauri/src/history.rs:486` requests every matching history row
whenever a search or contact-number filter is present. `history/database.rs:178`
loads and deserializes all of them. `project_page` then resolves contacts at
`history.rs:79` before applying the requested offset/limit. The resolver in
`crates/phone-core/src/contacts.rs` scans contact phone numbers for each row.

With unlimited retention (the default), a text search can therefore perform
O(history rows × contact phone numbers) matching and allocate the entire history
even for a 50-row page. No latency benchmark or native SIP-lock starvation is
claimed; projection happens outside the SIP owner. Use a transient contact index
with identical ambiguity/regional matching rules, then stream/filter rows while
retaining exact totals and pagination. Current authorized names must remain
transient and must not be stored in history to obtain SQL search performance.

## Review coverage and checks

The review covered frontend state/polling/cleanup, IPC capabilities and validation,
account persistence and credential rollback, history/contact projections, native
thread/callback ownership and caller-data validation, platform services, build
provenance, packaging and CI permissions. Larger files were inspected at their
boundaries and relevant flows; this is not an every-line formal verification or a
physical-platform certification. No additional high-confidence correctness finding
was established in those inspected flows.

Observed updated-tree checks:

- Formatting, version consistency, frozen Bun installation, portable workspace
  Clippy with warnings denied and generated-contract clean-diff checks passed.
- Frontend strict tooling/Svelte checks, production build and all 164 UI/tooling
  tests passed. Existing German/English browser flows and multiwindow localization
  passed using the updated CLI and Chromium.
- Portable workspace tests passed: 157 tests, one opt-in OS-keychain test ignored
  by default. Native Clippy with warnings denied and the complete native workspace
  runner passed on macOS ARM64. Generated DTOs remained unchanged.
- The rebuilt native SDK passed the network/lifetime regression harness, including
  65,536 resolver refreshes and native queue/BLF/audio lifecycle tests.
- The updated Asterisk image passed all 17 selected SIP/RTP tests, 14 TLS/SRTP
  security cases and the saved-account desktop restart test. Synthetic returned
  audio was measured; no physical microphone was opened for this run.
- A private Debian 12 ARM64 container built the SDK and passed native Clippy, the
  complete native runner and `test:linux` with isolated D-Bus, GTK/Xvfb, PulseAudio,
  GNOME Keyring and notification fixtures, including missing-session rejection.
  It used the changed tracked source snapshot; later Windows-only baseline and
  documentation edits do not affect this Linux result. This does not requalify
  Ubuntu/Fedora UTM desktops or physical Linux hardware.
- macOS ARM64 app and DMG packaging passed. Signed microphone/Contacts bundle
  checks and synthetic contact, notification and login-item service tests passed.
  Local bundles are ad-hoc signed, not notarized release artifacts.
- Clang static analysis produced zero diagnostics for ten production bridge files:
  bridge, security, audio devices/diagnostics, network roam, BLF, contacts,
  microphone permission, call notifications and login item. Native baresip/libre
  upstream source and Windows-only C were not covered by that macOS analyzer run.
- Pinned actionlint 1.7.12 passed. No hosted workflow for this dependency branch
  was executed; CI success on the base revision does not qualify these updates.

Initial refresh artifact SHA-256 values (native macOS ARM64, version 0.1.1):

| Artifact              | SHA-256                                                            |
| --------------------- | ------------------------------------------------------------------ |
| Ruf.app executable    | `2416423727b9c04774a40d4cd4601357371ce1f3763a3e15c7d58b5d95077809` |
| Ruf_0.1.1_aarch64.dmg | `81cc668435bee0c41c624af3cb25a9dd393d28f22426a9e73bc7843f779d084e` |

The installed Homebrew Rust command is broken on this host; checks used the pinned
Rustup compiler. macOS packaging still emitted `rust-objcopy` LLVM-library loading
warnings, including after trying the existing documented process-local library
path. Packaging returned success, but warning-free stripping is not established;
neither toolchain files nor strip policy were patched to bypass it.
No credentials, local recordings, build outputs
or container fixtures are tracked. Physical hardware, long-running usage,
Windows execution and incompatible upstream migrations remain outside this run.

## Follow-up remediation

The same branch now contains the following corrections, preserving IPC DTOs,
the SIP owner/thread model, credential boundaries and current contact permissions.

| Finding             | Implementation                                                                                                                                                                                                          | Observed validation / limit                                                                                                                                                                                    |
| ------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Windows OpenSSL     | Official vcpkg 3.5.4 port tree `737382595bac2a92c7f8f54f120b53d2637af521`, updated only to 3.5.9 and its GitHub tag-archive SHA-512. Existing x64/ARM64 static-md triplets and CryptoAPI trust validation are retained. | All nine patches apply to the downloaded, checksum-verified 3.5.9 source. Simulated MSVC discovery and SDK/SBOM version tests pass. Actual Windows compilation/runtime remain **unverified**.                  |
| GLib soundness      | Published 0.18.5 crate with the exact two-line upstream `VariantStrIter::impl_get` mutability fix; root Cargo patch selects it for the existing GTK graph.                                                              | Unpatched upstream release-mode string-iterator tests crashed with SIGSEGV. The same three tests pass after the fix. The Linux adapter regression passes in debug/native and release builds.                   |
| History search      | One transient authorized-contact index, streaming database rows and retaining only the requested page. Empty searches retain SQL pagination.                                                                            | Nineteen focused history-related tests pass, including new exact-total, late-error, contact-name/privacy and ambiguity cases. A deliberate ambiguity-guard mutation fails the regression; restoring it passes. |
| Rust/LLVM stripping | Select the existing Rustup proxies instead of raw toolchain binaries. Clean only cached release artifacts for adapter/desktop, including host helpers.                                                                  | Fresh native macOS ARM64 app/DMG build succeeds without any `rust-objcopy`/`libLLVM` warning; signed microphone/Contacts bundle checks pass.                                                                   |

### Boundaries and provenance

Windows builds explicitly pass `native/vcpkg-ports` to vcpkg. The overlay is part
of both SDK and independent OpenSSL cache keys. CMake discovery, Rust adapter
compilation and native SBOM/cache validation all require the maintained 3.5 LTS
series at least as new as the overlay. Tests reject 3.5.4, 3.6.5 and malformed
versions, and retain custom compatible 3.5.9/3.5.10 installations. Older staged
3.6 SDKs cannot silently qualify through the old manifest alone. No certificate
or plaintext-fallback policy was weakened. See the
[overlay provenance](../../native/vcpkg-ports/README.md).

The GLib crate archive checksum, upstream fix commit and removal condition are
recorded in [vendor provenance](../../vendor/README.md). Recursive comparison
against the original published source confirms the two intended source-line
changes. Original upstream source, tests, copyright and MIT notice are retained;
vendored files are excluded from project formatting and workspace membership.
The Linux SBOM records its original version, upstream patch/advisory identity and
patched source-file hash. No advisory is ignored or fictional release assigned.
Cargo audit reports zero vulnerability-category entries and four unmaintained
warnings. Its absence of the GLib warning after switching to a path package is
not the proof of remediation; the upstream correction and reproduced crash/pass
are. Older GLib source still emits upstream lifetime-style compiler warnings
under Rust 1.100; these were not suppressed or broadly refactored.

History query projection runs on the existing ordered database worker using
owned contact/account snapshots. SIP callbacks still only enqueue lifecycle
facts, and no phone/native owner is held while scanning. Additional working
memory is bounded by the current contact index plus at most 100 returned rows,
rather than the full search result. Exact totals still require scanning relevant
history rows; no latency benchmark or unlimited-history constant-time claim is
made. Canonical phone-core normalization, source/ID ambiguity rules, literal
Unicode matching, regional preferences, pagination and corruption errors remain
covered. Contact names/indexes are never stored. The
[history API documentation](../api/history.md) records this unchanged contract
and new processing behavior.

Rustup supplies `DYLD_FALLBACK_LIBRARY_PATH`, including its toolchain library
directory. The raw compiler-path workaround used in the initial refresh bypassed
that setup; setting only `DYLD_LIBRARY_PATH` was insufficient on the complete
packaging path. The existing Homebrew Rustup proxies fix both compiler selection
and helper loading. No installed binary, runtime search path or strip policy was
patched. See [developer setup](../development.md). The separate Homebrew Rust
installation was not repaired or removed.

### Fresh checks and qualification limits

- Strict TypeScript/Svelte checks, frontend build and **165 UI/tooling tests** pass.
  These include Windows version-rejection/replacement and Linux SBOM backport
  provenance tests. Existing release-note parity tests cover both changelogs.
- Portable workspace tests pass (**160 tests**, one opt-in keychain test ignored
  by default). Portable and native workspace Clippy pass with warnings denied
  for project code. DTO generation is unchanged.
- Native SDK rebuild and complete macOS native workspace runner pass. The full
  Asterisk run passes **17 SIP/RTP**, **14 TLS/SRTP** cases and the saved-account
  desktop restart test. These are synthetic media/security checks.
- A private Debian 12 ARM64 container passes native SDK construction, native
  workspace Clippy and the complete native runner. `test:linux` additionally
  passes the release-mode GLib iterator test and isolated GTK, PulseAudio,
  Secret Service and notification checks, including missing-session rejection.
  The new regression's initial Clippy `nth(0)` error was corrected to `next()`;
  the full run then passed. Later documentation, SBOM-test and Windows-only
  validation refinements do not change that Linux runtime result.
- The final native macOS app/DMG build and both signed permission checks pass.
  Artifacts remain ad-hoc signed, not notarized release packages.
- Formatting, frozen Bun install, version consistency, generated-contract
  clean-diff and pinned actionlint pass. An independent read-only candidate
  review found no additional concrete regression in the inspected paths.

Trying Windows OpenSSL configuration directly on macOS encountered missing NASM
and Windows-specific Perl path requirements. This is not Windows qualification.
The code/reproducible recipe changes are implemented, but verification of the
Windows finding remains **blocked on actual Windows x64/ARM64 builds and native
TLS/SRTP tests**. No new hosted workflow or physical hardware test was observed,
and no issues were closed. The existing Linux CI step gains only the focused
optimized iterator test, with no additional jobs or expanded platform matrix.

Final corrected local artifact SHA-256 values (version 0.1.1):

| Artifact                          | SHA-256                                                            |
| --------------------------------- | ------------------------------------------------------------------ |
| Native macOS ARM64 app executable | `7d755fd77c0121b73a136fbf0598e5f99f5ae537f2ce9280f1b065d1167aaf93` |
| Ruf_0.1.1_aarch64.dmg             | `95355ffc494df6cd9fa356ae11015c6dc478f1fe8266aa965750d63abe252ff7` |
