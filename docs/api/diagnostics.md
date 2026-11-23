# Anonymous diagnostics

Settings-only Tauri IPC; no HTTP endpoint/status codes. `get_diagnostics` takes no
arguments and returns `DiagnosticsReport`. `copy_diagnostics` takes no arguments,
reads a fresh report and copies compact one-line JSON through the existing native
clipboard bridge. It never accepts a user-supplied report, URL, file or path, writes
a report file, or uploads anything.

All fields are required; nullable readings remain null when unavailable:

| Field                    | Type                            | Meaning                                                                                                                                                                  |
| ------------------------ | ------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `version`                | string                          | Compiled application version.                                                                                                                                            |
| `platform`               | string                          | Static OS family, never hostname or user identity.                                                                                                                       |
| `telephony`              | `linked` / `notLinked`          | Compiled linkage only; no registration/media claim.                                                                                                                      |
| `accountsTotal`          | number / null                   | In-memory configured-account count; null before restoration, on contention or shutdown.                                                                                  |
| `accountsRegistered`     | number / null                   | Count with native `registered` state; does not establish inbound reachability.                                                                                           |
| `callActive`             | boolean / null                  | Native non-idle call slot, without peer or call token.                                                                                                                   |
| `notificationPermission` | `NotificationPermission` / null | Independent read-only OS authorization; null on read failure.                                                                                                            |
| `microphonePermission`   | `MicrophonePermission` / null   | Read-only authorization metadata; never opens capture.                                                                                                                   |
| `audioDevicesAvailable`  | boolean / null                  | True if both input/output inventory contain endpoints, false if either is empty, null on query failure. No capture, playback or physical audibility is established.      |
| `credentialError`        | `ErrorCode` / null              | Only a reported `credentialAccessRequired`, `credentialMissing` or `credentialFailure`. No credential probe; null means no reported failure, not proven keychain access. |
| `historyAvailable`       | boolean                         | No reported local history storage error.                                                                                                                                 |
| `lastError`              | `ErrorCode` / null              | Safe classification only, prioritizing snapshot/account, history, notification, microphone then inventory errors. No raw error text or protocol payload.                 |

Account/call reads use a nonblocking cached snapshot and never restore accounts,
load metadata, read passwords, request permission or hold a phone lock during OS
queries. Contact records are not read. Permission/inventory failures are isolated:
the report still returns other known readings. Worker failure returns
`nativeFailure`. This is a local observation, not a network/reachability test.

The report explicitly selects these twelve fields. It excludes credentials,
account names, usernames, registrar/SIP addresses, phone numbers, caller/contact
records, call IDs, device IDs/names, filesystem paths and raw native error text.
Copy uses the same whitelist as preview. Native plain text accepts 1–1024 UTF-8
bytes without controls; the report is compact JSON within this bound. Invalid text
would return `invalidInput`; native pasteboard failures return `nativeFailure`.
The native clipboard fallback uses the existing macOS implementation or the
GTK main loop on Linux desktop builds. Linux dispatch has a three-second
deadline. Unsupported platforms, including Windows, return `engineUnavailable`. Linux isolated service
tests exercise actual GTK clipboard dispatch; this is separate from user-initiated
copying in an installed desktop. See [Linux clipboard](contacts.md#linux-clipboard-and-contacts).

The settings UI displays when the report was retrieved. While refreshing or after
a failed refresh, it labels retained data as the last successful observation.
The copy action uses the preview's whitelisted JSON through the WebView Clipboard
API. If that fails for the current report, `copy_diagnostics` retrieves and copies
a fresh report, and the UI identifies the copy as fresh. Copying a retained
report uses WebView Clipboard only, since the native command always retrieves
fresh data. Preview and loading leave the clipboard untouched.
