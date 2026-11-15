//! Account-scoped BLF validation and legacy compatibility.
use phone_core::{AccountConfig, ErrorCode};
use serde_json::json;

fn account(targets: serde_json::Value) -> AccountConfig {
    serde_json::from_value(json!({"registrar":"pbx.test","username":"1000","password":"fixture","monitoredExtensions":targets})).unwrap()
}
#[test]
fn legacy_accounts_have_no_monitored_extensions() {
    let value: AccountConfig = serde_json::from_value(
        json!({"registrar":"pbx.test","username":"1000","password":"fixture"}),
    )
    .unwrap();
    assert!(value.monitored_extensions.is_empty());
}
#[test]
fn monitored_extensions_validate_exact_targets_and_names() {
    account(json!([{"extension":"00101","name":"Empfang"},{"extension":"102"}]))
        .validate()
        .unwrap();
    for targets in [
        json!([{"extension":""}]),
        json!([{"extension":"1\r\nEvent: dialog"}]),
        json!([{"extension":"sip:101@pbx.test"}]),
        json!([{"extension":"１２３"}]),
        json!([{"extension":"1".repeat(33)}]),
        json!([{"extension":"101"},{"extension":"101","name":"Duplicate"}]),
        json!([{"extension":"101","name":"x".repeat(65)}]),
        json!([{"extension":"101","name":"hello\nworld"}]),
        json!(
            (0..33)
                .map(|i| json!({"extension":i.to_string()}))
                .collect::<Vec<_>>()
        ),
    ] {
        assert_eq!(
            account(targets.clone()).validate().unwrap_err().code,
            ErrorCode::InvalidInput,
            "{targets}"
        );
    }
}
