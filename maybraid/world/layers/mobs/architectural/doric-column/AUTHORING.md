# Doric column kit — GLB authoring for Liam

This document is the drop-in contract for the Bikanwarda Doric column and pedestal kits ([#1133](https://github.com/ramate-io/maybraid/issues/1133)). Runtime integration lives in the [`doric-column`](.) crate; placeholders use the same transforms, anchors, and colliders until your GLBs replace them.

## Coordinate system and units

| Convention | Value |
|------------|--------|
| **Units** | 1 Blender unit = 1 meter in **world** space after placement |
| **Up** | **+Y** |
| **Forward** | **+Z** (encounter facing; chain anchors use this frame) |
| **Right** | **+X** |

Assembly roots place **`ground_contact`** at the world position passed to [`spawn_doric_column_encounter`](src/spawn.rs) / [`spawn_doric_column_summon`](src/spawn.rs). Terrain snap moves the root so `ground_contact` sits on the sampled surface.

## Mesh authoring convention (read this first)

**Runtime applies world size exactly once** via [`part_placements`](src/layout.rs) → [`Placement::scale`](../../../../../building-components). Meshes must be **unit-normalized in each part’s local space**; do **not** model parts at full metre heights (for example a 8 m shaft) unless placement scale is `(1,1,1)` only.

| Rule | Local part space (before placement) | World size (after placement) |
|------|-----------------------------------|------------------------------|
| Height | **Y ∈ [0, 1]** | `layout.<part>.height` metres |
| Plinth / abacus footprint | Square side **1** on **X** and **Z**, faces parallel to **±X** / **±Z** | `plinth_side_d × D` or `abacus_side_d × D` |
| Shaft footprint | Lower diameter **1**, upper diameter **`shaft_upper_diameter_d`** (default **0.8**) | Lower diameter **D**, upper **`shaft_upper_diameter_d × D`** |
| Capital placeholder | Echinus + abacus in one GLB, same unit rules | See [`DoricColumnLayout`](src/layout.rs) |

Constants mirrored in code: [`src/authoring.rs`](src/authoring.rs) (`UNIT_PART_HEIGHT`, `UNIT_SQUARE_SIDE`, …). Integration test: `simulated_authored_unit_mesh_matches_layout_anchors_and_colliders`.

**Why:** If a shaft GLB is modeled 8 m tall and runtime sets `scale.y = 8`, the column becomes 64 m tall while anchors and colliders stay at 8 m.

Gameplay [`DoricColumnSpec::uniform_scale`](src/spec.rs) multiplies **D** and every derived length together; it does not change the unit mesh.

## Classical proportions (multiples of **D**)

**D** = lower shaft diameter ([`DoricColumnDimensions::lower_shaft_diameter_d`](src/spec.rs)). Defaults live in [`DoricColumnRatios`](src/spec.rs); tune via spec, not by re-exporting GLBs.

| Quantity | Ratio (× **D**) | Default @ **D = 1.5 m** |
|----------|-----------------|-------------------------|
| Total column height | `column_height_d` (**6.5**) | 9.75 m |
| Plinth height | `plinth_height_d` (**0.4**) | 0.60 m |
| Plinth square side | `plinth_side_d` (**1.4**) | 2.10 m |
| Shaft height | `column_height_d − plinth_height_d − capital_height_d` (**5.6**) | 8.40 m |
| Shaft upper diameter | `shaft_upper_diameter_d` (**0.8**) | 1.20 m |
| Capital height (echinus + abacus) | `capital_height_d` (**0.5**) | 0.75 m |
| Echinus height | `echinus_height_d` (**0.3**) | 0.45 m |
| Abacus height | `abacus_height_d` (**0.2**) | 0.30 m |
| Abacus square side | `abacus_side_d` (**1.2**) | 1.80 m |
| Chain anchor outset | `chain_anchor_outset_d` (**0.05**) | 0.075 m |

### Plan shape

| Part | Plan | Notes |
|------|------|--------|
| **Plinth** | Square | Axis-aligned; corners on diagonals, faces on **±X** / **±Z** |
| **Shaft** | Round (entasis) | Circular; seam for POM on local **−Z** |
| **Echinus** | Round | Sits under the square abacus |
| **Abacus** | Square | Same axis alignment as plinth |

### Per-part origin (pivot)

- **Origin:** center of the bottom face on the XZ plane, **Y = 0**.
- **Mesh extends:** **Y ∈ [0, 1]** in local space (unit height).
- **Forward:** detail faces **+Z** at identity rotation.

Stacking in assembly space (world metres at `scale = 1`, default **D**):

1. Plinth base **Y = 0**
2. Shaft base **Y = `plinth_height_d × D`**
3. Capital (echinus + abacus) base **Y = plinth height + shaft height**

## File paths and naming

| Role | Blender source | Runtime GLB (`maybraid/assets/`) |
|------|----------------|----------------------------------|
| Plinth | `maybraid/art/bikanwarda/architectural/doric_column/pedestal_001.blend` | `bikanwarda/architectural/doric_column/pedestal_001.glb` |
| Shaft | `…/shaft_001.blend` | `…/shaft_001.glb` |
| Capital | `…/capital_001.blend` | `…/capital_001.glb` |
| Optional rig | `…/doric_column_rig_001.blend` | `…/doric_column_rig_001.glb` |

Constants: [`src/assets.rs`](src/assets.rs). Scene object names: `pedestal`, `shaft`, `capital`.

Material slots: `doric_column_pedestal`, `doric_column_shaft`, `doric_column_capital`. Shaft recipe **`doric-column-shaft-pom`** / **`doric_column`** ([#1132](https://github.com/ramate-io/maybraid/issues/1132), [#1143](https://github.com/ramate-io/maybraid/pull/1143)).

## UV layout and column POM ([#1132](https://github.com/ramate-io/maybraid/issues/1132))

Canonical contract: [`COLUMN_SURFACE.md`](../../../../urbanization/buildings/shaders/COLUMN_SURFACE.md) ([#1143](https://github.com/ramate-io/maybraid/pull/1143)). POM owns **surface relief only** on the **shaft**; plinth, echinus, abacus, and cover stay real geometry.

### Mesh islands

| Part | Geometry | POM material? |
|------|----------|---------------|
| Shaft | Circular cylinder of revolution (entasis in mesh + scale) | Yes |
| Echinus / abacus / plinth | Real mesh | No |
| Shaft caps | Omit, or separate UV island | Fluting UVs must not cover caps |

Runtime placeholders close the shaft for preview; cap UVs use **`U = 0.5`** off the fluting island.

### UV (shaft only)

Local **+Y** up the shaft, origin on the axis.

| Channel | Range | Meaning |
|---------|-------|---------|
| **U** | `0..1`, one wrap | Azimuth; **U = 0** and **U = 1** are the same seam |
| **V** | `0..1` | Bottom → top of the **authored shaft** bind pose (unit **Y ∈ [0, 1]** before placement) |

- Seam at **local −Z**; duplicate the seam ring; no triangle spans **U ≈ 1** to **U ≈ 0**.
- Height periodic in **U** (`fract(u)`); maps must tile across the seam.
- Do not mirror **U**. Flute count **`U × 20`** (override via material scalar 3 in [#1143](https://github.com/ramate-io/maybraid/pull/1143)).

### Tangent frame (shaft)

Bevy `ATTRIBUTE_TANGENT` (`xyz` + `w`):

| Vector | Direction |
|--------|-----------|
| **T** (+U) | Circumferential, **counterclockwise** viewed from **+Y** (`dP/dU`) |
| **B** (+V) | Up the shaft (`dP/dV`) |
| **N** | Outward from axis — **`T × B`** with **`tangent.w = +1`** |

Export tangents; must match this unwrap (not planar/mirrored). Placeholder: [`solid_shaft_mesh`](src/placeholder.rs).

### Texel density (baked maps)

| Axis | Target |
|------|--------|
| **U** | **64–128 texels per flute** (width = flute count × that) |
| **V** | **256–512 texels per metre** of authored shaft height in **world** metres after placement |

At default **D = 1.5 m**: lower radius **0.75 m**, shaft height **8.4 m**, 20 flutes, 96 texels/flute → **1920** px wrap width; 384 texels/m → **3226** px tall. Circumference **2πr ≈ 4.71 m** → ~408 texels/m in **U** (same band as **V**).

Uniform [`DoricColumnSpec::uniform_scale`](src/spec.rs) is fine; non-uniform **U** vs **V** stretch shears flutes.

## Anchor nodes

Empties (or rig bones) in **assembly space** (world metres). Runtime: [`DoricColumnAnchors`](src/anchors.rs).

| Name | Position (× **D** at `scale = 1`) | Orientation |
|------|-----------------------------------|-------------|
| `ground_contact` | `(0, 0, 0)` | Identity |
| `pedestal_top` | `(0, plinth_height_d, 0)` | Identity |
| `shaft_top` | `(0, plinth_height_d + shaft_height_d, 0)` | Identity |
| `capital_top` | `(0, column_height_d, 0)` | Identity |
| `chain_anchor_pos_x` | `((shaft_upper_diameter_d/2 + chain_anchor_outset_d), plinth_height_d + shaft_height_d, 0)` | **+X** outward |
| `chain_anchor_neg_x` | Negate **X** | **−X** outward |
| `chain_anchor_pos_z` | **Z** = same radius | **+Z** outward |
| `chain_anchor_neg_z` | Negate **Z** | **−Z** outward |

`shaft_height_d = column_height_d − plinth_height_d − capital_height_d`. Chain radius uses **upper** shaft radius + outset.

## Collision and cover

Cover uses layout-derived **boxes** (plinth, abacus) and **cylinders** (shaft, echinus) in [`cover_collider_shapes`](src/colliders.rs), not render meshes or POM depth. **Fixed** layer.

## Swapping placeholders for GLBs

1. Export **unit-normalized** meshes per the table above.
2. Export GLBs to [`src/assets.rs`](src/assets.rs) paths.
3. Enable [`DoricColumnUseAuthoredGlb`](src/parts.rs) or `use_glb: true` on spawn.
4. Verify in `doric-column-playground`: square plinth/abacus (top-down shot), anchors, red cover gizmos.

## Validation checklist

- [ ] Each part **Y ∈ [0, 1]** locally; world size comes only from placement scale.
- [ ] Square plinth and abacus aligned to **±X** / **±Z**.
- [ ] Shaft UV/tangents per [#1143](https://github.com/ramate-io/maybraid/pull/1143) `COLUMN_SURFACE.md`.
- [ ] Anchor names and ratios match [`DoricColumnRatios`](src/spec.rs).
- [ ] `cargo test -p doric-column authoring::` passes for your **D** / scale.
