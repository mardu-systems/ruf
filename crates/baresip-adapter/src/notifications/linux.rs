//! Freedesktop banners. The desktop service authorizes every opaque action again.
use super::{CallNotificationAction, NotificationPermission};
use phone_core::{ErrorCode, PhoneError};
use std::{
    collections::{BTreeMap, HashMap},
    sync::{Arc, Mutex, OnceLock, mpsc::Sender},
    time::Duration,
};
use zbus::{
    blocking::{Connection, MessageIterator, Proxy},
    message::Type,
    zvariant::Value,
};

const DESTINATION: &str = "org.freedesktop.Notifications";
const PATH: &str = "/org/freedesktop/Notifications";
// Match the installed icon and hidden desktop entry used by GTK's Wayland app id.
const DESKTOP_ENTRY: &str = "sip-desktop";
const MAX_BANNERS: usize = 32;
static SERVICE: OnceLock<Service> = OnceLock::new();
static INIT: Mutex<()> = Mutex::new(());

struct Service {
    connection: Connection,
    owner: String,
    bindings: Arc<Mutex<BTreeMap<u32, String>>>,
}

fn failure() -> PhoneError {
    PhoneError::new(
        ErrorCode::NativeFailure,
        "The desktop notification service is unavailable.",
    )
}

fn proxy(service: &Service) -> Result<Proxy<'_>, PhoneError> {
    Proxy::new(
        &service.connection,
        service.owner.as_str(),
        PATH,
        DESTINATION,
    )
    .map_err(|_| failure())
}

pub(super) fn init(sender: Sender<CallNotificationAction>) -> Result<(), PhoneError> {
    let _init = INIT.lock().map_err(|_| failure())?;
    if SERVICE.get().is_some() {
        return Ok(());
    }
    let connection = zbus::blocking::connection::Builder::session()
        .map_err(|_| failure())?
        .method_timeout(Duration::from_secs(3))
        .build()
        .map_err(|_| failure())?;
    let bus = zbus::blocking::fdo::DBusProxy::new(&connection).map_err(|_| failure())?;
    let owner = bus
        .get_name_owner(DESTINATION.try_into().map_err(|_| failure())?)
        .map_err(|_| failure())?
        .to_string();
    // Bind to this daemon's unique sender, never arbitrary session-bus signals.
    let rule = zbus::MatchRule::builder()
        .msg_type(Type::Signal)
        .sender(owner.as_str())
        .map_err(|_| failure())?
        .path(PATH)
        .map_err(|_| failure())?
        .interface(DESTINATION)
        .map_err(|_| failure())?
        .build();
    let messages =
        MessageIterator::for_match_rule(rule, &connection, Some(64)).map_err(|_| failure())?;
    let bindings = Arc::new(Mutex::new(BTreeMap::new()));
    let observed = Arc::clone(&bindings);
    std::thread::Builder::new()
        .name("linux-notification-actions".into())
        .spawn(move || {
            for message in messages {
                let Ok(message) = message else {
                    break;
                };
                let header = message.header();
                let Some(member) = header.member() else {
                    continue;
                };
                if member.as_str() == "ActionInvoked" {
                    if let Ok((id, action)) = message.body().deserialize::<(u32, String)>() {
                        let token = observed
                            .lock()
                            .ok()
                            .and_then(|bindings| bindings.get(&id).cloned());
                        if let Some(action) = token.and_then(|token| action_for(&action, token)) {
                            let _ = sender.send(action);
                        }
                    }
                } else if member.as_str() == "NotificationClosed"
                    && let Ok((id, _reason)) = message.body().deserialize::<(u32, u32)>()
                    && let Ok(mut bindings) = observed.lock()
                {
                    bindings.remove(&id);
                }
            }
            if let Ok(mut bindings) = observed.lock() {
                bindings.clear();
            }
        })
        .map_err(|_| failure())?;
    SERVICE
        .set(Service {
            connection,
            owner,
            bindings,
        })
        .map_err(|_| failure())
}

fn action_for(action: &str, token: String) -> Option<CallNotificationAction> {
    match action {
        "answer" => Some(CallNotificationAction::Answer(token)),
        "reject" => Some(CallNotificationAction::Reject(token)),
        "default" => Some(CallNotificationAction::Open(token)),
        _ => None,
    }
}

pub(super) fn permission() -> Result<NotificationPermission, PhoneError> {
    let Some(service) = SERVICE.get() else {
        return Ok(NotificationPermission::Unsupported);
    };
    let capabilities: Vec<String> = proxy(service)?
        .call("GetCapabilities", &())
        .map_err(|_| failure())?;
    Ok(if capabilities.iter().any(|value| value == "actions") {
        NotificationPermission::Authorized
    } else {
        NotificationPermission::Unsupported
    })
}

pub(super) fn show(
    token: &str,
    title: &str,
    body: &str,
    answer: &str,
    reject: &str,
) -> Result<(), PhoneError> {
    notify(
        Some(token),
        title,
        body,
        &["default", "Ruf", "answer", answer, "reject", reject],
    )
}

pub(super) fn test(title: &str, body: &str) -> Result<(), PhoneError> {
    notify(None, title, body, &[])
}

fn notify(
    token: Option<&str>,
    title: &str,
    body: &str,
    actions: &[&str],
) -> Result<(), PhoneError> {
    if permission()? != NotificationPermission::Authorized {
        return Err(failure());
    }
    let service = SERVICE.get().ok_or_else(failure)?;
    let mut bindings = service.bindings.lock().map_err(|_| failure())?;
    let previous = token
        .and_then(|token| {
            bindings
                .iter()
                .find(|(_, bound)| bound.as_str() == token)
                .map(|(id, _)| *id)
        })
        .unwrap_or(0);
    if token.is_some() && previous == 0 && bindings.len() >= MAX_BANNERS {
        return Err(failure());
    }
    let hints = HashMap::from([
        ("suppress-sound", Value::from(true)),
        ("desktop-entry", Value::from(DESKTOP_ENTRY)),
        ("urgency", Value::from(2u8)),
    ]);
    // Notification bodies may support markup. Keep caller/account text literal.
    let body = body
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");
    let id: u32 = proxy(service)?
        .call(
            "Notify",
            &(
                "Ruf",
                previous,
                DESKTOP_ENTRY,
                title,
                body,
                actions,
                hints,
                if token.is_some() { 0i32 } else { 5000i32 },
            ),
        )
        .map_err(|_| failure())?;
    if id == 0 {
        return Err(failure());
    }
    if previous != 0 {
        bindings.remove(&previous);
    }
    if let Some(token) = token {
        bindings.insert(id, token.to_owned());
    }
    Ok(())
}

pub(super) fn dismiss(token: &str) -> Result<(), PhoneError> {
    let Some(service) = SERVICE.get() else {
        return Ok(());
    };
    let mut bindings = service.bindings.lock().map_err(|_| failure())?;
    if let Some(id) = bindings
        .iter()
        .find(|(_, bound)| bound.as_str() == token)
        .map(|(id, _)| *id)
    {
        bindings.remove(&id);
        proxy(service)?
            .call::<_, _, ()>("CloseNotification", &(id,))
            .map_err(|_| failure())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn banner_clicks_only_open_and_unknown_actions_cannot_answer() {
        assert_eq!(
            action_for("default", "opaque".into()),
            Some(CallNotificationAction::Open("opaque".into()))
        );
        assert_eq!(
            action_for("answer", "opaque".into()),
            Some(CallNotificationAction::Answer("opaque".into()))
        );
        assert_eq!(
            action_for("reject", "opaque".into()),
            Some(CallNotificationAction::Reject("opaque".into()))
        );
        assert_eq!(action_for("answer:other-call", "opaque".into()), None);
    }

    #[test]
    #[ignore = "requires the isolated notification daemon started by test:linux"]
    fn linux_notification_service_binds_actions_and_withdraws_stale_banners() {
        assert_eq!(std::env::var("RUF_LINUX_TEST_SESSION").as_deref(), Ok("1"));
        let (sender, receiver) = std::sync::mpsc::channel();
        init(sender).unwrap();
        assert_eq!(permission().unwrap(), NotificationPermission::Authorized);
        show(
            "opaque-a",
            "Incoming",
            "<caller>&number",
            "Answer",
            "Reject",
        )
        .unwrap();
        let service = SERVICE.get().unwrap();
        let daemon = proxy(service).unwrap();
        let (icon, desktop_entry): (String, String) = daemon.call("Identity", &()).unwrap();
        assert_eq!(icon, "sip-desktop");
        assert_eq!(desktop_entry, "sip-desktop");
        let (id, body, actions, silent): (u32, String, Vec<String>, bool) =
            daemon.call("Recorded", &()).unwrap();
        assert_eq!(body, "&lt;caller&gt;&amp;number");
        assert!(silent);
        assert_eq!(
            actions,
            ["default", "Ruf", "answer", "Answer", "reject", "Reject"]
        );
        // An unrelated session-bus sender cannot forge an action for this id.
        service
            .connection
            .emit_signal(
                None::<&str>,
                PATH,
                DESTINATION,
                "ActionInvoked",
                &(id, "answer"),
            )
            .unwrap();
        assert!(receiver.recv_timeout(Duration::from_millis(100)).is_err());
        daemon
            .call::<_, _, ()>("Activate", &(id, "default"))
            .unwrap();
        assert_eq!(
            receiver.recv_timeout(Duration::from_secs(2)).unwrap(),
            CallNotificationAction::Open("opaque-a".into())
        );
        dismiss("opaque-a").unwrap();
        daemon
            .call::<_, _, ()>("Activate", &(id, "answer"))
            .unwrap();
        assert!(receiver.recv_timeout(Duration::from_millis(100)).is_err());
        test("Test", "Anonymous").unwrap();
        let (id, _, actions, _): (u32, String, Vec<String>, bool) =
            daemon.call("Recorded", &()).unwrap();
        assert!(actions.is_empty());
        daemon
            .call::<_, _, ()>("Activate", &(id, "default"))
            .unwrap();
        assert!(receiver.recv_timeout(Duration::from_millis(100)).is_err());
        daemon.call::<_, _, ()>("Stall", &()).unwrap();
        let started = std::time::Instant::now();
        assert!(permission().is_err());
        assert!(
            started.elapsed() < Duration::from_secs(4),
            "An unresponsive daemon must not hold the caller indefinitely"
        );
    }

    #[test]
    #[ignore = "requires test:linux's intentionally missing session bus"]
    fn linux_notifications_without_session_fail_closed() {
        assert_eq!(std::env::var("RUF_LINUX_NO_SESSION").as_deref(), Ok("1"));
        let (sender, _receiver) = std::sync::mpsc::channel();
        let started = std::time::Instant::now();
        assert!(init(sender).is_err());
        assert_eq!(permission().unwrap(), NotificationPermission::Unsupported);
        assert!(started.elapsed() < Duration::from_secs(4));
    }
}
