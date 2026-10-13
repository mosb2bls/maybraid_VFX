#!/usr/bin/env bash
# Dependency inspection, separate from assembly.
#   packaging/validate.sh macho <binary>
#   packaging/validate.sh elf <binary>
set -euo pipefail

usage() {
	echo "usage: packaging/validate.sh macho|elf <binary>" >&2
	exit 2
}

[[ $# -eq 2 ]] || usage
kind="$1"
bin="$2"

if [[ ! -f "$bin" ]]; then
	echo "validate: not found: $bin" >&2
	exit 1
fi

case "$kind" in
	macho)
		echo "==> Mach-O $bin"
		otool -L "$bin"
		if otool -L "$bin" | grep -Eq '/nix/store|/opt/homebrew|/usr/local/(lib|opt)'; then
			echo "validate: forbidden library path" >&2
			exit 1
		fi
		rpaths="$(otool -l "$bin" | awk '/cmd LC_RPATH/{p=1} p && /path /{print $2; p=0}')"
		if printf '%s\n' "$rpaths" | grep -Eq '/nix/store|/opt/homebrew|/usr/local|/Users/'; then
			echo "validate: forbidden rpath" >&2
			exit 1
		fi
		;;
	elf)
		echo "==> ELF $bin"
		interp="$(readelf -l "$bin" | sed -n 's/.*\[Requesting program interpreter: \(.*\)\]/\1/p')"
		echo "    interpreter: ${interp:-unknown}"
		if [[ -n "$interp" && "$interp" != /lib64/ld-linux-x86-64.so.2 ]]; then
			echo "validate: unexpected interpreter: $interp" >&2
			exit 1
		fi
		glibc="$(readelf -V "$bin" | grep -o 'GLIBC_[0-9.]*' | sed 's/GLIBC_//' | sort -Vu | tail -1)"
		echo "    newest glibc symbol: ${glibc:-none}"
		if [[ -n "$glibc" && "$(printf '%s\n2.31\n' "$glibc" | sort -V | tail -1)" != "2.31" ]]; then
			echo "validate: needs glibc $glibc; Steam Linux Runtime 3.0 provides 2.31" >&2
			exit 1
		fi
		if readelf -d "$bin" | grep -E 'RPATH|RUNPATH' | grep -Eq '/nix/store|/home/'; then
			echo "validate: forbidden rpath/runpath" >&2
			exit 1
		fi
		if command -v strings >/dev/null && strings "$bin" | grep -q '/nix/store'; then
			echo "validate: /nix/store reference in binary" >&2
			exit 1
		fi
		if command -v ldd >/dev/null; then
			ldd "$bin" || true
		fi
		;;
	*)
		usage
		;;
esac
