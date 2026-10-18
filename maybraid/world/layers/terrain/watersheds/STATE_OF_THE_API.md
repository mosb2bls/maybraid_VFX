# State of the Marazion API

Snapshot after the HydroNode / shoreline-roughness work. Spec backdrop:
[RFC-127 Marazion watersheds](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-127-marazion-watersheds).

## What this crate is for

Marazion authors **bounded hydrology** that Durham can gather and blend on the
shared origin-cell lattice. It does **not** (yet) author large drainage basins
or valley grades that make those water bodies look inevitable in the base
terrain.

Rough split:

| Layer | Role today |
| --- | --- |
| **Authored leaves** | Lake / stream / bog / streams-graph plans → `HydroNode`s |
| **Primitive blend** | Class-priority carve → rim → apron (+ optional backfill) |
| **Complex** | Bag of indexed nodes; sample-time elevation + `WaterFill` |
| **Durham** | Cellular gather, compose on terrain / water passes |

See also [`WATERSHED_CORRECTION.md`](src/WATERSHED_CORRECTION.md) (extents /
discoverability) and Durham
[`durham/CONTRIBUTING.md`](../durham/CONTRIBUTING.md) (cellular identity, water
fill, shoreline roughness).

## What is strong

**Safe, streamable, globally blendable pocket hydro.**

1. **`HydroNode` is the portable unit.** Hydraulic primitive + `HydroParams` +
   `max_correction_extent` (+ optional `HydroBackfill`). Leaves emit nodes;
   complexes and Durham do not re-author geometry.
2. **Extents are honest enough to stream.** Index by hydraulic support ⊕
   correction pad. `HydroNode::inbounds(cell)` + leaf `hydro_nodes()` filters
   keep support inside the pocket cell (no water walls from spilled aprons).
3. **Sample cost stays local.** Broadphase candidates → class blend
   (`TerrainBlendStage`) → optional soft-max backfill. Hot path is “nearby
   nodes,” not a global graph walk.
4. **Right algebra by band.** Soft-min beds, soft-max banks, bare blend for
   fill / freeboard, grit only on terrain elevation. Overlapping rim backfills
   soft-max; they must not sum.
5. **Water shares the terrain lattice.** `WaterFill` = carve occupancy ×
   half-space below \(W\). Shore cosmetics are terrain-side, not a denser water
   grid.

This is enough to author **wet basins** (where water sits) that compose cleanly
across chunks.

## What it is not

**A drainage-basin / valley author.**

Current stamps carve a reservoir-shaped depression, rim it, and fill below
\(W\). They do **not** grade large catchments so the water body reads as the
sink of a landscape. A dry spell (drop the water mesh) still leaves a punched
bowl in noise — evidence that we authored a wet pocket, not a basin.

That larger grading — hydrologically reasonable valleys, whole drainage
regions, erosional-looking structure — is the expensive **structure** pass.
It is also what will likely **replace many Jersey stamps**: real terrain is
mostly erosion; local recipes should stop inventing the watershed.

RFC path for that work:

- Pocket orchestration: [§3.1.3.4 Pocket complex](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-127-marazion-watersheds#3134-pocket-complex)
- Large chains / basin shaping: [§3.2 Basin water](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-127-marazion-watersheds#32-marazion-basin-water-stamping)

## Design bets going forward

Semantic composition has to answer three questions:

1. **Can we define large structures that make sense?** (catchments, rings, grades)
2. **Can we hierarchy them so we stream small parts?** (coarse cells emit fine nodes)
3. **Can sampling stay playable?** (expensive authorship amortized; cheap evaluate)

HydroNodes already push hard on (3). Leaves author nodes; optimizing banks and
depressions is mostly Node / Complex routine work from here.

(1) and (2) are the next product surface: coarse hydrology authors structure
once per region/cell; nodes keep doing interactive carve / rim / fill / wet
volume. Playable time means structure is not re-simulated per sample.

## Module map (quick)

```text
authored/     pre_pocket → pocket_cell → lake | stream | bog | streams_graph
primitive/    HydroNode, HydroComplex, HydroParams, backfill, WaterFill, hydro fields
```

Public entry points are re-exported from `lib.rs` (`Lake`, `Stream`, `Bog`,
`StreamsGraph`, `HydroNode`, `HydroComplex`, `WaterFill`, …).

## Known sharp edges

- **Extent vs budget mismatch** silently drops nodes via `inbounds` — keep
  inscription and `TARGET_RIM_WIDTH` / apron aligned.
- **`HydroComplex` is still a bag** — pocket-complex semantics (hubs, mouths,
  bog attenuation, invariants) are not first-class yet.
- **Base terrain mismatch** — steep pre-terrain + pocket carve still looks
  cut-in; grit helps shores, not valley inevitability.
- **One backfill per node** — enough for rim grit / bog hummocks; layered
  shore recipes may need a careful extension later.
