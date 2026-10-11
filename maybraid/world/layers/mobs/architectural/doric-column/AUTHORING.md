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

Shaft instances use material recipe **`doric-column-shaft-pom`** ([`DoricColumnMaterials`](src/materials.rs)); pedestal and capital use `doric-column-pedestal` and `doric-column-capital` until the POM lane wires the shaft recipe in the material lib.

## UV layout and column POM ([#1132](https://github.com/ramate-io/maybraid/issues/1132))

> **Pending POM lane:** No draft PR for [#1132](https://github.com/ramate-io/maybraid/issues/1132) was available when this spec was written. When that lane lands, replace this subsection with its UV/tangent/texel-density table and keep the part origins and anchor names below unchanged.

Until then, author the **shaft** with the following **integration placeholders** (the POM shader will own relief; collision stays on mesh silhouette):

| Topic | Shaft expectation (placeholder) |
|-------|----------------------------------|
| **UV0** | Cylindrical unwrap around **+Y**; seam on **−X** (back); **V** = 0 at bottom, **V** = 1 at top; **U** wraps once per 20 flutes at unit height (adjust U scale so one flute ≈ 0.05 U). |
| **UV channels** | **UV0** = albedo + POM height; leave **UV1** unused unless #1132 specifies otherwise. |
| **Texel density** | Target **~512 px/m** on the shaft at unit dimensions (≈ 7k px vertical span for 14 m); capital/pedestal may use 256 px/m. |
| **Tangents** | Mikktspace, **+V** along **+Y**, bitangent consistent with **+Z** forward. |
| **Tiling** | Flutes tile in **U** only; **V** spans full shaft height in one tile. |

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
