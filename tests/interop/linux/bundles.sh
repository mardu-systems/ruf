#!/usr/bin/env bash
# Verify the real packages, including the tray library loaded dynamically by Tauri.
set -euo pipefail
target=${1:?Pass the native Linux Rust target}
case "$target" in
  x86_64-unknown-linux-gnu) deb_arch=amd64; rpm_arch=x86_64 ;;
  aarch64-unknown-linux-gnu) deb_arch=arm64; rpm_arch=aarch64 ;;
  *) echo "Unsupported Linux package target: $target"; exit 1 ;;
esac
bundle="target/$target/release/bundle"
shopt -s nullglob
deb=("$bundle"/deb/*.deb)
rpm=("$bundle"/rpm/*.rpm)
appimage=("$bundle"/appimage/*.AppImage)
[[ ${#deb[@]} == 1 && ${#rpm[@]} == 1 && ${#appimage[@]} == 1 ]]
[[ $(dpkg-deb -f "${deb[0]}" Architecture) == "$deb_arch" ]]
[[ $(rpm -qp --qf '%{ARCH}' "${rpm[0]}") == "$rpm_arch" ]]
dpkg-deb -f "${deb[0]}" Depends | grep -Fq libayatana-appindicator3-1
rpm -qpR "${rpm[0]}" | grep -Fxq 'libayatana-appindicator3.so.1()(64bit)'
rpm -qpR "${rpm[0]}" | grep -Fxq 'libpulse.so.0()(64bit)'
image=$(realpath "${appimage[0]}")
temporary=$(mktemp -d)
trap 'rm -rf "$temporary"' EXIT
(cd "$temporary" && "$image" --appimage-extract >/dev/null)
tray=$(find "$temporary/squashfs-root" -name 'libayatana-appindicator3.so.1' -print -quit)
[[ -n "$tray" && -f "$tray" ]]
echo "LINUX_BUNDLES_OK: $target package architectures and tray/audio dependencies verified"
