# Chico vegetation crates (`RFC-183`)

Workspace crates under **`maybraid/world/layers/vegetation/`** implement [RFC-183: Chico Vegetation](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation). Epic: [#185 — Implement RFC-183: Chico Vegetation](https://github.com/ramate-io/maybraid/issues/185).

The highest-order model is [`chico`](./chico/) (`chico`). Lower-order crates are named by what they do.

## Layout

| Path | Crate | Role |
| --- | --- | --- |
| [`chico/`](./chico/) | **`chico`** | Highest-order vegetation model. |
| [`sbs-geometry/`](sbs-geometry/) | **`sbs-geometry`** | Stalk / ball-stick geometry plus tuft, frond, and bush shape IR. |
| [`components/`](components/) | **`vegetation-components`** | Domain IR (`StickNode` / `FoliageNode`) + `VegetationComponents` / `LodScene`. |
| [`sbs-trees/`](sbs-trees/) | **`sbs-trees`** | VegetationComponents plants for stalk and ball-stick trees ([§3.1](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation#31-stalk-and-ball-stick-trees)). Tracks [#186](https://github.com/ramate-io/maybraid/issues/186). |
| [`groves/`](groves/) | **`vegetation-groves`** | Grove recipes. |
| [`shaders/`](shaders/) | **`vegetation-shaders`** | Leaf / frond / stick / bump-out materials. |
| [`bumpout/`](bumpout/) | **`vegetation-bumpout`** | Canopy bump-outs. |

## Dependency direction

```text
sdf-common

sbs-geometry   (shape IR + chain / anchors / SBS)

sbs-trees ──► sdf-common, sbs-geometry, vegetation-components

chico ──► sbs-trees, vegetation-groves
```

Lower crates stay small and reusable; **`sbs-trees`** is the integration point for milestone **RFC-183 4.1** / issue **#186**.
