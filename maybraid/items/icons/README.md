# Item Icons

Maybraid first-encounter wiring on top of [`screen-capture`](../../screen-capture).
Hosts request both [`CaptureMode::Shaded`] and [`CaptureMode::Silhouette`] the
first time an inventory item is seen. The crate never mutates the live scene. Isolated captures spawn the same
posed hosts the inspect preview uses: [`firearm_user::GeneratedFirearm`] for
kits, a bind-pose clothing preview with catalog materials, the in-world
[`grenades::GrenadeMaterial`] charge, and skill-map tiles painted into a
texture then archived with [`CaptureRequest::from_image_handle`]. Silhouette
mode strips clothing/face/grenade shaders every frame so shader restamp cannot
leave custom materials in the extract. Clothing and face keep
[`material_ref::MaterialRefApplied`]; grenades get a [`StandardMaterial`]
stand-in so [`CaptureHostReady`] can still fire. Capture restamps unlit gray
through settle/readback. Icon
requests use [`CaptureFraming::FitExtents`]: the camera fits posed extents
(skinned meshes are CPU-skinned, so collapsed stock/barrel bones drop out),
renders 3× supersampled, then crops to the rendered alpha so the kit is
centered and fills the PNG. Hosts that know better can insert
[`screen_capture::CaptureExtents`] on the capture root. Firearm silhouettes
capture landscape so the HUD slot can match the gun's aspect.

Inventory catalogs look up **shaded** handles by
[`character_items::InventoryItem::icon_catalog_key`]. The skill-map HUD looks
up **silhouette** handles from [`ItemIconCache`]. Pending and failed captures
leave those lookups empty so callers can keep a placeholder.

Home-menu load spawns [`RequestPruneItemIconCache`], which deletes PNG files
whose names are not in any saved character inventory.

```rust
use item_icons::{request_inventory_icons, ItemIconPlugin};

app.add_plugins(ItemIconPlugin);
request_inventory_icons(&inventory, &mut discovery, &mut client);
let shaded = cache.shaded_for_key(item.icon_catalog_key()?);
let silhouette = cache.silhouette_for_item(&item);
```
