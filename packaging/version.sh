#!/usr/bin/env bash
# Shared package version (or --rust for rust-toolchain.toml).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"

if [[ "${1:-}" == "--rust" ]]; then
	awk -F'"' '/^channel[[:space:]]*=/{print $2; exit}' "$ROOT/rust-toolchain.toml"
	exit 0
fi

workspace_version=$(awk '
	$0 == "[workspace.package]" { hit = 1; next }
	hit && /^\[/ { exit }
	hit && $1 == "version" {
		gsub(/"/, "", $3)
		print $3
		exit
	}
' "$ROOT/Cargo.toml")

if [[ -z "${workspace_version:-}" ]]; then
	echo "could not read workspace.package.version from Cargo.toml" >&2
	exit 1
fi

if [[ "${GITHUB_EVENT_NAME:-}" == "release" ]]; then
	v="${GITHUB_REF_NAME:-}"
	v="${v#v}"
	printf '%s\n' "$v"
elif [[ -n "${GITHUB_SHA:-}" ]]; then
	printf '%s-%s\n' "$workspace_version" "${GITHUB_SHA:0:7}"
else
	printf '%s\n' "$workspace_version"
fi
