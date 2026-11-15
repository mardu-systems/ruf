//! Authoritative live dialogs and read-only selected-call projection.
use crate::engine::EngineState;
use phone_core::{CallSnapshot, CallState, EngineSnapshot, TransferState};

pub(crate) fn new_dialog(generation: u64, account_id: u32, state: CallState) -> CallSnapshot {
    CallSnapshot {
        id: generation.to_string(),
        account_id,
        state,
        contact: None,
        security: Default::default(),
        destination: None,
        caller: None,
        connected_at_ms: None,
        muted: false,
        local_held: false,
        remote_held: false,
        hold_pending: false,
        transfer: TransferState::Idle,
        consultation: false,
        events: Vec::new(),
        events_dropped: 0,
        last_error: None,
    }
}

impl EngineState {
    pub(crate) fn selected_dialog(&self) -> Option<&CallSnapshot> {
        let id = self.call_generation.to_string();
        self.snapshot
            .calls
            .iter()
            .find(|call| call.id == id)
            .or_else(|| self.pending_dial.as_ref().filter(|call| call.id == id))
    }

    pub(crate) fn call_state(&self) -> CallState {
        self.selected_dialog()
            .map_or(CallState::Idle, |call| call.state)
    }

    #[cfg(feature = "native")]
    pub(crate) fn select_dialog(&mut self, id: &str) -> bool {
        if self.snapshot.calls.iter().any(|call| call.id == id)
            && let Ok(generation) = id.parse()
        {
            self.call_generation = generation;
            self.snapshot.last_error = None;
            return true;
        }
        false
    }

    pub(crate) fn project_selected_dialog(&self, snapshot: &mut EngineSnapshot) {
        if let Some(call) = self.selected_dialog() {
            snapshot.call_id = Some(call.id.clone());
            snapshot.call_account_id = Some(call.account_id);
            snapshot.call = call.state;
            snapshot.caller = call.caller.clone();
            snapshot.caller_contact = call.contact.clone();
            snapshot.call_connected_at_ms = call.connected_at_ms;
            snapshot.call_events = call.events.clone();
            snapshot.call_events_dropped = call.events_dropped;
            // A non-dialog command error may temporarily overlay the selected call.
            if snapshot.last_error.is_none() {
                snapshot.last_error = call.last_error.clone();
            }
        }
    }

    #[cfg(feature = "native")]
    pub(crate) fn start_dialog(&mut self, generation: u64, account_id: u32, state: CallState) {
        let id = generation.to_string();
        if !self.snapshot.calls.iter().any(|call| call.id == id) {
            let mut call = if self.pending_dial.as_ref().is_some_and(|call| call.id == id) {
                self.pending_dial.take().expect("matching reservation")
            } else {
                new_dialog(generation, account_id, state)
            };
            call.consultation = !self.snapshot.calls.is_empty();
            call.destination = self
                .call_destinations
                .get(&generation)
                .cloned()
                .or(call.destination);
            self.snapshot.calls.push(call);
        }
        self.call_generation = generation;
        self.snapshot.call_events.clear();
        self.snapshot.call_events_dropped = 0;
        self.snapshot.last_error = None;
    }

    #[cfg(feature = "native")]
    pub(crate) fn close_dialog(&mut self, generation: u64) {
        let id = generation.to_string();
        let closed = if let Some(index) = self.snapshot.calls.iter().position(|call| call.id == id)
        {
            Some(self.snapshot.calls.remove(index))
        } else if self.pending_dial.as_ref().is_some_and(|call| call.id == id) {
            self.pending_dial.take()
        } else {
            None
        };
        self.local_endings.remove(&generation);
        self.call_destinations.remove(&generation);
        if self.snapshot.calls.len() == 1 {
            self.snapshot.calls[0].consultation = false;
        }
        if !self.snapshot.calls.is_empty() {
            self.snapshot.last_error = None;
        }
        if self.call_generation == generation {
            if let Some(call) = self.snapshot.calls.first() {
                self.call_generation = call.id.parse().expect("native generation");
            } else if let Some(closed) = closed {
                // Retain terminal facts for calls that start/end between UI reads.
                self.snapshot.call_events = closed.events;
                self.snapshot.call_events_dropped = closed.events_dropped;
                self.snapshot.last_error = closed.last_error;
            }
        }
    }
}

#[cfg(all(test, feature = "native"))]
mod tests {
    use super::*;

    #[test]
    fn selected_snapshot_is_derived_from_the_owned_dialog_on_every_read() {
        let mut state = EngineState::default();
        state.start_dialog(7, 42, CallState::Incoming);
        state.snapshot.calls[0].state = CallState::Answering;
        state.snapshot.calls[0].caller = Some(phone_core::CallerIdentity {
            name: Some("Fixture".into()),
            number: Some("100".into()),
        });
        let snapshot = state.snapshot_for_read();
        assert_eq!(snapshot.call, CallState::Answering);
        assert_eq!(snapshot.caller, snapshot.calls[0].caller);
        assert_eq!(snapshot.call_account_id, Some(42));
        assert_eq!(snapshot.call_id.as_deref(), Some("7"));
    }
}
