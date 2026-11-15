//! Bounded RFC 4235 state reduction for native dialog subscriptions.

use phone_core::ExtensionState;
use quick_xml::{
    events::{BytesStart, Event},
    name::ResolveResult,
    reader::NsReader,
};
use std::collections::BTreeMap;

const NAMESPACE: &str = "urn:ietf:params:xml:ns:dialog-info";
const MAX_BYTES: usize = 65_536;
const MAX_DIALOGS: usize = 128;
const MAX_DEPTH: usize = 32;

/// Evidence is valid only after a full document for the current subscription.
#[derive(Default)]
pub(crate) struct DialogInfo {
    version: Option<u64>,
    entity: Option<String>,
    dialogs: BTreeMap<String, ExtensionState>,
}

impl DialogInfo {
    /// Apply one complete notification. Incoherence requests native resubscription.
    pub(crate) fn apply(&mut self, xml: &str) -> Result<ExtensionState, ()> {
        let result = self.apply_document(xml);
        if result.is_err() {
            self.reset();
        }
        result
    }

    pub(crate) fn reset(&mut self) {
        *self = Self::default();
    }

    fn apply_document(&mut self, xml: &str) -> Result<ExtensionState, ()> {
        let document = parse(xml)?;
        if self
            .entity
            .as_ref()
            .is_some_and(|entity| *entity != document.entity)
        {
            return Err(());
        }
        if let Some(previous) = self.version {
            if document.version <= previous {
                return Ok(self.state());
            }
            if !document.full && previous.checked_add(1) != Some(document.version) {
                return Err(());
            }
        } else if !document.full {
            return Err(());
        }
        if document.full {
            self.dialogs.clear();
        }
        for (id, state) in document.dialogs {
            if let Some(state) = state {
                self.dialogs.insert(id, state);
            } else {
                self.dialogs.remove(&id);
            }
        }
        if self.dialogs.len() > MAX_DIALOGS {
            return Err(());
        }
        self.version = Some(document.version);
        self.entity = Some(document.entity);
        Ok(self.state())
    }

    fn state(&self) -> ExtensionState {
        let states = self.dialogs.values();
        if states.clone().any(|state| *state == ExtensionState::Busy) {
            ExtensionState::Busy
        } else if states
            .clone()
            .any(|state| *state == ExtensionState::Ringing)
        {
            ExtensionState::Ringing
        } else if states
            .clone()
            .any(|state| *state == ExtensionState::Unknown)
        {
            ExtensionState::Unknown
        } else {
            ExtensionState::Idle
        }
    }
}

struct Document {
    version: u64,
    full: bool,
    entity: String,
    dialogs: BTreeMap<String, Option<ExtensionState>>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Element {
    Root,
    Dialog,
    State,
    Ignored,
}

#[derive(Default)]
struct Dialog {
    id: String,
    state: Option<Option<ExtensionState>>,
}

fn attributes(element: &BytesStart<'_>) -> Result<BTreeMap<String, String>, ()> {
    let mut attributes = BTreeMap::new();
    for attribute in element.attributes() {
        let attribute = attribute.map_err(|_| ())?;
        let value = attribute
            .normalized_value(quick_xml::XmlVersion::Implicit1_0)
            .map_err(|_| ())?;
        if value.len() > 1024 || attribute.key.as_ref().len() > 256 {
            return Err(());
        }
        attributes.insert(attribute.key.as_ref().to_owned(), value.into_owned());
    }
    Ok(attributes)
}

fn parse(xml: &str) -> Result<Document, ()> {
    if xml.len() > MAX_BYTES
        || xml.chars().any(|ch| {
            (ch < ' ' && !matches!(ch, '\t' | '\n' | '\r')) || matches!(ch, '\u{fffe}' | '\u{ffff}')
        })
    {
        return Err(());
    }
    let mut reader = NsReader::from_str(xml);
    reader.config_mut().enable_all_checks(true);
    reader.config_mut().expand_empty_elements = true;
    let mut stack = Vec::new();
    let mut document: Option<Document> = None;
    let mut dialog = Dialog::default();
    let mut state_text = String::new();
    let mut root_closed = false;
    let mut declaration_seen = false;
    loop {
        let (namespace, event) = reader.read_resolved_event().map_err(|_| ())?;
        let known_namespace = match namespace {
            ResolveResult::Bound(namespace) => namespace.as_ref() == NAMESPACE,
            ResolveResult::Unbound => false,
            ResolveResult::Unknown(_) => return Err(()),
        };
        match event {
            Event::Start(element) => {
                if stack.len() >= MAX_DEPTH || root_closed {
                    return Err(());
                }
                let attributes = attributes(&element)?;
                let local = element.local_name();
                let node = match stack.last() {
                    None => {
                        if document.is_some() || !known_namespace || local.as_ref() != "dialog-info"
                        {
                            return Err(());
                        }
                        let version = attributes.get("version").ok_or(())?;
                        if version.is_empty() || !version.bytes().all(|byte| byte.is_ascii_digit())
                        {
                            return Err(());
                        }
                        let full = match attributes.get("state").map(String::as_str) {
                            Some("full") => true,
                            Some("partial") => false,
                            _ => return Err(()),
                        };
                        let entity = attributes.get("entity").ok_or(())?;
                        if entity.is_empty() || entity.chars().any(char::is_whitespace) {
                            return Err(());
                        }
                        document = Some(Document {
                            version: version.parse().map_err(|_| ())?,
                            full,
                            entity: entity.clone(),
                            dialogs: BTreeMap::new(),
                        });
                        Element::Root
                    }
                    Some(Element::Root) if known_namespace && local.as_ref() == "dialog" => {
                        let id = attributes.get("id").ok_or(())?;
                        if id.is_empty() || id.len() > 256 || id.chars().any(char::is_control) {
                            return Err(());
                        }
                        dialog = Dialog {
                            id: id.clone(),
                            state: None,
                        };
                        Element::Dialog
                    }
                    Some(Element::Dialog) if known_namespace && local.as_ref() == "state" => {
                        if dialog.state.is_some() {
                            return Err(());
                        }
                        state_text.clear();
                        Element::State
                    }
                    Some(Element::State) => return Err(()),
                    _ => Element::Ignored,
                };
                stack.push(node);
            }
            Event::End(_) => match stack.pop().ok_or(())? {
                Element::State => {
                    dialog.state = Some(match state_text.trim() {
                        "confirmed" => Some(ExtensionState::Busy),
                        "early" => Some(ExtensionState::Ringing),
                        "terminated" => None,
                        "trying" | "proceeding" => Some(ExtensionState::Unknown),
                        _ => return Err(()),
                    });
                }
                Element::Dialog => {
                    let document = document.as_mut().ok_or(())?;
                    if document.dialogs.len() >= MAX_DIALOGS
                        || document
                            .dialogs
                            .insert(
                                std::mem::take(&mut dialog.id),
                                dialog.state.take().ok_or(())?,
                            )
                            .is_some()
                    {
                        return Err(());
                    }
                }
                Element::Root => root_closed = true,
                Element::Ignored => {}
            },
            Event::Text(text) => append_text(&mut state_text, stack.last(), &text)?,
            Event::CData(text) => {
                if stack.is_empty() {
                    return Err(());
                }
                append_text(&mut state_text, stack.last(), &text)?;
            }
            Event::GeneralRef(reference) => {
                let character = reference.resolve_char_ref().map_err(|_| ())?;
                let value = match character {
                    Some(character) => character.to_string(),
                    None => quick_xml::escape::resolve_predefined_entity(&reference)
                        .ok_or(())?
                        .to_owned(),
                };
                append_text(&mut state_text, stack.last(), &value)?;
            }
            Event::Decl(_) if !declaration_seen && document.is_none() => declaration_seen = true,
            Event::Comment(_) | Event::PI(_) => {}
            Event::Eof if root_closed && stack.is_empty() => return document.ok_or(()),
            _ => return Err(()),
        }
    }
}

fn append_text(text: &mut String, element: Option<&Element>, value: &str) -> Result<(), ()> {
    match element {
        Some(Element::State) => {
            if text.len() + value.len() > 32 {
                return Err(());
            }
            text.push_str(value);
        }
        Some(Element::Ignored) => {}
        _ if value.trim().is_empty() => {}
        _ => return Err(()),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn document(version: u64, kind: &str, body: &str) -> String {
        format!(
            r#"<dialog-info xmlns="urn:ietf:params:xml:ns:dialog-info" version="{version}" state="{kind}" entity="sip:1000@example.test">{body}</dialog-info>"#
        )
    }

    #[test]
    fn full_empty_is_idle_and_call_transitions_merge_by_dialog_id() {
        let mut info = DialogInfo::default();
        assert_eq!(
            info.apply(&document(0, "full", "")),
            Ok(ExtensionState::Idle)
        );
        assert_eq!(
            info.apply(&document(
                1,
                "partial",
                r#"<dialog id="a"><state>early</state></dialog>"#
            )),
            Ok(ExtensionState::Ringing)
        );
        assert_eq!(
            info.apply(&document(
                2,
                "partial",
                r#"<dialog id="a"><state>confirmed</state></dialog>"#
            )),
            Ok(ExtensionState::Busy)
        );
        assert_eq!(
            info.apply(&document(
                3,
                "partial",
                r#"<dialog id="a"><state>terminated</state></dialog>"#
            )),
            Ok(ExtensionState::Idle)
        );
    }

    #[test]
    fn full_replaces_state_and_partial_keeps_unmentioned_dialogs() {
        let mut info = DialogInfo::default();
        assert_eq!(info.apply(&document(5, "full", r#"<dialog id="a"><state>confirmed</state></dialog><dialog id="b"><state>early</state></dialog>"#)), Ok(ExtensionState::Busy));
        assert_eq!(
            info.apply(&document(
                6,
                "partial",
                r#"<dialog id="a"><state>terminated</state></dialog>"#
            )),
            Ok(ExtensionState::Ringing)
        );
        assert_eq!(
            info.apply(&document(10, "full", "")),
            Ok(ExtensionState::Idle)
        );
    }

    #[test]
    fn missing_full_or_version_gap_invalidates_state_until_resynchronized() {
        let mut info = DialogInfo::default();
        assert!(info.apply(&document(0, "partial", "")).is_err());
        assert_eq!(
            info.apply(&document(0, "full", "")),
            Ok(ExtensionState::Idle)
        );
        assert!(
            info.apply(&document(
                2,
                "partial",
                r#"<dialog id="a"><state>confirmed</state></dialog>"#
            ))
            .is_err()
        );
        assert!(info.apply(&document(3, "partial", "")).is_err());
        assert_eq!(
            info.apply(&document(4, "full", "")),
            Ok(ExtensionState::Idle)
        );
        info.reset();
        assert!(info.apply(&document(5, "partial", "")).is_err());
    }

    #[test]
    fn delayed_and_duplicate_documents_cannot_revert_current_state() {
        let mut info = DialogInfo::default();
        assert_eq!(
            info.apply(&document(
                5,
                "full",
                r#"<dialog id="a"><state>confirmed</state></dialog>"#
            )),
            Ok(ExtensionState::Busy)
        );
        assert_eq!(
            info.apply(&document(4, "full", "")),
            Ok(ExtensionState::Busy)
        );
        assert_eq!(
            info.apply(&document(5, "full", "")),
            Ok(ExtensionState::Busy)
        );
    }

    #[test]
    fn prefixes_and_extension_elements_preserve_real_dialog_namespace() {
        let mut info = DialogInfo::default();
        let xml = r#"<?xml version="1.0"?><d:dialog-info xmlns:d="urn:ietf:params:xml:ns:dialog-info" xmlns:x="urn:other" version="1" state="full" entity="sip:1000@example.test"><x:dialog id="wrong"><d:state>confirmed</d:state></x:dialog><d:dialog id="a"><d:local><d:identity>Alice &amp; Bob</d:identity></d:local><d:state><![CDATA[early]]></d:state></d:dialog></d:dialog-info>"#;
        assert_eq!(info.apply(xml), Ok(ExtensionState::Ringing));
    }

    #[test]
    fn malformed_or_ambiguous_documents_never_report_idle() {
        let body = r#"<dialog id="a"><state>confirmed</state></dialog>"#;
        let invalid = [
            document(0, "full", r#"<dialog id="a"/>"#),
            document(
                0,
                "full",
                r#"<dialog id="a"><state>mystery</state></dialog>"#,
            ),
            document(
                0,
                "full",
                r#"<dialog id="a"><state>early</state><state>confirmed</state></dialog>"#,
            ),
            document(0, "full", &format!("{body}{body}")),
            document(0, "full", r#"<dialog><state>early</state></dialog>"#),
            document(0, "full", body).replace("version=\"0\"", "version=\"-1\""),
            document(0, "full", body).replace("version=\"0\"", "version=\"18446744073709551616\""),
            document(0, "full", body).replace("urn:ietf:params:xml:ns:dialog-info", "urn:other"),
            document(0, "full", body).replace("dialog-info", "wrong"),
            document(0, "full", body).replace(" entity=\"sip:1000@example.test\"", ""),
            document(0, "full", body).replace("</dialog-info>", ""),
            format!("{}{}", document(0, "full", ""), document(0, "full", "")),
            format!(
                r#"<!DOCTYPE dialog-info [<!ENTITY injected "confirmed">]>{}"#,
                document(
                    0,
                    "full",
                    r#"<dialog id="a"><state>&injected;</state></dialog>"#
                )
            ),
            document(0, "full", body).replace("state=\"full\"", "state=\"other\""),
        ];
        for xml in invalid {
            let mut info = DialogInfo::default();
            assert!(info.apply(&xml).is_err(), "accepted invalid XML: {xml}");
        }
    }

    #[test]
    fn malformed_partial_clears_previous_evidence() {
        let mut info = DialogInfo::default();
        assert_eq!(
            info.apply(&document(0, "full", "")),
            Ok(ExtensionState::Idle)
        );
        assert!(
            info.apply(&document(1, "partial", r#"<dialog id="a"/>"#))
                .is_err()
        );
        assert!(info.apply(&document(2, "partial", "")).is_err());
    }

    #[test]
    fn pending_outgoing_dialogs_are_unknown_instead_of_false_idle() {
        for state in ["trying", "proceeding"] {
            let mut info = DialogInfo::default();
            assert_eq!(
                info.apply(&document(
                    0,
                    "full",
                    &format!(r#"<dialog id="a"><state>{state}</state></dialog>"#)
                )),
                Ok(ExtensionState::Unknown)
            );
        }
    }

    #[test]
    fn cumulative_partial_dialog_growth_is_bounded() {
        let mut info = DialogInfo::default();
        let dialogs = (0..128)
            .map(|id| format!(r#"<dialog id="{id}"><state>early</state></dialog>"#))
            .collect::<String>();
        assert_eq!(
            info.apply(&document(0, "full", &dialogs)),
            Ok(ExtensionState::Ringing)
        );
        assert!(
            info.apply(&document(
                1,
                "partial",
                r#"<dialog id="new"><state>confirmed</state></dialog>"#
            ))
            .is_err()
        );
        assert!(info.apply(&document(2, "partial", "")).is_err());
    }

    #[test]
    fn a_different_entity_invalidates_the_current_subscription() {
        let mut info = DialogInfo::default();
        assert_eq!(
            info.apply(&document(0, "full", "")),
            Ok(ExtensionState::Idle)
        );
        assert!(
            info.apply(&document(1, "partial", "").replace("sip:1000@", "sip:1001@"))
                .is_err()
        );
    }

    #[test]
    fn character_references_and_maximum_version_are_supported() {
        let mut info = DialogInfo::default();
        assert_eq!(
            info.apply(&document(
                u64::MAX,
                "full",
                r#"<dialog id="a&amp;b"><state>con&#102;irmed</state></dialog>"#
            )),
            Ok(ExtensionState::Busy)
        );
        assert_eq!(
            info.apply(&document(u64::MAX, "full", "")),
            Ok(ExtensionState::Busy)
        );
    }

    #[test]
    fn oversized_deep_or_excessive_dialog_documents_are_rejected() {
        let excessive = (0..129)
            .map(|n| format!(r#"<dialog id="{n}"><state>early</state></dialog>"#))
            .collect::<String>();
        let deep = format!("{}{}", "<x>".repeat(40), "</x>".repeat(40));
        for xml in [
            document(0, "full", &excessive),
            document(0, "full", &deep),
            document(0, "full", &" ".repeat(65536)),
        ] {
            assert!(DialogInfo::default().apply(&xml).is_err());
        }
    }
}
