//! Deliver a telephone link as a draft in the main phone window.

use std::sync::Mutex;

const OPEN_TEL_ARGUMENT: &str = "--open-tel";
const MAX_URL_PATH_BYTES: usize = 512;

#[derive(Default)]
pub(crate) struct TelLinkState(Mutex<Option<String>>);

impl TelLinkState {
    #[cfg(any(target_os = "macos", test))]
    fn stage_urls(&self, urls: &[tauri::Url]) -> bool {
        urls.iter()
            .rev()
            .find_map(number_from_url)
            .is_some_and(|number| self.stage(number))
    }

    pub(crate) fn stage_launch_args(&self, args: &[String]) -> bool {
        let [_, flag, uri] = args else {
            return false;
        };
        if flag != OPEN_TEL_ARGUMENT
            || uri.len() > MAX_URL_PATH_BYTES + "tel:".len()
            || uri.chars().any(char::is_control)
        {
            return false;
        }
        tauri::Url::parse(uri)
            .ok()
            .and_then(|url| number_from_url(&url))
            .is_some_and(|number| self.stage(number))
    }

    fn stage(&self, number: String) -> bool {
        self.0.lock().is_ok_and(|mut pending| {
            *pending = Some(number);
            true
        })
    }

    fn take(&self) -> Option<String> {
        self.0.lock().ok()?.take()
    }
}

#[cfg(target_os = "macos")]
pub(crate) fn opened(app: &tauri::AppHandle, urls: &[tauri::Url]) {
    use tauri::Manager;

    if app.state::<TelLinkState>().stage_urls(urls) {
        show_pending_link(app);
    }
}

pub(crate) fn forward_launch(app: &tauri::AppHandle, args: &[String]) -> bool {
    use tauri::Manager;

    if !app.state::<TelLinkState>().stage_launch_args(args) {
        return false;
    }
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || show_pending_link(&handle));
    true
}

fn show_pending_link(app: &tauri::AppHandle) {
    use tauri::Emitter;

    let _ = crate::shell::show_phone(app);
    let _ = app.emit_to("main", "tel-link-pending", ());
}

/// Take the latest pending telephone number once. Returns null when none is pending.
#[tauri::command]
pub(crate) fn take_pending_tel_number(state: tauri::State<'_, TelLinkState>) -> Option<String> {
    state.take()
}

fn number_from_url(url: &tauri::Url) -> Option<String> {
    if url.scheme() != "tel"
        || !url.cannot_be_a_base()
        || url.query().is_some()
        || url.fragment().is_some()
        || url.path().len() > MAX_URL_PATH_BYTES
    {
        return None;
    }
    let mut bytes = url.path().bytes();
    let mut number = String::with_capacity(url.path().len());
    while let Some(byte) = bytes.next() {
        let byte = if byte == b'%' {
            let hex = [bytes.next()?, bytes.next()?];
            u8::from_str_radix(std::str::from_utf8(&hex).ok()?, 16).ok()?
        } else {
            byte
        };
        if !matches!(byte, b'0'..=b'9' | b'+' | b' ' | b'-' | b'.' | b'(' | b')') {
            return None;
        }
        number.push(char::from(byte));
    }
    phone_core::contact_dial_target(&number)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_links_keep_the_original_dialing_prefix() {
        for (url, expected) in [
            ("tel:%2B44%2020-7946-0018", "+442079460018"),
            ("TEL:0049(30)55501234", "00493055501234"),
            ("tel:100", "100"),
        ] {
            assert_eq!(
                number_from_url(&tauri::Url::parse(url).unwrap()).as_deref(),
                Some(expected)
            );
        }
    }

    #[test]
    fn unsupported_links_never_silently_change_the_destination() {
        for url in [
            "https://example.test/100",
            "tel://100",
            "tel:",
            "tel:100;ext=123",
            "tel:100%3Bphone-context=%2B49",
            "tel:100%0A",
            "tel:100?next=500",
            "tel:100#500",
            "tel:100%2F200",
            "tel:%2A21%23",
            "tel:%252B49100",
            "tel:%+100",
            "tel:+49(0)3055501234",
        ] {
            assert_eq!(
                number_from_url(&tauri::Url::parse(url).unwrap()),
                None,
                "{url}"
            );
        }
        assert_eq!(
            number_from_url(&tauri::Url::parse(&format!("tel:{}", "1".repeat(65))).unwrap()),
            None
        );
    }

    #[test]
    fn keeps_the_latest_valid_link_until_the_webview_takes_it() {
        let state = TelLinkState::default();
        let parse = |url: &str| tauri::Url::parse(url).unwrap();
        assert_eq!(state.take(), None);
        assert!(state.stage_urls(&[parse("tel:100")]));
        assert!(state.stage_urls(&[parse("tel:200"), parse("tel:300"), parse("tel:bad")]));
        assert!(!state.stage_urls(&[parse("tel:bad")]));
        assert_eq!(state.take().as_deref(), Some("300"));
        assert_eq!(state.take(), None);
    }

    #[test]
    fn launch_arguments_stage_only_an_explicit_single_telephone_link() {
        let state = TelLinkState::default();
        let launch = |args: &[&str]| {
            let args = args.iter().map(|arg| (*arg).to_owned()).collect::<Vec<_>>();
            state.stage_launch_args(&args)
        };
        assert!(launch(&["ruf", "--open-tel", "tel:%2B1-202-555-0142"]));
        assert!(!launch(&["ruf", "tel:100"]));
        assert!(!launch(&["ruf", "--open-tel"]));
        assert!(!launch(&["ruf", "--open-tel", "tel:100", "--autostart"]));
        assert!(!launch(&["ruf", "--autostart", "--open-tel", "tel:100"]));
        assert!(!launch(&["ruf", "--open-tel", "https://example.test"]));
        assert!(!launch(&["ruf", "--open-tel", "tel:10\t0"]));
        assert_eq!(state.take().as_deref(), Some("+12025550142"));
        assert!(launch(&["ruf", "--open-tel", "TEL:100"]));
        assert_eq!(state.take().as_deref(), Some("100"));
    }
}
