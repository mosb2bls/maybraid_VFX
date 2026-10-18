# `sbs-geometry`

Pure numerical / geometric utilities for stalk and ball-stick constructions ([RFC-183 §3.1](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation#31-stalk-and-ball-stick-trees)).

Keeps shared pose, extents, chain math, and foliage **shape IR** (tuft / frond / high-bush / jungle-growth) **free of `sdf-common`** so **`sbs-trees`** can compose plants without forcing every crate through SDF mesh spawn.

- `chain`: hysteresis rules defining how geometry branches out, similar to an L-system.
- `anchor`: the rule responsible for placing the first elements in the chain.
- `sbs`: the geometry frontend for a given tree-like construction, this typically conceals a lot of complexity and ensures reasonable parameters for the intended aesthetic.
- `tuft` / `frond` / `high_bush` / `jungle_growth`: authoring shapes and run builders consumed by VegetationComponents.