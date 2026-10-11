# Doric column POM UV / tangent contract

Copy this section into the [#1133](https://github.com/ramate-io/maybraid/issues/1133) column/pedestal GLB authoring spec. The shader is [`column_surface.wgsl`](src/column_surface.wgsl). Placeholders that already meet the contract: [`PlaceholderShaft`](src/column_surface/shaft.rs) (compact cylinder) and [`placeholder_tapered_shaft_mesh`](src/column_surface/shaft.rs) (unit-D taper).

**Kit-level source** for this convention is [#1144](https://github.com/ramate-io/maybraid/pull/1144) [`AUTHORING.md`](../../../mobs/architectural/doric-column/AUTHORING.md) (`maybraid/world/layers/mobs/architectural/doric-column/AUTHORING.md`). This file is the POM / UV / texel addendum for the shaft only.

POM owns **surface relief only**. Shaft outline, capital, pedestal, and cover collision stay real geometry. The material never displaces the silhouette and must not be treated as physical cover.

## Authored unit and runtime scale

Convention shared with [#1144](https://github.com/ramate-io/maybraid/pull/1144) [`AUTHORING.md`](../../../mobs/architectural/doric-column/AUTHORING.md) / `DoricColumnRatios`:

- **Authored D = 1.** Every part is modeled with **lower shaft diameter = 1**. Proportions are real D-relative numbers in that unit mesh.
- **Shaft height and neck** are [`DoricColumnRatios::shaft_height_d`](https://github.com/ramate-io/maybraid/pull/1144) / `shaft_upper_diameter_d` — do not keep a second copy here. U still wraps once while circumference shrinks up V.
- Full column, square plinth, and square abacus are also those ratios. Plinth and abacus are real geometry, not this material.
- **Pivot:** bottom-center on the shaft axis, **+Y up**, mesh `Y ∈ [0, H]`.
- **Runtime:** one **uniform scale `s = D`**, where D is the gameplay lower-shaft diameter in metres. Do not split XZ-diameter vs Y-height — that shears U vs V and the flute Jacobian.

There is no `s = D_game / D_authored`. The GLB is already D = 1; the instance scale *is* D.

## Mesh islands

| Part | Geometry | This material? |
|------|----------|----------------|
| Shaft | Circular or tapered (0.8 neck) surface of revolution | Yes — `doric_column` |
| Capital / necking / **square abacus** | Real mesh | No. Separate island or mesh. |
| Pedestal / **square plinth** | Real mesh | No. Separate island or mesh. |
| Caps | Omit on the shaft, or a distinct UV island | Fluting UVs must not cover caps |

The placeholders here are **open shafts** (no caps). [#1133](https://github.com/ramate-io/maybraid/issues/1133) owns capital, pedestal, and colliders. #1144 may close preview caps on a separate UV island (`U = 0.5`).

**Winding:** outward, CCW when looking against the authored normal. Triangle order is `i0 → i1 → i2`, `i1 → i3 → i2` (same as #1144 `solid_shaft_mesh`). Vertex normals point the same way; do not rely on culling or lighting to hide inward winding.

## UV

| Channel | Range | Meaning |
|---------|-------|---------|
| **U** | `0..1`, wraps once | Angle around the shaft. **U = 0 and U = 1 are the same physical seam.** Independent of D. |
| **V** | `0..1` | Bottom → top of the **authored** shaft (the D = 1 bind pose, before `s = D`). |

On a taper, authored circumference is `π` at `V = 0` and `≈ 0.8 π` at `V = 1`. After `s = D` those become `π D` and `0.8 π D`. **U still wraps once.** Flute count stays `U * 20`; channels follow meridians and get narrower toward the neck. POM marches in the UV Jacobian, so a world-metre of relief maps to a larger `ΔU` where the circumference is smaller.

Seam placement: put **U = 0/1 on local −Z** so a typical +Z / +X approach does not look at the cut. Duplicate the seam ring of vertices. Do **not** let a triangle interpolate from `U ≈ 1` to `U ≈ 0`.

Height is **periodic in U** (`fract(u)`). Authored height / normal maps must tile across the seam, or hide the seam on −Z and keep a few texels of overlap.

Do not mirror U. Flute count is `U * 20` (classical Doric; override with scalar 3).

## Tangent frame

Right-handed, Bevy `ATTRIBUTE_TANGENT` (`xyz` + `w` sign):

| Vector | Direction | Notes |
|--------|-----------|--------|
| **T** (+U) | Circumferential, counterclockwise when viewed from +Y | `dP/dU`. Authored length is `π` at the base; `s = D` scales it. |
| **B** (+V) | Up the shaft (along the slant on a taper) | `dP/dV` |
| **N** | Outward from the axis (tilted on a taper) | `T × B` (with `tangent.w = +1`) |

`T` must be perpendicular to `N`. Do not import a GLB whose tangents were generated against a different unwrap (planar box, mirrored U, or inverted V).

POM itself marches in screen-space UV Jacobian (same solver family as clothing / Durham). Tangents still matter for any later normal-map layer and for Bevy’s PBR specialization. Export them.

## Texel density (per unit of authored D)

Bake on the **D = 1** mesh. `s = D` does not change the authored map; world texel density falls as D grows. Do not rebake per encounter scale.

| Axis | Target per authored D | Worked example |
|------|------------------------|----------------|
| **U** | **64–128 texels per flute** | 96 texels/flute × 20 → **1920** px wide. Angular; bake against the **base** circumference `π` so the neck is slightly denser. |
| **V** | **256–512 texels per authored D** of shaft height | 384 texels/D × [`shaft_height_d()`](https://github.com/ramate-io/maybraid/pull/1144) (5.6 in #1144 `DoricColumnRatios` defaults) → **≈ 2150** px tall. Compact cylinder preview (H = 2.52) → **968** px. |

`texels_per_authored_d_v() = 384`. Map height = that × authored H, and authored H is `#1144`'s `shaft_height_d()`.

Non-uniform scale that stretches U vs V will shear flutes; don’t.

## Quality / sampling (not authoring, but kit knobs)

[`MaterialRef`](../../../../../material-ref) scalars on `doric_column`:

| Index | Default | Role |
|------|---------|------|
| 0 | look | Roughness override |
| 1 | 0.35 | Grain scale |
| 2 | 0.22 | Wear / lichen in flute floors |
| 3 | 20 | Flute count, clamped 8–32 |
| 4 | 28 m | POM fade start (world metres) |
| 5 | 85 m | POM fade end (world metres) |
| 6 | 48 | Max march steps, clamped 12–48 |
| 7 | **0.11 D** | Relief depth in **authored D**. World depth = `0.11 ×` local world diameter (from `|dP/dU| / π`), so it scales with `s = D`. **≈ 9 cm at D = 0.84 m.** 5.5 cm (≈ 0.065 D) still read weakly. Clamp 0.02–0.20 D. |
| 8 | 0 | Flat / POM-off (`>= 0.5` skips march and flute cavity) |

Far / tiny-footprint / grazing fragments drop to a cheap cavity tint. The march never exceeds the max step cap.

**Depth default.** 5.5 cm still read as a softly banded cylinder under a raking key: the A/B against flat was real, but a reviewer could not count 20 carved channels. **0.11 D** (9 cm-equivalent at D = 0.84 m) is the kit default. Because it is D-relative, a uniformly scaled encounter shaft keeps the same cut in shaft-diameters.

Playground evidence (`MAYBRAID_CAPTURE_DIR`) hides the command drawer, uses a **low raking key**, and writes `rake-grazing-pom-on` / `off` (scalar 8), `rake-silhouette-cavities`, and `rake-taper-*` (kit `DoricColumnRatios` taper).

## Recipe

```text
MaterialRef::named("doric_column")
```

Alias `doric` is accepted. Preview:

```text
cargo run -p world-materials-playground -- show doric_column
# then /mesh cylinder    (compact unit-D cylinder)
# or   /mesh taper       (authored D = 1, #1144 DoricColumnRatios)
```
