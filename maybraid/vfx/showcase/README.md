# Maybraid — Milestone 2.1 Review

Technical dust samples and initial wind/dust studies. Ready for review on Apple Silicon Mac. Windows validation is pending.

## Watch

Open [the gallery](gallery/index.html) in a web browser for matched B+ and C previews.

- **B+** — eight-view animated flipbook. Lower rendering cost; view blending can soften contours or produce ghosting. Best near the authored 20° elevation.
- **C** — animated 3D density. Supports close and inside views, at higher rendering cost.

## Download the Mac executable

The executable is attached to the [Milestone 2.1 showcase release](https://github.com/mosb2bls/maybraid_VFX/releases/tag/showcase-2.1) because GitHub does not permit new Git LFS uploads to public forks.

Download the `matched_dust` asset and place it at `runtime/matched_dust` inside this showcase folder. Then, from this folder, run:

```sh
chmod +x runtime/matched_dust
shasum -a 256 runtime/matched_dust
```

Expected SHA-256: `7279851373ffcab76c36741ae06a89add33d49509d1c934e1236208cf12e6286`.

With GitHub CLI, you can download it directly from this folder:

```sh
gh release download showcase-2.1 --repo mosb2bls/maybraid_VFX --pattern matched_dust --dir runtime
chmod +x runtime/matched_dust
```

The executable is not included in the repository ZIP or clone. All other showcase files are included here.

## Run on Mac

Double-click `Launch-Bplus.command` or `Launch-C.command`. No build or Houdini installation is needed. Close one demo before opening another.

For an orbit comparison, run from this folder:

```sh
./Launch-Bplus.command --view orbit --inspect-time 0.7
./Launch-C.command --view orbit --inspect-time 0.7
```

For the lower-cost C setting:

```sh
./Launch-C.command --view orbit --inspect-time 0.7 --profile lower
```

Click the window to focus it. **Space** pauses/resumes, **R** restarts, and **Left/Right** steps while paused. Camera paths are preset. `--inspect-time 0.7` holds the dust at that time while the camera moves.

## Build from source

`source/changed-files/` contains the four demo implementation files. `source/milestone21.patch` adds those files and their dependency to the Maybraid repository; unrelated prototype code is excluded. This is an example within Maybraid, not a standalone crate. Repository access is required.

Prerequisites: Git, Rust/Cargo **1.95.0**, and the platform build tools. Windows requires the Visual Studio C++ build tools and Windows SDK. The first build needs internet access for dependencies.

On macOS or Linux, start in this review folder:

```sh
REVIEW_DIR="$PWD"
git clone https://github.com/mosb2bls/maybraid.git ../maybraid-m21-build
cd ../maybraid-m21-build
git checkout --detach fde58b164868e6264224745f8db1278c00bf2d5c
git apply --check "$REVIEW_DIR/source/milestone21.patch"
git apply "$REVIEW_DIR/source/milestone21.patch"
cargo build --locked --release -p vfx-playground --example matched_dust
./target/release/examples/matched_dust 'B+' --data-root "$REVIEW_DIR/data" --view orbit --inspect-time 0.7
./target/release/examples/matched_dust C --data-root "$REVIEW_DIR/data" --view entry --inspect-time 0.7
```

Run one demo at a time. The source build reads the supplied assets directly; no Houdini installation or asset export is needed.

On Windows, use PowerShell from this review folder:

```powershell
$ReviewDir = (Get-Location).Path
git clone https://github.com/mosb2bls/maybraid.git ..\maybraid-m21-build
Set-Location ..\maybraid-m21-build
git checkout --detach fde58b164868e6264224745f8db1278c00bf2d5c
git apply --check "$ReviewDir\source\milestone21.patch"
git apply "$ReviewDir\source\milestone21.patch"
cargo build --locked --release -p vfx-playground --example matched_dust
.\target\release\examples\matched_dust.exe 'B+' --data-root "$ReviewDir\data" --view orbit --inspect-time 0.7
.\target\release\examples\matched_dust.exe C --data-root "$ReviewDir\data" --view entry --inspect-time 0.7
```

Windows and Linux execution remain unverified; the bundled executable is for Apple Silicon Mac only.

## Results and scope

Mac validation: M2 Pro, 16 GB, 2560×1440. Release checks passed with 57 tests. Representative whole-frame observations:

| View, one effect | B+ | C | C lower |
|---|---:|---:|---:|
| Medium | 3.29 ms | 8.04 ms | 5.22 ms |
| Close orbit | 2.93 ms | 17.24 ms | 9.96 ms |

These are preliminary desktop measurements, not final game-performance budgets. Gallery clips are encoded Bevy render sequences, not real-time screen recordings.

Both samples use simplified unlit shading and approximate local disturbance. Obstacle masks remove coverage/density; they do not simulate flow or sliding. Windows validation and agreement on boundary behavior remain open. Wind images are static studies; a runtime wind effect is not included.

This package contains the Mac executable, runtime assets, selected previews, and the Bevy demo source. Houdini projects, authoring tools, and simulation caches are not included.
