#!/usr/bin/env bash
# Two tar.xz archives from one Linux binary built in the sniper SDK:
#   Maybraid-<version>-steam-linux-x64.tar.xz   Steam depot staging
#   Maybraid-<version>-linux-x64.tar.xz         direct download
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
VERSION="${VERSION:-$("$ROOT/packaging/version.sh")}"
BIN="${BINARY:-$ROOT/target/release/maybraid}"
DIST="$ROOT/dist"

if [[ ! -f "$BIN" ]]; then
	echo "package-linux: binary not found: $BIN" >&2
	echo "   Build inside the sniper SDK; see packaging/README.md" >&2
	exit 1
fi
"$ROOT/packaging/validate.sh" elf "$BIN"

stage() {
	local out="$1"
	rm -rf "$out"
	mkdir -p "$out/assets"
	cp "$BIN" "$out/maybraid"
	chmod +x "$out/maybraid"
	cp -R "$ROOT/maybraid/assets/." "$out/assets/"
	cp "$ROOT/packaging/steamos/Maybraid/maybraid.desktop" "$out/"
}

archive() {
	local out="$1"
	rm -f "$out.tar.xz"
	tar -C "$DIST" -cJf "$out.tar.xz" "$(basename "$out")"
	echo "Created $out.tar.xz"
}

mkdir -p "$DIST"

steam="$DIST/Maybraid-${VERSION}-steam-linux-x64"
stage "$steam"
if [[ -f "$ROOT/packaging/steamos/Maybraid/steam_appid.txt" ]]; then
	cp "$ROOT/packaging/steamos/Maybraid/steam_appid.txt" "$steam/"
fi
cat > "$steam/README.txt" << EOF
Maybraid ${VERSION}: Steam depot (Linux x86_64)

Steamworks must launch this build with Steam Linux Runtime 3.0 (sniper).
Extracting the archive does not install or activate that runtime.
EOF
archive "$steam"

linux="$DIST/Maybraid-${VERSION}-linux-x64"
stage "$linux"
cat > "$linux/README.txt" << EOF
Maybraid ${VERSION}: Linux x86_64

Extract anywhere and run ./maybraid. Keep it next to assets/.
Requires glibc 2.31 or newer (Ubuntu 20.04, Debian 11, SteamOS 3, or newer),
ALSA, udev, and a Vulkan driver.
Saves: ~/.local/share/maybraid/saves
EOF
archive "$linux"
