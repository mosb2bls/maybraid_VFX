# Side Effects Analysis: Adding `player::Player` to World Player Entity

## Summary
Adding `player::Player` to the world player entity is **SAFE**. No world systems change behavior.

## Analysis

Audited all `With<Player>` queries in `maybraid/world`:

### Systems Using `world_player::Player` (No Change)
All these systems import `Player` from `world_player`, so they already matched the world player entity before the change:

- **control.rs**: `update_world_surface_ready`, `discovery_xz`, `grant_world_player_lubikan_owner`, `apply_intents_to_movement`, `sync_world_scenery`, `stamp_vegetation_motor_traction`, `stamp_world_player_motor`
  - Import: `use world_player::{...Player...}` (line 20)
  - Already query `world_player::Player`
  
- **lubikan_activate.rs**: `request_lubikan_activation`
  - Import: `use world_player::Player;` (line 6)
  - Already queries `world_player::Player`

- **pitch.rs**, **player_position.rs**: terrain pitch and position systems
  - Import `world_player::Player`
  - Already query the world player

### Systems Using `player::Player` (Checked for Safety)
Only **camera.rs** imports `player::Player` directly:

- **obstruct_world_camera** (line 169):
  ```rust
  Query<(Entity, &Transform), (With<Player>, With<CameraFollow>, Without<Camera3d>)>
  ```
  - Queries `With<Player>, With<CameraFollow>`
  - World player entity **does NOT have `CameraFollow`** (added separately by camera.rs or weapon.rs systems, not at spawn)
  - **No match**: Adding `player::Player` doesn't make the world player satisfy this query
  - **Safe**: System behavior unchanged

## Conclusion
✅ No behavioral changes to any world systems  
✅ `combat-hud` now correctly queries the world player (was broken before)  
✅ All existing tests pass
