## Proposal: Hydrology nodes with conservative correction extents

The correction pipeline should begin with authored `HydroNode`s and preserve those nodes as the source of truth for carving, rimming, and aproning. Each node describes its hydraulic primitive and correction parameters, while `max_correction_extent` describes only how far away a later correction pass may need to load that node.

```text
Authored watershed plan
        │
        │ emits
        ▼
HydroNode
  ├─ hydraulic primitive
  ├─ water / bed fields
  ├─ rim and apron parameters
  └─ max_correction_extent
        │
        │ indexed by:
        │ hydraulic support expanded by max_correction_extent
        ▼
World Spatial Index
```

`max_correction_extent` is not a stored AABB and does not describe a finalized apron. The node exposes a conservative distance from its intrinsic hydraulic support; the spatial index derives the expanded indexing bounds. Its purpose is to guarantee that any correction cell capable of being affected by the node can discover and load it.

```rust
pub struct HydroNode {
    pub primitive: HydroPrimitive,
    pub params: HydroParams,

    /// Maximum distance beyond the node's hydraulic support at which
    /// any watershed correction pass may need to reference this node.
    ///
    /// Used for safe spatial loading of carve, rim, and apron passes;
    /// it does not itself define the final correction profile.
    pub max_correction_extent: f32,
    pub backfill: Option<HydroBackfill>,
}
```

The indexed nodes are then gathered into a `HydroComplex`. Complex aggregation establishes the semantic relationship between nearby hydrology primitives: streams entering lakes, tributaries joining, overlapping depressions, and internal boundaries that should disappear.

```text
World Spatial Index
        │
        │ HydroComplex gathers related nodes
        ▼
HydroComplex
  ├─ member HydroNode references
  ├─ union hydraulic footprint
  ├─ combined bed / water fields
  ├─ exposed rim boundary
  └─ shared watershed metadata
```

The complex does not need to copy every carve, rim, or apron parameter into a separately prepared operation. It can retain references to its member nodes and expose a shared evaluation view. This keeps the original hydraulic geometry and authored parameters available to all later stages without introducing an `AproningNode` that merely repeats the same information.

Correction cells then gather every complex intersecting their region. Because the complexes originate from nodes indexed by `max_correction_extent`, the gather is conservative for all three correction passes. Carving and rimming may overfetch nodes whose larger correction support exists primarily for aproning, but their evaluators can cheaply reject nodes that do not affect the sampled point.

```text
HydroComplex
        │
        ├──────────────┬───────────────┐
        ▼              ▼               ▼
   CarvingCell     RimmingCell     AproningCell
        │              │               │
   union bed / φ   exposed rim / φ   rim + carve reference
        │              │               │
        └──────────────┴───────────────┘
                       │
                       │ ordered terrain modulation
                       ▼
             carve → rim → apron
                       │
                       ▼
                    Terrain
```

The essential safety invariant is:

[
\text{node may affect correction at }x
;\Longrightarrow;
x\text{ lies within the node's indexed correction support}
]

Therefore, `max_correction_extent` must include the largest permitted apron distance as well as any smoothing tail, boundary displacement, sampling margin, or other widening performed by correction evaluation. Overestimating the extent causes harmless overfetching; underestimating it can cause a correction cell to omit a required node and produce a hard discontinuity.

For the initial architecture, complex aggregation may combine, suppress, and reshape member effects, but it should not invent correction support beyond the extents declared by its member nodes. If later watershed-wide rules require an extent based on the total size or topology of the assembled complex, a derived `CorrectionNode` or separately indexed complex support can be introduced then.

The resulting pipeline keeps the responsibilities narrow: authored systems emit bounded hydrology nodes; `max_correction_extent` makes those nodes safely discoverable; complex cells assemble watershed semantics; and carving, rimming, and aproning cells perform ordered terrain correction against the same underlying hydrology.
