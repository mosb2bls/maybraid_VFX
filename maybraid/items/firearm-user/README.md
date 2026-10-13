# Firearm user

Tracks every world agent that holds a firearm. [`FirearmUser`](src/lib.rs) is a
Bevy relationship onto the kit (`held`); [`HeldBy`](src/lib.rs) is the reverse
index on the gun. Hold / aim knobs live on [`FirearmUserSettings`](src/lib.rs)
(defaults match the firing-range bullpup).

Pose and arm IK key off the user's [`CharacterRoot`](../../world/layers/mobs/characters/lib/src/member.rs)
child, so an [`Npc`](../../player/src/identity.rs) holds the same way as the
player. Pad fire and the world reticle stay on the followed [`Player`](../../player/src/identity.rs).
The two-bone reach resolves humerus roll with a closed-form signed angle rather
than sampling trial poses. Hold posing runs only for actively animated,
non-suspended body rigs, so culled LODs and ragdolls pay no arm-IK cost.

Writes player handoff slots ([`PlayerUse`](../../player/src/identity.rs),
[`PlayerCameraAim`](../../player/src/identity.rs),
[`WeaponTrigger`](../firearms/src/projectiles.rs)) and poses the kit + arm IK.
`PlayerLook` follows the camera's −Z convention; the pose reverses the local X
rotation when applying that pitch to the firearm's +Z bore.

Rolled inventory kits share [`GeneratedFirearm`](src/kit.rs) (world, range,
mobs, and the character-menu inspect). That type paints catalog looks onto
[`FirearmKit`](../firearms/src/kit.rs) so sight rest-scale / `sight_socket`
stay the same as a held gun. Inspect does not rest-fit kit bones: that left
`sight_arm` at bind while shrinking the hull, so the 1 m optic cube floated
off the receiver.

[`live_weapon_from_stats`](src/weapon.rs) bakes catalog
[`FirearmStats`](../../world/layers/mobs/characters/items/src/stats.rs) into the held
[`Weapon`](../firearms/src/projectiles.rs), payload, cadence, recoil
strength, and ADS FOV. Each shot noisily kicks yaw and pitch inside a range scaled by that
strength; the direction is hashed from the weapon identity and shot index so
the same gun repeats the same pattern. Each kick lerps along that path over
80 ms instead of snapping. Followed-player kicks land on
[`CameraController`](../../player-camera/src/look.rs) (so they survive the next
look sync) as well as [`PlayerLook`](../../player/src/identity.rs). NPCs get
the look kick; [`firearm-intelligence`](../../intelligence/combat/firearm/)
observes and skill-recovers that displacement instead of overwriting it.
Lasers emit recoil `0` and do not kick. Default spawn is a
25 DPC bolt (catalog recoil 1.9). The world reticle flashes when the followed player's shot applies
damage. Connected pads rumble on that same fire and hit-confirm: faster
projectiles are shorter, higher DPC (and headshots) are heavier. Lasers stay a
low constant pulse while the beam is up. Mouse and keyboard do not rumble.

Does not own the capsule, follow camera, or projectile flight.
