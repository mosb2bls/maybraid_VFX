# Doric column kit — GLB authoring for Liam

Runtime integration: [`doric-column`](.) crate. Placeholders follow the same contract as GLBs until assets land ([#1133](https://github.com/ramate-io/maybraid/issues/1133)).

## Coordinate system

| Convention | Value |
|------------|--------|
| **Units** | 1 Blender unit = **1× lower shaft diameter D** in the mesh file |
| **Up** | **+Y** |
| **Forward** | **+Z** |
| **Pivot** | Bottom-center of each part (**Y = 0** on the bottom face) |

World metres = **mesh coordinate × D_world**, where **D_world** = [`lower_shaft_diameter_d`](src/spec.rs) × [`uniform_scale`](src/spec.rs) ([#1143](https://github.com/ramate-io/maybraid/pull/1143) uniform-scale contract).

## Single scaling convention (matches [#1143](https://github.com/ramate-io/maybraid/pull/1143))

[`COLUMN_SURFACE.md`](../../../../urbanization/buildings/shaders/COLUMN_SURFACE.md) assumes a **bottom-center pivot**, **+Y** up, and **uniform** world scale so POM tangents and texel density stay valid.

| Stage | Rule |
|-------|------|
| **Authoring** | Each part mesh is **normalized by D**: lower shaft **diameter = 1**; all other dimensions are [`DoricColumnRatios`](src/spec.rs) multiples (e.g. shaft height ≈ **5.6**, plinth side **1.4**, abacus **1.2**). |
| **Runtime** | [`part_placements`](src/layout.rs) sets **`Placement::scale = Vec3::splat(D_world)`** only — **never** per-axis stretch on parts. |
| **Layout** | Anchors and cover colliders use **world metres** from `ratios × D_world` ([`DoricColumnLayout`](src/layout.rs)). |

Do **not** model a shaft at 8 m tall **and** apply `scale.y = 8`. Do **not** use `scale (side, height, side)` on parts.

Constants and tests: [`src/authoring.rs`](src/authoring.rs) (`authored_extents`, `d_normalized_mesh_with_uniform_scale_matches_layout_anchors_and_colliders`).

### GLB vs spec mismatch

- **Anchors / gameplay layout** default to [`DoricColumnLayout`] from the spec.
- Authored GLBs should match [`DoricColumnRatios`].
- When a loaded GLB’s bounds or empties disagree with the spec, the runtime **logs a warning** and **named anchor nodes in the GLB** (`ground_contact`, `shaft_top`, …) **override** spec-computed anchor transforms. **Cover colliders stay spec-based** until the asset is corrected.

## Classical proportions (mesh coordinates = × **D**)

Defaults in [`DoricColumnRatios`](src/spec.rs) (`DoricColumnSpec::default()` uses **D_world = 1.5 m**).

| Quantity | Ratio (mesh units) | Default world @ 1.5 m |
|----------|-------------------|------------------------|
| Total height | `column_height_d` **6.5** | 9.75 m |
| Plinth height | `plinth_height_d` **0.4** | 0.60 m |
| Plinth square side | `plinth_side_d` **1.4** | 2.10 m |
| Shaft height | **5.6** (= 6.5 − 0.4 − 0.5) | 8.40 m |
| Shaft upper diameter | `shaft_upper_diameter_d` **0.8** | 1.20 m |
| Capital height | `capital_height_d` **0.5** | 0.75 m |
| Echinus / abacus | **0.3** / **0.2** | 0.45 / 0.30 m |
| Abacus square side | `abacus_side_d` **1.2** | 1.80 m |

### Plan shape

| Part | Plan |
|------|------|
| Plinth | Square, faces on **±X** / **±Z** |
| Shaft | Round (entasis in mesh) |
| Echinus | Round |
| Abacus | Square, axis-aligned with plinth |

## Per-part mesh extents (local **D** units)

| Part | Local **Y** | Local **X/Z** |
|------|-------------|---------------|
| Plinth | `[0, plinth_height_d]` | side `plinth_side_d`, centered |
| Shaft | `[0, shaft_height_d]` | diameter **1** → **shaft_upper_diameter_d** |
| Capital | `[0, capital_height_d]` | echinus radii + abacus `abacus_side_d` |

Stacking (world metres): plinth **Y=0**; shaft base **plinth_height_d × D_world**; capital base **(plinth_height_d + shaft_height_d) × D_world**.

## Files and materials

Paths: [`src/assets.rs`](src/assets.rs). Objects: `pedestal`, `shaft`, `capital`. Shaft material **`doric-column-shaft-pom`** / **`doric_column`** ([#1143](https://github.com/ramate-io/maybraid/pull/1143)).

## UV / POM (shaft only) — [#1143](https://github.com/ramate-io/maybraid/pull/1143)

See [`COLUMN_SURFACE.md`](../../../../urbanization/buildings/shaders/COLUMN_SURFACE.md). Summary:

- **Shaft-only** island; plinth/capital/caps excluded from fluting UVs.
- **U** `0..1` once around; seam **local −Z**; duplicate seam ring; no **U** wrap across triangles.
- **V** `0..1` bottom → top of authored shaft bind pose (**local Y ∈ [0, shaft_height_d]**).
- **T** (+U) CCW from **+Y**; **B** (+V) up; **N** outward; **`tangent.w = +1`**.
- Flute count **U × 20** (material override scalar 3).
- Bake targets: **64–128 texels/flute**; **256–512 texels/m** along world shaft height (**V** spans bind pose; uniform **D** scale preserves density).

Placeholder: [`solid_shaft_mesh`](src/placeholder.rs).

## Anchor nodes (world metres)

From [`DoricColumnAnchors`](src/anchors.rs); same names as GLB empties:

| Name | **Y** (× **D_world**) |
|------|------------------------|
| `ground_contact` | 0 |
| `pedestal_top` | `plinth_height_d` |
| `shaft_top` | `plinth_height_d + shaft_height_d` |
| `capital_top` | `column_height_d` |

Chain anchors at **shaft_top** height, radius **shaft_upper_diameter_d/2 + chain_anchor_outset_d** (in world metres).

## Cover colliders

[`cover_collider_shapes`](src/colliders.rs): boxes (plinth, abacus) + cylinders (shaft, echinus) from layout — not render meshes.

## Validation

- [ ] Meshes in **D-normalized** coordinates; runtime **uniform** `Vec3::splat(D_world)` only.
- [ ] Square plinth/abacus aligned to **±X** / **±Z**.
- [ ] Shaft UV/tangents per **#1143** `COLUMN_SURFACE.md`.
- [ ] `cargo test -p doric-column authoring::` passes.
