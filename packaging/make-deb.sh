#!/bin/sh
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
version=0.1.0
stage=$(mktemp -d)
trap 'rm -rf "$stage"' EXIT
bin="$root/target/release/shadow-creator-studio"
if [ ! -x "$bin" ]; then
  echo "Build the release binary first: cargo build -p scs-ui --release" >&2
  exit 1
fi
mkdir -p "$stage/DEBIAN" \
  "$stage/usr/bin" \
  "$stage/usr/share/applications" \
  "$stage/usr/share/icons/hicolor/scalable/apps"
cat >"$stage/DEBIAN/control" <<EOF
Package: shadow-creator-studio
Version: $version
Section: video
Priority: optional
Architecture: amd64
Depends: libc6, libgtk-4-1, libadwaita-1-0, ffmpeg, gstreamer1.0-tools, gstreamer1.0-plugins-good, gstreamer1.0-plugins-base, pipewire
Recommends: libsecret-tools, gstreamer1.0-plugins-bad, gstreamer1.0-pipewire
Maintainer: Shadowfetch <dev@example.invalid>
Description: Linux creator studio for screen, camera, and YouTube-quality recording
EOF
cp "$bin" "$stage/usr/bin/shadow-creator-studio"
cp "$root/packaging/com.shadowfetch.creatorstudio.desktop" "$stage/usr/share/applications/"
cp "$root/packaging/com.shadowfetch.creatorstudio.svg" \
  "$stage/usr/share/icons/hicolor/scalable/apps/com.shadowfetch.creatorstudio.svg"
mkdir -p "$root/dist"
out="$root/dist/shadow-creator-studio_${version}_amd64.deb"
dpkg-deb --build "$stage" "$out"
echo "Wrote $out"
