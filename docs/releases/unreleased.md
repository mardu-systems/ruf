# Unreleased development changes

These changes follow the published **v0.1.1 Developer Preview** of 11 November 2026. The workspace version is still 0.1.1; a newer source build can therefore
show that version without matching the published installers. Record the source
revision and build manifest when reporting results. No new release is announced
by this document. The [0.1.1 notes](0.1.1.md) describe the original tagged assets.

## Calling and interface

- Account-scoped SIP BLF monitors up to 32 configured extensions per account.
  Unknown, idle, ringing and busy states come from native dialog subscriptions;
  the PBX must provide suitable hints and subscription rights. Failure to monitor
  an extension does not fail account registration. See the [BLF contract](../api/blf.md).
- The compact phone, first-run introduction, keyboard navigation and recovery of
  windows/call controls have been refined. Existing draft protection, account
  identity and explicit call actions remain in place.
- baresip/libre is pinned to 4.13.0. Internal adapter state and request ownership
  were simplified while retaining native thread confinement, bounded queues,
  credential wiping, stale-token isolation and joined shutdown.

## Platforms and audio

- Linux GNU x64/ARM64 builds provide DEB, RPM and AppImage packages, with
  Ubuntu 22.04/glibc 2.35 as the build baseline. Encrypted Secret Service,
  freedesktop call notifications, GTK clipboard, tray/background operation and
  installed-app autostart reuse the existing native desktop contracts.
- Linux audio uses baresip's PulseAudio backend, also usable with pipewire-pulse.
  Microphone, call output and ringtone output are separate. Fixed streams no
  longer move silently to another device. After failed owners are released and
  the device/server is ready, new tests/calls recover without restarting Ruf.
- Native macOS Intel and Apple Silicon builds retain the macOS 14 minimum,
  separate SDKs and per-target SBOM/hash manifests. The DMG uses a simplified
  Retina drag-to-Applications background.
- Windows x64 audio reports actual startup errors, follows the default
  communications endpoint when no fixed device is selected, and bounds device
  recovery. Diagnostics work without a configured account. Windows ARM64 adds an
  experimental native CI installer; it is not selected for release drafts.

See the [platform matrix](../platforms.md), [Linux guide](../linux.md) and
[Windows hardware acceptance matrix](../windows-audio-acceptance.md).

## Build and verification

PRs build Linux x64, macOS ARM64 and Windows x64. Main-branch, manual and release
workflows retain all six native targets. Shared checks run centrally; native
tests compile once and run their discovered executables with at most six test
processes, followed by serial Cargo doc tests. Package, OS-service, SIP and
security checks remain enabled. Platform-specific OpenSSL and test-fixture
failures were corrected.

[CI #57](https://github.com/mardu-systems/ruf/actions/runs/43870420990) passed all
seven jobs at `cf5102f` in 13:48, including SDK rebuilds. This measured PR run
does not qualify the full six-target matrix or predict warm-cache duration.
The [CI verification record](../verification/2026-11-16-ci-platform-fixes.md)
binds results and timings to that revision.

Ubuntu 22.04/GNOME and Fedora 44/KDE ARM64 UTM acceptance covers installed-app
startup, real keyring persistence, background call notifications, login modes and
package removal/reinstallation. Real Mac microphone signal and user-confirmed
speech echo passed through both guests; virtual device removal/recreation and
audio-server recovery also passed. Fedora used Ubuntu-built ARM64 executables.

Physical Linux headset unplug/replug and route verification remain open in
[#17](https://github.com/mardu-systems/ruf/issues/17). Windows physical-device,
provider and broader desktop qualification also remain separate. These are
experimental preview builds for controlled test networks.
