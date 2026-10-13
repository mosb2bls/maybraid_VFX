# Player camera

Follow camera, POV toggle, look cone, and FOV. Knobs live on
[`FollowCamera`](src/lib.rs) (defaults match the firing-range orbit). Runtime
yaw / pitch / POV / focus live on [`CameraController`](src/look.rs).
[`spawn_follow_camera`](src/lib.rs) also installs a `SpatialListener` so firearm
shots can pan and fall off with distance.

Reads [`PlayerLook`](../player/README.md) slots on the followed body; writes look
back so item users never query `Camera3d`.

[`PlayerCameraAim`](../player/src/identity.rs) is the handoff: a firearm-user writes a
sight pose and the held optic's ADS FOV, a melee-user could write a lock-on pose. Follow
lerps default POV → aim by `focus`. First-person ADS pose uses that blend.
LT / right mouse is optic ADS: FOV uses the aim slot when present, otherwise
[`FollowCamera::sight_fov`](src/lib.rs) (iron sights). Left bumper / middle
mouse / C is the same pose with iron FOV only, so long optics stay usable in
close combat. First-person face hide uses
[`hide_socketed_parts`](../world/layers/mobs/characters/lib/src/member.rs) with
[`CharacterPartSlot::hides_in_first_person`](../world/layers/mobs/characters/lib/src/assembly.rs).
