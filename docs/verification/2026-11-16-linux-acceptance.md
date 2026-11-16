# Linux desktop acceptance follow-up

The subsequent [audio recovery verification](2026-11-16-linux-audio-recovery.md)
records native device/server recovery fixes, successful real microphone and
speech-echo tests on both guests, and the remaining physical hardware acceptance.

Observed on 16 November 2026 at `daa238647e95e466ffba91c2ec837121d6b22bf4`,
on Ubuntu 22.04.5/GNOME and Fedora 44/KDE ARM64 UTM guests. This updates the
outstanding status in the [desktop record](2026-11-16-linux-desktop.md).
The [full Linux lab run](2026-11-16-linux-lab.md) remains a separate SIP/media
verification. VM-only helpers and raw logs remain outside Git.

## Real credential providers

Both existing user collections were unlocked using their existing credentials.
Neither collection was reset, replaced or migrated. The following existing
tests each executed exactly one test and passed on both desktops:

```text
store::tests::system_keychain_roundtrip_and_delete --exact --ignored
phone::tests::saved_accounts_survive_session_restart_without_password_readback --exact --ignored
```

The first test creates, reads and deletes a unique synthetic OS-keyring entry,
then verifies its absence. The second uses the actual loopback Asterisk lab,
restores saved synthetic accounts into a fresh `PhoneSession`, verifies their
registration and selected account, checks password-free metadata and orphan
cleanup, and removes its own accounts/secrets. Ubuntu's restart case passed in
0.32 seconds; Fedora's passed in 0.33 seconds. An earlier incorrectly qualified
Ubuntu filter selected zero tests; it is not counted as evidence.

Fedora executes the Ubuntu-built ARM64 test binary against its actual KDE Wallet
and session bus; this is not a separately compiled Fedora test binary. The
existing private-session Linux checks additionally cover locked background access
and unavailable stores without plaintext fallback. Those checks also passed in
both native Linux CI jobs. [#18](https://github.com/mardu-systems/ruf/issues/18)
was closed after all of its criteria were satisfied.

## Installation, removal and CI

The final DEB/RPM packages were removed and reinstalled on their corresponding
guests. The check required Ruf to be stopped, verified the installer hash against
the build manifest, and recorded package-owned files and application-data hashes.
Removal deleted all package-owned files while preserving application data
byte-for-byte and retaining existing keyring files. Reinstallation restored
package files; `dpkg -V ruf` and `rpm -V ruf` passed respectively. Afterward the
installed executable and final AppImage each passed the existing private
Xvfb/D-Bus startup check on both distributions: four successful startup checks.
Package hashes remain those in the [desktop record](2026-11-16-linux-desktop.md).

Hosted [workflow run 43851873020](https://github.com/mardu-systems/ruf/actions/runs/43851873020)
completed with these observed results:

| Job                          | Result                                                             |
| ---------------------------- | ------------------------------------------------------------------ |
| Native Linux x86_64          | Passed: SDK, native/service checks, installers, startup and upload |
| Native Linux ARM64           | Passed: SDK, native/service checks, installers, startup and upload |
| Portable quality checks      | Passed                                                             |
| Asterisk SIP and RTP echo    | Passed                                                             |
| Browser UI state regressions | Failed                                                             |
| macOS ARM64 and Intel        | Failed during native SDK/OpenSSL validation                        |
| Windows x86_64               | Failed in UI/tooling tests                                         |

Local distribution installation/removal acceptance was ARM64. x86_64 has a
native hosted build/startup result. The supported distributions, architectures
and delivery formats are defined in [platforms](../platforms.md) and
[Linux setup](../linux.md). [#22](https://github.com/mardu-systems/ruf/issues/22)
was closed for its Linux packaging criteria. This does not establish that the
overall PR is green or ready to merge. Implementation is on draft
[PR #32](https://github.com/mardu-systems/ruf/pull/32), stacked on PR #31.

Fedora's private startup still logged an accessibility-bus activation permission
warning. Its windows appeared and the checks passed; complete accessibility
qualification is not claimed. SELinux remained enforcing.

## Desktop interaction and remaining acceptance

The installed release applications were driven through WebKit's native WebDriver
using Tauri's existing process-local `TAURI_WEBVIEW_AUTOMATION=true` support.
Commands went through the actual native IPC and existing window capabilities;
the application backend was not mocked. Test-only WebKit driver packages were
installed in the VMs. No production dependency or security configuration changed.

On each desktop, the settings command enabled login startup, generated exactly
one `~/.config/autostart/Ruf.desktop` with
`Exec=/usr/bin/sip-desktop --autostart`, disabled startup and removed the entry.
The original disabled state and notification preference were restored afterward.

Three further rounds used actual desktop logout through GNOME Session Manager
or KDE Shutdown and human login at each VM's greeter. Ruf was not manually
launched before observing startup. Each round created a new graphical session:

| Mode                                    | Ubuntu GUI session / Ruf PID | Fedora GUI session / Ruf PID | Result                                                                           |
| --------------------------------------- | ---------------------------- | ---------------------------- | -------------------------------------------------------------------------------- |
| Autostart enabled, `startHidden: true`  | 221 / 105119                 | 117 / 16675                  | One `--autostart` process, registered tray, no visible Ruf frame                 |
| Autostart enabled, `startHidden: false` | 228 / 106741                 | 124 / 18374                  | One `--autostart` process, visible Ruf phone frame                               |
| Autostart disabled                      | 236 / none                   | 130 / none                   | No autostart entry or Ruf process, including a 25-second observation after login |

GNOME's `app-gnome-Ruf-<pid>.scope` and KDE's
`app-Ruf@autostart.service` identified desktop-owned launches. A manual second
launch in both enabled rounds exited successfully, restored the existing phone
window and retained the original single process on both systems. These login
launches did not run under WebDriver; WebDriver was used between rounds to save
preferences through the existing settings IPC. The original complete preferences
and disabled autostart state were restored after the final round.
[#21](https://github.com/mardu-systems/ruf/issues/21) was closed after these checks.

Both actual installed applications registered a unique synthetic account against
their local Asterisk instance. The user closed their native phone windows;
read-only AT-SPI then showed no visible Ruf frames, while the original processes
remained registered and their main webviews remained reachable. Asterisk's
`channel originate PJSIP/1001 application Wait 30` produced a real incoming offer
on each hidden application. Each actual exported StatusNotifier/DBusMenu Answer
action established the call; Hang up returned it to idle. Native snapshots
confirmed `incoming`, `active` with a connected timestamp, then `idle`.

Menu discovery checked the D-Bus owner against the exact Ruf process ID. Show
restored a visible phone window and Quit terminated the application on both
desktops. No tray mouse event was required. Only the test-owned account, secret
and single history row were removed afterward. These calls used the existing
test-tone mode and do not qualify physical audio. [#20](https://github.com/mardu-systems/ruf/issues/20)
was closed after these observations.

Closing a WebDriver browsing context is not counted as closing the native phone
window: that destroys the webview, whereas Ruf's native close handler hides the
window and keeps the backend alive.

Post-fix UTM window screenshots also showed the Ruf brand icon in Fedora's
settings title bar, taskbar and tray, and Ubuntu's dock/tray and GNOME's native
application-ready banner. These are observations of the installed packages;
AppImage desktop integration and an exhaustive icon/theme matrix are not claimed.

Still open: [#17](https://github.com/mardu-systems/ruf/issues/17) for physical
Linux audio and device reconnection. The microphone speech checks had failed at
this stage; the later audio record documents successful capture and audible echo
through UTM on both desktops. Virtual device tests do not replace physical
headset disconnection/reconnection acceptance.

No production code, API/DTO, native ownership, threading or authorization boundary
was changed for this follow-up. Unrelated Windows ARM64/DMG working-tree changes
were excluded.
