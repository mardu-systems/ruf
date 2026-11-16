# Linux audio recovery and real VM signal verification

Observed on 16 November 2026 on the working tree based on `daa2386`, using
Ubuntu 22.04.5/GNOME/PulseAudio and Fedora 44/KDE/PipeWire ARM64 guests in UTM
5.0.8 on Apple Silicon. This supplements the
[desktop acceptance](2026-11-16-linux-acceptance.md). The user has the Mac's
built-in microphone/speakers; no USB headset or physical Linux machine was
available. Hardware qualification in #17 is therefore still incomplete.

## Reproduced failures and minimal corrections

The existing private-session audio integration test was extended before the
production change. Removing a selected virtual microphone initially failed the
new assertion: capture continued on a different input. Fixed source/sink IDs
now use libpulse's existing `PA_STREAM_DONT_MOVE` flag. Default routes retain
the server's routing policy; saved device IDs are not rewritten.

With only that correction, the same test failed at the next assertion:
recreating the selected microphone still returned native error 19 (`ENODEV`).
The adapter's global failure remained set even though the PulseAudio context
was healthy. The SIP loop now acknowledges recovery after all failed diagnostic,
ringtone and call owners have been released, and only while the existing
PulseAudio lock confirms `PA_CONTEXT_READY`. Workers continue to signal errors;
they never destroy or join themselves. The existing bounded baresip reconnect
timer owns reconnection. No second media pipeline or dependency was added.

The extended test passes and covers fixed-input/output removal and replacement,
unchanged routing, new owner tokens, stale-stop isolation, missing-device
rejection and audio-server loss/restart in the same `Engine`. Its replacement
server belongs to the private fixture and is killed/reaped on teardown.

A separate temporary harness also passed endpoint removal/replacement against
each actual desktop audio server, including Fedora's real pipewire-pulse. It
created and removed only its own virtual endpoints, opened no physical input,
left saved routing unchanged and did not stop either desktop audio server.

Public IPC/DTOs and their validation/error types are unchanged. Runtime behavior
and recovery rules are documented in [audio diagnostics](../api/audio-diagnostics.md)
and [Linux setup](../linux.md).

## Physical Mac microphone through UTM

The user confirmed UTM's macOS microphone permission and spoke during both
ten-second native microphone diagnostics. The analog input was temporarily
unmuted at 100%; both diagnostics measured peak `0.082` and passed the existing
`0.005` signal threshold. Both stopped automatically and discarded samples.
Original input volume/mute were restored. This supersedes the earlier `0.002`
failed signal observation, without proving which earlier environmental factor
caused it.

Actual system-audio Asterisk echo calls used the existing native `Engine`,
explicit analog input/output/ring routes, synthetic loopback credentials and
extension 600. They did not use test-tone/file mode, save secrets, create UI
accounts/history or record audio. Input/output were temporarily set to 100%/35%
and restored afterward.

- Ubuntu: local speaker/ringtone tones were heard. The initial human response
  reported no speech echo. The call established and ended normally; Asterisk
  reported 355 received and 355 transmitted packets at seven seconds, no loss.
  A separate six-second route inspection confirmed unmuted/uncorked analog
  source/sink streams at 100% stream volume, mono signed-16-bit PCM at 8 kHz.
  In-memory PCMU statistics measured nonzero returned audio (peak 4604, roughly
  RMS 205), with 300 outgoing/296 returned packets. This verifies signal transport
  independently of audibility. The user subsequently confirmed the full repeated
  tone/ringtone/speech-echo test worked; that run measured peak 5116 with 751
  outgoing/747 returned PCMU packets and nonzero returned RMS. No additional
  Ubuntu production change was made between the initial unconfirmed echo and this successful repetition;
  the earlier missing-echo observation has no established software root cause.
- Fedora: the user confirmed the test worked. The actual capture/playback
  streams were unmuted, uncorked, at 100% stream volume and on the selected
  analog source/sink, using mono signed-16-bit PCM at 8 kHz. Asterisk reported
  356 received/transmitted packets at seven seconds, no loss. In-memory PCMU
  statistics measured nonzero signal in both directions: peaks 3132, RMS
  approximately 164, and 747 outgoing/743 returned packets during observation.
  Packets and audio samples were discarded; only aggregate statistics were saved.

Fedora executes the Ubuntu-built ARM64 native executables against its own real
desktop services. This does not constitute a separately compiled Fedora build,
physical Linux USB disconnection or complete hardware acceptance.

## Verification and packages

| Check                                              | Observed result                                                       |
| -------------------------------------------------- | --------------------------------------------------------------------- |
| Original fixed-input removal regression            | Failed: stream silently survived on another input                     |
| Fixed route only, reconnect regression             | Failed: new capture rejected with native error 19                     |
| `bun run test:linux`, Ubuntu                       | Six service/failure cases passed, including extended audio recovery   |
| `bun run test:linux`, Fedora                       | Six cases passed using updated Ubuntu-built test executables          |
| Real desktop endpoint removal/replacement          | Passed on PulseAudio and pipewire-pulse; owned virtual endpoints only |
| `bun run test:native`, Ubuntu                      | 334 passed across all selected partitions                             |
| `bun run test:asterisk`, Ubuntu                    | Fresh synthetic RTP echo, 17 interop and 14 TLS/SRTP cases passed     |
| Host `cargo test --locked --workspace`             | 157 passed                                                            |
| Host and Linux native workspace/all-targets Clippy | Passed with warnings denied                                           |
| UI unit tests                                      | 159 passed                                                            |
| Format, type/Svelte, frontend build, versions      | Passed                                                                |
| Type generation                                    | Passed; generated files unchanged                                     |
| Native Linux `desktop:build --features native`     | DEB, RPM, AppImage, SBOM and installer hashes built                   |
| Updated installed DEB/RPM                          | Installed and `dpkg -V ruf` / `rpm -V ruf` passed                     |
| Updated installed binary and AppImage startup      | Four private Xvfb/D-Bus checks passed across both guests              |

Host checks used the existing Rustup 1.98.1 toolchain through a process-local
PATH. An initial frontend build and six UI cases failed with the broken Homebrew
LLVM/Z3 toolchain. With Rustup selected, one UI case correctly rejected the stale
host SDK after the patch changed. Rebuilding the SDK with the documented pinned
macOS-14 OpenSSL prefix resolved that failure; all 159 cases then passed. The
default Homebrew OpenSSL build was also correctly rejected for requiring macOS
27 rather than Ruf's macOS-14 baseline. No compiler, SDK-identity or deployment
check was disabled.

The new local ARM64 artifact hashes are:

| Artifact | SHA-256                                                            |
| -------- | ------------------------------------------------------------------ |
| DEB      | `637ea25a1e01f3b5bf16659cfc8fa5355572ae7aea7152314886f8be398d972f` |
| RPM      | `b810bb73eba35bdd23f0b950f90af5ed104e90d02350910a9b616ed575bf05a7` |
| AppImage | `d008917884daae2ca09d0bef7545396e299dd62db382fe9d2e1dabd678b4731e` |

Hashes matched the new build manifest. These packages are separate from the
earlier icon-fix artifacts. VM tools, logs, packets, credentials and build output
are excluded from Git. The unrelated Windows ARM64/DMG edits remain untouched.
Both disposable Asterisk services were stopped afterward. The temporary virtual
devices were removed, original physical-input/output volume and mute restored,
and no capture or playback streams remained on either real desktop server.

The earlier hosted CI run covers the preceding committed head, not this audio
recovery change. No new hosted CI, x86_64 runtime, physical Linux audio or USB
headset acceptance is claimed. Fedora's private startup still logs the known
accessibility activation warning with SELinux enforcing; startup passes, but
complete accessibility qualification is not claimed.
