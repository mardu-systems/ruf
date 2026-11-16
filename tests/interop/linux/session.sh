#!/usr/bin/env bash
# Called only by scripts/check-linux.ts inside a private D-Bus/Xvfb session.
set -euo pipefail
mkdir -p "$XDG_CONFIG_HOME" "$XDG_DATA_HOME" "$XDG_RUNTIME_DIR/keyring"
chmod 700 "$XDG_RUNTIME_DIR" "$XDG_RUNTIME_DIR/keyring"
cleanup() {
  kill "${notification_pid:-}" "${pulse_pid:-}" "${keyring_pid:-}" 2>/dev/null || true
  wait 2>/dev/null || true
}
trap cleanup EXIT
printf 'ruf-synthetic-test-password' | gnome-keyring-daemon --foreground --unlock --components=secrets --control-directory="$XDG_RUNTIME_DIR/keyring" >"$XDG_RUNTIME_DIR/keyring.log" 2>&1 &
keyring_pid=$!
pulseaudio --daemonize=no --exit-idle-time=-1 --use-pid-file=no --disable-shm=yes -n \
  --load="module-native-protocol-unix socket=$XDG_RUNTIME_DIR/pulse-native auth-anonymous=1" \
  --load="module-null-sink sink_name=ruf_linux_sink" \
  --load="module-remap-source master=ruf_linux_sink.monitor source_name=ruf_linux_mic" >"$XDG_RUNTIME_DIR/pulse.log" 2>&1 &
pulse_pid=$!
python3 tests/interop/linux/notifications.py >"$XDG_RUNTIME_DIR/notifications.log" 2>&1 &
notification_pid=$!
for attempt in $(seq 1 100); do
  if pactl info >/dev/null 2>&1 && dbus-send --session --print-reply --dest=org.freedesktop.Notifications /org/freedesktop/Notifications org.freedesktop.Notifications.GetCapabilities >/dev/null 2>&1; then break; fi
  sleep 0.1
done
pactl info >/dev/null
dbus-send --session --print-reply --dest=org.freedesktop.Notifications /org/freedesktop/Notifications org.freedesktop.Notifications.GetCapabilities >/dev/null
cargo run --locked -p baresip-adapter --features native --example sound_smoke
cargo test --locked --workspace --features sip-desktop/native linux_ -- --ignored --skip without_session --test-threads=1
DBUS_SESSION_BUS_ADDRESS="unix:path=$XDG_RUNTIME_DIR/intentionally-missing-bus" RUF_LINUX_NO_SESSION=1 \
  cargo test --locked --workspace --features sip-desktop/native without_session -- --ignored --test-threads=1
