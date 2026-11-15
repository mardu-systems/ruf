//! Private unsafe boundary: callback ownership outlives the joined C event loop.

use crate::engine::EngineState;
use phone_core::{
    AccountConfig, AudioDevice, AudioDevices, AudioMode, AudioTestKind, AudioTestSnapshot,
    CallDirection, CallEvent, CallEventKind, CallState, CallerIdentity, ErrorCode, PhoneError,
    RegistrationState, SoundSettings,
};
use std::{
    ffi::{CStr, CString, c_void},
    panic::{AssertUnwindSafe, catch_unwind},
    path::PathBuf,
    ptr::NonNull,
    sync::{
        Arc, Condvar, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread::{self, JoinHandle},
};

static ENGINE_LEASE: AtomicBool = AtomicBool::new(false);

use crate::protocol::*;

const SECURITY_CERTIFICATE: i32 = 20001;
const SECURITY_TRANSPORT: i32 = 20002;
const SECURITY_MEDIA: i32 = 20003;
const CALL_EVENT_CAPACITY: usize = 64;

/// Private ABI mirrored by bridge.c; every pointer is borrowed for the callback only.
#[repr(C)]
#[derive(Clone, Copy)]
struct NativeEvent {
    kind: i32,
    code: i32,
    account_id: u32,
    call_generation: u64,
    caller_name: *const std::ffi::c_char,
    caller_number: *const std::ffi::c_char,
    detail: *const std::ffi::c_char,
}

// Mirrored by native/bridge.h; all pointers are borrowed until the request returns.
#[repr(C)]
struct NativeAccountConfig {
    registrar: *const std::ffi::c_char,
    username: *const std::ffi::c_char,
    password: *const std::ffi::c_char,
    auth_username: *const std::ffi::c_char,
    outbound_proxy: *const std::ffi::c_char,
    monitored_extensions: *const std::ffi::c_char,
    transport: i32,
    media_security: i32,
    enabled: i32,
}
#[repr(C)]
struct NativeSoundSettings {
    input_device: *const std::ffi::c_char,
    output_device: *const std::ffi::c_char,
    ring_device: *const std::ffi::c_char,
    ringtone_enabled: i32,
    ringtone_variant: i32,
    ringtone_volume: u32,
}

unsafe extern "C" {
    fn sip_engine_set_blf_callback(
        handle: *mut c_void,
        callback: extern "C" fn(*const NativeEvent, *mut c_void) -> i32,
    );
    fn sip_engine_create(
        callback: extern "C" fn(*const NativeEvent, *mut c_void),
        arg: *mut c_void,
        audio_mode: i32,
        recording: *const std::ffi::c_char,
    ) -> *mut c_void;
    #[cfg(feature = "native-test-hooks")]
    fn sip_engine_set_test_ca(handle: *mut c_void, path: *const std::ffi::c_char) -> i32;
    #[cfg(feature = "native-test-hooks")]
    fn sip_engine_set_test_dns(handle: *mut c_void, ip: *const std::ffi::c_char, port: u16) -> i32;
    fn sip_engine_run(handle: *mut c_void) -> i32;
    fn sip_engine_reserve_call_generation(handle: *mut c_void) -> u64;
    fn sip_engine_account(
        handle: *mut c_void,
        operation: i32,
        account_id: u32,
        config: *const NativeAccountConfig,
    ) -> i32;
    fn sip_engine_command(
        handle: *mut c_void,
        operation: i32,
        account_id: u32,
        generation: u64,
        destination: *const std::ffi::c_char,
        enabled: i32,
    ) -> i32;
    fn sip_engine_sound(
        handle: *mut c_void,
        restore: i32,
        settings: *const NativeSoundSettings,
    ) -> i32;
    fn sip_engine_audio(
        handle: *mut c_void,
        operation: i32,
        generation: u64,
        device: *const std::ffi::c_char,
    ) -> i32;
    fn sip_engine_stop(handle: *mut c_void) -> i32;
    fn sip_audio_devices(
        callback: extern "C" fn(
            i32,
            *const std::ffi::c_char,
            *const std::ffi::c_char,
            i32,
            *mut c_void,
        ),
        arg: *mut c_void,
    ) -> i32;
    fn sip_engine_destroy(handle: *mut c_void);
}

#[derive(Clone, Copy)]
struct Handle(NonNull<c_void>);
// The C request queue synchronizes cross-thread access. Only the event-loop thread
// calls run; destroy is called only after join. No direct UA/call access is exposed.
unsafe impl Send for Handle {}
impl Handle {
    fn run(self) -> i32 {
        unsafe { sip_engine_run(self.0.as_ptr()) }
    }
}

struct CallbackContext {
    snapshot: Arc<Mutex<EngineState>>,
    changed: Arc<Condvar>,
    ready: mpsc::Sender<Result<(), PhoneError>>,
}

// Called synchronously on the native event loop; return 1 requests a deferred full resync.
extern "C" fn blf_callback(event: *const NativeEvent, arg: *mut c_void) -> i32 {
    catch_unwind(AssertUnwindSafe(|| {
        if event.is_null() || arg.is_null() {
            return 1;
        }
        // The C subscription owns all strings for this invocation; copy/parse before returning.
        let event = unsafe { &*event };
        if event.caller_number.is_null() {
            return 1;
        }
        let Ok(extension) = (unsafe { CStr::from_ptr(event.caller_number) }).to_str() else {
            return 1;
        };
        let context = unsafe { &*arg.cast::<CallbackContext>() };
        let Ok(mut engine) = context.snapshot.lock() else {
            return 1;
        };
        let Some(account_index) = engine
            .snapshot
            .accounts
            .iter()
            .position(|a| a.id == event.account_id && a.enabled && !a.removing)
        else {
            return 0;
        };
        let Some(target_index) = engine.snapshot.accounts[account_index]
            .monitored_extensions
            .iter()
            .position(|t| t.extension == extension)
        else {
            return 0;
        };
        let key = (event.account_id, extension.to_owned());
        let result = if event.kind == EV_BLF_UNKNOWN {
            engine.blf_dialogs.entry(key.clone()).or_default().reset();
            Ok(phone_core::ExtensionState::Unknown)
        } else if event.kind == EV_BLF_NOTIFY && !event.detail.is_null() {
            match (unsafe { CStr::from_ptr(event.detail) }).to_str() {
                Ok(xml) => engine
                    .blf_dialogs
                    .entry(key.clone())
                    .or_default()
                    .apply(xml),
                Err(_) => Err(()),
            }
        } else {
            Err(())
        };
        let resync = result.is_err();
        engine.snapshot.accounts[account_index].monitored_extensions[target_index].state =
            result.unwrap_or_default();
        if resync {
            engine.blf_dialogs.remove(&key);
        }
        i32::from(resync)
    }))
    .unwrap_or(1)
}

// Notify after the state guard is released, including early returns and caught unwinding.
struct NotifyOnDrop<'a>(&'a Condvar);
impl Drop for NotifyOnDrop<'_> {
    fn drop(&mut self) {
        self.0.notify_all();
    }
}

extern "C" fn callback(event: *const NativeEvent, arg: *mut c_void) {
    // arg points to the owned box held until the engine thread has joined.
    let _ = catch_unwind(AssertUnwindSafe(|| {
        if event.is_null() || arg.is_null() {
            return;
        }
        let NativeEvent {
            kind,
            code,
            account_id,
            call_generation,
            caller_name,
            caller_number,
            detail,
        } = unsafe { *event };
        let context = unsafe { &*arg.cast::<CallbackContext>() };
        let _notify = NotifyOnDrop(&context.changed);
        if kind == EV_READY {
            let _ = context.ready.send(if code == 0 {
                Ok(())
            } else {
                Err(native_error(code))
            });
            return;
        }
        if let Ok(mut engine) = context.snapshot.lock() {
            let engine = &mut *engine;
            if (EV_TEST_STARTED..=EV_TEST_FAILED).contains(&kind) {
                let id = call_generation.to_string();
                match kind {
                    EV_TEST_STARTED => {
                        engine.snapshot.audio_test = Some(AudioTestSnapshot {
                            id,
                            kind: if account_id == 0 {
                                AudioTestKind::Microphone
                            } else {
                                AudioTestKind::Speaker
                            },
                            level: 0.0,
                        })
                    }
                    EV_TEST_LEVEL => {
                        if let Some(test) = engine
                            .snapshot
                            .audio_test
                            .as_mut()
                            .filter(|test| test.id == id)
                        {
                            test.level = (code.clamp(0, 1000) as f32) / 1000.0;
                        }
                    }
                    EV_TEST_STOPPED => {
                        if engine
                            .snapshot
                            .audio_test
                            .as_ref()
                            .is_some_and(|test| test.id == id)
                        {
                            engine.snapshot.audio_test = None;
                        }
                    }
                    EV_TEST_FAILED => engine.snapshot.audio_test_error = Some(native_error(code)),
                    _ => {}
                }
                return;
            }
            if kind == EV_SECURITY {
                if let Some(call) = engine.snapshot.calls.iter_mut().find(|call| {
                    call.id == call_generation.to_string() && call.account_id == account_id
                }) {
                    call.security = phone_core::CallSecurity {
                        signaling_transport: match code & 3 {
                            EV_READY => Some(phone_core::SipTransport::Udp),
                            EV_REGISTERING => Some(phone_core::SipTransport::Tcp),
                            EV_REGISTERED => Some(phone_core::SipTransport::Tls),
                            _ => None,
                        },
                        tls_peer_verified: code & 4 != 0,
                        media: match (code >> 3) & 3 {
                            EV_READY => phone_core::MediaProtection::Unprotected,
                            EV_REGISTERING => phone_core::MediaProtection::SdesSrtp,
                            _ => phone_core::MediaProtection::Unknown,
                        },
                    };
                }
                return;
            }
            let owns_dialog = engine
                .snapshot
                .calls
                .iter()
                .any(|call| call.id == call_generation.to_string());
            if kind == EV_ERROR && !owns_dialog {
                let error = native_error(code);
                engine.engine_error = Some(error.clone());
                engine.snapshot.last_error = Some(error);
                return;
            }
            if kind == EV_CALL_ERROR && !owns_dialog {
                return;
            }
            // Journal events run synchronously before any UI snapshot projection.
            match kind {
                EV_INCOMING | EV_BUSY_INCOMING | EV_DIAL_STARTED => {
                    // Public metadata is optimistic while an update is queued.
                    // Offers still belong to the existing native UA until its
                    // replacement is acknowledged; it may reject that update.
                    let account = match engine.pending_accounts.get(&account_id) {
                        Some(previous) => previous.clone(),
                        None => engine
                            .snapshot
                            .accounts
                            .iter()
                            .find(|a| a.id == account_id)
                            .cloned(),
                    };
                    let number = owned_caller_text(caller_number, 80)
                        .filter(|value| caller_number_is_display_safe(value));
                    let name = (kind != EV_DIAL_STARTED)
                        .then(|| owned_caller_text(caller_name, 128))
                        .flatten();
                    if kind == EV_DIAL_STARTED {
                        engine.snapshot.last_call_failure = None;
                    }
                    if kind == EV_DIAL_STARTED
                        && let Some(number) = &number
                    {
                        engine
                            .call_destinations
                            .insert(call_generation, number.clone());
                    }
                    engine.call_journal.start(
                        call_generation,
                        account,
                        if kind == EV_DIAL_STARTED {
                            CallDirection::Outgoing
                        } else {
                            CallDirection::Incoming
                        },
                        number,
                        name,
                    );
                    if kind == EV_BUSY_INCOMING {
                        engine.call_journal.busy(call_generation);
                    }
                }
                EV_ACTIVE => engine.call_journal.connected(call_generation),
                EV_CLOSED => engine.call_journal.finish(call_generation, code, false),
                EV_DIAL_FAILED | EV_ANSWER_CLOSED => {
                    engine.call_journal.finish(call_generation, code, true)
                }
                EV_LOCAL_END => engine.call_journal.local_end(call_generation),
                EV_STOPPED | EV_INTERRUPTING => engine.call_journal.interrupt(),
                _ => {}
            }
            if kind == EV_DIAL_FAILED
                && engine.call_destinations.remove(&call_generation).is_some()
                && code != 0
            {
                engine.snapshot.last_call_failure = Some(phone_core::CallFailure {
                    call_id: call_generation.to_string(),
                    account_id,
                    error: native_error(code),
                });
            }
            if kind == EV_LOCAL_END {
                engine.local_endings.insert(call_generation);
            }
            if kind == EV_INTERRUPTING {
                let generations: Vec<u64> = engine
                    .snapshot
                    .calls
                    .iter()
                    .filter_map(|call| call.id.parse().ok())
                    .collect();
                engine.local_endings.extend(generations);
            }
            if kind == EV_BUSY_INCOMING || kind == EV_LOCAL_END || kind == EV_INTERRUPTING {
                return;
            }
            if (matches!(
                kind,
                EV_RINGING | EV_ACTIVE | EV_CLOSED | EV_ANSWER_FAILED | EV_ANSWER_CLOSED
            ) || (EV_PROGRESS..=EV_REDIRECT).contains(&kind))
                && engine.call_generation != call_generation
                && !engine
                    .snapshot
                    .calls
                    .iter()
                    .any(|call| call.id == call_generation.to_string())
            {
                return;
            }
            if kind == EV_DIAL_STARTED {
                if matches!(engine.call_state(), CallState::Idle | CallState::Calling) {
                    let mut call =
                        crate::dialogs::new_dialog(call_generation, account_id, CallState::Calling);
                    call.destination = engine.call_destinations.get(&call_generation).cloned();
                    engine.pending_dial = Some(call);
                    engine.call_generation = call_generation;
                }
                return;
            }
            if kind == EV_DIAL_FAILED && engine.call_generation != call_generation && !owns_dialog {
                if engine
                    .pending_dial
                    .as_ref()
                    .is_some_and(|call| call.id == call_generation.to_string())
                {
                    engine.pending_dial = None;
                }
                return;
            }
            if kind == EV_AUDIO_APPLIED || kind == EV_AUDIO_FAILED {
                engine.sound_result = Some(if kind == EV_AUDIO_APPLIED {
                    Ok(())
                } else {
                    Err(native_error(code))
                });
                return;
            }
            if kind == EV_ACCOUNT_APPLIED || kind == EV_ACCOUNT_REJECTED {
                let previous = engine.pending_accounts.remove(&account_id);
                let result = if kind == EV_ACCOUNT_APPLIED {
                    if let Some(account) = engine
                        .snapshot
                        .accounts
                        .iter_mut()
                        .find(|account| account.id == account_id)
                    {
                        account.enabled_changing = false;
                    }
                    engine.snapshot.sync_selected_registration();
                    Ok(())
                } else {
                    if let Some(previous) = previous {
                        engine.snapshot.accounts.retain(|a| a.id != account_id);
                        if let Some(previous) = previous {
                            engine.snapshot.accounts.push(previous);
                            engine.snapshot.accounts.sort_by_key(|a| a.id);
                        }
                        if engine.snapshot.selected_account_id == Some(account_id)
                            && !engine.snapshot.accounts.iter().any(|a| a.id == account_id)
                        {
                            engine.snapshot.selected_account_id =
                                engine.snapshot.accounts.first().map(|a| a.id);
                        }
                        engine.snapshot.sync_selected_registration();
                    }
                    Err(PhoneError::new(
                        ErrorCode::Busy,
                        "Native account configuration was rejected; the previous account is unchanged.",
                    ))
                };
                engine.account_results.insert(account_id, result);
                return;
            }
            if kind == EV_STOPPED {
                engine.stopped = true;
            }
            if (EV_SELECTED..=EV_CONSULT_FAILED).contains(&kind) {
                let id = call_generation.to_string();
                if kind == EV_SELECTED {
                    engine.select_dialog(&id);
                    return;
                }
                // A remote BYE may end the INVITE usage before the REFER result.
                // Preserve that last call's outcome without selecting/resurrecting it
                // or attributing an old result to a replacement conversation.
                if kind == EV_TRANSFER_RESULT
                    && !owns_dialog
                    && engine.call_generation == call_generation
                    && engine.snapshot.calls.is_empty()
                {
                    let state = &mut engine.snapshot;
                    push_call_event(
                        &mut state.call_events,
                        &mut state.call_events_dropped,
                        id,
                        if (200..300).contains(&code) {
                            CallEventKind::TransferSucceeded
                        } else {
                            CallEventKind::TransferFailed
                        },
                    );
                    return;
                }
                let mut terminal_failure = None;
                if let Some(call) = engine.snapshot.calls.iter_mut().find(|call| call.id == id) {
                    match kind {
                        EV_MUTED => call.muted = code != 0,
                        EV_LOCAL_HOLD_PENDING => call.hold_pending = true,
                        EV_LOCAL_HOLD_DONE => {
                            call.hold_pending = false;
                            call.local_held = code != 0;
                        }
                        EV_CONTROL_FAILED => {
                            call.hold_pending = false;
                            call.last_error = Some(
                                u16::try_from(code)
                                    .ok()
                                    .and_then(PhoneError::from_sip_status)
                                    .unwrap_or_else(|| {
                                        PhoneError::new(
                                            ErrorCode::NativeFailure,
                                            "Call control failed or timed out.",
                                        )
                                    }),
                            );
                        }
                        EV_CONSULT_FAILED => {
                            let mut error = u16::try_from(code)
                                .ok()
                                .and_then(PhoneError::from_sip_status)
                                .unwrap_or_else(|| {
                                    PhoneError::new(
                                        ErrorCode::NativeFailure,
                                        "The consultation could not be started.",
                                    )
                                });
                            error.message = format!(
                                "Consultation failed: {} The original call is retained.",
                                error.message
                            );
                            call.last_error = Some(error);
                        }
                        EV_RECOVERING => {
                            call.hold_pending = true;
                            call.last_error = Some(PhoneError::new(
                                ErrorCode::NativeFailure,
                                "The SIP hold change failed; confirmed session recovery is required.",
                            ));
                        }
                        EV_RECOVERY_FAILED => {
                            call.hold_pending = false;
                            let mut error = u16::try_from(code)
                                .ok()
                                .and_then(PhoneError::from_sip_status)
                                .unwrap_or_else(|| PhoneError::new(ErrorCode::NativeFailure, ""));
                            error.message =
                                "The SIP hold recovery failed; the uncertain call was ended."
                                    .into();
                            terminal_failure = Some(error.clone());
                            call.last_error = Some(error);
                        }
                        EV_CONTROL_REJECTED => {
                            call.last_error = Some(PhoneError::new(
                                ErrorCode::Busy,
                                "A call control is still pending.",
                            ))
                        }
                        EV_TRANSFER_PENDING => call.transfer = phone_core::TransferState::Pending,
                        EV_TRANSFER_RESULT => {
                            call.transfer = if (200..300).contains(&code) {
                                phone_core::TransferState::Succeeded
                            } else {
                                // A snapshot may be read before the following
                                // legacy failure event; publish its error now.
                                call.last_error = Some(transfer_error(code));
                                phone_core::TransferState::Failed
                            };
                        }
                        _ => {}
                    }
                    let event = match kind {
                        EV_MUTED => Some(CallEventKind::MuteChanged { muted: code != 0 }),
                        EV_LOCAL_HOLD_DONE => {
                            Some(CallEventKind::LocalHoldChanged { held: code != 0 })
                        }
                        EV_CONTROL_FAILED | EV_RECOVERY_FAILED | EV_CONSULT_FAILED => {
                            Some(CallEventKind::ControlFailed)
                        }
                        EV_TRANSFER_PENDING => Some(CallEventKind::TransferStarted),
                        EV_TRANSFER_RESULT if (200..300).contains(&code) => {
                            Some(CallEventKind::TransferSucceeded)
                        }
                        _ => None,
                    };
                    if let Some(event) = event {
                        push_call_event(
                            &mut call.events,
                            &mut call.events_dropped,
                            call.id.clone(),
                            event,
                        );
                    }
                }
                if let Some(error) = terminal_failure {
                    engine.terminal_call_errors.insert(call_generation, error);
                }
                if engine.call_generation == call_generation {
                    engine.snapshot.last_error = None;
                }
                return;
            }
            if kind == EV_CALLING || kind == EV_INCOMING {
                engine.engine_error = None;
                engine.snapshot.last_call_failure = None;
                engine.start_dialog(
                    call_generation,
                    account_id,
                    if kind == EV_CALLING {
                        CallState::Calling
                    } else {
                        CallState::Incoming
                    },
                );
            }
            if kind == EV_CALL_ERROR {
                engine
                    .terminal_call_errors
                    .insert(call_generation, native_error(code));
            }
            if matches!(kind, EV_CALLING..=EV_ERROR | EV_ANSWER_FAILED | EV_DIAL_FAILED | EV_PROGRESS..=EV_ANSWER_CLOSED | EV_CALL_ERROR)
            {
                engine.snapshot.last_error = None;
                let id = call_generation.to_string();
                let local_ending = engine.local_endings.contains(&call_generation);
                let mut terminal_error =
                    if matches!(kind, EV_CLOSED | EV_DIAL_FAILED | EV_ANSWER_CLOSED) {
                        engine.terminal_call_errors.remove(&call_generation)
                    } else {
                        None
                    };
                let call = engine
                    .snapshot
                    .calls
                    .iter_mut()
                    .find(|call| call.id == id)
                    .or_else(|| engine.pending_dial.as_mut().filter(|call| call.id == id));
                if let Some(call) = call {
                    match kind {
                        EV_CALLING..=EV_ACTIVE => {
                            call.state = match kind {
                                EV_CALLING => CallState::Calling,
                                6 if call.state == CallState::EarlyMedia => CallState::EarlyMedia,
                                EV_RINGING => CallState::Ringing,
                                EV_INCOMING => CallState::Incoming,
                                _ if call.remote_held => CallState::Held,
                                _ => CallState::Active,
                            };
                            if kind == EV_ACTIVE && call.connected_at_ms.is_none() {
                                call.connected_at_ms = std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .ok()
                                    .and_then(|duration| u64::try_from(duration.as_millis()).ok());
                            }
                            if kind == EV_INCOMING {
                                call.caller = Some(CallerIdentity {
                                    name: owned_caller_text(caller_name, 128),
                                    number: owned_caller_text(caller_number, 80)
                                        .filter(|value| caller_number_is_display_safe(value)),
                                });
                            } else if kind == EV_CALLING {
                                call.caller = None;
                            }
                        }
                        EV_CLOSED | EV_DIAL_FAILED | EV_ANSWER_CLOSED => {
                            let outgoing_setup = matches!(
                                call.state,
                                CallState::Calling | CallState::Ringing | CallState::EarlyMedia
                            );
                            let closing_error = if kind == EV_ANSWER_CLOSED && !local_ending {
                                Some(PhoneError::new(
                                    ErrorCode::NativeFailure,
                                    "The incoming call ended before establishment after its answer was sent.",
                                ))
                            } else if kind == EV_DIAL_FAILED && code != 0 {
                                Some(native_error(code))
                            } else if !local_ending
                                && let Ok(status) = u16::try_from(code)
                                && let Some(error) = PhoneError::from_sip_status(status)
                            {
                                Some(error)
                            } else if !local_ending && code >= 700 {
                                Some(native_error(code))
                            } else if !local_ending
                                && code == 0
                                && outgoing_setup
                                && terminal_error.is_none()
                            {
                                Some(PhoneError::new(
                                    ErrorCode::NativeFailure,
                                    "The outgoing call ended before establishment without a final SIP response.",
                                ))
                            } else {
                                None
                            };
                            if closing_error.is_some() {
                                terminal_error = closing_error;
                            }
                            if let Some(error) = &terminal_error {
                                call.last_error = Some(error.clone());
                            }
                        }
                        EV_ERROR | EV_CALL_ERROR => call.last_error = Some(native_error(code)),
                        EV_ANSWER_FAILED => {
                            if call.state == CallState::Answering {
                                call.state = CallState::Incoming;
                            }
                            call.last_error = Some(native_error(code));
                        }
                        EV_PROGRESS => {
                            if matches!(
                                call.state,
                                CallState::Calling | CallState::Ringing | CallState::EarlyMedia
                            ) {
                                call.state = CallState::EarlyMedia;
                            }
                            record_call_event(call, CallEventKind::Progress);
                        }
                        EV_HOLD | EV_RESUME => {
                            call.remote_held = kind == EV_HOLD;
                            if matches!(call.state, CallState::Active | CallState::Held) {
                                call.state = if call.remote_held {
                                    CallState::Held
                                } else {
                                    CallState::Active
                                };
                            }
                            record_call_event(
                                call,
                                if call.remote_held {
                                    CallEventKind::Held
                                } else {
                                    CallEventKind::Resumed
                                },
                            );
                        }
                        EV_DTMF_START => {
                            if !detail.is_null() {
                                // The borrowed native digit buffer contains two bytes including NUL.
                                let digit = unsafe { CStr::from_ptr(detail) }.to_bytes();
                                if digit.len() == 1 && b"0123456789*#ABCD".contains(&digit[0]) {
                                    record_call_event(
                                        call,
                                        CallEventKind::DtmfStart {
                                            digit: char::from(digit[0]).to_string(),
                                        },
                                    );
                                }
                            }
                        }
                        EV_DTMF_END => record_call_event(call, CallEventKind::DtmfEnd),
                        EV_TRANSFER_FAILED => {
                            record_call_event(call, CallEventKind::TransferFailed);
                            call.last_error = Some(transfer_error(code));
                        }
                        EV_REDIRECT => {
                            if let Ok(status @ 300..=399) = u16::try_from(code) {
                                record_call_event(
                                    call,
                                    CallEventKind::Redirected { sip_status: status },
                                );
                                call.last_error = PhoneError::from_sip_status(status);
                            }
                        }
                        _ => {}
                    }
                    if let Some(error) = terminal_error {
                        engine.snapshot.last_call_failure = Some(phone_core::CallFailure {
                            call_id: id,
                            account_id: call.account_id,
                            error,
                        });
                    }
                }
                if matches!(kind, EV_CLOSED | EV_DIAL_FAILED | EV_ANSWER_CLOSED) {
                    engine.close_dialog(call_generation);
                }
                return;
            }
            match kind {
                EV_REGISTERING..=EV_REG_FAILED => {
                    if let Some(account) = engine
                        .snapshot
                        .accounts
                        .iter_mut()
                        .find(|a| a.id == account_id && a.enabled)
                    {
                        if kind == EV_REGISTERED {
                            account.registered_at_ms = std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .ok()
                                .and_then(|duration| u64::try_from(duration.as_millis()).ok());
                        }
                        account.registration = match kind {
                            EV_REGISTERING => RegistrationState::Registering,
                            EV_REGISTERED => RegistrationState::Registered,
                            _ => RegistrationState::Failed,
                        };
                        account.last_error = (kind == EV_REG_FAILED).then(|| {
                            if let Ok(status) = u16::try_from(code)
                                && let Some(error) = PhoneError::from_sip_status(status)
                            {
                                return error;
                            }
                            if (SECURITY_CERTIFICATE..=SECURITY_MEDIA).contains(&code) {
                                native_error(code)
                            } else {
                                PhoneError::new(
                                    ErrorCode::NativeFailure,
                                    "Registration failed; verify the registrar and credentials.",
                                )
                            }
                        });
                    }
                    engine.snapshot.sync_selected_registration();
                }
                EV_STOPPED => {
                    engine.blf_dialogs.clear();
                    engine.snapshot.calls.clear();
                    engine.pending_dial = None;
                    engine.snapshot.audio_test = None;
                    engine.snapshot.ringtone_playing = false;
                    engine.snapshot.registration = RegistrationState::Unconfigured;
                    for account in &mut engine.snapshot.accounts {
                        account.registration = RegistrationState::Unconfigured;
                        for target in &mut account.monitored_extensions {
                            target.state = phone_core::ExtensionState::Unknown;
                        }
                    }
                    if code != 0 {
                        engine.snapshot.last_error = Some(native_error(code));
                    }
                }
                EV_STALE => {
                    let error = PhoneError::new(
                        ErrorCode::InvalidState,
                        "The call changed before this action executed.",
                    );
                    engine.snapshot.last_error = Some(error);
                }

                EV_REMOVED => {
                    engine.blf_dialogs.retain(|(id, _), _| *id != account_id);
                    engine.snapshot.accounts.retain(|a| a.id != account_id);
                    if engine.snapshot.selected_account_id == Some(account_id) {
                        engine.snapshot.selected_account_id =
                            engine.snapshot.accounts.first().map(|a| a.id);
                    }
                    engine.snapshot.sync_selected_registration();
                }
                EV_REMOVE_FAILED => {
                    if let Some(account) = engine
                        .snapshot
                        .accounts
                        .iter_mut()
                        .find(|a| a.id == account_id)
                    {
                        account.removing = false;
                    }
                    engine.snapshot.last_error = Some(PhoneError::new(
                        ErrorCode::Busy,
                        "An incoming call prevented account removal.",
                    ));
                    engine.snapshot.sync_selected_registration();
                }
                EV_RING_STARTED => engine.snapshot.ringtone_playing = true,
                EV_RING_STOPPED => engine.snapshot.ringtone_playing = false,
                _ => {}
            }
        }
    }));
}

fn record_call_event(call: &mut phone_core::CallSnapshot, event: CallEventKind) {
    push_call_event(
        &mut call.events,
        &mut call.events_dropped,
        call.id.clone(),
        event,
    );
}

fn push_call_event(
    events: &mut Vec<CallEvent>,
    dropped: &mut u32,
    call_id: String,
    event: CallEventKind,
) {
    let sequence = events
        .last()
        .map_or(1, |last| last.sequence.wrapping_add(1));
    if events.len() == CALL_EVENT_CAPACITY {
        events.remove(0);
        *dropped = dropped.saturating_add(1);
    }
    events.push(CallEvent {
        sequence,
        call_id,
        event,
    });
}

// C supplies NUL-terminated arrays bounded to limit + 1 and borrows them only
// during this synchronous callback. Copy before returning; no pointer escapes.
fn owned_caller_text(value: *const std::ffi::c_char, limit: usize) -> Option<String> {
    if value.is_null() {
        return None;
    }
    let bytes = unsafe { std::slice::from_raw_parts(value.cast::<u8>(), limit + 1) };
    let end = bytes.iter().position(|byte| *byte == 0)?;
    sanitize_caller_text(std::str::from_utf8(&bytes[..end]).ok()?, limit)
}

fn caller_number_is_display_safe(value: &str) -> bool {
    value
        .chars()
        .all(|c| c.is_alphanumeric() || "_.+-*#".contains(c))
}

fn sanitize_caller_text(value: &str, limit: usize) -> Option<String> {
    if value.chars().any(|c| c.is_control() || matches!(c,
        '\u{061c}' | '\u{200e}' | '\u{200f}' | '\u{2028}' | '\u{2029}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}'
    )) {
        return None;
    }
    let value = value.trim();
    let mut end = value.len().min(limit);
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    (!value[..end].is_empty()).then(|| value[..end].to_owned())
}

pub(crate) struct NativeEngine {
    handle: Option<Handle>,
    thread: Option<JoinHandle<i32>>,
    context: Option<NonNull<CallbackContext>>,
}
// Moving ownership does not change the boxed callback address. Public Engine
// operations require exclusive access; the event loop accesses only synchronized state.
unsafe impl Send for NativeEngine {}

impl NativeEngine {
    pub(crate) fn start(
        snapshot: Arc<Mutex<EngineState>>,
        changed: Arc<Condvar>,
        audio: AudioMode,
        recording: Option<PathBuf>,
        test_ca: Option<PathBuf>,
        test_dns: Option<std::net::SocketAddr>,
    ) -> Result<Self, PhoneError> {
        let recording = recording.as_deref().and_then(|p| p.to_str()).unwrap_or("");
        if matches!(audio, AudioMode::TestTone) && (recording.is_empty() || recording.len() > 127) {
            return Err(PhoneError::new(
                ErrorCode::InvalidInput,
                "Test audio requires a UTF-8 recording path of at most 127 bytes.",
            ));
        }
        let recording = cstring(recording)?;
        #[cfg(feature = "native-test-hooks")]
        let test_ca = test_ca
            .map(|path| {
                let path = path
                    .to_str()
                    .filter(|path| !path.is_empty() && path.len() < 1024)
                    .ok_or_else(|| {
                        PhoneError::new(
                            ErrorCode::InvalidInput,
                            "Test trust path must fit 1023 UTF-8 bytes.",
                        )
                    })?;
                cstring(path)
            })
            .transpose()?;
        #[cfg(feature = "native-test-hooks")]
        let test_dns = test_dns
            .map(|dns| {
                if !dns.ip().is_loopback() || dns.port() == 0 {
                    return Err(PhoneError::new(
                        ErrorCode::InvalidInput,
                        "Test DNS must be a nonzero loopback endpoint.",
                    ));
                }
                Ok((cstring(&dns.ip().to_string())?, dns.port()))
            })
            .transpose()?;
        if ENGINE_LEASE.swap(true, Ordering::AcqRel) {
            return Err(PhoneError::new(
                ErrorCode::Busy,
                "Only one native engine can run in this process.",
            ));
        }
        let (ready, receiver) = mpsc::channel();
        let context = NonNull::new(Box::into_raw(Box::new(CallbackContext {
            snapshot,
            changed,
            ready,
        })))
        .expect("Box is non-null");
        let raw = unsafe {
            sip_engine_create(
                callback,
                context.as_ptr().cast(),
                i32::from(matches!(audio, AudioMode::TestTone)),
                recording.as_ptr(),
            )
        };
        let Some(pointer) = NonNull::new(raw) else {
            unsafe {
                drop(Box::from_raw(context.as_ptr()));
            }
            ENGINE_LEASE.store(false, Ordering::Release);
            return Err(native_error(-1));
        };
        unsafe {
            sip_engine_set_blf_callback(raw, blf_callback);
        }
        #[cfg(feature = "native-test-hooks")]
        if let Some(path) = test_ca {
            let code = unsafe { sip_engine_set_test_ca(raw, path.as_ptr()) };
            if code != 0 {
                unsafe {
                    sip_engine_destroy(raw);
                    drop(Box::from_raw(context.as_ptr()));
                }
                ENGINE_LEASE.store(false, Ordering::Release);
                return Err(native_error(code));
            }
        }
        #[cfg(feature = "native-test-hooks")]
        if let Some((ip, port)) = test_dns {
            let code = unsafe { sip_engine_set_test_dns(raw, ip.as_ptr(), port) };
            if code != 0 {
                unsafe {
                    sip_engine_destroy(raw);
                    drop(Box::from_raw(context.as_ptr()));
                }
                ENGINE_LEASE.store(false, Ordering::Release);
                return Err(native_error(code));
            }
        }
        #[cfg(not(feature = "native-test-hooks"))]
        debug_assert!(test_ca.is_none() && test_dns.is_none());
        let handle = Handle(pointer);
        let thread = match thread::Builder::new()
            .name("baresip-event-loop".into())
            .spawn(move || handle.run())
        {
            Ok(thread) => thread,
            Err(_) => {
                unsafe {
                    sip_engine_destroy(raw);
                    drop(Box::from_raw(context.as_ptr()));
                }
                ENGINE_LEASE.store(false, Ordering::Release);
                return Err(native_error(-1));
            }
        };
        let engine = Self {
            handle: Some(handle),
            thread: Some(thread),
            context: Some(context),
        };
        receiver.recv().map_err(|_| native_error(-1))??;
        Ok(engine)
    }

    pub(crate) fn reserve_call_generation(&self) -> Result<u64, PhoneError> {
        let handle = self
            .handle
            .ok_or_else(|| PhoneError::new(ErrorCode::EngineUnavailable, "Engine is stopped."))?;
        // The same native queue lock serializes reservations and incoming tokens.
        Ok(unsafe { sip_engine_reserve_call_generation(handle.0.as_ptr()) })
    }

    fn handle(&self) -> Result<*mut c_void, PhoneError> {
        self.handle
            .map(|handle| handle.0.as_ptr())
            .ok_or_else(|| PhoneError::new(ErrorCode::EngineUnavailable, "Engine is stopped."))
    }

    pub(crate) fn request(
        &self,
        operation: i32,
        account_id: u32,
        destination: Option<&str>,
        generation: u64,
        enabled: bool,
    ) -> Result<(), PhoneError> {
        let handle = self.handle()?;
        let destination = cstring(destination.unwrap_or(""))?;
        // C synchronously copies the destination into its owned bounded queue.
        native_result(unsafe {
            sip_engine_command(
                handle,
                operation,
                account_id,
                generation,
                destination.as_ptr(),
                i32::from(enabled),
            )
        })
    }

    pub(crate) fn account_request(
        &self,
        operation: i32,
        id: u32,
        config: Option<&AccountConfig>,
        enabled: bool,
    ) -> Result<(), PhoneError> {
        let handle = self.handle()?;
        let Some(config) = config else {
            // Only disabling an existing native identity accepts no configuration.
            return native_result(unsafe {
                sip_engine_account(handle, operation, id, std::ptr::null())
            });
        };
        let registrar = cstring(&config.registrar)?;
        let username = cstring(&config.username)?;
        let password = cstring(&config.password)?;
        let auth_username = cstring(config.auth_username.as_deref().unwrap_or(""))?;
        let outbound_proxy = cstring(config.outbound_proxy.as_deref().unwrap_or(""))?;
        let monitored_extensions = cstring(
            &config
                .monitored_extensions
                .iter()
                .map(|target| target.extension.as_str())
                .collect::<Vec<_>>()
                .join("\n"),
        )?;
        let native = NativeAccountConfig {
            registrar: registrar.as_ptr(),
            username: username.as_ptr(),
            password: password.as_ptr(),
            auth_username: auth_username.as_ptr(),
            outbound_proxy: outbound_proxy.as_ptr(),
            monitored_extensions: monitored_extensions.as_ptr(),
            transport: match config.transport {
                phone_core::SipTransport::Udp => 0,
                phone_core::SipTransport::Tcp => 1,
                phone_core::SipTransport::Tls => 2,
            },
            media_security: i32::from(
                config.media_security == phone_core::MediaSecurityPolicy::Required,
            ),
            enabled: i32::from(enabled),
        };
        // C copies every field before returning; no pointer escapes this stack frame.
        native_result(unsafe { sip_engine_account(handle, operation, id, &native) })
    }

    pub(crate) fn sound_request(
        &self,
        restore: bool,
        settings: &SoundSettings,
    ) -> Result<(), PhoneError> {
        let handle = self.handle()?;
        let input = cstring(&settings.input_device)?;
        let output = cstring(&settings.output_device)?;
        let ring = cstring(&settings.ring_device)?;
        let native = NativeSoundSettings {
            input_device: input.as_ptr(),
            output_device: output.as_ptr(),
            ring_device: ring.as_ptr(),
            ringtone_enabled: i32::from(settings.ringtone_enabled),
            ringtone_variant: match settings.ringtone {
                phone_core::Ringtone::Classic => 0,
                phone_core::Ringtone::Soft => 1,
                phone_core::Ringtone::Short => 2,
            },
            ringtone_volume: u32::from(settings.ringtone_volume),
        };
        native_result(unsafe { sip_engine_sound(handle, i32::from(restore), &native) })
    }

    pub(crate) fn audio_test_request(
        &self,
        operation: i32,
        id: u64,
        device: &str,
    ) -> Result<(), PhoneError> {
        let handle = self.handle()?;
        let device = cstring(device)?;
        native_result(unsafe { sip_engine_audio(handle, operation, id, device.as_ptr()) })
    }

    pub(crate) fn preview_request(&self, operation: i32, device: &str) -> Result<(), PhoneError> {
        self.audio_test_request(operation, 0, device)
    }

    pub(crate) fn shutdown(&mut self) -> Result<(), PhoneError> {
        if self.handle.is_none() {
            return Ok(());
        }
        let request_result = native_result(unsafe { sip_engine_stop(self.handle()?.cast()) });
        let result = self
            .thread
            .take()
            .map(|t| t.join())
            .transpose()
            .map_err(|_| native_error(-1));
        if let Some(handle) = self.handle.take() {
            unsafe {
                sip_engine_destroy(handle.0.as_ptr());
            }
        }
        if let Some(context) = self.context.take() {
            unsafe {
                drop(Box::from_raw(context.as_ptr()));
            }
        }
        ENGINE_LEASE.store(false, Ordering::Release);
        match result? {
            Some(code) if code != 0 => Err(native_error(code)),
            _ => request_result,
        }
    }
}

impl Drop for NativeEngine {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}

fn native_result(code: i32) -> Result<(), PhoneError> {
    if code == 0 {
        Ok(())
    } else {
        Err(native_error(code))
    }
}

fn cstring(input: &str) -> Result<CString, PhoneError> {
    CString::new(input)
        .map_err(|_| PhoneError::new(ErrorCode::InvalidInput, "NUL bytes are not allowed."))
}
fn transfer_error(code: i32) -> PhoneError {
    u16::try_from(code)
        .ok()
        .and_then(PhoneError::from_sip_status)
        .unwrap_or_else(|| {
            PhoneError::new(
                ErrorCode::NativeFailure,
                "Call transfer failed without a SIP response.",
            )
        })
}

fn native_error(code: i32) -> PhoneError {
    let security = match code {
        SECURITY_CERTIFICATE => Some((
            ErrorCode::CertificateRejected,
            "The TLS certificate chain or server hostname was rejected.",
        )),
        SECURITY_TRANSPORT => Some((
            ErrorCode::TransportFailure,
            "The configured SIP transport failed; no plaintext fallback was attempted.",
        )),
        SECURITY_MEDIA => Some((
            ErrorCode::MediaSecurityRequired,
            "The peer did not negotiate valid mandatory SDES-SRTP; plaintext media was blocked.",
        )),
        _ => None,
    };
    if let Some((code, message)) = security {
        return PhoneError::new(code, message);
    }
    PhoneError::new(
        ErrorCode::NativeFailure,
        format!("Native operation failed (code {code})."),
    )
}

extern "C" fn device_callback(
    input: i32,
    id: *const std::ffi::c_char,
    name: *const std::ffi::c_char,
    selectable: i32,
    arg: *mut c_void,
) {
    // The synchronous C inventory borrows this stack context only until it returns.
    let _ = catch_unwind(AssertUnwindSafe(|| {
        if id.is_null() || name.is_null() {
            return;
        }
        let devices = unsafe { &mut *arg.cast::<AudioDevices>() };
        let (Ok(id), Ok(name)) = (
            unsafe { CStr::from_ptr(id) }.to_str(),
            unsafe { CStr::from_ptr(name) }.to_str(),
        ) else {
            return;
        };
        let device = AudioDevice {
            id: id.into(),
            name: name.into(),
            selectable: selectable != 0,
        };
        if input != 0 {
            devices.inputs.push(device);
        } else {
            devices.outputs.push(device);
        }
    }));
}
pub(crate) fn audio_devices() -> Result<AudioDevices, PhoneError> {
    let mut devices = AudioDevices::default();
    let error =
        unsafe { sip_audio_devices(device_callback, (&mut devices as *mut AudioDevices).cast()) };
    if error != 0 {
        return Err(native_error(error));
    }
    Ok(devices)
}

#[cfg(test)]
mod caller_tests {
    use super::*;

    #[test]
    fn caller_number_rejects_uri_and_password_delimiters() {
        for text in [
            "sip:user",
            "user:password",
            "user@host",
            "user;param",
            "user?header",
            "bad user",
        ] {
            assert!(!caller_number_is_display_safe(text), "{text}");
        }
        assert!(caller_number_is_display_safe("+4930123"));
        assert!(caller_number_is_display_safe("alice.sip_42"));
    }

    #[test]
    fn caller_text_rejects_controls_bidi_and_preserves_utf8_boundaries() {
        for text in [
            "bad\nname",
            "bad\0name",
            "safe\u{202e}evil",
            "safe\u{2066}evil",
            "safe\u{2028}evil",
            "safe\u{2029}evil",
        ] {
            assert!(sanitize_caller_text(text, 128).is_none());
        }
        assert_eq!(
            sanitize_caller_text("  Büro  ", 128).as_deref(),
            Some("Büro")
        );
        assert_eq!(sanitize_caller_text("  ", 128), None);
        assert_eq!(sanitize_caller_text("ééé", 5).as_deref(), Some("éé"));
    }
}

#[cfg(test)]
mod observation_tests {
    use super::*;
    use phone_core::{CallOutcome, ObservedCallEvent, ObservedCallKind};

    struct Harness {
        context: Box<CallbackContext>,
        events: Arc<Mutex<Vec<ObservedCallEvent>>>,
    }
    impl Harness {
        fn new() -> Self {
            let events = Arc::new(Mutex::new(Vec::new()));
            let sink = Arc::clone(&events);
            let mut state = EngineState::default();
            state.call_journal.observer =
                Some(Arc::new(move |event| sink.lock().unwrap().push(event)));
            Self {
                context: Box::new(CallbackContext {
                    snapshot: Arc::new(Mutex::new(state)),
                    changed: Arc::new(Condvar::new()),
                    ready: mpsc::channel().0,
                }),
                events,
            }
        }
        fn event(&mut self, kind: i32, code: i32, token: u64, name: &str, number: &str) {
            self.event_for_account(42, kind, code, token, name, number);
        }
        fn event_for_account(
            &mut self,
            account_id: u32,
            kind: i32,
            code: i32,
            token: u64,
            name: &str,
            number: &str,
        ) {
            let mut caller_name = [0u8; 129];
            let mut caller_number = [0u8; 81];
            caller_name[..name.len()].copy_from_slice(name.as_bytes());
            caller_number[..number.len()].copy_from_slice(number.as_bytes());
            let event = NativeEvent {
                kind,
                code,
                account_id,
                call_generation: token,
                caller_name: caller_name.as_ptr().cast(),
                caller_number: caller_number.as_ptr().cast(),
                detail: std::ptr::null(),
            };
            callback(&event, (&mut *self.context as *mut CallbackContext).cast());
        }
    }

    #[test]
    fn account_removal_rejection_remains_visible_during_an_incoming_call() {
        let mut harness = Harness::new();
        harness.event(EV_INCOMING, 0, 10, "Fixture", "100");
        harness.event(EV_REMOVE_FAILED, 16, 10, "", "");
        {
            let state = harness.context.snapshot.lock().unwrap();
            let snapshot = state.snapshot_for_read();
            assert_eq!(snapshot.call, CallState::Incoming);
            assert_eq!(snapshot.last_error.as_ref().unwrap().code, ErrorCode::Busy);
            assert!(snapshot.calls[0].last_error.is_none());
        }
        harness.event(EV_ACTIVE, 0, 10, "", "");
        assert!(
            harness
                .context
                .snapshot
                .lock()
                .unwrap()
                .snapshot_for_read()
                .last_error
                .is_none()
        );
    }

    #[test]
    fn native_account_completion_and_shutdown_wake_blocked_waiters() {
        for kind in [EV_ACCOUNT_APPLIED, EV_ACCOUNT_REJECTED, EV_STOPPED] {
            let mut harness = Harness::new();
            let state = Arc::clone(&harness.context.snapshot);
            let changed = Arc::clone(&harness.context.changed);
            let (entered, waiting) = mpsc::channel();
            let waiter = thread::spawn(move || {
                let mut guard = state.lock().unwrap();
                entered.send(()).unwrap();
                while !guard.account_results.contains_key(&42) && !guard.stopped {
                    let (next, timeout) = changed
                        .wait_timeout(guard, std::time::Duration::from_secs(2))
                        .unwrap();
                    guard = next;
                    assert!(
                        !timeout.timed_out(),
                        "native completion did not notify the waiter"
                    );
                }
            });
            waiting.recv().unwrap();
            harness.event(kind, 0, 0, "", "");
            waiter.join().unwrap();
        }
    }

    #[test]
    fn native_sound_completion_wakes_a_blocked_waiter() {
        for kind in [EV_AUDIO_APPLIED, EV_AUDIO_FAILED, EV_STOPPED] {
            let mut harness = Harness::new();
            let state = Arc::clone(&harness.context.snapshot);
            let changed = Arc::clone(&harness.context.changed);
            let (entered, waiting) = mpsc::channel();
            let waiter = thread::spawn(move || {
                let mut guard = state.lock().unwrap();
                entered.send(()).unwrap();
                while guard.sound_result.is_none() && !guard.stopped {
                    let (next, timeout) = changed
                        .wait_timeout(guard, std::time::Duration::from_secs(2))
                        .unwrap();
                    guard = next;
                    assert!(
                        !timeout.timed_out(),
                        "native sound result did not notify the waiter"
                    );
                }
            });
            waiting.recv().unwrap();
            harness.event(kind, 0, 0, "", "");
            waiter.join().unwrap();
        }
    }
    #[test]
    fn removed_and_stopped_accounts_release_blf_dialog_tables() {
        let mut h = Harness::new();
        let mut other = crate::blf::DialogInfo::default();
        other.apply(r#"<dialog-info xmlns="urn:ietf:params:xml:ns:dialog-info" entity="sip:101@pbx.test" version="0" state="full"/>"#).unwrap();
        {
            let mut state = h.context.snapshot.lock().unwrap();
            state
                .blf_dialogs
                .insert((42, "101".into()), crate::blf::DialogInfo::default());
            state.blf_dialogs.insert((43, "101".into()), other);
        }
        h.event(13, 0, 0, "", "");
        {
            let state = h.context.snapshot.lock().unwrap();
            assert!(!state.blf_dialogs.contains_key(&(42, "101".into())));
            assert!(state.blf_dialogs.contains_key(&(43, "101".into())));
        }
        h.event(11, 0, 0, "", "");
        assert!(h.context.snapshot.lock().unwrap().blf_dialogs.is_empty());
    }

    #[test]
    fn security_observations_are_owned_reset_and_failures_survive_close() {
        let mut fixture = Harness::new();
        fixture.event_for_account(1, 7, 0, 100, "", "100");
        fixture.event_for_account(1, 51, 3 | 4 | (2 << 3), 100, "", "");
        {
            let state = fixture.context.snapshot.lock().unwrap();
            let security = &state.snapshot.calls[0].security;
            assert!(security.tls_peer_verified);
            assert_eq!(security.media, phone_core::MediaProtection::SdesSrtp);
        }
        fixture.event_for_account(1, 51, 3, 999, "", "");
        fixture.event_for_account(1, 50, 20003, 100, "", "");
        fixture.event_for_account(1, 9, 0, 100, "", "");
        {
            let state = fixture.context.snapshot.lock().unwrap();
            assert_eq!(
                state
                    .snapshot
                    .last_call_failure
                    .as_ref()
                    .unwrap()
                    .error
                    .code,
                ErrorCode::MediaSecurityRequired
            );
        }
        fixture.event_for_account(1, 7, 0, 101, "", "101");
        let state = fixture.context.snapshot.lock().unwrap();
        assert_eq!(
            state.snapshot.calls[0].security,
            phone_core::CallSecurity::default()
        );
    }

    #[test]
    fn activation_rejection_restores_prior_identity_and_confirmed_registration() {
        let mut h = Harness::new();
        let previous = phone_core::AccountSnapshot {
            monitored_extensions: Vec::new(),
            transport: Default::default(),
            media_security: Default::default(),
            auth_username: None,
            outbound_proxy: None,
            id: 42,
            name: "Original".into(),
            registrar: "pbx.test".into(),
            username: "synthetic".into(),
            enabled: true,
            enabled_changing: false,
            registered_at_ms: Some(123),
            registration: RegistrationState::Registered,
            last_error: None,
            removing: false,
        };
        {
            let mut state = h.context.snapshot.lock().unwrap();
            state.snapshot.selected_account_id = Some(42);
            state.pending_accounts.insert(42, Some(previous.clone()));
            state.snapshot.accounts.push(phone_core::AccountSnapshot {
                monitored_extensions: Vec::new(),
                transport: Default::default(),
                media_security: Default::default(),
                auth_username: None,
                outbound_proxy: None,
                enabled: false,
                enabled_changing: true,
                registered_at_ms: None,
                registration: RegistrationState::Unconfigured,
                ..previous
            });
        }
        h.event(16, 16, 0, "", "");
        let mut state = h.context.snapshot.lock().unwrap();
        assert!(state.snapshot.accounts[0].enabled);
        assert!(!state.snapshot.accounts[0].enabled_changing);
        assert_eq!(state.snapshot.accounts[0].registered_at_ms, Some(123));
        assert_eq!(state.snapshot.selected_account_id, Some(42));
        assert_eq!(
            state.account_results.remove(&42).unwrap().unwrap_err().code,
            ErrorCode::Busy
        );
        state.snapshot.accounts[0].enabled = false;
        state.snapshot.accounts[0].registered_at_ms = None;
        state.snapshot.accounts[0].registration = RegistrationState::Unconfigured;
        drop(state);
        h.event(3, 0, 0, "", "");
        let state = h.context.snapshot.lock().unwrap();
        assert!(state.snapshot.accounts[0].registered_at_ms.is_none());
        assert_eq!(
            state.snapshot.accounts[0].registration,
            RegistrationState::Unconfigured
        );
    }

    #[test]
    fn hold_resume_preserves_caller_and_native_connection_time() {
        let mut h = Harness::new();
        h.event(7, 0, 10, "Lab Caller", "123");
        h.event(8, 0, 10, "", "");
        let connected = h
            .context
            .snapshot
            .lock()
            .unwrap()
            .snapshot_for_read()
            .call_connected_at_ms;
        assert!(connected.is_some());
        h.event(28, 0, 10, "", "");
        {
            let state = h.context.snapshot.lock().unwrap();
            let snapshot = state.snapshot_for_read();
            assert_eq!(snapshot.call, CallState::Held);
            assert_eq!(
                snapshot.caller.as_ref().unwrap().number.as_deref(),
                Some("123")
            );
        }
        h.event(29, 0, 10, "", "");
        let state = h.context.snapshot.lock().unwrap();
        let snapshot = state.snapshot_for_read();
        assert_eq!(snapshot.call, CallState::Active);
        assert_eq!(snapshot.call_connected_at_ms, connected);
        assert_eq!(snapshot.call_events.len(), 2);
    }

    #[test]
    fn late_close_and_hold_cannot_replace_a_newer_call() {
        let mut h = Harness::new();
        h.event(7, 0, 10, "Old", "100");
        h.event(9, 0, 10, "", "");
        h.event(7, 0, 11, "New", "200");
        h.event(28, 0, 10, "", "");
        h.event(9, 486, 10, "", "");
        let state = h.context.snapshot.lock().unwrap();
        let snapshot = state.snapshot_for_read();
        assert_eq!(snapshot.call, CallState::Incoming);
        assert_eq!(snapshot.call_id.as_deref(), Some("11"));
        assert_eq!(
            snapshot.caller.as_ref().unwrap().number.as_deref(),
            Some("200")
        );
        assert!(snapshot.last_error.is_none());
    }

    #[test]
    fn event_retention_is_bounded_and_explicit_instead_of_collapsing_signals() {
        let mut h = Harness::new();
        h.event(5, 0, 10, "", "");
        for _ in 0..80 {
            h.event(27, 0, 10, "", "");
        }
        let state = h.context.snapshot.lock().unwrap();
        let snapshot = state.snapshot_for_read();
        assert_eq!(snapshot.call, CallState::EarlyMedia);
        assert_eq!(snapshot.call_events.len(), 64);
        assert_eq!(snapshot.call_events_dropped, 16);
        assert_eq!(snapshot.call_events[0].sequence, 17);
        assert_eq!(snapshot.call_events[63].sequence, 80);
    }

    #[test]
    fn final_protocol_failures_are_separate_from_errno_and_local_rejection() {
        let mut h = Harness::new();
        h.event(5, 0, 10, "", "");
        h.event(9, 486, 10, "", "");
        {
            let state = h.context.snapshot.lock().unwrap();
            let snapshot = state.snapshot_for_read();
            let error = snapshot.last_error.as_ref().unwrap();
            assert_eq!(error.code, ErrorCode::SipBusy);
            assert_eq!(error.sip_status, Some(486));
        }
        h.event(7, 0, 11, "", "100");
        h.event(25, 0, 11, "", "");
        h.event(9, 486, 11, "", "");
        assert!(
            h.context
                .snapshot
                .lock()
                .unwrap()
                .snapshot_for_read()
                .last_error
                .is_none()
        );
        h.event(10, 503, 11, "", "");
        let state = h.context.snapshot.lock().unwrap();
        let snapshot = state.snapshot_for_read();
        let error = snapshot.last_error.as_ref().unwrap();
        assert_eq!(error.code, ErrorCode::NativeFailure);
        assert_eq!(error.sip_status, None);
    }

    #[test]
    fn transfer_failure_without_a_response_does_not_invent_a_sip_status() {
        let mut h = Harness::new();
        h.event(5, 0, 10, "", "");
        h.event(8, 0, 10, "", "");
        h.event(32, 0, 10, "", "");
        let state = h.context.snapshot.lock().unwrap();
        let snapshot = state.snapshot_for_read();
        let error = snapshot.last_error.as_ref().unwrap();
        assert_eq!(error.code, ErrorCode::NativeFailure);
        assert_eq!(error.sip_status, None);
        assert_eq!(snapshot.call, CallState::Active);
        assert!(matches!(
            snapshot.call_events.last().unwrap().event,
            CallEventKind::TransferFailed
        ));
    }

    #[test]
    fn failed_transfer_result_includes_its_error_before_the_legacy_event() {
        for status in [0, 403, 488, 500, 603] {
            for background in [false, true] {
                let mut h = Harness::new();
                h.event(5, 0, 10, "", "");
                h.event(8, 0, 10, "", "");
                if background {
                    h.event(23, 0, 20, "", "200");
                    h.event(5, 0, 20, "", "");
                    h.event(8, 0, 20, "", "");
                    h.event(EV_SELECTED, 0, 20, "", "");
                }
                h.event(EV_TRANSFER_PENDING, 0, 10, "", "");
                h.event(EV_TRANSFER_RESULT, status, 10, "", "");
                {
                    let state = h.context.snapshot.lock().unwrap();
                    let snapshot = state.snapshot_for_read();
                    let call = state.snapshot.calls.iter().find(|c| c.id == "10").unwrap();
                    assert_eq!(call.transfer, phone_core::TransferState::Failed);
                    let error = call
                        .last_error
                        .as_ref()
                        .expect("failure must include its error");
                    assert_eq!(error.sip_status, (status != 0).then_some(status as u16));
                    if status == 0 {
                        assert_eq!(error.code, ErrorCode::NativeFailure);
                    }
                    if background {
                        assert_eq!(snapshot.call_id.as_deref(), Some("20"));
                        assert!(snapshot.last_error.is_none());
                    } else {
                        let selected_error = snapshot.last_error.as_ref().unwrap();
                        assert_eq!(selected_error.code, error.code);
                        assert_eq!(selected_error.sip_status, error.sip_status);
                    }
                }
                h.event(EV_TRANSFER_FAILED, status, 10, "", "");
                let state = h.context.snapshot.lock().unwrap();
                let call = state.snapshot.calls.iter().find(|c| c.id == "10").unwrap();
                assert_eq!(
                    call.events
                        .iter()
                        .filter(|e| matches!(e.event, CallEventKind::TransferFailed))
                        .count(),
                    1
                );
            }
        }
    }

    #[test]
    fn unestablished_outgoing_close_without_a_response_is_a_native_failure() {
        for progress in [None, Some(6), Some(EV_PROGRESS)] {
            let mut h = Harness::new();
            h.event(5, 0, 10, "", "");
            if let Some(progress) = progress {
                h.event(progress, 0, 10, "", "");
            }
            h.event(9, 0, 10, "", "");
            let state = h.context.snapshot.lock().unwrap();
            let snapshot = state.snapshot_for_read();
            let error = snapshot.last_error.as_ref().expect("setup failure");
            assert_eq!(error.code, ErrorCode::NativeFailure);
            assert_eq!(error.sip_status, None);
            assert_eq!(snapshot.call, CallState::Idle);
        }
    }

    #[test]
    fn expected_zero_status_closures_do_not_invent_a_setup_failure() {
        for (start, established, ending) in [
            (7, false, None),     // Remote cancellation of an incoming offer.
            (5, false, Some(25)), // Accepted local outgoing cancellation.
            (5, false, Some(26)), // Engine shutdown interrupts a pending attempt.
            (5, true, None),      // Remote termination of an established call.
        ] {
            let mut h = Harness::new();
            h.event(start, 0, 10, "", "");
            if established {
                h.event(8, 0, 10, "", "");
            }
            if let Some(ending) = ending {
                h.event(ending, 0, 10, "", "");
            }
            h.event(9, 0, 10, "", "");
            let state = h.context.snapshot.lock().unwrap();
            let snapshot = state.snapshot_for_read();
            assert!(snapshot.last_error.is_none());
            assert_eq!(snapshot.call, CallState::Idle);
        }
    }

    #[test]
    fn a_sent_answer_closure_reports_failure_but_a_queued_answer_cancellation_does_not() {
        for (closed, expected_error) in [(9, false), (EV_ANSWER_CLOSED, true)] {
            let mut h = Harness::new();
            h.event(7, 0, 10, "", "100");
            h.context.snapshot.lock().unwrap().snapshot.calls[0].state = CallState::Answering;
            h.event(closed, 0, 10, "", "");
            let state = h.context.snapshot.lock().unwrap();
            let snapshot = state.snapshot_for_read();
            assert_eq!(snapshot.call, CallState::Idle);
            assert_eq!(snapshot.last_error.is_some(), expected_error);
            if let Some(error) = &snapshot.last_error {
                assert_eq!(error.code, ErrorCode::NativeFailure);
                assert_eq!(error.sip_status, None);
            }
            let events = h.events.lock().unwrap();
            assert!(matches!(events.last().unwrap().kind,
                ObservedCallKind::Finished { outcome, sip_status: None, .. }
                if outcome == if expected_error { CallOutcome::Failed } else { CallOutcome::Missed }
            ));
        }
    }

    #[test]
    fn an_old_failed_answer_cannot_close_a_new_offer() {
        let mut h = Harness::new();
        h.event(7, 0, 10, "", "100");
        h.event(9, 0, 10, "", "");
        h.event(7, 0, 11, "", "200");
        h.event(EV_ANSWER_CLOSED, 0, 10, "", "");
        let state = h.context.snapshot.lock().unwrap();
        let snapshot = state.snapshot_for_read();
        assert_eq!(snapshot.call, CallState::Incoming);
        assert_eq!(snapshot.call_id.as_deref(), Some("11"));
        assert!(snapshot.last_error.is_none());
    }

    #[test]
    fn native_call_close_failure_is_reported_without_claiming_a_sip_response() {
        let mut h = Harness::new();
        h.event(5, 0, 10, "", "");
        // Pinned baresip reports native RTP stream failures with its internal 701 sentinel.
        h.event(9, 701, 10, "", "");
        let state = h.context.snapshot.lock().unwrap();
        let snapshot = state.snapshot_for_read();
        let error = snapshot
            .last_error
            .as_ref()
            .expect("native close failure must be visible");
        assert_eq!(error.code, ErrorCode::NativeFailure);
        assert_eq!(error.sip_status, None);
        assert_eq!(snapshot.call, CallState::Idle);
    }
    #[test]
    fn incoming_offers_keep_effective_account_identity_while_replacement_is_pending() {
        for kind in [7, 22] {
            let mut h = Harness::new();
            let original = phone_core::AccountSnapshot {
                monitored_extensions: Vec::new(),
                transport: Default::default(),
                media_security: Default::default(),
                auth_username: None,
                outbound_proxy: None,
                id: 42,
                name: "Original".into(),
                registrar: "old.example.test".into(),
                username: "old-user".into(),
                registration: RegistrationState::Registered,
                last_error: None,
                removing: false,
                enabled: true,
                enabled_changing: false,
                registered_at_ms: None,
            };
            {
                let mut state = h.context.snapshot.lock().unwrap();
                state.pending_accounts.insert(42, Some(original.clone()));
                state.snapshot.accounts.push(phone_core::AccountSnapshot {
                    monitored_extensions: Vec::new(),
                    transport: Default::default(),
                    media_security: Default::default(),
                    auth_username: None,
                    outbound_proxy: None,
                    name: "Replacement".into(),
                    registrar: "new.example.test".into(),
                    username: "new-user".into(),
                    registration: RegistrationState::Registering,
                    ..original
                });
            }
            h.event(kind, 0, 10, "Caller", "100");
            h.event(16, 16, 10, "", "");
            let events = h.events.lock().unwrap();
            match &events[0].kind {
                ObservedCallKind::Started {
                    account: Some(account),
                    ..
                } => {
                    assert_eq!(account.name, "Original");
                    assert_eq!(account.registrar, "old.example.test");
                    assert_eq!(account.username, "old-user");
                }
                _ => panic!("Missing effective account identity"),
            }
        }
    }

    #[test]
    fn fatal_audio_error_survives_foreground_zero_status_close() {
        let mut h = Harness::new();
        h.event(7, 0, 10, "Original", "100");
        h.event(8, 0, 10, "", "");
        h.event(50, 5, 10, "", "");
        {
            let state = h.context.snapshot.lock().unwrap();
            assert_eq!(
                state.snapshot.calls[0].last_error.as_ref().unwrap().code,
                ErrorCode::NativeFailure
            );
        }
        h.event(9, 0, 10, "", "");
        let state = h.context.snapshot.lock().unwrap();
        let snapshot = state.snapshot_for_read();
        assert_eq!(
            snapshot.last_error.as_ref().unwrap().code,
            ErrorCode::NativeFailure
        );
        let failure = state.snapshot.last_call_failure.as_ref().unwrap();
        assert_eq!(failure.call_id, "10");
        assert_eq!(failure.account_id, 42);
        assert_eq!(failure.error.code, ErrorCode::NativeFailure);
    }

    #[test]
    fn audio_error_is_owned_by_background_dialog_without_selected_contamination() {
        let mut h = Harness::new();
        h.event(7, 0, 10, "Original", "100");
        h.event(8, 0, 10, "", "");
        h.event_for_account(84, 23, 0, 20, "", "200");
        h.event_for_account(84, 5, 0, 20, "", "");
        h.event_for_account(84, 8, 0, 20, "", "");
        h.event(10, 11, 10, "", "");
        {
            let state = h.context.snapshot.lock().unwrap();
            let snapshot = state.snapshot_for_read();
            assert!(snapshot.last_error.is_none());
            assert!(state.snapshot.calls[0].last_error.is_some());
            assert!(state.snapshot.last_call_failure.is_none());
        }
        h.event(50, 5, 10, "", "");
        {
            let state = h.context.snapshot.lock().unwrap();
            let snapshot = state.snapshot_for_read();
            assert_eq!(snapshot.call_id.as_deref(), Some("20"));
            assert!(snapshot.last_error.is_none());
            assert!(state.snapshot.calls[1].last_error.is_none());
            assert_eq!(
                state.snapshot.calls[0].last_error.as_ref().unwrap().code,
                ErrorCode::NativeFailure
            );
        }
        h.event(9, 0, 10, "", "");
        {
            let state = h.context.snapshot.lock().unwrap();
            let snapshot = state.snapshot_for_read();
            assert_eq!(snapshot.call_id.as_deref(), Some("20"));
            assert_eq!(snapshot.call_account_id, Some(84));
            assert!(state.snapshot.calls[0].last_error.is_none());
            assert!(snapshot.last_error.is_none());
            let failure = state.snapshot.last_call_failure.as_ref().unwrap();
            assert_eq!(failure.call_id, "10");
            assert_eq!(failure.account_id, 42);
            assert_eq!(failure.error.code, ErrorCode::NativeFailure);
        }
        h.event_for_account(84, 9, 200, 20, "", "");
        assert_eq!(
            h.context
                .snapshot
                .lock()
                .unwrap()
                .snapshot
                .last_call_failure
                .as_ref()
                .unwrap()
                .call_id,
            "10"
        );
        h.event(7, 0, 30, "Next", "300");
        assert!(
            h.context
                .snapshot
                .lock()
                .unwrap()
                .snapshot
                .last_call_failure
                .is_none()
        );
    }

    #[test]
    fn recovered_control_error_is_not_a_terminal_call_failure() {
        let mut h = Harness::new();
        h.event(7, 0, 10, "Original", "100");
        h.event(8, 0, 10, "", "");
        h.event(EV_CONTROL_FAILED, 488, 10, "", "");
        h.event(25, 0, 10, "", "");
        h.event(9, 0, 10, "", "");
        assert!(
            h.context
                .snapshot
                .lock()
                .unwrap()
                .snapshot
                .last_call_failure
                .is_none()
        );
    }

    #[test]
    fn engine_wide_error_is_preserved_without_attaching_it_to_a_dialog() {
        let mut h = Harness::new();
        h.event(7, 0, 10, "Original", "100");
        h.event(8, 0, 10, "", "");
        h.event(10, 5, 999, "", "");
        h.event(EV_HOLD, 0, 10, "", "");
        let state = h.context.snapshot.lock().unwrap();
        assert_eq!(
            state.snapshot_for_read().last_error.as_ref().unwrap().code,
            ErrorCode::NativeFailure
        );
        assert!(state.snapshot.calls[0].last_error.is_none());
        assert!(state.snapshot.last_call_failure.is_none());
    }

    #[test]
    fn consultation_background_events_and_end_intent_keep_dialog_ownership() {
        let mut h = Harness::new();
        h.event(7, 0, 10, "Original", "100");
        h.event(8, 0, 10, "", "");
        h.event(23, 0, 20, "", "200");
        h.event(5, 0, 20, "", "");
        h.event(8, 0, 20, "", "");
        h.event(EV_HOLD, 0, 10, "", "");
        {
            let state = h.context.snapshot.lock().unwrap();
            let snapshot = state.snapshot_for_read();
            assert_eq!(snapshot.call_id.as_deref(), Some("20"));
            assert_eq!(snapshot.call, CallState::Active);
            assert_eq!(state.snapshot.calls.len(), 2);
            assert!(state.snapshot.calls[0].remote_held);
            assert_eq!(
                state.snapshot.calls[0]
                    .caller
                    .as_ref()
                    .unwrap()
                    .name
                    .as_deref(),
                Some("Original")
            );
            assert_eq!(state.snapshot.calls[0].account_id, 42);
            assert_eq!(state.snapshot.calls[1].destination.as_deref(), Some("200"));
            assert_eq!(state.snapshot.calls[0].events.len(), 1);
            assert!(state.snapshot.calls[1].events.is_empty());
        }
        h.event(25, 0, 20, "", "");
        h.event(EV_RESUME, 0, 10, "", "");
        h.event(9, 0, 20, "", "");
        {
            let state = h.context.snapshot.lock().unwrap();
            let snapshot = state.snapshot_for_read();
            assert_eq!(snapshot.call_id.as_deref(), Some("10"));
            assert_eq!(state.snapshot.calls.len(), 1);
            assert_eq!(snapshot.call_events.len(), 2);
            assert!(snapshot.last_error.is_none());
        }
        h.event(9, 0, 10, "", "");
        let events = h.events.lock().unwrap();
        assert_eq!(
            events
                .iter()
                .filter(|event| matches!(event.kind, ObservedCallKind::Started { .. }))
                .count(),
            2
        );
        assert_eq!(
            events
                .iter()
                .filter(|event| matches!(
                    event.kind,
                    ObservedCallKind::Finished {
                        outcome: CallOutcome::Completed,
                        ..
                    }
                ))
                .count(),
            2
        );
    }

    #[test]
    fn transfer_success_projection_follows_closure_order_and_late_notify() {
        for (first, last) in [(10, 20), (20, 10)] {
            for status in [None, Some(500), Some(200)] {
                let mut h = Harness::new();
                h.event(7, 0, 10, "Original", "100");
                h.event(8, 0, 10, "", "");
                h.event(23, 0, 20, "", "200");
                h.event(5, 0, 20, "", "");
                h.event(8, 0, 20, "", "");
                h.event(EV_SELECTED, 0, 20, "", "");
                if let Some(status) = status {
                    h.event(EV_TRANSFER_RESULT, status, 10, "", "");
                    h.event(EV_TRANSFER_RESULT, status, 20, "", "");
                }
                h.event(9, 0, first, "", "");
                h.event(9, 0, last, "", "");
                let state = h.context.snapshot.lock().unwrap();
                let snapshot = state.snapshot_for_read();
                assert!(state.snapshot.calls.is_empty());
                assert_eq!(snapshot.call, CallState::Idle);
                let success: Vec<_> = snapshot
                    .call_events
                    .iter()
                    .filter(|event| matches!(event.event, CallEventKind::TransferSucceeded))
                    .collect();
                if status == Some(200) {
                    assert_eq!(success.len(), 1);
                    assert_eq!(success[0].call_id, last.to_string());
                } else {
                    assert!(success.is_empty());
                }
            }
        }

        let mut h = Harness::new();
        h.event(7, 0, 10, "Original", "100");
        h.event(8, 0, 10, "", "");
        h.event(9, 0, 10, "", "");
        {
            let state = h.context.snapshot.lock().unwrap();
            let snapshot = state.snapshot_for_read();
            assert!(state.snapshot.calls.is_empty());
            assert!(snapshot.call_events.is_empty());
        }
        h.event(EV_TRANSFER_RESULT, 200, 10, "", "");
        let state = h.context.snapshot.lock().unwrap();
        let snapshot = state.snapshot_for_read();
        assert!(state.snapshot.calls.is_empty());
        assert!(snapshot.call_events.iter().any(|event| {
            event.call_id == "10" && matches!(event.event, CallEventKind::TransferSucceeded)
        }));
    }

    #[test]
    fn busy_offer_preserves_active_snapshot_token_and_identity() {
        let mut h = Harness::new();
        h.event(7, 0, 10, "First", "100");
        h.event(8, 0, 10, "", "");
        h.event(22, 0, 11, "Second", "200");
        let state = h.context.snapshot.lock().unwrap();
        let snapshot = state.snapshot_for_read();
        assert_eq!(state.call_generation, 10);
        assert_eq!(snapshot.call_id.as_deref(), Some("10"));
        assert_eq!(snapshot.call, CallState::Active);
        assert_eq!(
            snapshot.caller.as_ref().unwrap().number.as_deref(),
            Some("100")
        );
        let events = h.events.lock().unwrap();
        assert_eq!(events.len(), 4);
        assert!(matches!(
            events[3].kind,
            ObservedCallKind::Finished {
                outcome: CallOutcome::Busy,
                ..
            }
        ));
    }
    #[test]
    fn failed_dial_attempt_does_not_close_an_incoming_call_that_won_the_race() {
        let mut h = Harness::new();
        h.event(7, 0, 10, "First", "100");
        h.event(23, 0, 11, "", "4321");
        h.event(24, 500, 11, "", "");
        let state = h.context.snapshot.lock().unwrap();
        let snapshot = state.snapshot_for_read();
        assert_eq!(snapshot.call, CallState::Incoming);
        assert_eq!(state.call_generation, 10);
        assert!(snapshot.last_error.is_none());
        let failure = state.snapshot.last_call_failure.as_ref().unwrap();
        assert_eq!(failure.call_id, "11");
        assert_eq!(failure.account_id, 42);
        assert_eq!(failure.error.code, ErrorCode::NativeFailure);
        assert_eq!(failure.error.sip_status, None);
        let events = h.events.lock().unwrap();
        assert!(
            matches!(&events[1].kind, ObservedCallKind::Started { direction: CallDirection::Outgoing, number: Some(number), .. } if number == "4321")
        );
        assert!(matches!(
            events[2].kind,
            ObservedCallKind::Finished {
                outcome: CallOutcome::Failed,
                sip_status: None,
                ..
            }
        ));
    }
    #[test]
    fn a_failed_current_dial_still_reports_its_native_error() {
        let mut h = Harness::new();
        h.event(23, 0, 10, "", "4321");
        h.event(24, 16, 10, "", "");
        let state = h.context.snapshot.lock().unwrap();
        let snapshot = state.snapshot_for_read();
        assert_eq!(snapshot.call, CallState::Idle);
        let error = snapshot.last_error.as_ref().expect("dial failure");
        assert_eq!(error.code, ErrorCode::NativeFailure);
        assert_eq!(error.sip_status, None);
    }

    #[test]
    fn unaccepted_stale_action_cannot_change_a_remote_cancel_to_local_rejection() {
        let mut h = Harness::new();
        h.event(7, 0, 10, "", "100");
        h.event(12, 0, 10, "", "");
        h.event(9, 487, 10, "", "");
        let events = h.events.lock().unwrap();
        assert_eq!(events.len(), 2);
        assert!(matches!(
            events[1].kind,
            ObservedCallKind::Finished {
                outcome: CallOutcome::Missed,
                ..
            }
        ));
    }
    #[test]
    fn invalid_caller_text_is_absent_from_history_as_well_as_the_snapshot() {
        let mut h = Harness::new();
        h.event(7, 0, 10, "name\u{202e}private", "user:secret@host");
        h.event(9, 487, 10, "", "");
        let events = h.events.lock().unwrap();
        assert!(matches!(
            events[0].kind,
            ObservedCallKind::Started {
                remote_name: None,
                number: None,
                ..
            }
        ));
    }
    #[test]
    fn accepted_local_intent_and_engine_stop_are_observed_before_close() {
        let mut h = Harness::new();
        h.event(7, 0, 10, "", "100");
        h.event(25, 0, 10, "", "");
        h.event(9, 486, 10, "", "");
        h.event(23, 0, 11, "", "4321");
        h.event(5, 0, 11, "", "");
        h.event(26, 0, 11, "", "");
        h.event(9, 0, 11, "", "");
        h.event(11, 0, 11, "", "");
        let events = h.events.lock().unwrap();
        assert_eq!(events.len(), 4);
        assert!(matches!(
            events[1].kind,
            ObservedCallKind::Finished {
                outcome: CallOutcome::Rejected,
                ..
            }
        ));
        assert!(matches!(
            events[3].kind,
            ObservedCallKind::Finished {
                outcome: CallOutcome::Interrupted,
                ..
            }
        ));
    }
}

#[cfg(test)]
mod audio_dsp_tests {
    unsafe extern "C" {
        fn sip_audio_peak_s16(samples: *const i16, count: usize) -> u32;
        fn sip_ringtone_sample(variant: i32, volume: u32, index: u32) -> i16;
    }
    #[test]
    fn synthetic_meter_handles_silence_half_scale_and_minimum_sample() {
        for (samples, expected) in [
            ([0, 0, 0], 0),
            ([0, 16384, -100], 500),
            ([0, -32768, 0], 1000),
        ] {
            assert_eq!(
                unsafe { sip_audio_peak_s16(samples.as_ptr(), samples.len()) },
                expected
            );
        }
    }
    #[test]
    fn pcm_ringtones_preserve_classic_and_scale_amplitude_without_clipping() {
        let mut peaks = [0_i32; 3];
        for i in 0..96000 {
            let seconds = f64::from(i) / 48000.0;
            let local = seconds % 0.6;
            let expected = if seconds < 1.0 && local < 0.4 {
                (2200.0
                    * 1.0_f64.min((local / 0.01).min((0.4 - local) / 0.01))
                    * ((2.0 * std::f64::consts::PI * 440.0 * seconds).sin()
                        + (2.0 * std::f64::consts::PI * 480.0 * seconds).sin()))
                    as i16
            } else {
                0
            };
            assert_eq!(unsafe { sip_ringtone_sample(0, 100, i) }, expected);
            for variant in 0..3 {
                let sample = unsafe { sip_ringtone_sample(variant, 100, i) };
                peaks[variant as usize] = peaks[variant as usize].max(i32::from(sample).abs());
                assert_eq!(unsafe { sip_ringtone_sample(variant, 0, i) }, 0);
                assert!(
                    (i32::from(unsafe { sip_ringtone_sample(variant, 50, i) }) * 2
                        - i32::from(sample))
                    .abs()
                        <= 1
                );
            }
            if i >= 12000 {
                assert_eq!(unsafe { sip_ringtone_sample(2, 100, i) }, 0);
            }
        }
        assert!(peaks[0] <= 4400 && peaks[0] > 4000);
        assert!(peaks[1] < peaks[0]);
        assert!(peaks[2] > 0);
    }
}
