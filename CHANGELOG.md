# What's new in Ruf?

## 0.2.0 · Preview

- Play a short local tone when the last connected call has ended.
- Stream history searches with a transient contact index, backport the GLib string-iterator soundness fix and build Windows OpenSSL from a pinned maintained LTS recipe. Reject stale Windows TLS SDKs and document correct Rustup invocation for macOS stripping.
- Refresh Rust, Tauri, SQLite, localization, frontend tooling and pinned CI/lab dependencies. Keep incompatible major upgrades deferred; the dependency review records remaining upstream maintenance and OpenSSL support gaps.
- Build the native Linux desktop on GNU x64 or ARM64, with DEB, RPM and AppImage packages.
- Use Linux Secret Service for encrypted account passwords, desktop notifications with answer/reject actions, tray/background operation and installed-app autostart.
- Keep Linux microphone, conversation and ringtone outputs separate through PulseAudio or pipewire-pulse. Fixed routes stay on the selected device; new tests and calls recover after failed owners are released and the device or audio server returns.
- Monitor up to 32 extensions per account using native SIP BLF. Show unknown, idle, ringing and busy states; the PBX must support dialog subscriptions and authorize them.
- Run native macOS builds on both Apple Silicon and Intel with the macOS 14 minimum. Add an experimental native Windows ARM64 build target, excluded from release-draft assets.
- Improve Windows native audio startup errors, communications-default routing and recovery after device invalidation. Add account-free microphone and speaker diagnostics.
- Use a more compact phone layout, simpler first-run setup and stronger window, keyboard and call-control recovery.
- Update the native baresip/libre engine to 4.13.0 and improve call-state synchronization and shutdown handling.
- Simplify the macOS drag-to-Applications DMG background while retaining its Retina artwork.
- Keep PR checks to Linux x64, macOS ARM64 and Windows x64, with all six native targets for main, manual runs and releases. Keep package, native calling and security checks enabled across the selected platforms.
- Confirm real microphone signal and audible speech echo through Ubuntu/GNOME and Fedora/KDE ARM64 VMs. Physical Linux headset removal/reconnection and Windows hardware acceptance remain open.

## 0.1.1 · Preview

- Free Developer Preview with German and English phone/settings UI, native menus and app-owned notifications. No in-app purchases.
- Fix platform-specific menu compilation and keep synthetic localization polling consistent across desktop targets.
- Add current UI media, a first-call tester guide and a compatibility-report form for controlled SIP testing.

## 0.1.0 · Preview

- Search Ruf with ⌘K on Mac or Ctrl+K on Windows for actions, authorized contacts and past calls. Results open existing details; a call starts only through its explicit call action. Keyboard shortcuts and native menus provide direct access to call controls.
- Dial formatted numbers with spaces and a leading plus, and use contact suggestions as you type.
- Show the in-call keypad for digits, star and hash; tones are sent natively through baresip.
- Use current contact names for outgoing and incoming calls, notifications and older history entries, with optional German national and international number matching.

- Mute and hold calls natively, with separate indicators for remote hold and pending changes.
- Transfer directly or start a consultation, switch between both calls, return to the original party and complete the transfer. Success follows native confirmation; incoming REFER instructions remain rejected.
- Save UDP/TCP/TLS transport, required SDES-SRTP with TLS, a separate authentication username and an outbound proxy per account. Server verification uses operating-system trust without automatic plaintext fallback. Displayed call security follows native observations and does not guarantee end-to-end encryption.

- Manage up to eight telephone accounts, enable or disable each independently, and select the outgoing account.
- Use eight settings sections with field search, keyboard navigation and protection against losing unsaved changes.
- Reconnect saved accounts after a pending Keychain authorization using the reconnect action.
- Select microphone, call speaker and ringtone output separately; choose among three ringtones and adjust ringtone volume.
- Test the microphone with a short level meter and the speaker with a two-second tone. Incoming calls stop active audio tests.
- Check notification, microphone and contacts permissions, then explicitly request access or open system settings as needed.
- Answer or reject incoming calls in the background through system notifications, with optional caller-detail hiding.
- Request a neutral test notification after five seconds. Focus and system settings can still suppress its display.
- Start Ruf at login with or without a visible phone window; manually opening the app keeps the window visible.
- Search and call macOS contacts after permission, and configure favorite ordering and company display.
- View past calls with their outcomes and duration, and call back through the original account.
- Disable saving future calls to history or remove completed entries after 30, 90 or 365 days; unlimited retention remains the default.
- Distinguish caller information, call states and SIP errors more precisely. Unsupported transfer requests are explicitly rejected.
- View and explicitly copy a diagnostic report without personal account, contact or caller data.
- Migrate existing settings while backing up the original metadata, with improved long account names, clear account actions and search focus.
- First free Developer Preview, with German/English UI and native presentation, a preview tester guide and a dedicated compatibility-report form. No in-app purchases.
