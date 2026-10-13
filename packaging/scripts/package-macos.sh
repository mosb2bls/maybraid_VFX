#!/usr/bin/env bash
# Assemble Maybraid.app and a UDZO DMG. Does not compile.
#
#   MACOSX_DEPLOYMENT_TARGET=13.0 cargo build -p maybraid --release --locked
#   packaging/scripts/package-macos.sh
#
# Unsigned unless SIGN_IDENTITY is set. SKIP_NOTARY=1 signs only.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
TEMPLATE="$ROOT/packaging/macos/Maybraid.app"
ENTITLEMENTS="$ROOT/packaging/macos/entitlements.plist"
ASSETS="$ROOT/maybraid/assets"
ICON_SRC="$ROOT/maybraid/assets/iconography/maybraid_logo_icon_home.png"
VERSION="${VERSION:-$("$ROOT/packaging/version.sh")}"
DIST="$ROOT/dist"
APP="$DIST/Maybraid.app"
DMG_STAGE="$DIST/dmg-root"
DMG="$DIST/Maybraid-${VERSION}-macos-arm64.dmg"
BINARY="${BINARY:-$ROOT/target/release/maybraid}"

if [[ "$(uname -s)" != "Darwin" ]]; then
	echo "package-macos.sh must run on macOS." >&2
	exit 1
fi
if [[ ! -f "$BINARY" ]]; then
	echo "package-macos: binary not found: $BINARY" >&2
	echo "   Build outside nix develop; see packaging/README.md" >&2
	exit 1
fi
if [[ ! -d "$TEMPLATE" || ! -d "$ASSETS" ]]; then
	echo "Missing template or assets." >&2
	exit 1
fi

"$ROOT/packaging/validate.sh" macho "$BINARY"

echo "==> Staging app bundle"
rm -rf "$DIST"
mkdir -p "$DIST"
/usr/bin/ditto "$TEMPLATE" "$APP"
rm -f "$APP/Contents/MacOS/.gitkeep"
rm -rf "$APP/Contents/Resources/assets"
/usr/bin/ditto "$BINARY" "$APP/Contents/MacOS/maybraid"
chmod +x "$APP/Contents/MacOS/maybraid"
/usr/bin/ditto "$ASSETS" "$APP/Contents/Resources/assets"

if [[ -x /usr/libexec/PlistBuddy ]]; then
	/usr/libexec/PlistBuddy -c "Set :CFBundleVersion ${VERSION}" "$APP/Contents/Info.plist"
	/usr/libexec/PlistBuddy -c "Set :CFBundleShortVersionString ${VERSION}" "$APP/Contents/Info.plist"
fi

if [[ -f "$ICON_SRC" ]] && command -v sips >/dev/null && command -v iconutil >/dev/null; then
	echo "==> AppIcon.icns"
	iconset="$DIST/Maybraid.iconset"
	rm -rf "$iconset"
	mkdir -p "$iconset"
	for sz in 16 32 128 256 512; do
		sips -z "$sz" "$sz" "$ICON_SRC" --out "$iconset/icon_${sz}x${sz}.png" >/dev/null
		sips -z "$((sz * 2))" "$((sz * 2))" "$ICON_SRC" --out "$iconset/icon_${sz}x${sz}@2x.png" >/dev/null
	done
	iconutil -c icns "$iconset" -o "$APP/Contents/Resources/AppIcon.icns"
	rm -rf "$iconset"
fi

"$ROOT/packaging/validate.sh" macho "$APP/Contents/MacOS/maybraid"

if [[ -n "${SIGN_IDENTITY:-}" ]]; then
	echo "==> Signing"
	/usr/bin/codesign --force --options runtime --timestamp \
		--entitlements "$ENTITLEMENTS" \
		--sign "$SIGN_IDENTITY" \
		"$APP/Contents/MacOS/maybraid"
	/usr/bin/codesign --force --options runtime --timestamp \
		--entitlements "$ENTITLEMENTS" \
		--sign "$SIGN_IDENTITY" \
		"$APP"
	/usr/bin/codesign --verify --deep --strict --verbose=2 "$APP"
else
	echo "==> Unsigned (set SIGN_IDENTITY to Developer ID Application: …)"
fi

hdiutil_detach_maybraid() {
	/usr/bin/hdiutil detach "/Volumes/Maybraid" -force 2>/dev/null || true
	/usr/bin/hdiutil detach "Maybraid" -force 2>/dev/null || true
}

hdiutil_detach_dist_images() {
	local info image_path dev
	info="$(/usr/bin/hdiutil info 2>/dev/null || true)"
	image_path=""
	while IFS= read -r line; do
		if [[ "$line" =~ ^[[:space:]]*image-path:[[:space:]]*(.+)$ ]]; then
			image_path="${BASH_REMATCH[1]}"
		elif [[ -n "$image_path" && "$line" =~ ^(/dev/disk[0-9]+) ]]; then
			if [[ "$image_path" == "$DIST/"* ]]; then
				dev="${BASH_REMATCH[1]}"
				/usr/bin/hdiutil detach "$dev" -force 2>/dev/null \
					|| /usr/bin/hdiutil detach "$image_path" -force 2>/dev/null \
					|| true
			fi
			image_path=""
		fi
	done <<< "$info"
}

hdiutil_cleanup_before_retry() {
	hdiutil_detach_maybraid
	hdiutil_detach_dist_images
	if [[ -f "$DMG" ]]; then
		/usr/bin/hdiutil detach "$DMG" -force 2>/dev/null || true
	fi
	rm -f "$DMG"
}

hdiutil_busy_error() {
	case "$1" in
		*"Resource busy"*|*"Device busy"*|*"resource temporarily unavailable"*)
			return 0
			;;
	esac
	return 1
}

create_udzo_dmg() {
	local attempt output status max_attempts=5
	sync
	for (( attempt=1; attempt<=max_attempts; attempt++ )); do
		output="$(/usr/bin/hdiutil create \
			-volname "Maybraid" \
			-srcfolder "$DMG_STAGE" \
			-ov \
			-format UDZO \
			"$DMG" 2>&1)" && return 0
		status=$?
		if hdiutil_busy_error "$output" && (( attempt < max_attempts )); then
			echo "hdiutil create busy (attempt ${attempt}/${max_attempts}), retrying..." >&2
			printf '%s\n' "$output" >&2
			hdiutil_cleanup_before_retry
			sync
			sleep $((attempt * 2))
			continue
		fi
		printf '%s\n' "$output" >&2
		return "$status"
	done
}

echo "==> DMG"
rm -rf "$DMG_STAGE"
mkdir -p "$DMG_STAGE"
/usr/bin/ditto "$APP" "$DMG_STAGE/Maybraid.app"
ln -s /Applications "$DMG_STAGE/Applications"
create_udzo_dmg

if [[ -n "${SIGN_IDENTITY:-}" ]]; then
	/usr/bin/codesign --timestamp --sign "$SIGN_IDENTITY" "$DMG"
	if [[ "${SKIP_NOTARY:-}" != "1" && -n "${NOTARY_PROFILE:-}" ]]; then
		echo "==> Notarizing"
		/usr/bin/xcrun notarytool submit "$DMG" --keychain-profile "$NOTARY_PROFILE" --wait
		/usr/bin/xcrun stapler staple "$DMG"
		/usr/sbin/spctl --assess --type open --context context:primary-signature -v "$DMG"
	fi
fi

echo "Created $APP"
echo "Created $DMG"
