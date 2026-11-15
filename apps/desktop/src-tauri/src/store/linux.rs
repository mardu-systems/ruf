//! Secret Service credentials. Background operations never execute OS prompts.
use super::{credential_access_required, credential_failure};
use phone_core::{ErrorCode, PhoneError};
use secret_service::{
    EncryptionType,
    blocking::{Item, SecretService},
};
use std::{collections::HashMap, time::Duration};

const SERVICE: &str = "org.rufapp.desktop";

fn connect() -> Result<SecretService<'static>, PhoneError> {
    let connection = zbus::blocking::connection::Builder::session()
        .map_err(|_| credential_failure())?
        .method_timeout(Duration::from_secs(5))
        .build()
        .map_err(|_| credential_failure())?;
    SecretService::connect_with_existing(EncryptionType::Dh, connection).map_err(error)
}

fn attributes(key: &str) -> HashMap<&str, &str> {
    HashMap::from([("service", SERVICE), ("username", key)])
}

fn find<'a>(service: &'a SecretService<'a>, key: &str) -> Result<Option<Item<'a>>, PhoneError> {
    let mut found = service.search_items(attributes(key)).map_err(error)?;
    found.unlocked.append(&mut found.locked);
    if found.unlocked.len() > 1 {
        // Never arbitrarily select a secret from ambiguous collections.
        return Err(credential_failure());
    }
    Ok(found.unlocked.pop())
}

fn unlock(item: &Item<'_>, interactive: bool) -> Result<(), PhoneError> {
    if item.is_locked().map_err(error)? {
        if !interactive {
            return Err(credential_access_required());
        }
        item.unlock().map_err(error)?;
    }
    Ok(())
}

pub(super) fn read(key: &str, interactive: bool) -> Result<String, PhoneError> {
    let service = connect()?;
    let item = find(&service, key)?.ok_or_else(|| PhoneError::new(
        ErrorCode::CredentialMissing, "No saved password exists for this account. Enter the SIP password and save the account.",
    ))?;
    unlock(&item, interactive)?;
    String::from_utf8(item.get_secret().map_err(error)?).map_err(|_| credential_failure())
}

pub(super) fn write(key: &str, password: &str) -> Result<(), PhoneError> {
    let service = connect()?;
    if let Some(item) = find(&service, key)? {
        unlock(&item, true)?;
        return item
            .set_secret(password.as_bytes(), "text/plain; charset=utf-8")
            .map_err(error);
    }
    let collection = match service.get_default_collection() {
        Ok(collection) => collection,
        Err(secret_service::Error::NoResult) => {
            service.create_collection("Ruf", "default").map_err(error)?
        }
        Err(cause) => return Err(error(cause)),
    };
    if collection.is_locked().map_err(error)? {
        collection.unlock().map_err(error)?;
    }
    collection
        .create_item(
            "Ruf SIP account",
            attributes(key),
            password.as_bytes(),
            false,
            "text/plain; charset=utf-8",
        )
        .map_err(error)?;
    Ok(())
}

pub(super) fn delete(key: &str) -> Result<(), PhoneError> {
    let service = connect()?;
    let Some(item) = find(&service, key)? else {
        return Ok(());
    };
    unlock(&item, false)?;
    // Item::delete would automatically execute a returned deletion prompt.
    // Keep cleanup noninteractive: a prompt leaves the durable journal retryable.
    let connection = zbus::blocking::connection::Builder::session()
        .map_err(|_| credential_failure())?
        .method_timeout(Duration::from_secs(5))
        .build()
        .map_err(|_| credential_failure())?;
    let proxy = zbus::blocking::Proxy::new(
        &connection,
        "org.freedesktop.secrets",
        item.item_path.as_str(),
        "org.freedesktop.Secret.Item",
    )
    .map_err(|_| credential_failure())?;
    let prompt: zbus::zvariant::OwnedObjectPath = proxy
        .call("Delete", &())
        .map_err(|_| credential_failure())?;
    if prompt.as_str() != "/" {
        return Err(credential_access_required());
    }
    Ok(())
}

fn error(cause: secret_service::Error) -> PhoneError {
    match cause {
        secret_service::Error::Locked
        | secret_service::Error::Prompt
        | secret_service::Error::PromptDisconnected => credential_access_required(),
        _ => credential_failure(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secret_service_errors_keep_unlock_separate_from_backend_failure() {
        for cause in [
            secret_service::Error::Locked,
            secret_service::Error::Prompt,
            secret_service::Error::PromptDisconnected,
        ] {
            assert_eq!(error(cause).code, ErrorCode::CredentialAccessRequired);
        }
        assert_eq!(
            error(secret_service::Error::Unavailable).code,
            ErrorCode::CredentialFailure
        );
    }

    #[test]
    #[ignore = "requires the isolated Secret Service started by test:linux"]
    fn linux_secret_service_roundtrip_and_locked_background_access() {
        assert_eq!(std::env::var("RUF_LINUX_TEST_SESSION").as_deref(), Ok("1"));
        let key = "linux-foundation-synthetic";
        write(key, "synthetic-secret").unwrap();
        assert_eq!(read(key, false).unwrap(), "synthetic-secret");
        write(key, "synthetic-replacement").unwrap();
        assert_eq!(read(key, false).unwrap(), "synthetic-replacement");
        delete(key).unwrap();
        assert_eq!(
            read(key, false).unwrap_err().code,
            ErrorCode::CredentialMissing
        );
        delete(key).unwrap();
        write(key, "synthetic-for-lock-test").unwrap();
        let service = connect().unwrap();
        let collection = service.get_default_collection().unwrap();
        collection.lock().unwrap();
        assert_eq!(
            read(key, false).unwrap_err().code,
            ErrorCode::CredentialAccessRequired
        );
        assert_eq!(
            delete(key).unwrap_err().code,
            ErrorCode::CredentialAccessRequired
        );
        assert!(collection.is_locked().unwrap());
        // The isolated test session is discarded afterward; never unlock a real user's store.
    }

    #[test]
    #[ignore = "requires test:linux's intentionally missing session bus"]
    fn linux_credentials_without_session_fail_closed() {
        assert_eq!(std::env::var("RUF_LINUX_NO_SESSION").as_deref(), Ok("1"));
        let started = std::time::Instant::now();
        assert_eq!(
            read("synthetic-missing-bus", false).unwrap_err().code,
            ErrorCode::CredentialFailure
        );
        assert_eq!(
            write("synthetic-missing-bus", "synthetic")
                .unwrap_err()
                .code,
            ErrorCode::CredentialFailure
        );
        assert_eq!(
            delete("synthetic-missing-bus").unwrap_err().code,
            ErrorCode::CredentialFailure
        );
        assert!(started.elapsed() < Duration::from_secs(6));
    }
}
