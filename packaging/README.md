# Packaging

`nix develop` is the development shell. Release builds use each platform's
native Rust toolchain (channel from `rust-toolchain.toml`) and do not go through
Nix. The Nix dev shell points macOS at Nix's libiconv, which is why release
binaries must be built outside it.

| Platform | Compile | Package |
|---|---|---|
| macOS arm64 | `cargo` + Apple SDK, `MACOSX_DEPLOYMENT_TARGET=13.0` | `package-macos.sh` → DMG |
| Linux x86_64 | `cargo` in the Steam Linux Runtime 3.0 (sniper) SDK | `package-linux.sh` → Steam + general `tar.xz` |
| Windows x64 | `cargo` + MSVC | `package-windows.sh` → zip |

Shared helpers: `version.sh` (artifact version, `--rust` for the channel),
`validate.sh` (linked-library checks), `smoke.sh` (bounded startup from `/tmp`;
it catches loader errors, not GPU problems).

## macOS arm64

Run from a normal terminal, not `nix develop`:

```bash
MACOSX_DEPLOYMENT_TARGET=13.0 cargo build -p maybraid --release --locked
packaging/scripts/package-macos.sh
```

`13.0` matches `LSMinimumSystemVersion`. `validate.sh macho` rejects links to
`/nix/store`, Homebrew, or `/usr/local`. Unsigned unless `SIGN_IDENTITY` is
set; see `identity.local.example.md`.

## Linux x86_64

Build once inside the pinned sniper SDK (same digest as CI):

```bash
docker run --rm -it -v "$PWD:/src" -w /src \
  registry.gitlab.steamos.cloud/steamrt/sniper/sdk@sha256:1c33c507bc75d012e77df5727f93b0d5b8c3f7c8d4142ba5f7a16882cc92e014 \
  bash -c 'curl -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain "$(packaging/version.sh --rust)" \
    && . "$HOME/.cargo/env" \
    && cargo build -p maybraid --release --locked \
    && packaging/scripts/package-linux.sh'
```

The same binary is packaged twice:

- `Maybraid-<ver>-steam-linux-x64.tar.xz` is for Steam depot staging.
  Steamworks must launch it with **Steam Linux Runtime 3.0 (sniper)**.
  Extracting the archive does not install that runtime.
- `Maybraid-<ver>-linux-x64.tar.xz` is the direct download. It needs glibc
  2.31+ (Ubuntu 20.04, Debian 11, SteamOS 3, or newer), ALSA, udev, and a
  Vulkan driver from the host.

`validate.sh elf` fails if the binary needs a glibc newer than 2.31 or points
at `/nix/store`.

## Windows x64

```bash
cargo build -p maybraid --release --locked --target x86_64-pc-windows-msvc
packaging/scripts/package-windows.sh
```

Uses the dynamic MSVC CRT. Install the
[VC++ 2015–2022 x64 redistributable](https://learn.microsoft.com/en-us/cpp/windows/latest-supported-vc-redist)
if `VCRUNTIME140.dll` is missing.

## CI

[`.github/workflows/package.yml`](../.github/workflows/package.yml) runs on
`main`, published releases, `workflow_dispatch`, and `ci-action::package`. Each
platform job runs the commands above. Published releases also get the
artifacts attached.

| Variable | Role |
|---|---|
| `MAYBRAID_ASSETS` | Bevy asset root |
| `MAYBRAID_SAVES` | Save directory |
| `MAYBRAID_PACKAGED` | Force user-data saves |
| `BINARY` | Override the binary a package script picks up |
| `VERSION` | Override the artifact version |
