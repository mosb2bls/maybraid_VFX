# Doric column POM UV / tangent contract

Copy this section into the [#1133](https://github.com/ramate-io/maybraid/issues/1133) column/pedestal GLB authoring spec. The shader is [`column_surface.wgsl`](src/column_surface.wgsl). The placeholder that already meets the contract is [`PlaceholderShaft`](src/column_surface/shaft.rs).

POM owns **surface relief only**. Shaft outline, capital, pedestal, and cover collision stay real geometry. The material never displaces the silhouette and must not be treated as physical cover.

## Mesh islands

| Part | Geometry | This material? |
|------|----------|----------------|
| Shaft | Circular (or slight entasis) cylinder of revolution | Yes — `doric_column` |
| Capital / necking | Real mesh | No. Separate island or mesh. |
| Pedestal / base | Real mesh | No. Separate island or mesh. |
| Caps | Omit on the shaft, or a distinct UV island | Fluting UVs must not cover caps |

The placeholder is an **open shaft** (no caps). [#1133](https://github.com/ramate-io/maybraid/issues/1133) owns capital, pedestal, and colliders.

## UV

Local space: **+Y up the shaft**, origin on the shaft axis.

| Channel | Range | Meaning |
|---------|-------|---------|
| **U** | `0..1`, wraps once | Angle around the shaft. **U = 0 and U = 1 are the same physical seam.** |
| **V** | `0..1` | Bottom → top of the **authored shaft** (bind pose, before gameplay scale). |

Seam placement: put **U = 0/1 on local −Z** so a typical +Z / +X approach does not look at the cut. Duplicate the seam ring of vertices. Do **not** let a triangle interpolate from `U ≈ 1` to `U ≈ 0`.

Height is **periodic in U** (`fract(u)`). Authored height / normal maps must tile across the seam, or hide the seam on −Z and keep a few texels of overlap.

Do not mirror U. Flute count is `U * 20` (classical Doric; override with scalar 3).

## Tangent frame

Right-handed, Bevy `ATTRIBUTE_TANGENT` (`xyz` + `w` sign):

| Vector | Direction | Notes |
|--------|-----------|--------|
| **T** (+U) | Circumferential, counterclockwise when viewed from +Y | `dP/dU` |
| **B** (+V) | Up the shaft | `dP/dV` |
| **N** | Outward from the axis | `T × B` (with `tangent.w = +1`) |

`T` must be perpendicular to `N`. Do not import a GLB whose tangents were generated against a different unwrap (planar box, mirrored U, or inverted V).

POM itself marches in screen-space UV Jacobian (same solver family as clothing / Durham). Tangents still matter for any later normal-map layer and for Bevy’s PBR specialization. Export them.

## Texel density

Procedural height is the current default. When baking height / normal maps for the final GLB:

| Axis | Target | Placeholder (`r = 0.42 m`, `h = 2.52 m`, 20 flutes) |
|------|--------|-----------------------------------------------------|
| U | **64–128 texels per flute** (one wrap = flute count × that) | 96 texels/flute → **1920** px wide |
| V | **256–512 texels per metre** of authored shaft | 384 texels/m → **968** px tall |

Circumferential metres per wrap are `2πr`. At `r = 0.42 m` that is ~2.64 m, so 1920 U ≈ 730 texels/m — in the same band as V.

Uniform gameplay scale is fine. Non-uniform scale that stretches U vs V will shear flutes; don’t.

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
| 7 | 0.055 m | Relief depth |
| 8 | 0 | Flat / POM-off (`>= 0.5` skips march and flute cavity) |

Far / tiny-footprint / grazing fragments drop to a cheap cavity tint. The march never exceeds the max step cap.

Playground evidence (`MAYBRAID_CAPTURE_DIR`) hides the command drawer, uses a **low raking key** (roughly perpendicular to the view), and writes the same grazing pose twice: `rake-grazing-pom-on` vs `rake-grazing-pom-off` (scalar 8). `rake-silhouette-cavities` is the 3/4 pose that shows shaft outline and flute cavities together.

## Recipe

```text
MaterialRef::named("doric_column")
```

Alias `doric` is accepted. Preview:

```text
cargo run -p world-materials-playground -- show doric_column
# then /mesh cylinder
```
