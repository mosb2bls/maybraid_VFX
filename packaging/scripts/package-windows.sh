#!/usr/bin/env bash
# Zip an already-built MSVC maybraid.exe with assets. Does not compile.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
VERSION="${VERSION:-$("$ROOT/packaging/version.sh")}"
DIST="$ROOT/dist"
OUT="$DIST/Maybraid-${VERSION}-windows-x64"
ZIP="${OUT}.zip"
EXE="${BINARY:-$ROOT/target/x86_64-pc-windows-msvc/release/maybraid.exe}"

if [[ ! -f "$EXE" ]]; then
	echo "package-windows: $EXE not found" >&2
	echo "   cargo build -p maybraid --release --locked --target x86_64-pc-windows-msvc" >&2
	exit 1
fi

rm -rf "$OUT"
mkdir -p "$OUT/assets"
cp "$ROOT/packaging/windows/Maybraid/Maybraid.exe.manifest" "$OUT/"
cp "$EXE" "$OUT/Maybraid.exe"
if command -v ditto >/dev/null; then
	ditto "$ROOT/maybraid/assets" "$OUT/assets"
else
	cp -R "$ROOT/maybraid/assets/." "$OUT/assets/"
fi
cat > "$OUT/README-WINDOWS.txt" << EOF
Maybraid ${VERSION} for Windows x64

Needs the Microsoft Visual C++ Redistributable 2015–2022 (x64) if
VCRUNTIME140.dll is missing:
https://learn.microsoft.com/en-us/cpp/windows/latest-supported-vc-redist

Keep Maybraid.exe next to assets/. Saves: %APPDATA%\\Maybraid\\saves
EOF

rm -f "$ZIP"
if command -v ditto >/dev/null; then
	ditto -c -k --keepParent "$OUT" "$ZIP"
elif command -v zip >/dev/null; then
	(cd "$DIST" && zip -r "$(basename "$ZIP")" "$(basename "$OUT")")
elif command -v powershell.exe >/dev/null; then
	src="$OUT"
	dest="$ZIP"
	if command -v cygpath >/dev/null; then
		src="$(cygpath -w "$OUT")"
		dest="$(cygpath -w "$ZIP")"
	fi
	powershell.exe -NoProfile -Command \
		"Compress-Archive -LiteralPath '${src}' -DestinationPath '${dest}' -Force"
else
	echo "Need ditto, zip, or powershell.exe" >&2
	exit 1
fi
[[ -f "$ZIP" ]] || { echo "package-windows: zip missing" >&2; exit 1; }
echo "Created $ZIP"
