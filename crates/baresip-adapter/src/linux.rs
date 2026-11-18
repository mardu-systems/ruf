//! GTK clipboard integration on the application's existing Linux main loop.
use phone_core::{ErrorCode, PhoneError};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::Duration,
};

pub(super) fn copy_text(text: &str) -> Result<(), PhoneError> {
    let text = text.to_owned();
    let cancelled = Arc::new(AtomicBool::new(false));
    let pending = Arc::clone(&cancelled);
    let (sender, receiver) = mpsc::sync_channel(1);
    gtk::glib::idle_add_once(move || {
        if pending.load(Ordering::Acquire) {
            return;
        }
        let result = if gtk::gdk::Display::default().is_some() {
            gtk::Clipboard::get(&gtk::gdk::SELECTION_CLIPBOARD).set_text(&text);
            Ok(())
        } else {
            Err(failure())
        };
        let _ = sender.send(result);
    });
    match receiver.recv_timeout(Duration::from_secs(3)) {
        Ok(result) => result,
        Err(_) => {
            cancelled.store(true, Ordering::Release);
            Err(failure())
        }
    }
}

fn failure() -> PhoneError {
    PhoneError::new(
        ErrorCode::NativeFailure,
        "The system clipboard is unavailable.",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn glib_variant_string_iteration_preserves_both_directions_and_borrowed_text() {
        use gtk::glib::variant::ToVariant;
        let texts = ["", "Ruf", "Grüße", "☎"];
        let variant = texts.to_variant();
        let iter = || variant.array_iter_str().unwrap();
        assert_eq!(iter().collect::<Vec<_>>(), texts);
        assert_eq!(
            iter().rev().collect::<Vec<_>>(),
            texts.into_iter().rev().collect::<Vec<_>>()
        );
        let mut mixed = iter();
        assert_eq!(mixed.next_back(), Some("☎"));
        assert_eq!(mixed.next(), Some(""));
        assert_eq!(mixed.next_back(), Some("Grüße"));
        assert_eq!(mixed.next(), Some("Ruf"));
        assert_eq!(mixed.next(), None);
        assert_eq!(mixed.next_back(), None);
        assert_eq!(iter().nth(2), Some("Grüße"));
        assert_eq!(iter().nth_back(2), Some("Ruf"));
        assert_eq!(iter().last(), Some("☎"));
        let empty = Vec::<String>::new().to_variant();
        assert_eq!(empty.array_iter_str().unwrap().next_back(), None);
    }
    #[test]
    #[ignore = "requires the isolated X11 session started by test:linux"]
    fn linux_clipboard_uses_the_running_gtk_main_loop() {
        assert_eq!(std::env::var("RUF_LINUX_TEST_SESSION").as_deref(), Ok("1"));
        gtk::init().unwrap();
        let worker = std::thread::spawn(|| copy_text("+49 30 12345"));
        let deadline = std::time::Instant::now() + Duration::from_secs(4);
        while !worker.is_finished() {
            assert!(std::time::Instant::now() < deadline);
            gtk::glib::MainContext::default().iteration(false);
            std::thread::sleep(Duration::from_millis(1));
        }
        worker.join().unwrap().unwrap();
        let clipboard = gtk::Clipboard::get(&gtk::gdk::SELECTION_CLIPBOARD);
        assert_eq!(clipboard.wait_for_text().as_deref(), Some("+49 30 12345"));
        clipboard.clear();
    }

    #[cfg(feature = "native")]
    #[test]
    #[ignore = "requires the isolated audio server started by test:linux"]
    fn linux_audio_inventory_and_capture_use_the_baresip_backend() {
        use crate::Engine;
        use phone_core::{AudioMode, AudioTestKind};
        assert_eq!(std::env::var("RUF_LINUX_TEST_SESSION").as_deref(), Ok("1"));
        let devices = crate::audio_devices().unwrap();
        assert!(
            devices
                .outputs
                .iter()
                .any(|device| device.id == "ruf_linux_sink")
        );
        assert!(
            devices
                .inputs
                .iter()
                .any(|device| device.id == "ruf_linux_mic")
        );
        assert!(
            !devices
                .inputs
                .iter()
                .any(|device| device.id.ends_with(".monitor"))
        );
        let mut engine = Engine::start(AudioMode::System, None).unwrap();
        let owner = engine
            .start_audio_test(AudioTestKind::Microphone, "ruf_linux_mic")
            .unwrap();
        assert_eq!(engine.snapshot().unwrap().audio_test.unwrap().id, owner);
        engine.stop_audio_test(&owner).unwrap();
        assert!(engine.snapshot().unwrap().audio_test.is_none());
        let speaker = engine
            .start_audio_test(AudioTestKind::Speaker, "ruf_linux_sink")
            .unwrap();
        engine.stop_audio_test(&speaker).unwrap();
        assert!(
            engine
                .start_audio_test(AudioTestKind::Speaker, "missing-linux-device")
                .is_err()
        );

        // A fixed input must fail closed when removed, then work again under
        // the same ID without replacing the engine or changing saved routing.
        let load_module = |arguments: &[&str]| {
            let output = std::process::Command::new("pactl")
                .arg("load-module")
                .args(arguments)
                .output()
                .unwrap();
            assert!(output.status.success(), "{output:?}");
            String::from_utf8(output.stdout)
                .unwrap()
                .trim()
                .parse::<u32>()
                .unwrap()
        };
        let load_input = || {
            load_module(&[
                "module-remap-source",
                "master=ruf_linux_sink.monitor",
                "source_name=ruf_linux_replug_mic",
            ])
        };
        let module = load_input();
        let settings = phone_core::SoundSettings {
            input_device: "ruf_linux_replug_mic".into(),
            ..Default::default()
        };
        engine.set_sound_settings(&settings).unwrap();
        let removed_owner = engine
            .start_audio_test(AudioTestKind::Microphone, "ruf_linux_replug_mic")
            .unwrap();
        assert!(
            std::process::Command::new("pactl")
                .args(["unload-module", &module.to_string()])
                .status()
                .unwrap()
                .success()
        );
        let deadline = std::time::Instant::now() + Duration::from_secs(4);
        loop {
            let state = engine.snapshot().unwrap();
            assert_eq!(state.sound_settings, settings);
            if state.audio_test.is_none() {
                assert!(state.audio_test_error.is_some());
                break;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "Removing a fixed microphone must stop capture, not move it to another input"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(
            engine
                .start_audio_test(AudioTestKind::Microphone, "ruf_linux_replug_mic")
                .is_err()
        );
        let module = load_input();
        let recovered_owner = engine
            .start_audio_test(AudioTestKind::Microphone, "ruf_linux_replug_mic")
            .expect("Reconnected microphone must work without restarting Ruf");
        assert_ne!(recovered_owner, removed_owner);
        engine.stop_audio_test(&removed_owner).unwrap();
        assert_eq!(
            engine.snapshot().unwrap().audio_test.unwrap().id,
            recovered_owner,
            "An old owner must not stop the recovered capture"
        );
        engine.stop_audio_test(&recovered_owner).unwrap();
        assert!(
            std::process::Command::new("pactl")
                .args(["unload-module", &module.to_string()])
                .status()
                .unwrap()
                .success()
        );
        assert_eq!(engine.snapshot().unwrap().sound_settings, settings);

        let load_output = || load_module(&["module-null-sink", "sink_name=ruf_linux_replug_sink"]);
        let module = load_output();
        let settings = phone_core::SoundSettings {
            output_device: "ruf_linux_replug_sink".into(),
            ring_device: "ruf_linux_replug_sink".into(),
            ..Default::default()
        };
        engine.set_sound_settings(&settings).unwrap();
        let removed_owner = engine
            .start_audio_test(AudioTestKind::Speaker, "ruf_linux_replug_sink")
            .unwrap();
        assert!(
            std::process::Command::new("pactl")
                .args(["unload-module", &module.to_string()])
                .status()
                .unwrap()
                .success()
        );
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        loop {
            let state = engine.snapshot().unwrap();
            if state.audio_test.is_none() {
                assert!(state.audio_test_error.is_some());
                break;
            }
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(10));
        }
        let module = load_output();
        let recovered_owner = engine
            .start_audio_test(AudioTestKind::Speaker, "ruf_linux_replug_sink")
            .expect("Reconnected speaker must work without restarting Ruf");
        assert_ne!(recovered_owner, removed_owner);
        engine.stop_audio_test(&recovered_owner).unwrap();
        engine.preview_ringtone("ruf_linux_replug_sink").unwrap();
        engine.stop_ringtone_preview().unwrap();
        assert_eq!(engine.snapshot().unwrap().sound_settings, settings);
        assert!(
            std::process::Command::new("pactl")
                .args(["unload-module", &module.to_string()])
                .status()
                .unwrap()
                .success()
        );

        let _owner = engine
            .start_audio_test(AudioTestKind::Microphone, "ruf_linux_mic")
            .unwrap();
        assert!(
            std::process::Command::new("pactl")
                .arg("exit")
                .status()
                .unwrap()
                .success()
        );
        let deadline = std::time::Instant::now() + Duration::from_secs(4);
        loop {
            let state = engine.snapshot().unwrap();
            if state.audio_test.is_none() {
                assert!(state.audio_test_error.is_some());
                break;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "Audio-server loss must terminate capture with an observed error"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
        let started = std::time::Instant::now();
        assert!(crate::audio_devices().is_err());
        assert!(started.elapsed() < Duration::from_secs(4));

        // Restart only the private fixture server; keep this same Engine alive.
        struct PrivateServer(std::process::Child);
        impl Drop for PrivateServer {
            fn drop(&mut self) {
                let _ = self.0.kill();
                let _ = self.0.wait();
            }
        }
        let socket = std::env::var("PULSE_SERVER").unwrap();
        let socket = socket.strip_prefix("unix:").unwrap();
        let mut server = PrivateServer(
            std::process::Command::new("pulseaudio")
                .args([
                    "--daemonize=no",
                    "--exit-idle-time=-1",
                    "--use-pid-file=no",
                    "--disable-shm=yes",
                    "-n",
                    &format!("--load=module-native-protocol-unix socket={socket} auth-anonymous=1"),
                    "--load=module-null-sink sink_name=ruf_linux_sink",
                    "--load=module-remap-source master=ruf_linux_sink.monitor source_name=ruf_linux_mic",
                ])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
                .unwrap(),
        );
        let deadline = std::time::Instant::now() + Duration::from_secs(8);
        let recovered_owner = loop {
            assert!(
                server.0.try_wait().unwrap().is_none(),
                "Private audio server exited"
            );
            if let Ok(owner) = engine.start_audio_test(AudioTestKind::Microphone, "ruf_linux_mic") {
                break owner;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "Audio must recover after its server restarts without recreating the engine"
            );
            std::thread::sleep(Duration::from_millis(20));
        };
        engine.stop_audio_test(&recovered_owner).unwrap();
        assert_eq!(engine.snapshot().unwrap().sound_settings, settings);
        engine.shutdown().unwrap();
    }
}
