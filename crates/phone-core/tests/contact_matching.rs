//! Caller matching must never guess ownership from names or short number suffixes.
use phone_core::{Contact, ContactPhone, ContactSource, contact_match_key, resolve_contact};

fn contact(id: &str, number: &str) -> Contact {
    Contact {
        id: id.into(),
        source: ContactSource::Macos,
        name: "Testkontakt".into(),
        organization: String::new(),
        favorite: false,
        phones: vec![ContactPhone {
            label: "Arbeit".into(),
            number: number.into(),
            destination: phone_core::contact_dial_target(number),
        }],
    }
}

#[test]
fn resolves_formatting_and_explicit_international_prefixes_only() {
    let contacts = [contact("a", "+49 (30) 123-456")];
    assert_eq!(resolve_contact(&contacts, "004930123456").unwrap().id, "a");
    assert_eq!(resolve_contact(&contacts, "+49 30 123456").unwrap().id, "a");
    for target in [
        "123456",
        "030123456",
        "anonymous",
        "",
        "sip:4930123456@pbx",
        "+49(0)30123456",
    ] {
        assert!(resolve_contact(&contacts, target).is_none(), "{target}");
    }
}

#[test]
fn namesakes_and_duplicate_owners_never_become_a_false_identity() {
    let mut first = contact("a", "1234");
    first.phones.push(first.phones[0].clone());
    assert_eq!(resolve_contact(&[first.clone()], "1234").unwrap().id, "a");
    assert!(resolve_contact(&[first, contact("b", "1234")], "1234").is_none());
    assert_eq!(contact_match_key("0012").as_deref(), Some("0012"));
    assert_eq!(contact_match_key("*21*1234#").as_deref(), Some("*21*1234#"));
}

#[test]
fn german_number_matching_is_explicit_and_never_matches_short_suffixes() {
    let contacts = [contact("mobile", "+49 151 55501234")];
    let preferences = phone_core::ContactsPreferences::default();
    for number in ["0151 55501234", "+4915155501234", "004915155501234"] {
        assert_eq!(
            phone_core::resolve_contact_with_preferences(&contacts, number, &preferences)
                .unwrap()
                .id,
            "mobile"
        );
    }
    for number in ["55501234", "1234", "+4315155501234", "anonymous"] {
        assert!(
            phone_core::resolve_contact_with_preferences(&contacts, number, &preferences).is_none()
        );
    }
    let disabled = phone_core::ContactsPreferences {
        match_german_numbers: false,
        ..preferences
    };
    assert!(
        phone_core::resolve_contact_with_preferences(&contacts, "015155501234", &disabled)
            .is_none()
    );
    assert!(
        phone_core::resolve_contact_with_preferences(
            &[contact("a", "015155501234"), contact("b", "+4915155501234")],
            "015155501234",
            &phone_core::ContactsPreferences::default()
        )
        .is_none()
    );
}

#[test]
fn matching_uses_the_selected_numbering_plan_and_preserves_significant_zeroes() {
    use phone_core::{ContactsPreferences, resolve_contact_with_preferences};

    for (region, national, international) in [
        ("GB", "020 7946 0018", "+44 20 7946 0018"),
        ("US", "(202) 555-0123", "+1 202 555 0123"),
        ("IT", "02 3661 8300", "+39 02 3661 8300"),
        ("AU", "0412 345 678", "+61 412 345 678"),
        ("TK", "3101", "+690 3101"),
    ] {
        let preferences = ContactsPreferences {
            number_region: region.into(),
            ..Default::default()
        };
        let contacts = [contact("regional", international)];
        assert_eq!(
            resolve_contact_with_preferences(&contacts, national, &preferences)
                .unwrap_or_else(|| panic!("no alias for {region}: {national}"))
                .id,
            "regional"
        );
        assert_eq!(
            phone_core::contact_dial_target(national).unwrap(),
            national.replace([' ', '(', ')', '-'], "")
        );
        assert!(
            resolve_contact_with_preferences(
                &[
                    contact("regional", international),
                    contact("other", national)
                ],
                national,
                &preferences,
            )
            .is_none()
        );
    }
}

#[test]
fn matching_uses_the_region_only_for_full_national_forms() {
    use phone_core::{ContactsPreferences, contact_match_key_with_preferences};

    let preferences = ContactsPreferences {
        number_region: "US".into(),
        ..Default::default()
    };
    for spelling in ["1234", "0012", "911", "*21*1234#", "011442079460018"] {
        assert_eq!(
            contact_match_key_with_preferences(spelling, &preferences).as_deref(),
            Some(spelling)
        );
    }
    assert_eq!(
        contact_match_key_with_preferences("00442079460018", &preferences).as_deref(),
        Some("+442079460018")
    );
    assert_eq!(
        contact_match_key_with_preferences("2079460018", &preferences).as_deref(),
        Some("+12079460018")
    );
}

#[test]
fn legacy_region_defaults_to_germany_and_rejects_unsupported_values() {
    let preferences: phone_core::ContactsPreferences = serde_json::from_value(serde_json::json!({
        "favoritesFirst": true,
        "showOrganization": true,
        "matchGermanNumbers": false
    }))
    .unwrap();
    assert!(!preferences.match_german_numbers);
    assert_eq!(preferences.number_region, "DE");
    assert_eq!(
        serde_json::to_value(&preferences).unwrap()["numberRegion"],
        "DE"
    );
    for region in ["AA", "de", " DE", "001"] {
        let invalid = phone_core::ContactsPreferences {
            number_region: region.into(),
            ..preferences.clone()
        };
        assert_eq!(
            invalid.validate().unwrap_err().code,
            phone_core::ErrorCode::InvalidInput
        );
    }
}
