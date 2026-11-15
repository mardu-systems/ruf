#!/usr/bin/env bash
# Run under xvfb-run -a dbus-run-session; never use the current desktop's data.
set -euo pipefail
binary=$(realpath "${1:?Pass the built Ruf executable}")
task_directory=$(mktemp -d)
trap 'rm -rf "$task_directory"' EXIT
export XDG_DATA_HOME="$task_directory/data"
export XDG_CONFIG_HOME="$task_directory/config"
export XDG_CACHE_HOME="$task_directory/cache"
export XDG_RUNTIME_DIR="$task_directory"
mkdir -p "$XDG_DATA_HOME" "$XDG_CONFIG_HOME" "$XDG_CACHE_HOME"
timeout 12s "$binary" >"$task_directory/startup.log" 2>&1 &
application_pid=$!
visible=false
for attempt in $(seq 1 80); do
  if xwininfo -root -tree | grep -q '"Ruf"'; then visible=true; break; fi
  if ! kill -0 "$application_pid" 2>/dev/null; then break; fi
  sleep 0.1
done
result=0
wait "$application_pid" || result=$?
if [[ "$visible" != true || "$result" != 124 ]]; then
  cat "$task_directory/startup.log"
  echo "Linux startup failed: visible=$visible exit=$result"
  exit 1
fi
echo "LINUX_STARTUP_OK: Ruf window appeared and the desktop stayed running"
