# Linux desktop setup and acceptance

Ruf has an experimental native Linux desktop target. The initial distribution
baseline is Ubuntu 22.04/glibc 2.35, with DEB, RPM and AppImage artifacts for
both x64 and ARM64. Both architectures have native hosted build jobs. See the
[platform matrix](platforms.md) for distribution and architecture boundaries. Build on the oldest distribution you intend to support.
There is no cross-compilation in the SDK builder.

## Build

Install the pinned Bun and Rustup versions from `.bun-version` and
`rust-toolchain.toml`, then the desktop/native prerequisites:

```sh
sudo apt-get update
sudo apt-get install build-essential git curl file cmake pkg-config libssl-dev libpulse-dev \
  libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev libxdo-dev \
  patchelf libfuse2
bun install --frozen-lockfile
bun run native:build
bun run dev:native
bun run desktop:build --features native --bundles deb,rpm,appimage
```

The SDK and desktop must use the same native architecture. A portable build
without `native` still reports `notLinked` and cannot make calls. Building an
installer is not proof of microphone, playback or provider compatibility.

For containerized AppImage builds without a FUSE device, set
`APPIMAGE_EXTRACT_AND_RUN=1` for the build command. CI uses this mode for the
packaging tools. It changes neither the application's sandbox nor media path.

## Platform services

- Audio uses baresip's `pulse` module and the system `libpulse` library. Either
  PulseAudio or PipeWire's `pipewire-pulse` service must be running. There is no
  browser/WebAudio recording or second media pipeline. Device discovery opens
  no recording stream, omits monitor sources, and has a three-second deadline.
  Explicit device routes stay on their selected microphone/speaker; removal
  reports an audio failure rather than moving a fixed stream to another device.
  After failed owners are released and the backend is ready, a new test/call can
  use the reconnected device/server without restarting Ruf. See
  [audio diagnostics](api/audio-diagnostics.md) for recovery and ownership rules.
- Credentials use an encrypted Secret Service session over the user's D-Bus,
  with service `org.rufapp.desktop` and existing opaque account keys. A compatible
  unlocked provider is required, for example GNOME Keyring or a configured
  Secret Service implementation. Background restore and deletion never execute
  unlock/deletion prompts. Locked entries return `credentialAccessRequired`;
  missing entries return `credentialMissing`; unavailable/ambiguous stores return
  `credentialFailure`. Foreground save/reconnect may request an unlock, outside
  the phone-session lock. Passwords never enter frontend DTOs or metadata files.
- Notifications use the freedesktop notification daemon over the user's D-Bus.
  Actions must be supported. `authorized` means the service supports delivery;
  it cannot establish the desktop's per-app settings or Do Not Disturb state.
  Banners are silent, escape body markup and retain the existing opaque-token
  authorization in the desktop service. Only signals from the daemon's unique
  bus owner are accepted. OS calls have three-second deadlines. A daemon restart
  requires restarting Ruf; an old daemon's ids cannot authorize a new banner.
  The notification icon and `desktop-entry` hint use `sip-desktop`, matching the
  packaged icon and GTK's Wayland application id. DEB and RPM include a hidden
  `sip-desktop.desktop` alias so desktops can resolve window and notification
  icons. The visible launcher remains `Ruf.desktop`; the alias adds no menu item.
  Tauri also copies DEB files into AppImages, so AppImage packaging explicitly
  replaces the alias with a visible Ruf launcher with freedesktop categories.
  AppImages require desktop integration for host-side window/icon association;
  bundled files are not installed into the host's application or icon directories.
- Clipboard writes run on Tauri's existing GTK main loop and have a three-second
  deadline. No external clipboard command or unrestricted shell access is added.
- Autostart uses the existing Tauri plugin for installed release executables in
  `/usr/bin` or `/usr/local/bin`. Development binaries and AppImages do not enable
  automatic login setup. Linux tray interaction uses menu entries; mouse events
  are not required. Availability of a visible tray depends on the desktop.
  Launching Ruf again restores its existing window through single-instance IPC.
- The generic microphone permission query remains `unsupported`; an explicit
  microphone test determines actual capture access. No macOS-style permission
  grant is fabricated. There is no portable command to open a Linux permission
  settings pane. The notification UI omits that action on Linux.
- System contacts remain explicitly unsupported. Existing dialing, local history,
  account settings and call controls use their existing cross-platform services.

## Automated verification

```sh
sudo apt-get install dbus-x11 gnome-keyring pulseaudio pulseaudio-utils xvfb xauth x11-utils python3-dbus python3-gi
bun run test:linux
bun run test:native
bun run test:asterisk
```

The desktop workflow also checks that an actual Ruf window appears and the
built executable stays running in a fresh Xvfb/D-Bus session:

```sh
xvfb-run -a dbus-run-session -- bash tests/interop/linux/startup.sh path/to/sip-desktop
```

The startup check exports its temporary XDG directories to D-Bus activation too,
so portal/keyring services do not reuse the logged-in desktop's storage. Cleanup
unmounts an owned document-portal FUSE mount before deleting the test directory.

`test:linux` creates a private D-Bus/Xvfb session, temporary XDG directories,
synthetic Secret Service entries, an in-process test notification daemon and
virtual PulseAudio endpoints. It does not use the logged-in desktop's keyring,
notifications or microphone. It verifies credential replacement and locked-store
background access, caller-text escaping, action origin/binding/withdrawal,
clipboard dispatch, native device routing and audio tests. It is an OS-service
integration check, not a physical hardware or Wayland qualification.

The notification integration test checks the actual icon name and desktop-entry
hint received over D-Bus. `tests/interop/linux/bundles.sh <target>` also checks
the visible launcher and icon in the built DEB/AppImage, the hidden DEB desktop
alias and its content digest in the RPM file manifest. It rejects an AppImage
whose visible launcher is replaced by the hidden alias.

The isolated checks also cover missing D-Bus services, a stalled notification
daemon, idempotent credential deletion and terminal audio-server loss.

The Linux audio patch waits up to three seconds for native connection/stream
acceptance. Streams retain the connection that owns their callbacks. Terminal
server/stream failures are observed on the SIP event loop, which ends active
tests/calls and reports a native failure instead of claiming capture continues.
After a device/server failure, failed audio owners must be released before
the SIP loop acknowledges recovery from a ready backend. Subsequent tests/calls
can then start without restarting Ruf; the existing reconnect timer remains
bounded. Capture workers never destroy or join themselves, and teardown does not
wait indefinitely to flush playback.

Existing portable/native Rust tests, Clippy, frontend/UI checks and generated DTO
verification remain required. The Linux desktop workflow builds the SDK before
native checks and preserves separate Asterisk interoperability coverage. Build
hash manifests and compiled SBOMs accompany all three installer formats.

## Observed results

Ubuntu 22.04.5/GNOME and Fedora 44/KDE ARM64 UTM sessions passed real keyring
persistence, hidden-window incoming calls/actions, tray behavior, all three
login/autostart modes and package removal/reinstallation. Real Mac microphone
signal and user-confirmed speech echo passed through both guests. Virtual
endpoint removal/recreation, fixed routing and audio-server recovery also passed.
Fedora ran Ubuntu-built ARM64 native executables against its actual services.

The [desktop acceptance](verification/2026-11-16-linux-acceptance.md),
[audio recovery record](verification/2026-11-16-linux-audio-recovery.md) and
[full lab run](verification/2026-11-16-linux-lab.md) retain source/package bindings
and limitations. The newer [CI #57 verification](verification/2026-11-16-ci-platform-fixes.md)
passed Linux x64 native/service tests, all three packages and isolated startup.
Physical headset unplug/replug and route verification remain open in
[#17](https://github.com/mardu-systems/ruf/issues/17); VM and virtual-device
results do not qualify that physical-device criterion.

## Manual release acceptance

Before declaring Linux supported, record observed results for:

- Installation/start/uninstall on the intended Ubuntu version and architecture.
- Wayland and X11; GNOME/KDE window focus, settings drafts, app relaunch, tray
  absence, close/background operation and explicit quit during a call.
- Real microphone/speaker/headset selection, ringing, mute, hold, transfer,
  unplug/replug, default-device changes and audio-server interruption.
- Locked/missing credential service, rejected unlock, restart/restore, credential
  replacement and deletion-journal retry without an unexpected background prompt.
- Notification actions with Ruf hidden, stale banners after call end, daemon
  failure/restart and caller-details privacy settings.
- UDP/TCP/TLS registration, SRTP requirements, actual two-way RTP, network changes,
  and the intended provider/PBX. Hosted builds and synthetic audio do not qualify
  hardware, OS permission state or real provider behavior.
