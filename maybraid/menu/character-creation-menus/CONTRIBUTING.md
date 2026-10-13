# Contributing to the `character-creation-menus` crate

## General

## Adding clothing options

`Braidman` and `Brodler` clothing menus share one catalog. Species menu structs in
`src/characters/braidman.rs` and `src/characters/brodler.rs` store worn layers in
`Section<ClothingMenu>`; [`ClothingMenu`](src/shared.rs) in `src/shared.rs` builds
the UI from `ClothingMesh::VALUES` via [`MenuComponent::menu_node`](src/shared.rs).
You do not add per-species menu fields or render rows for a new garment.

To expose a new clothing mesh in both species:

1. The blend-export pre-commit hook writes the canonical `.glb` under
   `maybraid/assets/characters/clothes/body/`.
   Menu thumbnails always use this catalog path. Body garments also have host-fit
   GLBs at `clothes/body/{body_stem}/{file_stem}.glb` from
   `scripts/clothes-fit/fit.sh`; blend-export must not overwrite those.
2. In `maybraid/world/layers/mobs/characters/items/src/clothing.rs`:
   - Add a `CLOTHING_*` path constant for the file.
   - Add a variant to `ClothingMesh`.
   - Append it to `ClothingMesh::VALUES`.
   - Add matching `label()`, `file_stem()`, `path()`, and `nouns()` arms. Labels use
     kebab-case (for example `harem-pants-upper`); asset filenames may use snake_case.
     `nouns()` feeds hashed item names with look/color adjectives
     (`hashed_item_name` in `character-items`).
   - Body garments always resolve through `path_on(host)` as
     `clothes/body/{body_stem}/{file_stem}.glb`. Generate those files with
     `scripts/clothes-fit/fit.sh`.
   - Clothing color and surface recipe are independent of the mesh. Species
     menus share `ClothingMenu`: color swatches and look tiles (`space-suit`,
     `tattered`, `hawaiian`, `cloth`, `scales`, `wizards-veins`, `glitter`) per
     worn layer. Each look also has `adjectives()` used by hashed item names.
     Unset layers fall back to `clothing_material` / `clothing_default`.
     Assembly stamps `MaterialRef::named(recipe_id).with_palette([color])`; Crozon’s
     MaterialLib claims those names and packs palette / noise / scalars / rasters
     onto the clothing shader uniform.
3. `ListValues` and `AssetOption` for `ClothingMesh` in `characters` pick up the
   new variant automatically, so the UI, CLI (`--clothing`), and preview assembly work
   without further changes in this crate.

Verify with:

```bash
cargo test -p character-creation-menus
cargo check -p character-concepts-playground
```
