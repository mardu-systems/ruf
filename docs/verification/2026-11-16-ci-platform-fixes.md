# PR CI platform corrections — 16 November 2026

Observed [CI #57](https://github.com/mardu-systems/ruf/actions/runs/43870420990),
attempt 1, completed successfully at source
`cf5102f0108caeab6ce1f7b0be78e5b1e1c0f8b2`. All seven jobs passed. The run
started at 12:51:55 UTC and completed at 13:05:43 UTC: **13:48 elapsed**.
Job durations below use GitHub's job start/completion timestamps, including
setup and post-job work. Jobs run concurrently; their durations are not summed.

| Job                                      | Result | Duration |
| ---------------------------------------- | ------ | -------- |
| Linux x64 desktop                        | Passed | 13:28    |
| macOS ARM64 desktop                      | Passed | 12:23    |
| Windows x64 desktop                      | Passed | 11:24    |
| Asterisk SIP and RTP echo                | Passed | 3:50     |
| German/English browser state regressions | Passed | 3:05     |
| Portable checks                          | Passed | 0:52     |
| Desktop target selection                 | Passed | 0:12     |

## Corrections included

- Native test groups previously invoked Cargo concurrently. Repeated workspace
  rebuilds could replace a test executable before another partition launched
  it. The runner now reads Cargo JSON artifacts from one build and launches
  those executables directly from their package directories. NAT and timed SIP
  cases retain separate processes; at most six test processes run together.
  Doc tests run through Cargo afterward, including after a partition failure.
- Older Apple `lipo` consumes every argument after `-verify_arch` as an
  architecture. OpenSSL inspection now supplies the archive first and preserves
  CPU/deployment rejection, with inspection errors included in diagnostics.
- Browser permission fixtures now select their modeled OS independently of the
  host. An explicit Linux notification-capability scenario supplements the
  existing macOS permission and Windows audio cases.
- Windows artifact tests now compare native paths. Malicious newline metadata is
  tested on every OS; physical newline filenames are additionally tested on Unix,
  since Windows forbids creating them.

Regression tests cover complete executable/integration selection, one compilation,
bounded concurrency, immediate output, child cleanup, retained failure logs,
missing native discovery, package working directories and doc-test failure.

## Observed gates and limits

Every selected native desktop job passed SDK construction, native Clippy/tests,
network/call-action checks, package generation, inventory/hash generation and
artifact upload. Linux additionally passed isolated desktop services, all three
package checks and actual window/startup observation under Xvfb. macOS passed
service-timeout tests and signed microphone/Contacts bundle checks. Windows
executed its native tests and produced its NSIS installer on Windows Server 2022.

The separate Asterisk job passed all 17 selected SIP/RTP scenarios and 14
TLS/SRTP cases, plus the restricted diagnostic check. Browser checks passed in
German and English, including multiwindow localization. Portable checks covered
formatting, workflows, versions, generated contracts, frontend checks/build,
UI/tooling tests and portable Rust checks.

This run included SDK rebuilds; macOS OpenSSL alone took 5:03. No warm-cache
benchmark is inferred. Only the three-target PR matrix ran: Intel macOS, Linux
ARM64 and Windows ARM64 are not qualified by this run. Main/manual/release
workflows continue to select all six targets. No release was published by CI #57.

Synthetic media, headless service tests and signed-bundle metadata checks do not
qualify physical microphones/headsets, permission prompts or provider behavior.
The separate [Linux VM/audio acceptance](2026-11-16-linux-audio-recovery.md)
records real speech through UTM and virtual endpoint recovery; physical Linux
device unplug/replug remains open in
[#17](https://github.com/mardu-systems/ruf/issues/17).
