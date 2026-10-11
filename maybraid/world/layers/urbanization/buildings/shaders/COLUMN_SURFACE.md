# Doric column POM UV / tangent contract

Copy this section into the [#1133](https://github.com/ramate-io/maybraid/issues/1133) / [#1144](https://github.com/ramate-io/maybraid/pull/1144) column/pedestal GLB authoring spec. The shader is [`column_surface.wgsl`](src/column_surface.wgsl). Placeholders that already meet the contract: [`PlaceholderShaft`](src/column_surface/shaft.rs) (compact cylinder) and [`placeholder_tapered_shaft_mesh`](src/column_surface/shaft.rs) (D-driven taper).

POM owns **surface relief only**. Shaft outline, capital, pedestal, and cover collision stay real geometry. The material never displaces the silhouette and must not be treated as physical cover.

## Proportions (driven by lower diameter D)

[#1144](https://github.com/ramate-io/maybraid/pull/1144) is switching the kit to classical Doric proportions from a single lower-shaft diameter **D**. This contract is written in those units so texel targets stay valid when #1144 locks the final D.

| Quantity | In terms of D | Notes |
|----------|----------------|--------|
| Lower shaft diameter | **D** | Bottom of the fluted shaft |
| Neck / upper shaft diameter | **≈ 0.8 D** | Circumference shrinks up V; U still wraps once |
| Full column height | **≈ 6–7 D** | Square plinth + shaft + square abacus. Shaft height is the remainder; preview taper uses **4.5 D** |
| Plinth / abacus plan | **square** | Real geometry, not this material |

### Single scaling convention

Author at a reference D (or `D = 1`) and apply **one uniform scale** `s = D_game / D_authored` to the whole assembly. Do **not** split scale into XZ-diameter vs Y-height — that shears U vs V and the flute Jacobian. Gameplay that only changes D should not re-unwrap.

### Pivot (same as #1144 `AUTHORING.md`)

Local space: **+Y up**, **+Z forward**, **+X right**.

- **Origin:** bottom-center on the shaft axis (`Y = 0` at the lower ring).
- **Mesh extends:** `Y ∈ [0, H_shaft]`.
- **Winding:** outward, CCW when looking against the authored normal. Triangle order is `i0 → i1 → i2`, `i1 → i3 → i2` (same as #1144 `solid_shaft_mesh`). Vertex normals point the same way; do not rely on culling or lighting to hide inward winding.

## Mesh islands

| Part | Geometry | This material? |
|------|----------|----------------|
| Shaft | Circular (or tapered / slight entasis) surface of revolution | Yes — `doric_column` |
| Capital / necking / **square abacus** | Real mesh | No. Separate island or mesh. |
| Pedestal / **square plinth** | Real mesh | No. Separate island or mesh. |
| Caps | Omit on the shaft, or a distinct UV island | Fluting UVs must not cover caps |

The placeholders here are **open shafts** (no caps). [#1133](https://github.com/ramate-io/maybraid/issues/1133) owns capital, pedestal, and colliders. #1144 may close preview caps on a separate UV island (`U = 0.5`).

## UV

| Channel | Range | Meaning |
|---------|-------|---------|
| **U** | `0..1`, wraps once | Angle around the shaft. **U = 0 and U = 1 are the same physical seam.** Independent of D and of the local radius. |
| **V** | `0..1` | Bottom → top of the **authored shaft** (bind pose, before the single uniform scale). |

On a taper, physical circumference is `π D` at `V = 0` and `≈ 0.8 π D` at `V = 1`. **U still wraps once.** Flute count stays `U * 20`; channels follow meridians and get narrower toward the neck. POM marches in the UV Jacobian, so a metre of relief maps to a larger `ΔU` where the circumference is smaller.

Seam placement: put **U = 0/1 on local −Z** so a typical +Z / +X approach does not look at the cut. Duplicate the seam ring of vertices. Do **not** let a triangle interpolate from `U ≈ 1` to `U ≈ 0`.

Height is **periodic in U** (`fract(u)`). Authored height / normal maps must tile across the seam, or hide the seam on −Z and keep a few texels of overlap.

Do not mirror U. Flute count is `U * 20` (classical Doric; override with scalar 3).

## Tangent frame

Right-handed, Bevy `ATTRIBUTE_TANGENT` (`xyz` + `w` sign):

| Vector | Direction | Notes |
|--------|-----------|--------|
| **T** (+U) | Circumferential, counterclockwise when viewed from +Y | `dP/dU`. Length is `π D(v)` (shrinks up a taper). |
| **B** (+V) | Up the shaft (along the slant on a taper) | `dP/dV` |
| **N** | Outward from the axis (tilted on a taper) | `T × B` (with `tangent.w = +1`) |

`T` must be perpendicular to `N`. Do not import a GLB whose tangents were generated against a different unwrap (planar box, mirrored U, or inverted V).

POM itself marches in screen-space UV Jacobian (same solver family as clothing / Durham). Tangents still matter for any later normal-map layer and for Bevy’s PBR specialization. Export them.

## Texel density (in terms of D)

Procedural height is the current default. When baking height / normal maps, use D so the budget survives #1144’s final diameter:

| Axis | Target | In terms of D |
|------|--------|----------------|
| **U** | **64–128 texels per flute** | **D-invariant** (U is angular). Map width = flute count × that. Bake against the **base** circumference `π D` so the neck (`0.8 π D`) is slightly denser, not the reverse. |
| **V** | **256–512 texels per metre** of authored shaft | Equivalently **(256–512) × D texels per D of shaft height**. Map height = that × (`H_shaft / D`). `H_shaft / D` is owned by #1144 (6–7 D column minus square plinth and abacus). |

Preview at `D = 0.84 m`, 20 flutes, 96 texels/flute → **1920** px wide. Compact cylinder `H = 2.52 m` at 384 tex/m → **968** px tall. Tapered preview `H = 4.5 D` → **1452** px tall. `texels_per_d_v(D) = 384 × D`.

The single uniform scale does not change authored texel density. Non-uniform scale that stretches U vs V will shear flutes; don’t.

## Quality / sampling (not authoring, but kit knobs)

[`MaterialRef`](../../../../../material-ref) scalars on `doric_column`:

| Index | Default | Role |
|------|---------|------|
| 0 | look | Roughness override |
| 1 | 0.35 | Grain scale |
| 2 | 0.22 | Wear / lichen in flute floors |
| 3 | 20 | Flute count, clamped 8–32 |
| 4 | 28 m | POM fade start |
| 5 | 85 m | POM fade end |
| 6 | 48 | Max march steps, clamped 12–48 |
| 7 | 0.09 m | Relief depth. **≈ 0.11 D** at the `D = 0.84 m` preview. **5.5 cm (≈ 0.065 D) still read weakly** under a raking key — soft corrugation, not 20 carved channels. **9 cm is the recommended default** at this D; scale scalar 7 with D on giant shafts. Clamp 0.8–12 cm. |
| 8 | 0 | Flat / POM-off (`>= 0.5` skips march and flute cavity) |

Far / tiny-footprint / grazing fragments drop to a cheap cavity tint. The march never exceeds the max step cap.

**Depth default.** 5.5 cm still read as a softly banded cylinder under a low raking key: the A/B against flat was real, but a reviewer could not count 20 carved channels. **9 cm ≈ 0.11 D** is the recommended kit default at the preview diameter. Kits that want a more classical cut can push scalar 7 down; encounter shafts should keep ~0.11 D rather than a fixed 9 cm.

Playground evidence (`MAYBRAID_CAPTURE_DIR`) hides the command drawer, uses a **low raking key** (roughly perpendicular to the view), and writes the same grazing pose twice: `rake-grazing-pom-on` vs `rake-grazing-pom-off` (scalar 8). `rake-silhouette-cavities` shows shaft outline and flute cavities together. `rake-taper-*` is the 0.8 D neck so POM still reads as circumference shrinks up V.

## Recipe

```text
MaterialRef::named("doric_column")
```

Alias `doric` is accepted. Preview:

```text
cargo run -p world-materials-playground -- show doric_column
# then /mesh cylinder    (compact D-radius cylinder)
# or   /mesh taper       (0.8 D neck, 4.5 D tall)
```
