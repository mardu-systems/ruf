# Live extension status (SIP BLF)

BLF uses the existing account commands (`save_account` / `configure_account`),
`get_engine_snapshot` and the `phone-state-changed` event. No new IPC command,
remote HTTP service or frontend SIP connection is introduced. Commands retain
`PhoneError` results rather than HTTP status codes. Calls still use the existing
`dial` command and its account/registration/call guards.

## Account configuration

`AccountConfig.monitoredExtensions` is optional input, defaulting to `[]` for
legacy callers. It accepts at most 32 objects per account:

| Field       | Input                         | Meaning                                                                                                                          |
| ----------- | ----------------------------- | -------------------------------------------------------------------------------------------------------------------------------- |
| `extension` | Required string               | Exact target, 1–32 ASCII digits. Leading zeros are significant. SIP URIs, separators, whitespace and service codes are rejected. |
| `name`      | Optional string, default `""` | Display label, at most 64 Unicode characters without control characters. Empty uses the extension number.                        |

Targets must be unique within their account. The same target on two accounts
represents two independent subscriptions. Unknown object fields are rejected.
Invalid targets, duplicates, excessive counts or invalid labels return
`invalidInput` before credentials are accessed. Existing account-save errors,
including `busy`, credential access failures, `storageFailure`, `invalidState`,
`engineUnavailable` and `nativeFailure`, retain their existing meanings.
Saving still replaces the account through the existing acknowledged native queue;
it is blocked during calls. Queue acceptance or successful SIP registration is
not evidence of BLF availability.

Configuration is saved in the existing password-free `accounts.json` metadata,
under the stable saved account identity, and restored on startup, reconnect,
activation and credential recovery. Missing fields default to no monitoring.
An empty target list is omitted from metadata, preserving the previous empty
configuration format. Observed state and notification XML are never persisted.
Older application versions without BLF cannot read account records containing
this new field; remove monitoring with a BLF-capable version before downgrading.

## Snapshot

Every `AccountSnapshot` has a required `monitoredExtensions` array. Each entry
contains required `extension`, `name` and `state` fields; configuration order is
preserved. `state` is one of:

| State     | Meaning                                                                                                                                                                     |
| --------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `unknown` | No coherent current notification, pending/failed subscription, unsupported/malformed body, uncertain dialog state, account disabled/replaced, or expired/lost subscription. |
| `idle`    | A coherent dialog document has no remaining live dialogs. This does not prove registration, reachability or willingness to accept a call.                                   |
| `ringing` | At least one `early` dialog and no `confirmed` dialog.                                                                                                                      |
| `busy`    | At least one `confirmed` dialog, including a held call. Takes precedence over ringing.                                                                                      |

`trying`/`proceeding` dialogs yield unknown unless another dialog proves busy or
ringing. A `terminated` dialog is removed. Lack of ordinary state changes alone
does not make an otherwise valid subscription stale. Failure of one target does
not fail account registration or erase other target states. A target with no PBX
hint or insufficient subscription rights remains unknown.

## Native subscriptions and recovery

The adapter reuses `uag_sipevent_sock()`, `sipevent_subscribe()`, `account_auth()`,
account identity, configured transport and outbound proxy. It sends `Event:
dialog` and `Accept: application/dialog-info+xml`. libre handles digest challenges
and routine expiry renewal. The application starts monitoring after native
registration success and applies bounded retries after failure; Retry-After is
honored. Initial NOTIFY has a ten-second watchdog; active NOTIFY expiry governs
freshness. Network transport recovery invalidates subscriptions and registration
success starts them again. Account retirement and shutdown cancel owned timers
and subscriptions before releasing native ownership.

A separate synchronous private BLF callback copies bounded notification data into
Rust; native pointers never escape. XML is limited to 64 KiB, nesting to 32,
dialogs to 128, dialog IDs to 256 bytes and attribute values to 1024 bytes. DTDs,
unknown entities, invalid namespaces, duplicate dialog IDs/states and malformed
XML are rejected. No raw XML, credentials or peer identities enter diagnostic
logs. `quick-xml`, already present in the lockfile, is used directly by the adapter.

RFC 4235 full documents replace the dialog table; partial documents update IDs
without erasing unmentioned dialogs. The first accepted document must be full.
Old/duplicate versions are ignored. A gap in partial versions or incoherent XML
invalidates state and schedules a fresh subscription outside the active NOTIFY
callback. Subscription renewal, resynchronization and missing-data handling do
not start another media pipeline or consume a call slot.

## UI and verification

Account settings edit the same guarded account draft. The selected account's
extension list appears in the phone's dialer independently of macOS Contacts and
contact authorization. Status has text as well as color; target buttons follow
the existing dialing guard. German and English are supported.

The isolated Asterisk fixture adds real PJSIP endpoint hints. The native interop
suite covers idle → ringing → busy → idle during an actual call, missing hints,
disable/enable recovery and target removal. Run `bun run test:asterisk`; it does
not qualify arbitrary providers, Windows hardware or hosted releases. XML reducer
and loopback protocol tests additionally cover full/partial coherence and recovery.

Local results and qualification limits are recorded in the
[15 November verification record](../verification/2026-11-15-blf.md).
