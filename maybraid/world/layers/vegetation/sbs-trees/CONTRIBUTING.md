# Contributing (`sbs-trees`)

Merge + quantization is how a construction stays cheap when a grove instances it many times. Canonical tree: [`src/storybook_tree.rs`](src/storybook_tree.rs). Also done: [`src/vase_tree.rs`](src/vase_tree.rs), [`src/jungle_storybook_tree.rs`](src/jungle_storybook_tree.rs), [`src/braid_oak_tree.rs`](src/braid_oak_tree.rs), [`src/rorys_head_trained.rs`](src/rorys_head_trained.rs), [`src/penmarch_torch.rs`](src/penmarch_torch.rs), [`src/kamakura_torch.rs`](src/kamakura_torch.rs), [`src/high_bush_shoots.rs`](src/high_bush_shoots.rs), [`src/friends_conifer.rs`](src/friends_conifer.rs), [`src/liams_conifer.rs`](src/liams_conifer.rs), [`src/northern_conifer.rs`](src/northern_conifer.rs), [`src/temperate_conifer.rs`](src/temperate_conifer.rs), [`src/simplemans_hedge.rs`](src/simplemans_hedge.rs), [`src/date_palm.rs`](src/date_palm.rs), [`src/waialea_palm.rs`](src/waialea_palm.rs), [`src/palm_bush.rs`](src/palm_bush.rs), [`src/honu_banyan.rs`](src/honu_banyan.rs), [`src/sopes_banyan.rs`](src/sopes_banyan.rs). Canonical tuft: [`src/tuft_patch.rs`](src/tuft_patch.rs). Grove wiring lives in [`vegetation-groves` CONTRIBUTING](../groves/CONTRIBUTING.md).

Comfortable unique visible meshes: a few hundred. `tree_variants` / `patch_variants` default **100**.

## Why two steps

Without quantization, every plant is a unique grown mesh (`SceneRef` / `MultiSceneMerge` cache miss per placement). Without merge, each stick and cheap ball is its own spawn even after quantization.

Quantize first (shared unit archetype), then merge (one stick collection + one cheap-ball collection per level).

## Tree construction

Do this on the **plant type** (`StorybookTree`, `VaseTree`, …), not only in the grove.

### 1. Unit archetype

Add `unit_from_num` / `into_unit_from_num` on params and the built type (see `StorybookTreeParams::into_unit_from_num`):

1. Record pre-normalize world size (tree height, or tuft `max(extent, blade_length)`).
2. Scale authored lengths/radii so the construction is **unit** height or footprint.
3. Key layout / canopy / tuft **seed by `num`**, not by world position.
4. Return `(unit_params, world_size)`. World size goes on plant [`Placement`](../components/src/placed.rs) scale in the grove.

Same `num` must rebuild the same chain. Different `num` must differ. Test both. The grown chain AABB must also be unit-sized — hop lengths, stick radii, and palm droop / arch authored in meters for a default height (Sope flair `1..4` on a 20 m stalk; Waialea droop `0.72` on a 12 m stalk) have to be fractions of that height, not leftover world meters. Date droop / arch also divide by `frond_world_scale` the same way length does, so a Palm Shade unit mesh keeps `/show` hang when grove noise picks a smaller scale.

Do **not** bake world height into the mesh. Grove placement scale (`placed.scale * world_size`) is the instance size.

### 2. Merge on emit

In `VegetationComponents` for High (and Medium if it still uses kits):

```rust
fn merge_sticks(nodes: Vec<StickNode>) -> Vec<StickNode> {
    StickNode::merge_standard(nodes).into_iter().collect()
}

fn merge_foliage(nodes: Vec<FoliageNode>) -> Vec<FoliageNode> {
    let (cheap, rest) = /* split CheapBall / CheapBallCollection vs other */;
    let mut out = rest;
    if let Some(merged) = FoliageNode::merge_cheap_balls(cheap) {
        out.insert(0, merged);
    }
    out
}
```

[`StickNode::merge_standard`](../components/src/sticks/node.rs) and [`FoliageNode::merge_cheap_balls`](../components/src/foliage/node.rs) become one [`MultiSceneMerge`](../../scene-ref) per collection. Merge already packs kit-local positions into vertex **COLOR** so [`LeafMaterial`](../shaders) breakup still works after the bake.

Leave layered / frond / procedural fallbacks as separate nodes. Palm Low is a shared five-chord star (`PalmCrownParams::unit_low_star` / `low_star_collection_nodes`) — one singleton collection per blade so UltraLow merge cannot chord the fan. Do not key that star on the High variant seed.

### 3. LOD bands stay local

Palm `structural_lod` and High/Low collection nodes bake at `from_params` ([`DatePalm`](src/date_palm.rs), [`WaialeaPalm`](src/waialea_palm.rs), [`PalmBush`](src/palm_bush.rs), [`PalmCrown`](src/palm_crown.rs)). Produce / nest emit must not rebuild rings or walk the crown AABB.

Pass `AzimuthHeightBands` at the `*_banded` call site. Do not call `torch_tree::stick_nodes_high` (or similar) if that hides another construction’s cell counts. Declare High / Medium / Low band constants on **this** module. If High draws a crook (or otherwise posed) trunk, Medium must emit those same trunk members and only thin branches — do not redraw the axis as ball-stick chords.

### 4. Widening High is a shader problem, then a factor

Raising `STRUCTURAL_HIGH_FACTOR` keeps the High mesh (merged cheap-ball cards + `LeafMaterial`) on trees that used to be Medium. Triangle count is usually fine; **draw cost is fragment cheese + `discard` overdraw**, not verts. Fronds use [`FrondMaterial`](../shaders/src/frond_material.wgsl) (palette + tip-weighted sway, opaque — no cheese / `discard`).

[`LeafMaterial`](../shaders/src/leaf_material.wgsl) `discard`s a noisy rim and interior swiss cheese **in ball-radii** (`LEAF_MID_DIST` 80 / sway cut 60: `abs / scale`). A unit kit cheeses to 80 m; a larger card keeps the same angular window so big banyans do not go rectangular while they still fill the screen. Farther: solid hub, no hole `discard`, so overlapping cards keep early-Z. Lighting is hard Lambert + sky, times fake canopy occlusion (inward faces and puff hubs) — no clustered PBR or leaf shadow maps. Opaque — alpha-to-coverage looked like a window screen.

That path is shared. Any construction that emits `CheapBall` / `CheapBallCollection` with `leaf_material_ref()` gets it. To push another tree’s High the same way:

1. Confirm High foliage is merged cheap balls on `LeafMaterial` (not layered / frond / a custom WGSL).
2. Raise that module’s `STRUCTURAL_HIGH_FACTOR` (tall woody constructions share Storybook’s `10 / 30 / 50`). Bushes and hedges stay tighter. Do not copy those numbers blindly — they are `distance / tree_radius`.
3. Profile `LeafMaterial` **fragment** time, not triangle count. If it is still hot, thin High `AzimuthHeightBands` (card count) before adding another material.
4. A plant-specific shader must copy the distance bands itself. Sticks do not need this. Fronds stay on `FrondMaterial` — do not point them at `LeafMaterial`.

Grove **tile** bands stay independent ([groves CONTRIBUTING](../groves/CONTRIBUTING.md)).

### 5. Tests

- `unit_from_num(n)` is unit-sized and deterministic.
- `grow_num(n)` returns the same `Arc` for the same type and `n` (`grow_num_reuses_arc_for_same_type_and_num`).
- `into_unit_from_num` returns the pre-normalize world size.
- High emit is one stick collection and one cheap-ball collection (Storybook: `high_emits_merged_stick_and_cheap_ball_collections`).

## Grove construction

Quantization is wasted if the grove still grows a unique `T` per cell. See [groves CONTRIBUTING](../groves/CONTRIBUTING.md): `tree_variants` / `patch_variant_index` / [`QuantizedPlant::grow_num`](src/quantized.rs) / `nest_flattened_plant_chunk`. The cache is `(construction type, num)` for the process — not a hash of params. `grow_num` returns `Arc<Self::Unit>` so groves still nest the base tree the playground already hosts. Preset silhouettes (Braid Oak, Honu, Sope, Jungle Storybook) use `type Unit = Self` and put sampled height on placement: groves must not remix SBS projection / descenders / growth onto those meshes. A grove that remixed the same model differently implements `QuantizedPlant` on a wrapper (`type Unit = StorybookTree` / `DatePalm`). Jungle Storybook `into_unit_from_num` must divide `jungle_growth_radius_scale` by height so fronds stay proportional after placement scale.
