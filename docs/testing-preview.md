# Try the Ruf Developer Preview

Ruf 0.1.1 is a free Developer Preview for people with a test SIP account or PBX.
There are no in-app purchases. Use an isolated test environment and keep another
way to make important calls. Provider and platform qualification is still open.

## Get the preview

The published [v0.1.1 Developer Preview](https://github.com/mardu-systems/ruf/releases/tag/v0.1.1)
provides macOS Apple Silicon and Windows x64 installers. These are the original
11 November assets. [Newer development changes](releases/unreleased.md), including
Linux and BLF, require a source build or a retained artifact from the matching
[Actions run](https://github.com/mardu-systems/ruf/actions/workflows/ci.yml).
Source archives are not desktop installers, and a CI artifact is not a published
release. See [source setup](../README.md#run-locally) and [Linux setup](linux.md).

| Development target      | Package              | Observed scope / remaining qualification                                                                               |
| ----------------------- | -------------------- | ---------------------------------------------------------------------------------------------------------------------- |
| macOS 14+ Apple Silicon | App / DMG            | Native PR CI passed; inspect the exact asset's signing/notarization status.                                            |
| macOS 14+ Intel         | App / DMG            | Native build target and local Intel-toolchain evidence; physical Intel acceptance remains separate.                    |
| Windows 11 x64          | NSIS EXE             | Native tests and installer passed on Windows Server CI; installed Windows 11 and physical audio remain unqualified.    |
| Windows 11 ARM64        | NSIS EXE             | Experimental full-matrix CI artifact, excluded from release-draft assets; hardware/SIP acceptance remains open.        |
| Linux GNU x64           | DEB / RPM / AppImage | Native PR CI, service checks, packages and isolated startup passed.                                                    |
| Linux GNU ARM64         | DEB / RPM / AppImage | Ubuntu/GNOME and Fedora/KDE UTM acceptance; Fedora used Ubuntu-built executables. Physical headset checks remain open. |

The [platform matrix](platforms.md) defines distro, CPU and ABI boundaries.
The [verification index](verification/README.md) records source-specific results.
The latest three-target PR result does not establish a passing six-target run.

For release downloads, compare your installer with its matching line in
`SHA256SUMS`. On macOS use `shasum -a 256 <installer>.dmg`, on Linux
`sha256sum <installer>`, and on Windows
`Get-FileHash -Algorithm SHA256 .\<installer>.exe`. Compare the resulting digest
with the exact filename in `SHA256SUMS`; you do not need to download other
platforms' installers. CI artifacts instead include build manifests with their
installer/SBOM hashes. Checksums detect changes, not publisher identity.
Read the exact package's signing information. Do not disable Gatekeeper or other
system protection to test an untrusted package. No automatic updater is included.

## Connect your test account

1. Install and open Ruf. It starts without a configured account.
2. Open **Settings → SIP accounts**, add an account and enter the server,
   username and password supplied for your test PBX. A separate authentication
   username and outbound proxy are available when your PBX requires them.
3. Match the PBX's transport and media policy. TLS verifies the certificate and
   hostname. Required SRTP needs TLS and fails instead of falling back to
   plaintext. UDP/TCP with ordinary RTP is unencrypted. See
   [account security](api/account-security.md) for exact behavior.
4. Save, then wait for the account to show **Registered**. Registration alone
   does not prove that audio works.
5. In **Sound**, choose the microphone, call output and ringtone output. Grant
   microphone access when explicitly requested by the OS. Linux uses the desktop
   audio service rather than a macOS-style permission dialog. Contacts are
   available only on macOS and access is optional;
   you can dial a test extension manually.
6. Call a known echo extension or a consenting test peer. Confirm outgoing and
   incoming audio independently before trying any other actions.

The German equivalents are **Einstellungen → SIP-Konten**, **Angemeldet** and
**Ton**. The app follows the system language or your saved German/English choice.

## Help test compatibility

Run only scenarios available in your controlled test environment. Record each
as passed, failed or not tested; never turn an untested row into a passing claim.

| Scenario                 | What to observe                                                                                                                       |
| ------------------------ | ------------------------------------------------------------------------------------------------------------------------------------- |
| Registration and restart | Correct identity restores, failures remain visible and secrets are not printed.                                                       |
| Incoming/outgoing audio  | Both peers can hear one another; answer, decline and hangup complete on both sides.                                                   |
| Mute and hold            | The other peer confirms mute, hold and restored audio after resume.                                                                   |
| Keypad                   | A known IVR/echo service receives digits, star and hash.                                                                              |
| Direct/attended transfer | The destination and original peer remain connected after Ruf hands the call over.                                                     |
| Extension status / BLF   | Configure a PBX-supported extension, observe ringing/busy/idle transitions, then loss/recovery; unknown is not proof of availability. |
| Two accounts             | Incoming identity and selected outgoing identity remain correct.                                                                      |
| Background/recovery      | Hide/reopen, permission denial, sleep/wake and network recovery are observed separately.                                              |
| Audio hardware           | Record headset/output model, removal/reconnection and a longer test call.                                                             |
| Security                 | Record the negotiated transport/media status; test certificate rejection only on your own test server.                                |

Submit a [compatibility report](https://github.com/mardu-systems/ruf/issues/new?template=compatibility_report.yml)
with app version, OS/architecture, PBX version, transport, codec, media protection
and network topology. See the [qualification matrix](release-checklist.md#supported-beta-qualification)
for the larger supported-beta acceptance scope. One successful call does not
qualify every PBX, network or platform.

Windows testers should use the [Windows 11 x64 physical audio acceptance
matrix](windows-audio-acceptance.md), including communications-device routing,
microphone privacy, device removal and installed-app behavior. Its rows start as
not tested; record exact versions and sanitized evidence for each result.

Linux testers should record their distribution, CPU, desktop/session type,
PulseAudio or pipewire-pulse, credential provider and package format. Follow the
[manual Linux acceptance list](linux.md#manual-release-acceptance), especially
physical headset removal/reconnection and routing afterward. VM speech echo and
virtual endpoint recovery are recorded separately from physical device results.

Reports are public. Use fictional identities and sanitized results; exclude
passwords, real phone numbers, contact lists, addresses and raw SIP/SDP captures.
Use [SECURITY.md](../SECURITY.md) for security-sensitive findings.

## First contributions

- Review the setup instructions on a fresh Mac and report the missing step.
- Exercise the Linux or Windows test matrix with a real microphone/headset.
- Check Intel macOS and ARM64 targets on the corresponding hardware.
- Exercise the FreeSWITCH matrix with the exact server version and settings.
- Improve reproducible examples and German/English wording after discussing scope.

Follow [CONTRIBUTING.md](../CONTRIBUTING.md) and check the
[current issues](https://github.com/mardu-systems/ruf/issues) before starting.
