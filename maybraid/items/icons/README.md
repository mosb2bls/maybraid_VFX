# Item Icons

Maybraid first-encounter wiring on top of [`screen-capture`](../../screen-capture).
Hosts request both [`CaptureMode::Shaded`] and [`CaptureMode::Silhouette`] the
first time an inventory item is seen. The crate never mutates the live scene. Isolated captures spawn the same
posed hosts the inspect preview uses: [`firearm_user::GeneratedFirearm`] for
kits and a bind-pose clothing preview with catalog materials. Silhouette
mode strips clothing/face shaders every frame so shader restamp cannot leave
meshes bare, and capture restamps unlit gray through settle/readback.

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
