## Safe cellular generation

Terrain generation is cellular. Chunks evaluate only their own sample domain.
Continuity is achieved by ensuring that every modulation has bounded support and
reduces exactly to the identity outside its owning modulation cell.

### Why modulation support must be bounded

Suppose two neighboring terrain cells \(A\) and \(B\), and two modulations
\(F\) and \(G\), with deterministic composition order

\[
F \circ G.
\]

Assume

- \(F \cap A\),
- \(F \cap B\),
- \(G \cap A\),
- \(G \not\cap B\),
- \(F \cap G\).

Then the shared boundary evaluates as

\[
H_A(x)=F_x(G_x(h_0(x))),
\]

while

\[
H_B(x)=F_x(h_0(x)).
\]

These are equal iff

\[
F_x(G_x(z))=F_x(z),
\]

for \(z=h_0(x)\).

A sufficient condition is simply

\[
G_x=\mathrm{Id}
\]

everywhere outside its support, including the shared boundary.

This allows chunk \(B\) to omit \(G\) entirely while still producing exactly the
same result as chunk \(A\). Consequently, each terrain chunk only needs to load
the subset of modulations whose support intersects its own sample domain.

Identity blending is therefore the preferred implementation,

\[
\widetilde M(x,z)
=
z+w(x)\bigl(M(x,z)-z\bigr),
\]

where

\[
w(x)=0
\Longrightarrow
\widetilde M(x,z)=z.
\]

The support of a modulation is the region where \(w(x)\neq0\).

### Why mesh aprons break this

A mesh apron evaluates the terrain field outside the chunk's owned sample
domain.

Suppose chunk \(A\) evaluates its apron using

\[
H_A(x)=F_x(G_x(h_0(x))),
\]

while the neighboring chunk correctly evaluates

\[
H_B(x)=F_x(h_0(x)),
\]

because \(x\) lies inside terrain cell \(B\), where \(G\) is not loaded.

Even if

\[
H_A(x)=H_B(x)
\]

on the shared boundary, the marching-cubes stencil samples points on both sides
of that boundary. The apron therefore introduces inconsistent scalar samples
into the interpolation, producing visible seams.

The problem is not marching cubes—it is evaluating different procedural fields
over the same spatial region.

Instead, every chunk evaluates only its own sample domain. Neighboring chunks
produce identical boundary samples because every modulation is exactly the
identity outside its bounded support. No apron or overlap region is required.

### Practical rules

1. Every modulation has compact, bounded support.
2. Outside its support, a modulation is exactly the identity.
3. Terrain chunks load only modulations whose support intersects their own
   sample domain.
4. Chunks never evaluate terrain beyond their owned sample domain.
5. Modulation composition order is globally deterministic.

## Water fill composition (same lattice as terrain)

Water is a **second composition pass** on the same origin cells as terrain — not a
separate spatial tiling and not a fitted vertical AABB.

| Concern | Owner |
| --- | --- |
| Origin-cell tiling / cell size | [`TerrainCellLayout`](src/terrain/cell.rs); `PreWatershedTerrain` tiles it, `Terrain` and `Water` reuse those ids through `GeneratingSpatialIndex::original_ids_for` |
| Heightfield composition | [`ComposedTerrain`](src/terrain/sdf.rs) / `Terrain::compose_sdf` |
| Wet-volume composition | [`ComposedWater`](src/water/composed.rs) / `ComposedWater::compose` |
| Cascade chunk (`origin`, extent, Y, `res_2`) | [`cascade_chunk_for_cell`](src/terrain/render.rs) for **both** `Terrain::scene` and `Water::scene` |
| Mesh resolution | `TerrainMeshAssets` bands on the terrain cell; `Water` copies `terrain.res_2` |

Marazion lake and stream stamps author [`WaterFill`](../watersheds/src/primitive/fill.rs): softmask +
undercut gate columns, then a **half-space below \(W\)** (flat or graded). That is
what lets water share terrain's tall cell Y without vanishing under marching cubes.
Subterranean wet volume is intentional. Stream fills should stay **liberal** vs the
carved channel (wider support / undercut); narrow look comes from terrain.

### Rules

1. **Do not** introduce a water-only cell layout or a water-only `res_2`.
2. **Do not** fit a water-only Y span for meshing. Prefer free-surface half-spaces
   (or otherwise thick wet solids) on the shared lattice.
3. Softmask bleed / overspill stays on the fill region; empty cells may skip the
   water pass entirely.
4. Optional later: split chunks per feature (e.g. water material/color) while
   keeping the same cascade lattice.

### How to add a new wet body (checklist)

Use lakes / streams as the template. The failure mode to avoid is a **thin**
\([h, W]\) slab on the terrain cell's tall Y grid (or a fitted water-only Y AABB
that then fights graded surfaces sharing a cell).

1. **Stamp owns the fill product.** In Marazion (or the relevant stamp crate),
   emit a [`WaterFill`](../watersheds/src/primitive/fill.rs) whose hydro surface is a
   [`HydroComplex`](../watersheds/src/primitive/complex.rs). The complex owns \(W\) blend
   and the water SDF — do not invent a second meshing grid in the stamp.
2. **Wet solid = carve × half-space.** Outside carve, return approximate distance
   to \(\phi = 0\). Inside carve, distance is \(y - W\) (flat free surface;
   terrain occludes subterranean volume). Optional later: small \(\mu\) on \(\phi\).
3. **Carve for look; fill follows carve.** Terrain mods shape the visible bed.
   Water XZ support is the same carve occupancy (hard boundary). Narrow
   silhouettes come from the heightfield, not from starving a softmask.
4. **Collect on the terrain cell.** Ride fills on `Terrain::marazion_fills` (or
   the analogue) after the heightfield is fully composed. `Water` filters empty
   fills and builds [`ComposedWater`](src/water/composed.rs) for that same origin
   cell id.
5. **Mesh with `cascade_chunk_for_cell`.** `Water::scene` must use the same helper
   and `res_2` as `Terrain::scene`. Do not reintroduce per-fill Y fitting for the
   cascade chunk.
6. **Prove continuity analytically if graded.** Unit-test that mid-segment columns
   stay wet under the stamp's \(W(s)\) and bed (see stream / fill tests). If the
   analytic fill is continuous but the mesh beads, the bug is lattice vs thickness
   — deepen / widen the fill, do not shrink the cascade Y span.

### What did not work (do not revive)

- **Fitted water Y AABB** sized to \([h_{\mathrm{eff}}, W]\): helps one shallow lake,
  then breaks when a long graded stream (or mixed fills) inflate the shared cell
  span and undersample lakes.
- **Thin \(W-h\) slabs for presentation** on terrain-scale Y: adjacent samples can
  lose overlapping wet intervals on steep grades; MC beads even when the SDF is
  continuous.
- **Separate water `res_2` / cell layout:** shore samples drift from terrain.
- **Segment cuboid / multi-mesh ribbons** for streams: gaps, z-fighting, faceted
  look — prefer one continuous fill SDF on the shared lattice (or a future single
  lofted mesh, still not overlapping transforms).
