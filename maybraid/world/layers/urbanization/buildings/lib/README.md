# Richmond Buildings

Higher-order building authorship on top of [`building-components`](../components/) (`building-components` crate). This crate emits domain IR (`PartitionNode`, `FloorNode`, `StairNode`, …) via [`BuildingComponents`](../components/src/lib.rs); it does **not** tessellate kits or own GLB path strings. Present component-only buildings as [`ComponentsOnly`](../components/src/lib.rs)`<T>` unless the type needs a custom `LodScene` (hosts, silhouettes, lights).

## Urban kit model

Art sources live under [`maybraid/art/urban/`](../../../../../art/urban/README.md). Buildings helpers should treat that layout as the kit taxonomy:

**Shared within a style** — geometry reused across many primitive types:

- **`panels/`** — rectangles, right triangles, planar fillers
- **`arcs/`** — 15° / 90° / 180° bodies, slices, frames
- **`joints/`** — joints between abutting partition runs

**Parts** — `parts/` libraries of small reusable pieces (stones, thatch elements, …) used when assembling or detailing kits.

**Domain folders** — `floors/`, `partitions/`, `stairs/`, and similar remain for **function-specific** components and **fast authorship paths**. Prefer these when a kit is tied to one use (floor slab, tread, bespoke partition) rather than promoting everything into the shared layer.

Runtime paths are registered in building-components [`assets.rs`](../components/src/assets.rs). Shared kits are often **aliased** into domain consumers (e.g. partition `LINEAR_*` → panel rectangle; `ARC_*` / `SLICE_*` → `arcs/`; `JOINT_*` → `joints/`).

## What this crate owns

| Module | Role |
|--------|------|
| [`arcs`](src/arcs.rs) | Fitted circular [`ArcSweep`](src/arcs/sweep.rs) / [`ClippedArcSweep`](src/arcs/clipped_sweep.rs) / [`portal_ring`](src/arcs/portal_ring.rs) (not IR `partitions::ArcSweep`); ellipses deferred as a sibling |
| [`paneling`](src/paneling.rs) | Irregular panel primitives → [`PanelComplex`](src/paneling/panel_complex.rs) / rectangle kits + crease joints |
| [`portals`](src/portals.rs) | Portal vocabulary + assignment along unit path \(t\) |
| [`wall_demo`](src/wall_demo.rs) | Playground joinery demos (e.g. noisy path → rectangle strip) |
| [`wizards_tower`](src/wizards_tower.rs) | Authored tower hierarchy |
| [`stacked_rings`](src/stacked_rings.rs) | Circular wall stack |
| [`arc_spire`](src/arc_spire.rs) | Spire / storey binding helpers |
| [`constraints`](src/constraints.rs) | Cell / boundary / circulation IR |
| [`openings`](src/openings.rs) | Opening plans / shell records / mapped contact geometry |
| [`shells`](src/shells.rs) | Envelope shells (`ArcFloor`, `Trazaloid`, `ConnectingHall`, …) |
| [`storeys`](src/storeys.rs) | Storey typologies (Les Halles commercial / livable full storey, I-Apartment) |
| [`placer`](src/placer.rs) | Predicate-based rectangular layout trier (`KindSpec` catalogs) |
| [`usage_areas`](src/usage_areas.rs) | Program fill for residual confines (commercial stalls, [`common_bedroom`](src/usage_areas/common_bedroom/), [`livable_quarters`](src/usage_areas/livable_quarters.rs), LivableApartments, …) |
| [`fit`](src/fit.rs) | `Confines` / `Fit` / `FillableRegions` |

**Livable quarters** (under [`usage_areas/livable_quarters`](src/usage_areas/livable_quarters.rs)): `Kitchen`, `DiningRoom`, `LivingRoom`, `SittingRoom`, `Study`, `ResidentialBathroom`, `ResidentialHalfBathroom`. Shared layout substrate: [`placer`](src/placer.rs) (KindSpec trier) + [`clearance`](src/usage_areas/clearance.rs) door approach. Playground: `/show kitchen-examples`, `/show dining-room-examples`, `/show living-room-examples`, `/show sitting-room-examples`, `/show study-examples`, `/show residential-bathroom`, `/show residential-half-bathroom`, `/show residential-bathroom-examples`.

See [CONTRIBUTING.md](CONTRIBUTING.md) for the Les Halles parameterized → plan → full pattern, openings, and usage-area fill.

### `paneling` contents

| Type | Role |
|------|------|
| [`PanelComplex`](src/paneling/panel_complex.rs) | Point-id triangle/quad mesh → panels + crease `JointNode`s |
| [`QuadPanel`](src/paneling/quad_panel.rs) / [`QuadPanelComplex`](src/paneling/quad_panel_complex.rs) | Ruled quad / quad-face mesh wrappers |
| [`RuledStrip`](src/paneling/ruled_strip.rs) / [`RuledPitch`](src/paneling/ruled_pitch.rs) | Two-rail skew quads; roof eave/ridge wrapper |
| [`RectangularStrip`](src/paneling/rectangular_strip.rs) / [`ClippedRectangularStrip`](src/paneling/clipped_rectangular_strip.rs) | Node-chain oriented `PanelGeometry::Rectangle` kits (+ optional per-bay inset frames) + crease joints on bay folds |
| [`Rectangle`](src/paneling/rectangle.rs) / [`ClippedRectangle`](src/paneling/rectangle.rs) | Single-bay oriented rectangle (lowest-edge vector + height + roll); inset = frame of rectangle kits |
| [`FittedRectangularStrip`](src/paneling/fitted_rectangular_strip.rs) / [`ClippedFittedRectangularStrip`](src/paneling/clipped_fitted_rectangular_strip.rs) | Two-rail best-fit ordinary rectangle kits (+ optional per-bay inset frames) |
| [`FittedRectangle`](src/paneling/fitted_rectangle.rs) / [`ClippedFittedRectangle`](src/paneling/fitted_rectangle.rs) | Single-bay best-fit rectangle from four (possibly skew) corners |
| [`RectangularNTube`](src/paneling/rectangular_n_tube.rs) | Closed n-gon cross-section polyline → n clipped rectangular strips; [`without_face_edges`](src/paneling/rectangular_n_tube.rs) omits presentation on listed `a_i→a_{i+1}` edges |
| [`ClippedTessellatedTriangle`](src/paneling/clipped_tessellated_triangle.rs) / [`ClippedQuadPanel`](src/paneling/clipped_quad_panel.rs) / [`ClippedRuledStrip`](src/paneling/clipped_ruled_strip.rs) | Clipped triangle / quad / ruled strip |
| [`TessellatedTrianglePanel`](src/paneling/tessellated_triangle_panel.rs) | World-space triangle → panels |
| [`panel_plane`](src/paneling/panel_plane.rs) | Shared panel \(XZ\) frame for a world triangle |

Authoring guidance for Richmond (LOD, `ParentConfines`, wall vs partition) lives in [`../../CONTRIBUTING.md`](../../CONTRIBUTING.md). Kit normalization and leaf styles live in the [building-components README](../components/README.md).
