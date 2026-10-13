# Furniture art

Blender sources for painted furniture kits. Runtime GLBs mirror this layout under `maybraid/assets/furniture/`. Path constants live in [`furniture-components` `assets.rs`](../../world/layers/furnishing/components/src/assets.rs). Skip `.blend1`.

Authored space is Blender \(Z\)-up. Engine remap is \((X,Y,Z)_{\text{Blender}} \mapsto (X,Z,Y)_{\text{engine}}\) — see [`kit_space.rs`](../../world/layers/furnishing/components/src/kit_space.rs). **\(+Y\) is the wall / back**; the room is \(−Y\).

| Convention | Bounds | Anchor |
|---|---|---|
| Box carcass (bed, chest, counter, fridge / wardrobe / range body, table top) | \(X,Y \in [-1,1]\), \(Z \in [0,1]\) | Floor at \(Z=0\) |
| Shelf row | poles \(Z \in [0,1]\), slanted deck | Deck / lip start at \(Z=0\); stack on \(Z\) |
| Hinge door (fridge, wardrobe) | \(X \in [0,1]\), \(Y\) thin about \(0\), \(Z \in [0,1]\) | Hinge-front-bottom at origin; panel in \(+X\), proud toward \(−Y\) |
| Basin | plan \(\approx [-1,1]\), bowl in \(+Z\) | Sit-on-top; contact at \(Z=0\) |
| Faucet | column \(+Z\), spout \(−Y\) | Deck mount at origin |
| Table leg / pedestal | plan about origin, \(Z \in [0,1]\) | Floor at \(Z=0\) |
| Range door | \(X \in [-1,1]\), \(Y\) thin about \(0\), \(Z \in [0,1]\) | Bottom-front hinge; panel in \(+Z\) |
| Rug | plan fill, thin \(Z\) | Floor at \(Z=0\) |
| Sit-on props (fruit, bread, food display, cookware) | around unit size | Rest on \(Z=0\) |

One mesh per file, origin at world 0, default Camera + Light kept. Paint is engine-side (`MaterialRef`); do not rely on authored materials.

Regenerate the fill kits with:

```bash
blender --background --python scripts/furniture-author/main.py
```
