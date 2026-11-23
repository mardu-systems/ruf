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
dpkg-deb -x "${deb[0]}" "$temporary/deb"
for package in "$temporary/deb" "$temporary/squashfs-root"; do
  desktop="$package/usr/share/applications/Ruf.desktop"
  [[ -f "$desktop" ]]
  grep -Fxq 'Icon=sip-desktop' "$desktop"
  grep -Fxq 'Exec=sip-desktop --open-tel %u' "$desktop"
  grep -Fxq 'MimeType=x-scheme-handler/tel;' "$desktop"
  if grep -Fxq 'NoDisplay=true' "$desktop"; then
    echo "Visible Ruf launcher is hidden: $desktop"
    exit 1
  fi
  [[ -f "$package/usr/share/icons/hicolor/128x128/apps/sip-desktop.png" ]]
done
cmp apps/desktop/src-tauri/linux/sip-desktop.desktop \
  "$temporary/deb/usr/share/applications/sip-desktop.desktop"
grep -Fxq 'NoDisplay=true' "$temporary/deb/usr/share/applications/sip-desktop.desktop"
cmp apps/desktop/src-tauri/linux/ruf.desktop \
  "$temporary/squashfs-root/sip-desktop.desktop"
if grep -Fxq 'NoDisplay=true' "$temporary/squashfs-root/sip-desktop.desktop"; then
  echo "AppImage Ruf launcher is hidden"
  exit 1
fi
rpm_manifest=$(rpm -qp --qf '[%{FILENAMES} %{FILEDIGESTS}\n]' "${rpm[0]}")
for entry in 'Ruf.desktop:ruf.desktop' 'sip-desktop.desktop:sip-desktop.desktop'; do
  desktop_digest=$(sha256sum "apps/desktop/src-tauri/linux/${entry#*:}" | cut -d ' ' -f 1)
  grep -Fxq "/usr/share/applications/${entry%:*} $desktop_digest" <<<"$rpm_manifest"
done
tray=$(find "$temporary/squashfs-root" -name 'libayatana-appindicator3.so.1' -print -quit)
[[ -n "$tray" && -f "$tray" ]]
echo "LINUX_BUNDLES_OK: $target architectures, telephone links, icon aliases and tray/audio dependencies verified"
