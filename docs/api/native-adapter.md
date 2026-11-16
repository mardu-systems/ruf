# Private native adapter ABI

`crates/baresip-adapter/native/bridge.h` is an internal, synchronously called C
boundary, compiled together with Rust. It is not desktop IPC or a network API.
`protocol.def` defines operation/event codes once; the C header consumes it and
Cargo generates private Rust constants in OUT_DIR. Existing public Rust methods,
IPC commands, error classifications and generated TypeScript DTOs are unchanged,
including `configure_account`.

`sip_engine_request(handle, request)` is the single request entry point. Both
arguments are required. The named `sip_request` fields in bridge.h are mirrored
by repr(C) NativeRequest in Rust. Unused numeric fields are zero; unused string
fields are null. There is no per-family payload object or string-encoded flag.

Strings are borrowed NUL-terminated UTF-8. Before enqueueing, C copies the request
and all populated strings into one owned allocation. Null optional strings mean
empty/default; no pointer is retained from Rust. C wipes the copied password
before freeing that allocation on execution, enqueue failure or shutdown. Callback
strings remain borrowed only for the synchronous callback; Rust copies them.

| Operation                                           | Required fields                                               | Optional fields and meaning                                                                                                                                                                                                                                                              |
| --------------------------------------------------- | ------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Account create/replace, enable                      | operation, account_id, registrar, username, password          | auth_username defaults to username; outbound_proxy defaults to registrar; monitored_extensions is a bounded newline-separated numeric list. Transport is 0 UDP, 1 TCP or 2 TLS; media_security is 0 disabled or 1 required; enabled is 0/1. Disable needs only operation and account_id. |
| Dial, answer, hangup, reject, remove, call controls | operation, owning call_generation/account_id where applicable | Destination is required for dial, transfer, consultation and DTMF. Enabled is the numeric 0/1 mute/hold flag. Unused identity fields are zero.                                                                                                                                           |
| Sound settings                                      | OP_SOUND or OP_SOUND_RESTORE                                  | input_device, output_device and ring_device default when null/empty. Enabled is 0/1, ringtone_variant is 0–2 and ringtone_volume is 0–100. Restore retains saved routing without requiring currently present hardware.                                                                   |
| Audio diagnostic or preview                         | operation; call_generation for test start/stop                | device defaults when null/empty. Preview and preview-stop use generation zero.                                                                                                                                                                                                           |
| Stop                                                | OP_STOP                                                       | Flags shutdown under the existing queue mutex before copying, allocating or enqueueing; queue saturation cannot prevent stop.                                                                                                                                                            |

The safe Engine API performs existing account, device, number, call-token and
state validation before these private calls. Native execution rechecks call
generations, current call ownership, pending controls and inventory. Account
commands retain dedicated upstream authentication setters and TLS/SRTP policy;
there is no credential-bearing account URI or second audio path.

All calls return zero for acceptance or an errno for synchronous rejection:
`EINVAL` for an unknown operation, invalid boolean flag, missing required account/destination fields or invalid monitored targets,
`ENOMEM` for copying/allocation failure, `ENOTCONN` for stopped/stopping engines,
and `EBUSY` at the existing sixteen-request queue bound. Sound variant/volume,
inventory and call/account state are checked on the native thread and reported
through existing completion/error events. Queue acceptance does not establish a
call, prove registration or acknowledge protocol success. No HTTP status exists.

Account/audio completion waits release the state mutex through a condition
variable. Native callback completion wakes waiters after releasing that mutex,
including rejection and engine shutdown. Results already delivered before a wait
are consumed immediately. There is deliberately no abandonment timeout that
could let persistent metadata diverge from a delayed native operation.

Live dialog state is stored once, per CallSnapshot; selected-call compatibility
fields are derived when producing an owned EngineSnapshot. Pending outbound
reservations and retained terminal events preserve immediate guarded hangup and
fast call start/end observations. Background events never temporarily select a
dialog, and an old generation cannot update a replacement dialog.
