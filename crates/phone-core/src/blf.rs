//! Account-scoped busy lamp field configuration and observed state.
use crate::{ErrorCode, PhoneError};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Maximum independently subscribed extensions per SIP account.
pub const MAX_MONITORED_EXTENSIONS: usize = 32;

/// One extension watched through the owning account's SIP dialog subscription.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MonitoredExtension {
    /// Exact extension, including significant leading zeros; 1–32 ASCII digits.
    pub extension: String,
    /// Optional display label; defaults to empty, at most 64 characters without controls.
    #[serde(default)]
    pub name: String,
}

/// Observed dialog activity; never inferred from account registration alone.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum ExtensionState {
    /// No coherent current subscription evidence, or monitoring failed/stopped.
    #[default]
    Unknown,
    /// Coherent current dialog state contains no active or early dialogs.
    Idle,
    /// At least one early dialog and no confirmed dialog.
    Ringing,
    /// At least one confirmed dialog, including held calls.
    Busy,
}

/// Password-free configuration plus ephemeral observed state of a watched extension.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct MonitoredExtensionSnapshot {
    /// Exact subscribed extension, scoped to its parent account.
    pub extension: String,
    /// Saved display label; empty uses the extension as its display name.
    pub name: String,
    /// Current native subscription evidence; resets to unknown after loss/replacement.
    pub state: ExtensionState,
}

impl From<&MonitoredExtension> for MonitoredExtensionSnapshot {
    fn from(value: &MonitoredExtension) -> Self {
        Self {
            extension: value.extension.clone(),
            name: value.name.clone(),
            state: ExtensionState::Unknown,
        }
    }
}

/// Validate the bounded per-account target list without normalizing extension identities.
/// Invalid counts, targets, labels or duplicates return `InvalidInput`.
pub fn validate_monitored_extensions(targets: &[MonitoredExtension]) -> Result<(), PhoneError> {
    if targets.len() > MAX_MONITORED_EXTENSIONS
        || targets.iter().enumerate().any(|(index, target)| {
            target.extension.is_empty()
                || target.extension.len() > 32
                || !target.extension.bytes().all(|c| c.is_ascii_digit())
                || target.name.chars().count() > 64
                || target.name.chars().any(char::is_control)
                || targets[..index]
                    .iter()
                    .any(|other| other.extension == target.extension)
        })
    {
        return Err(PhoneError::new(
            ErrorCode::InvalidInput,
            "Monitor at most 32 unique extensions of 1–32 ASCII digits with labels up to 64 characters without controls.",
        ));
    }
    Ok(())
}
