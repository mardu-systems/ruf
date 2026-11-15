# Private native adapter ABI

`crates/baresip-adapter/native/bridge.h` is an internal, synchronously called C
boundary, compiled together with Rust. It is not desktop IPC or a network API.
`protocol.def` defines operation/event codes once; the C header consumes it and
Cargo generates private Rust constants in OUT_DIR. Existing public Rust methods,
IPC commands, error classifications and generated TypeScript DTOs are unchanged,
including `configure_account`.

Every request requires a live engine handle. Strings are borrowed NUL-terminated
UTF-8, copied into queue-owned memory before return; null optional strings mean
empty. No pointer is retained from Rust. C wipes the owned account password before
freeing its request on execution, rejection, enqueue failure or shutdown. Callback
strings remain borrowed only for the synchronous callback; Rust copies them.

| Entry point          | Required inputs                                                                                                      | Optional inputs and meaning                                                                                                                                                                                                                                                                                |
| -------------------- | -------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `sip_engine_account` | operation (`OP_ACCOUNT`, `OP_ENABLE`, `OP_DISABLE`), account ID; config for account creation/replacement or enabling | Only disable accepts null config. Config has registrar, username, password, optional auth username/proxy/monitored numeric extensions, transport (0 UDP, 1 TCP, 2 TLS), media policy (0 disabled, 1 required), enabled (0/1). Null optional strings are empty. Extensions retain bounded newline encoding. |
| `sip_engine_command` | operation (dial, answer, hangup, reject, remove or call control), account ID, generation, enabled (0/1)              | Destination is required for dial, transfer, consultation and DTMF; otherwise null/empty. Enabled is the typed mute/hold flag, not a destination string. Generation identifies the owning call; unused account/generation fields are zero.                                                                  |
| `sip_engine_sound`   | restore (zero validates current inventory, nonzero restores saved routing), settings                                 | Empty/null input, output or ring device selects default. Settings carry enabled (0/1), ringtone variant (0–2) and volume (0–100) as numeric fields.                                                                                                                                                        |
| `sip_engine_audio`   | preview/test operation, generation for test start/stop                                                               | Null/empty device selects default; preview and preview-stop use generation zero.                                                                                                                                                                                                                           |
| `sip_engine_stop`    | engine handle                                                                                                        | No payload; flags shutdown under the existing queue mutex without allocating or enqueueing.                                                                                                                                                                                                                |

The safe Engine API performs existing account, device, number, call-token and
state validation before these private calls. Native execution rechecks call
generations, current call ownership, pending controls and inventory. Account
commands retain dedicated upstream authentication setters and TLS/SRTP policy;
there is no credential-bearing account URI or second audio path.

All calls return zero for acceptance or an errno for synchronous rejection:
`EINVAL` for the wrong operation family or invalid structural payload,
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
