# Contributing to `buildings`

Higher-order building procedures on top of [`building-components`](../components/) (`building-components` crate).
For Richmond-wide IR / LOD rules, see [../../CONTRIBUTING.md](../../CONTRIBUTING.md).

This guide walks the **Les Halles storey** pattern as a template for
parameterized → plan → full fitting, openings, and usage-area fill.

---

## Fitting contract (quick)

Most complete types implement [`Fit`](src/fit.rs):

```rust
fn fit_to_confines(
    confines: &Confines,
    noise: NoiseParams,
) -> Result<(Self, FillableRegions), FitError>;

// default: map fit_to_confines; keep successes; residualize TooSmall
fn fit_to_multi_confines(
    multi: &MultiConfines,
    noise: NoiseParams,
) -> Result<MultiFit<Self>, FitError>;
```

- **`Confines`** — AABB + roll + [`Openings`](src/openings.rs) the type must honor
  (openings for a residual live on that region’s confines).
- **`MultiConfines`** — several typed [`FillRegion`]s (L / grouped cells). Leaf
  types keep implementing only `fit_to_confines`; joint multi-cell layouts can
  override `fit_to_multi_confines`.
- **`MultiFit`** — `fitted: Vec<(Self, FillableRegions)>` plus `residual` for
  soft rejects and nested leftovers.
- **`FillableRegions`** — residuals after a successful fit:
  - `within` — typed [`FillRegion`]s (kind + confines) for child fill.
  - `atop` — stack footprints for towering (often ignored when reusing one floor plan).
- Soft capacity failures use **`FitError::TooSmall`** so callers can try another
  catalog entry or leave the region unfilled.

Preferred authorship shape:

```text
Parameterized::sample(confines, noise)
        │
        ▼
Plan::from_parameterized(params, confines)  →  structure + FillableRegions
        │
        ▼
Full::from_plan / fill_from_regions         →  presentable type + residuals
```

`Fit::fit_to_confines` usually chains those three steps.

---

## Les Halles as the worked example

### What the storey means

Les Halles is an **outer commercial gallery ring** around an **inner balcony**,
with optional **shafts** (stairs) in corners or mid-sides:

| Band | Role |
|------|------|
| Gallery | External shop strip — walls, floor, stall doors on the inner wall, façade apertures outside |
| Balcony | Walkway — floor only, open to the courtyard |
| Shafts | Internal residuals — only when inbound openings map onto a slot |

Module entry: [`storeys/les_halles.rs`](src/storeys/les_halles.rs).

### Layer cake

```text
LesHallesParameterized::sample
        │  gallery/balcony widths, shaft placement, door/window catalogs, …
        ▼
LesHallesFloorPlan::from_parameterized
        │  ring shell + authored openings + FillableRegions
        │    within: ExternalSpace strips (gallery bays) + Walkway/Internal shafts
        ▼
LesHallesFullStorey::fill_from_regions
        │  for each ExternalSpace → CommercialStallStrip::fit_to_confines
        ▼
CommercialStallStrip
        │  voronoi bays from Passage openings → CommercialStall per bay
        ▼
CommercialStallInterior (catalog first-fit)
        │  Bites / MiniMart / Parts / KnickKnack / PublicRestroom / … / Lounge
        ▼
(optional) nested FillableRegions  e.g. MiniMart office, restroom stalls
```

Playground: `/show les-halles-full-storey`, `/show les-halles-livable-full-storey`,
`/show les-halles-livable-full-storey-examples`, `/show mixed-use-les-halles-monotower`,
`/show commercial-stall-strip`, and the per-interior galleries
(`mini-mart-examples`, `public-restroom-examples`, …).

### Livable gallery fill (Les Halles livable Full\*)

[`LesHallesLivableFullStorey`](src/storeys/les_halles/livable_full_storey.rs) shares
the same [`LesHallesFloorPlan`](src/storeys/les_halles/floor_plan.rs) as the
commercial Full\*. It samples deeper galleries via
[`LesHallesParameterized::sample_livable`](src/storeys/les_halles/parameterized.rs),
then fills each `ExternalSpace` strip with **lengthwise passage bays** (voronoi
along the strip, merge undersized cells — same idea as
[`CommercialStallStrip`](src/usage_areas/commercial_stall_strip.rs)). Each bay
is a single rectangle fitted with
[`RectangularLivableArea`](src/usage_areas/rectangular_livable_area.rs)
(SingleClosed → Guillotine → AllOpen; courtyard `Passage` as RLA opening; no
SpineHall). Party walls + RLA internals use the same High-only
`internal_walls` band as [`LivableApartments`](src/usage_areas/livable_apartments.rs);
structural probe is the whole-storey outer footprint (local), with fine-phase
viewer mapping through host `GlobalTransform` (~80 m outside → High).
Within-strip bay cuts always wall; cross-strip shared edges wall from noise
(~50%) so some corners stay open as L-shaped living. No
[`HallsToShafts`](src/usage_areas/halls_to_shafts.rs), suite packing, or
[`LivableApartment`](src/usage_areas/livable_apartment/) entry carve on this
path. Prefer larger footprints than commercial demos (playground default
`72,4,54`) so strip depth hosts livable bays.

### I-frame rectangularization (I-Apartment)

[`storeys/i_apartment`](src/storeys/i_apartment.rs) starts from the I-frame alone:

1. Sample I-layout knobs from seed (I/T/L/Z arms; stem may be narrower than the
   apartment-favoring end bars); fit an [`IFloor`](src/shells/i_floor.rs) to confines.
2. Floor plan samples a corridor `hall_width`, emits the shell’s natural **1–3
   primary rectangles**, packs exterior apertures, remaps inbound **shaft**
   openings onto 3×3 pocket centroids, and authors hall-width **passages** on
   shared edges between primary rects.
3. Full\* fills each primary rect with [`LivableApartments`](src/usage_areas/livable_apartments.rs):
   sample [`LivableApartmentsParameterized`](src/usage_areas/livable_apartments.rs)
   (target m² catalog) →
   [`HallEnclosedSuites`](src/usage_areas/hall_connected_suites.rs)
   ([`HallsToShafts`](src/usage_areas/halls_to_shafts.rs) → split →
   [`pack_apartments_to_targets`](src/usage_areas/plan_cells.rs) with
   [`MIN_GROUP_CONNECTIVITY`](src/usage_areas/plan_cells.rs) **2.0 m** for
   both inter-cell joins and hall frontage → one hall door / group +
   partition / hall-edge / host-perimeter walls, skipping
   [`OpeningLabel::Boundary`](src/openings.rs)) →
   [`LivableApartment`](src/usage_areas/livable_apartment/) fill per suite.
   Multi-rect I plans fill primary rects **progressively** (earlier rect walls
   shared edges, then injects Boundary onto later siblings).
   (entryway → [`RectPassageCluster`](src/usage_areas/rect_passage_cluster.rs)
   → per-rect
   [`RectangularLivableArea`](src/usage_areas/rectangular_livable_area.rs)).
   Shared XZ helpers: [`plan_geom`](src/usage_areas/plan_geom.rs); RLA carve
   sizes: [`slot_policy`](src/usage_areas/rectangular_livable_area/slot_policy.rs).
   Strategies default to `CaseAttempt`. Min hall clear inside apartments is
   **1.0 m**. Bedroom / bathroom walls only at apartment level; ungrouped /
   soft-failed pockets stay [`SpaceKind::InternalSpace`](src/fit.rs); Full\*
   maps leftovers to [`SpaceKind::ClosetSpace`](src/fit.rs).

Playground: `/show i-apartment-floor-plan`, `/show i-apartment-floor-plan-examples`,
`/show i-apartment-full-storey`, `/show i-apartment-full-storey-examples` (gallery),
`/show livable-apartments-examples` (standalone packs; multi-cell groups),
`/show livable-apartment-examples` (small→large rect + L/T layouts; max-rect /
passage / hall gizmos),
`/show livable-rectangles-examples` (standalone `RectangularLivableArea` cells;
sizes, 1–3 passages, forced strategies),
`/show halls-to-shafts` (hall / shaft / passage / residual AABB gizmos).

### Parameterized → floor plan

[`LesHallesParameterized`](src/storeys/les_halles/parameterized.rs) samples knobs
from spatial noise (gallery depth, balcony depth, shaft mode, opening density,
door/window size catalogs).

[`LesHallesFloorPlan`](src/storeys/les_halles/floor_plan.rs) turns those knobs into:

1. **Shell geometry** — gallery walls / floors, balcony floor (ceiling optional).
2. **Authored openings** — stable ids via [`OpeningId::scoped`](src/openings.rs)
   (`les_halles_*`). Preserve inbound confine opening ids unchanged.
3. **Mapped residuals** — `FillableRegions::within` with the right [`SpaceKind`](src/fit.rs):
   - `ExternalSpace` — gallery strips that Full\* will fill with stalls.
   - `Walkway` / `InternalSpace` — balcony / shafts left for later or other types.

Inbound **shaft** openings are remapped onto fitted slots (corner quadrants or
mid-side bands). A shaft is only authored when at least one inbound opening maps
to that slot.

### Full storey fills usage areas

[`LesHallesFullStorey`](src/storeys/les_halles/full_storey.rs) does **not** rebuild
the ring. It:

1. Fits (or accepts) a `LesHallesFloorPlan`.
2. Iterates `regions.within`.
3. For each `SpaceKind::ExternalSpace`, calls
   [`CommercialStallStrip::fit_to_confines`](src/usage_areas/commercial_stall_strip.rs)
   with a per-strip seed offset.
4. On `TooSmall`, leaves the strip in residual `within` (unfilled gallery).
5. Passes other kinds through unchanged.

[`LesHallesLivableFullStorey`](src/storeys/les_halles/livable_full_storey.rs) is the
same loop with residential program: passage-bay split →
[`RectangularLivableArea`](src/usage_areas/rectangular_livable_area.rs) per bay
+ within-strip party walls + noisy cross-strip walls. Commercial vs livable is a
Full\* choice over one floor plan.

That is the **FloorPlan → Full\*** split: the plan owns structure + residual
confines; Full\* owns program fill.

### Openings: author, forward, subset

| Stage | Behavior |
|-------|----------|
| Floor plan | Authors stall doors / façade apertures / shafts with scoped ids. Ring mapping truncates a leaf slightly to fit (or drops it); only **mapped** Passage/Aperture AABBs are kept and subset onto external strips. |
| Stall strip | Voronoi-assigns **Passage** openings to bays (each bay owns ≥1 passage uniquely; leading/trailing runs without their own door stay on the end bays). A bay that soft-fails is absorbed by extending the previous stall — never leave an uncovered along-run or a passage-less stall. |
| Stall shell | Punches Passage / Aperture / Shaft into wall strips; **forwards** only Passage + Aperture into the interior fit. |
| Interior | Consumes passages for clearance / counters / doors; may **author** nested openings (office door, stalls door) with its own scope (`mini_mart`, `public_restroom`, …) and put them on residual `within` confines. |

Rules of thumb:

- Generated ids → `OpeningId::scoped(SCOPE, role, slot)`.
- Soft-fail with `TooSmall` when required openings or mins cannot be met.
- Never forward a boarded / unmapped connectable void into usage-area fill.
- Keep walkways free with [`usage_areas::clearance`](src/usage_areas/clearance.rs)
  (`PassageClearance`, `pack_abutting_clearance`).

### Usage areas

[`usage_areas`](src/usage_areas.rs) types fill residual confines. Commercial
stalls are one family:

| Type | Role |
|------|------|
| `CommercialStallStrip` | Pack bays along a gallery band |
| `CommercialStall` | Shell walls + catalog interior |
| `CommercialStallInterior` | Weighted preferred kind, then first-fit; Lounge last |
| Per-interior modules | Parameterized → plan pack → labels/panels |

Interiors that introduce private rooms (MiniMart office, restroom stalls) emit
those rooms as `FillableRegions::within` so a later pass can fill them—or so
tests/playgrounds can assert tracked doors.

### Program tiers (prefer structure, then residual fill)

When a usage area greedily packs many kinds into one host, use the shared
[`placer`](src/placer.rs) **KindSpec catalog** model rather than ad
hoc concept enums:

| Piece | Role |
|-------|------|
| [`KindSpec`](src/placer/kind.rs) | Tier, weight, caps, propose knobs, predicates, commit effect |
| [`pick_kind`](src/placer/tiers.rs) / [`pack_kinds`](src/placer/pack.rs) | Weighted pick + propose/predicate/commit loop |
| [`ProposeKnobs::FreeExtentFrac`](src/placer/kind.rs) | Host-relative free extents (dining tables, …) |
| [`SoftGoalRole`](src/placer/kind.rs) | Credits soft-goal (`Appointed` / closet / ensuite) to open fillers |
| [`try_corner_l`](src/placer/composition.rs) / peninsula | Shared L-run / peninsula composition |
| [`OccupiedBudget`](src/placer/budget.rs) | Furniture vs structure occupancy caps |
| [`WalledRoomFill`](src/placer/walled_room.rs) | Presentable partition + door for `FillableRegions::within` |

Tiers:

1. **Enclosure / structure** — rooms with doors (ensuite, closet, office, …)
2. **Appointed** — primary furniture / counters tied to the program
3. **Filler** — leftover mid-size boxes (`BedroomFurniture`, extra displays, …)

Gate fillers until an enclosure soft-goal is met (and soft-cap them). Partition
door commits use [`commit_door_clear`](src/usage_areas/clearance.rs) with
[`PASSAGE_APPROACH_PAD`](src/usage_areas/clearance.rs) for inward approach
keep-outs. See [`common_bedroom/layout.rs`](src/usage_areas/common_bedroom/layout.rs).

### Livable quarters

[`livable_quarters`](src/usage_areas/livable_quarters/) holds residential program
rooms (kitchen, dining, sitting, study, living, bathrooms). Each follows
**parameterized → plan pack → Fit + BuildingComponents**, sharing the placer
furniture loop in [`placer::pack`](src/placer/pack.rs).
[`EatingArea`](src/usage_areas/livable_quarters/eating_area.rs) bipartitions a
host to place dining beside a kitchen, or falls back to kitchen-only;
[`RectangularLivableArea`](src/usage_areas/rectangular_livable_area.rs) prefers
`RectQuarterKind::Eating` over separate kitchen/dining slots.
[`CommonBedroom`](src/usage_areas/common_bedroom/) composes
[`ResidentialBathroom`](src/usage_areas/livable_quarters/residential_bathroom.rs)
into ensuite `within` residuals when fit succeeds.

---

## Adding a new commercial interior

1. Add `stall_layout/<name>.rs` packer (clearances, mins, soft-fails).
   For a wall-seeded private room with a sales-face door + panels, reuse
   [`usage_areas/enclosed_room.rs`](src/usage_areas/enclosed_room.rs)
   (MiniMart / Parts offices, PublicRestroom stalls, and CommonBedroom
   closet / ensuite partitions already do).
2. Add `<name>_stall/{parameterized.rs,..}` — `sample` → `Plan::from_parameterized`.
3. Implement `Fit` + `BuildingComponents` (labels; panels if you author walls).
4. Register in [`interior.rs`](src/usage_areas/commercial_stall_strip/commercial_stall/interior.rs)
   catalog + first-fit order.
5. Export from the commercial_stall / usage_areas / lib roots as needed.
6. Add playground `/show` + examples gallery when visual QA helps.
7. Soft-fail tiny / no-passage bays; never panic on capacity.

---

## Paneling primitives and higher-order types

This crate sits **above** kit IR. Prefer composing existing panel / strip helpers
rather than inventing new tessellation paths.

**Primitives** (in [`paneling`](src/paneling.rs); type table in [README.md](README.md)):

| Need | Reach for |
|------|-----------|
| Oriented single bay | [`Rectangle`](src/paneling/rectangle.rs) / [`ClippedRectangle`](src/paneling/rectangle.rs) |
| Node-chain wall run | [`RectangularStrip`](src/paneling/rectangular_strip.rs) / [`ClippedRectangularStrip`](src/paneling/clipped_rectangular_strip.rs) |
| Best-fit from skew corners | [`FittedRectangle`](src/paneling/fitted_rectangle.rs) / strip variants |
| Closed n-gon tube | [`RectangularNTube`](src/paneling/rectangular_n_tube.rs) |
| Freeform mesh → panels + crease joints | [`PanelComplex`](src/paneling/panel_complex.rs) |

Shell helpers in [`enclosure_panels`](src/usage_areas/enclosure_panels.rs)
(`face_rectangle`, `face_span_rectangle`, …) build those primitives on AABB
faces; [`enclosed_room`](src/usage_areas/enclosed_room.rs) uses them for
office/stall/bedroom partitions.

**Higher-order composition** (this crate’s job):

```text
Fit / Parameterized → Plan
        │  structure: walls, floors, authored openings
        │  residuals: FillableRegions (typed confines for children)
        ▼
Full* / usage area
        │  fill residuals with nested Fit types
        ▼
BuildingComponents → PanelNode / LabelNode / … (no GLB paths here)
```

- **Floor plans / shells** own envelope geometry and opening ids.
- **Usage areas** pack program into residual confines and may emit further
  `within` rooms (office, restroom stalls).
- Present with [`ComponentsOnly`](../components/src/lib.rs)`<T>` unless
  the type needs a custom `LodScene`.

Richmond-wide IR / LOD / `ParentConfines` rules stay in
[../../CONTRIBUTING.md](../../CONTRIBUTING.md). Kit taxonomy and asset aliases stay in
the [buildings README](README.md) and
[building-components README](../components/README.md).

---

## Related

- [Richmond CONTRIBUTING](../../CONTRIBUTING.md) — IR nodes, LOD, `ParentConfines`
- [buildings README](README.md) — kit taxonomy + paneling type table
- [`fit.rs`](src/fit.rs) — `Confines` / `MultiConfines` / `MultiFit` / `FillableRegions` / `SpaceKind`
- [`placer`](src/placer.rs) — predicate-based rectangular KindSpec layout trier
- [`openings.rs`](src/openings.rs) — opening labels and scoped ids
- [`paneling`](src/paneling.rs) — panel primitives used by shells and enclosures
- [`enclosed_room`](src/usage_areas/enclosed_room.rs) — shared wall-seeded room + door + panels
- [`common_bedroom`](src/usage_areas/common_bedroom/) — residential fill (beds + walled closet / ensuite)
