# Doric column kit — GLB authoring for Liam

This document is the drop-in contract for the Bikanwarda Doric column and pedestal kits ([#1133](https://github.com/ramate-io/maybraid/issues/1133)). Runtime integration lives in the [`doric-column`](.) crate; placeholders use the same transforms, anchors, and colliders until your GLBs replace them.

## Coordinate system and units

| Convention | Value |
|------------|--------|
| **Units** | 1 Blender unit = 1 meter |
| **Up** | **+Y** |
| **Forward** | **+Z** (encounter facing; chain anchors use this frame) |
| **Right** | **+X** |

Assembly roots place **`ground_contact`** at the world position passed to [`spawn_doric_column_encounter`](src/spawn.rs) / [`spawn_doric_column_summon`](src/spawn.rs). Terrain snap moves the root so `ground_contact` sits on the sampled surface.

## Reference dimensions (unit kit)

Authored meshes should match these extents at **scale `(1,1,1)`** on the part node. Gameplay scales via [`DoricColumnSpec::scale`](src/spec.rs) and [`DoricColumnDimensions`](src/spec.rs) without re-exporting GLBs.

| Part | Height (m) | Footprint (radius at bottom → top) |
|------|------------|-------------------------------------|
| **Pedestal** | 1.2 | 0.95 → 0.75 |
| **Shaft** | 14.0 | 0.75 → 0.75 (cylinder) |
| **Capital** | 1.0 | 0.95 → 0.95 |

**Total assembly height:** 16.2 m at unit scale.

### Per-part origin (pivot)

Each part GLB is instanced with a **bottom-center** origin:

- **Origin:** center of the bottom face on the XZ plane, **Y = 0**.
- **Mesh extends:** **Y ∈ [0, part_height]**.
- **Forward:** mesh features (flutes, abacus detail) should face **+Z** when rotation is identity.

Stacking in assembly space (before root yaw):

1. Pedestal at **Y = 0**
2. Shaft at **Y = 1.2** (top of pedestal)
3. Capital at **Y = 15.2** (top of shaft)

Scaling maps authored unit meshes via non-uniform `Placement.scale` on each part (`diameter` on X/Z, `height` on Y). Do **not** bake encounter scale into the GLB; keep the reference dimensions above.

## File paths and naming

| Role | Blender source | Runtime GLB (`maybraid/assets/`) |
|------|----------------|----------------------------------|
| Pedestal | `maybraid/art/bikanwarda/architectural/doric_column/pedestal_001.blend` | `bikanwarda/architectural/doric_column/pedestal_001.glb` |
| Shaft | `…/shaft_001.blend` | `…/shaft_001.glb` |
| Capital | `…/capital_001.blend` | `…/capital_001.glb` |
| Optional rig | `…/doric_column_rig_001.blend` | `…/doric_column_rig_001.glb` |

Constants: [`src/assets.rs`](src/assets.rs).

### Scene / object naming (GLB export)

- Export **Scene 0** as today’s urban/furniture kits do.
- Root object names should match part labels: `pedestal`, `shaft`, `capital`.
- Material slot names (for material-ref routing):
  - `doric_column_pedestal`
  - `doric_column_shaft`
  - `doric_column_capital`

Shaft instances use material recipe **`doric-column-shaft-pom`** ([`DoricColumnMaterials`](src/materials.rs)) — same contract as **`doric_column`** in the POM lane ([#1132](https://github.com/ramate-io/maybraid/issues/1132), draft PR [#1143](https://github.com/ramate-io/maybraid/pull/1143)). Pedestal and capital use `doric-column-pedestal` and `doric-column-capital`.

## UV layout and column POM ([#1132](https://github.com/ramate-io/maybraid/issues/1132))

Canonical shader and recipe documentation: [`COLUMN_SURFACE.md`](../../../../urbanization/buildings/shaders/COLUMN_SURFACE.md) ([#1143](https://github.com/ramate-io/maybraid/pull/1143)). POM owns **surface relief only**. Shaft outline, capital, pedestal, and cover collision stay real geometry. The material never displaces the silhouette and must not be treated as physical cover.

### Mesh islands

| Part | Geometry | This material? |
|------|----------|----------------|
| Shaft | Circular (or slight entasis) cylinder of revolution | Yes — `doric_column` / `doric-column-shaft-pom` |
| Capital / necking | Real mesh | No. Separate island or mesh. |
| Pedestal / base | Real mesh | No. Separate island or mesh. |
| Caps | Omit on the shaft, or a distinct UV island | Fluting UVs must not cover caps |

Runtime **placeholders** close the shaft with top/bottom caps for preview; cap faces use **`U = 0.5`** (off the fluting island). Authored GLBs should omit shaft caps or keep them on a separate UV island as above.

### UV (shaft side wall)

Local space: **+Y** up the shaft, origin on the shaft axis.

| Channel | Range | Meaning |
|---------|-------|---------|
| **U** | `0..1`, wraps once | Angle around the shaft. **U = 0 and U = 1 are the same physical seam.** |
| **V** | `0..1` | Bottom → top of the **authored shaft** (bind pose, before gameplay scale). |

Seam placement: put **U = 0/1 on local −Z** so a typical +Z / +X approach does not look at the cut. Duplicate the seam ring of vertices. Do **not** let a triangle interpolate from `U ≈ 1` to `U ≈ 0`.

Height is **periodic in U** (`fract(u)`). Authored height / normal maps must tile across the seam, or hide the seam on −Z and keep a few texels of overlap.

Do not mirror U. Flute count is `U * 20` (classical Doric; override with material scalar 3 in the POM recipe).

### Tangent frame (shaft)

Right-handed, Bevy `ATTRIBUTE_TANGENT` (`xyz` + `w` sign):

| Vector | Direction | Notes |
|--------|-----------|--------|
| **T** (+U) | Circumferential, counterclockwise when viewed from +Y | `dP/dU` |
| **B** (+V) | Up the shaft | `dP/dV` |
| **N** | Outward from the axis | `T × B` (with `tangent.w = +1`) |

`T` must be perpendicular to `N`. Do not import a GLB whose tangents were generated against a different unwrap (planar box, mirrored U, or inverted V). Export tangents for the shaft.

The [`doric-column`](.) placeholder shaft follows this contract on the side wall (see `solid_shaft_mesh` in [`src/placeholder.rs`](src/placeholder.rs)).

### Texel density (when baking maps)

| Axis | Target |
|------|--------|
| **U** | **64–128 texels per flute** (one wrap = flute count × that) |
| **V** | **256–512 texels per metre** of authored shaft |

Example at preview radius `r = 0.42 m`, height `2.52 m`, 20 flutes: 96 texels/flute → **1920** px wide; 384 texels/m → **968** px tall.

Uniform gameplay scale is fine. Non-uniform scale that stretches U vs V will shear flutes; don’t.

Pedestal and capital: planar or box-projected UV0 is fine at similar texel density; they do not use the column POM recipe.

## Anchor nodes

Add **empty** objects (or bones on the optional rig) with **exact** names. Positions are in **assembly space** at unit scale; the code mirrors these in [`DoricColumnAnchors`](src/anchors.rs).

| Name | Position (m) | Orientation |
|------|----------------|-------------|
| `ground_contact` | `(0, 0, 0)` | Identity |
| `pedestal_top` | `(0, 1.2, 0)` | Identity |
| `shaft_top` | `(0, 15.2, 0)` | Identity |
| `capital_top` | `(0, 16.2, 0)` | Identity |
| `chain_anchor_pos_x` | `(0.83, 15.2, 0)` | **+X** outward (local **+Y** = world **+Y**, local **+Z** = **+X**) |
| `chain_anchor_neg_x` | `(-0.83, 15.2, 0)` | **−X** outward |
| `chain_anchor_pos_z` | `(0, 15.2, 0.83)` | **+Z** outward |
| `chain_anchor_neg_z` | `(0, 15.2, -0.83)` | **−Z** outward |

Chain anchor radius = `shaft_radius + 0.08 m` at unit scale. Chain systems attach to entities tagged [`DoricColumnChainAnchor`](src/anchors.rs).

Part-to-part sockets align with `pedestal_top` / `shaft_top` — shaft bottom sits on pedestal top, capital bottom on shaft top.

## Collision and cover

- **Do not** rely on render mesh for physics.
- Silhouette flutes and echinus overhangs should be in the **visible** mesh; cover uses **conservative cylinders** per part ([`cover_collider_shapes`](src/colliders.rs)), not POM depth.
- Colliders are **Fixed** layer compounds; they participate in movement blocking and sight/trajectory probes the same way building geometry does.

## Swapping placeholders for GLBs

1. Export the three part GLBs to the paths in [`src/assets.rs`](src/assets.rs).
2. In the spawning lane (encounter or summon), pass `use_glb: true` to the spawn helpers **or** set [`DoricColumnUseAuthoredGlb`](src/parts.rs) resource to `true` before spawn.
3. Each part node with `use_authored_glb` gets a [`SceneRefRoot`](../../../../../scene-ref/src/scene_ref.rs) for its GLB; placeholders are skipped.
4. Verify in `doric-column-playground`: chain anchors (cyan), ground contact (green), cover debug (red lines).

No change to [`DoricColumnSpec`](src/spec.rs), anchor names, or collider layout is required if meshes honor the origins and reference dimensions above.

## Validation checklist

- [ ] Part heights and radii match the reference table at scale 1.
- [ ] `ground_contact` at bottom center; assembly sits on terrain without floating.
- [ ] All eight anchor names present with orientations as specified.
- [ ] Shaft UV0 seam and density ready for POM lane ([#1132](https://github.com/ramate-io/maybraid/issues/1132)).
- [ ] Materials named for material-ref fulfillment.
- [ ] Playground preview: encounter + summon instances, anchors and cover gizmos aligned.
