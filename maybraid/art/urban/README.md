# Urban art

Blender sources for Richmond urbanization kits. Runtime GLBs mirror this layout under `maybraid/assets/urban/` (exported by the art pipeline). Path constants live in [`building-components` `assets.rs`](../../world/layers/urbanization/buildings/components/src/assets.rs).

## Two kit layers

Urban kits split into **shared style geometry** and **domain-specific components**.

### Shared within a style — `panels/`, `arcs/`, `joints/`

These folders hold **widely reusable geometry** for a given material / look (e.g. `rough_stonework`, `shepherds_thatch`). The same rectangle, arc, or joint is meant to appear across many primitive types (partitions, roofs, floors, doors, …), not only under one domain.

| Folder | Role |
|--------|------|
| [`panels/`](panels/) | Rectangles and right triangles on the ground (\(X,Z\) unit edges, thin \(Y\)); wall/pitch consumers rotate as needed |
| [`arcs/`](arcs/) | Angular bodies (`arc_15` / `arc_90` / `arc_180`), height **slices**, frames |
| [`joints/`](joints/) | Segment joints / corner fillers between abutting runs |

Organize by **style subfolder**, and do **not** repeat the style name in the filename (`rough_stonework/arc_90_001`, not `rough_stonework/rough_stonework_90_001`).

### Panel normalization

- **Rectangle** (`rectangle_001`): \(X, Z \in [0, 1]\), \(Y \in [-0.2, 0.2]\)
- **Right triangle**: \(X \in [0, 1]\), \(Z \in [-1, 0]\), \(Y \in [-0.2, 0.2]\)

Partition walls scale \((\texttt{length}, \texttt{thick}, \texttt{height})\) on \((X,Y,Z)\) then pitch \(\pi/2\) about \(+X\) so \(+Z\) stands up (see building-components `wall_placement*`). Length scale is the full span — the kit edge is \(1\), not \(2\).

### Micro-parts — `parts/`

[`parts/`](parts/) holds **reusable piece libraries** for a style — individual stones, thatch tufts, and other sub-components that authors instance or bake into finished kits. These are authoring sources more often than direct runtime leaves.

### Domain paths — `floors/`, `partitions/`, `stairs/`, …

Some geometries, uses, and styles need **function-specific** components that do not belong in the shared layer. Domain folders keep those kits next to their authorship intent and remain the **fast path** when building floors, walls, stairs, and similar features without hunting through shared panels/arcs.

| Folder | Role |
|--------|------|
| [`floors/`](floors/) | Floor-slab and floor-only fillers |
| [`partitions/`](partitions/) | Partition-only / wall-specific kits still awaiting promotion or that stay bespoke |
| [`stairs/`](stairs/) | Treads and stair-only pieces |

Roof pitch / dome leaves currently consume shared **panels** (and eventually arcs); domain roof folders may return when roof-only kits appear.

## Layout sketch

```
urban/
  panels/
    unit_right_triangle          # style-agnostic when useful
    rough_stonework/
      rectangle_001[+ LOD]
      inscribed_square_001
    shepherds_thatch/
      right_triangle_001_*_res
  arcs/
    rough_stonework/
      arc_{15,90,180}_001[+ LOD]
      arc_{15,90}_slice_001[+ LOD]
      arc_90_frame_001
  joints/
    rough_stonework/
      joint_001_{high,mid}_res
  parts/
    rough_stonework/parts
    shepherds_thatch/parts
  floors/
    rough_stonework/…
  partitions/
    quarried_stone/…             # domain / style-specific leftovers
  stairs/
    rough_stonework/…
```

## LOD and exports

Prefer `{name}_{high,mid,low}_res.blend` for resolution variants, with an optional bare `{name}.blend` default. Export to the matching relative path under `assets/urban/`. Consumers resolve paths through Rust `AssetPath` constants — do not hardcode old domain paths after a kit moves into `panels/`, `arcs/`, or `joints/`.

## See also

- [`building-components` README](../../world/layers/urbanization/buildings/components/README.md) — normalization spaces and IR
- [`buildings` README](../../world/layers/urbanization/buildings/lib/README.md) — higher-order authorship on top of kits
