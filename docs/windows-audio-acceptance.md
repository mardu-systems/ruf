# Windows 11 x64 physical audio acceptance

This is an execution checklist, not evidence that Windows audio works. Every row
below is **not tested** until a tester records results on physical Windows 11 x64
hardware using the installed native Ruf build. A portable build, browser IPC
fixture, Asterisk lab, Windows Server CI build or successful SIP registration
does not replace these observations. Windows remains experimental until this
acceptance is recorded for the exact release under test.

## Run record

Copy this document into a private test record and fill every field before testing.
Use `pass`, `fail` or `not tested` for each row; for unavailable hardware retain
`not tested` and explain the gap. A failed row needs reproduction steps and an
issue reference. A passing row needs its observations and sanitized evidence ID.

| Field          | Exact value to record                                                                                          |
| -------------- | -------------------------------------------------------------------------------------------------------------- |
| Run            | Date/time, tester alias, run ID                                                                                |
| Ruf            | App version, commit SHA, installer filename and SHA-256, native `linked` status                                |
| Packaging      | NSIS installer, install scope, clean install or upgrade, signing result                                        |
| OS             | Windows 11 edition, version and full build number, x64 architecture, update level                              |
| Machine        | Manufacturer/model, CPU, audio controller; omit serial numbers                                                 |
| Runtime        | Installed WebView2 version; Rust/Bun/native SDK versions from build provenance                                 |
| Native SDK     | Exact baresip/libre/OpenSSL source versions, patch revision and built architecture                             |
| Internal audio | Input/output model, driver provider/version/date, selected sample rate                                         |
| USB            | Headset model, driver version, direct port or dock/hub and its model                                           |
| Bluetooth      | Headset model/firmware, adapter and driver version, profile used during calls                                  |
| Routing        | Selected input/call/ring devices; Windows default and communications roles for each direction                  |
| Privacy        | Global microphone access and desktop-app access enabled/disabled for each scenario                             |
| PBX/peer       | Exact server/peer version, test transport, codec, RTP/SRTP mode, sanitized network topology                    |
| Evidence       | Private artifact location, sanitized log/screenshot IDs, issue references; no credentials or call/contact data |

Record observations on both endpoints with a consenting test peer or a controlled
echo/IVR service. Use fictional identities. Do not publish endpoint IDs, raw SIP/SDP,
real numbers, network addresses, contact data, passwords or unreviewed screenshots.
Preserve timing and error codes in sanitized evidence without sensitive values.

## Routing and permission contract

- **System default** is the Windows **default communications device**
  (`eCommunications`) independently for capture, call playback and ringtone
  playback. It is not the separate multimedia default role. Configure the roles
  in Windows sound settings and deliberately make the roles different when
  testing. Ringtone output also uses the communications role when left at default.
- Explicitly selected device IDs persist across app restarts. Removing a device
  must not silently select another explicit device. Replug the same endpoint or
  select another device deliberately. Driver reinstallation may change IDs;
  reselect and save the replacement if the saved device is missing.
- Windows `get_microphone_permission` currently returns `unsupported`: its
  permission query is unimplemented. This is neither denied authorization nor
  proof that capture is unavailable. Opening Privacy reads metadata only; it
  must not prompt or record. The OS settings action opens the fixed
  `ms-settings:privacy-microphone` target. Check microphone access and desktop-app
  access there, then explicitly run a microphone test or place a test call.
- Local speaker/microphone tests and ringtone previews use the existing native
  audio pipeline. A moving local level alone does not prove that a remote peer
  receives microphone audio; verify both directions on a real call.

See [native audio diagnostics](api/audio-diagnostics.md) for command inputs,
ownership, busy cases, startup failure and terminal stream-error behavior. OS
settings opening may fail with `nativeFailure`; opening is an action, not a
successful permission query.

## Hardware coverage

Run the applicable scenarios below for each configuration. Keep a separate
result/evidence column in the execution record for each hardware configuration.

| Configuration | Input / call output / ring output                                               | Result     | Evidence |
| ------------- | ------------------------------------------------------------------------------- | ---------- | -------- |
| H1            | Internal microphone / internal speakers / internal speakers, explicit selection | not tested | —        |
| H2            | USB headset / USB headset / internal speakers, explicit selection               | not tested | —        |
| H3            | Bluetooth headset / Bluetooth headset / internal speakers, explicit selection   | not tested | —        |
| H4            | All three System default; communications devices differ from multimedia devices | not tested | —        |
| H5            | Mixed default and explicit routes; swap the explicit and default assignments    | not tested | —        |

## Scenario matrix

For removal/reconnection rows, exercise USB and Bluetooth independently. Test
input loss, call-output loss and ring-output loss separately so a working route
cannot mask the failed route. Note the observed interruption/recovery duration,
error state and actual route after every transition.

| ID  | Procedure                                                                                                                                            | Required observation                                                                                                                                                                                                   | Result     | Evidence |
| --- | ---------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------- | -------- |
| W01 | Clean install; launch without accounts. Open Privacy and Sound.                                                                                      | Empty account state; device enumeration and explicit local tests work independently of registration. Native `linked` alone is not presented as media success.                                                          | not tested | —        |
| W02 | Open Privacy with Windows permission query unsupported; follow OS settings action.                                                                   | Honest unavailable-query label and desktop-app guidance; fixed microphone privacy page opens. No automatic prompt/capture. A manual microphone test is allowed to try capture.                                         | not tested | —        |
| W03 | Disable global microphone access, then separately desktop-app access. Start microphone test and outgoing/incoming calls; re-enable access and retry. | Real capture denial produces visible failure, never false recording success; no crash or endless retry. Speaker test remains independent. Recovery uses a fresh explicit test/call.                                    | not tested | —        |
| W04 | Disable/disconnect all capture devices, then playback devices; enumerate and start tests/calls.                                                      | Missing/failed endpoints are visible. Failed stream startup returns an error and never advertises a successfully running test. Ring/call errors do not freeze controls.                                                | not tested | —        |
| W05 | On H1/H2/H3, speak in a microphone test and run speaker test; stop each.                                                                             | Level responds to speech and falls in silence; tone uses selected call output; stop/navigation releases owned test. No leftover recording/tone.                                                                        | not tested | —        |
| W06 | On H2/H3, preview ringtone and receive a call with separate explicit call/ring outputs.                                                              | Ring sounds only on ring output at chosen volume; answered call uses call output; answering/rejecting stops ring. Both peers confirm microphone route.                                                                 | not tested | —        |
| W07 | Save explicit H1/H2/H3 routes, restart Ruf and reboot Windows.                                                                                       | Same available endpoints restore without silently changing selections. Missing endpoint remains identifiable as missing.                                                                                               | not tested | —        |
| W08 | Remove/replug explicit endpoint while idle; refresh devices. Repeat with driver reinstall on a disposable test device.                               | Saved selection survives transient absence. Same ID returns on ordinary replug where the driver preserves it. Changed driver IDs require deliberate reselection; no arbitrary fallback.                                | not tested | —        |
| W09 | Remove/replug input during microphone test and output during speaker test. Repeat several times.                                                     | Native error/recovery is bounded, UI remains responsive, no stale active-test success after terminal failure. Retry uses same explicit endpoint or current communications default as selected. No callback after stop. | not tested | —        |
| W10 | Remove/replug ring endpoint during preview and during an unanswered incoming call.                                                                   | No crash/hang; ring stops or recovers coherently on the intended route. Answer/reject remains available and ends all ring playback; later incoming calls work.                                                         | not tested | —        |
| W11 | Remove/replug input and call output separately during an established call.                                                                           | Remote peer and local tester record actual interruption/recovery. Bounded recovery uses intended routing; terminal errors remain visible, hangup works, subsequent call works. No false claim of successful media.     | not tested | —        |
| W12 | With all routes default, change communications capture/playback roles while idle, testing, ringing and in a call. Keep multimedia default unchanged. | New default streams and active default streams follow the current communications endpoint after bounded reopen. Capture, call and ring are verified independently; log interruption duration.                          | not tested | —        |
| W13 | Change multimedia defaults while communications roles stay fixed; then change communications roles with explicit Ruf selections.                     | Multimedia-only changes do not reroute default streams. Explicit selections stay on their saved endpoints through OS role changes.                                                                                     | not tested | —        |
| W14 | Provide silence, speech and silence again; use low, normal and zero OS input volume/mute. Repeat with call and output volume/mute.                   | Silence does not replay stale samples, saturate meter or generate bursts. Volumes/mutes behave as set; remote peer confirms genuine silence and restored speech.                                                       | not tested | —        |
| W15 | Make outgoing and incoming calls for each H1–H5 route; answer, decline and hang up from each side.                                                   | Both directions are independently audible, state ends on both peers, no ring/tone/capture left behind.                                                                                                                 | not tested | —        |
| W16 | During incoming and outgoing calls, mute/unmute and hold/resume repeatedly.                                                                          | Peer confirms muted microphone and restored speech; hold/resume restores correct route and two-way audio without stale mute state.                                                                                     | not tested | —        |
| W17 | Call a controlled IVR; send digits, star and hash. Exercise direct and attended transfers, including consultation cancel.                            | IVR receives digits; successful transfer connects intended test peers; cancel restores original call audio. Record PBX-specific limits as failures or gaps.                                                            | not tested | —        |
| W18 | Keep a two-way call active for at least 60 minutes; alternate speech/silence, mute and hold.                                                         | No progressive dropout, stale-buffer bursts, resource leak or unbounded CPU use; record duration and observed memory/CPU before/after. Subsequent call works.                                                          | not tested | —        |
| W19 | Sleep/wake idle and in a call; disconnect/reconnect network and switch networks separately.                                                          | No crash/hang or stale media-success state; restoration or failure is visible, user can hang up/reconnect, subsequent registration and two-way call work. Record which original calls survive.                         | not tested | —        |
| W20 | Disable an active device in Windows; exercise available device-busy/driver failure conditions; stop immediately during recovery.                     | Startup/runtime backend errors reach test/call state, retry count stays bounded, stopped streams are never resurrected, controls stay usable and later explicit retry works. Preserve sanitized error code/timing.     | not tested | —        |
| W21 | Install the actual EXE, close to tray, reopen, receive a call while hidden and quit via tray.                                                        | One instance, correct tray state, usable incoming-call actions and restored window; quit terminates media and process. No duplicate ringtone/capture.                                                                  | not tested | —        |
| W22 | Enable autostart/start hidden, sign out/in or reboot; repeat with explicit opt-out.                                                                  | Enabled app starts once with intended visibility and saved routes; disabled app does not start. No unexpected capture at login.                                                                                        | not tested | —        |
| W23 | Enable/disable app and OS notifications; receive calls while hidden, foreground and notifications denied.                                            | Delivery and actions match settings, lack of delivery is not reported as permission success, ringing remains correct and no duplicate alerts appear. Verify installed identity and exact Windows version.              | not tested | —        |

## Decision record

Overall result: **not tested**. Physical Windows execution and artifacts have not
been supplied with this checklist. Record all failed and untested row IDs, affected
hardware, release-blocking decisions and retest links before changing that result.
Keep Windows experimental when required physical coverage is incomplete. Automated
checks belong in a separate evidence section with the exact command, environment
and observed result; they do not turn any hardware row into a pass.
