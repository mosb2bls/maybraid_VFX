# Maybraid Firearms

Firearm recipes assembled from [`firearms-components`](../firearms-components/).

A [`FirearmKit`](src/kit.rs) is a required [`BodyMesh`](src/parts.rs) plus optional barrel / trigger-box / grip / stock / sight. Named [`FirearmConcept`](src/concepts.rs) values are presets of that kit. Mix parts with `kit --trigger-box paddle --grip bump-handle --sight holorand`. Scale kit bones with `scale barrel --length 1.5 --thickness 0.8`.

Iron sights keep the current ADS FOV. [`SightMesh::Holorand`](src/parts.rs) rolls 1–3× of that FOV; [`SightMesh::Leskop`](src/parts.rs) rolls 3–5×. Magnification is stored as vertical FOV (`fov' = 2 atan(tan(fov/2) / zoom)`). Sights socket onto `sight_socket` at a rest scale of the 1 m authored cube. First-person focus stays on `sight_camera_socket`.

[`FirearmWeaponsPlugin`](src/projectiles.rs) registers
[`register_firearm_runtime`](src/runtime.rs) (armed flag + [`WeaponFired`](src/cadence.rs))
and [`register_firearm_presentation`](src/presentation.rs) (muzzle / laser / impact / audio).
`fire_weapons` still advances its own clock while spawning; a follow-up will
make one authoritative fire clock and have presentation consume `WeaponFired`.
Put a [`Weapon`](src/projectiles.rs) on the [`FirearmRoot`](src/lib.rs). Auto-fire is the default; add [`FireOnTrigger`](src/projectiles.rs) to require [`WeaponTrigger`](src/projectiles.rs) (written per gun by [`firearm-user`](../firearm-user/)):

| Load | Shape | Motion |
|------|--------|--------|
| Bolt | Capsule (length, radius, speed) | No gravity; despawn when path, through-solid, or age is exhausted |
| Bullet | Same capsule | Gravity on; same budgets |
| Laser | Beam along bone +Y | Grows from the muzzle to the first bore hit (or max length). Despawned when the trigger is released. A ray along the bore writes [`Hit`](../../damage/src/lib.rs) about every 0.15 s |

Muzzle is the barrel tail (`bone-local +Y` of rest length 1). Runtime rest (after the armature’s glTF +90° X) has bore along +Z and grip down; [`aim_plus_x`](src/pose.rs) yaws that onto world +X. Bolts and bullets live in [`projectiles`](../../projectiles/); they sweep [`Fixed`](../../lod/avian/src/layers.rs) and charge [`Flight::through`](../../projectiles/src/lib.rs) with optional [`PenetrationCost`](../../projectiles/src/lib.rs). Each distinct collider crossed emits one [`ProjectileContact`](../../projectiles/src/lib.rs), including multiple contacts in one frame; this crate stamps [`HitPayload`](../../damage/src/lib.rs) onto ballistic flights at spawn and spawns a short Hanabi spark + smoke burst. Bolts without a payload deal no damage.

Every shot plays world-fixed [`WEAPON_FIRE`](src/sound.rs) and [`WEAPON_HAMMER`](src/sound.rs) at the muzzle (they do not follow the barrel after the bang). Bolts and bullets carry a looping [`WEAPON_FIZZ`](src/sound.rs) that follows the projectile with its simulation velocity. Each [`ProjectileContact`](../../projectiles/src/lib.rs) plays a world-fixed [`WEAPON_IMPACT`](src/sound.rs). A laser loops fire on the beam and plays hammer once when the beam starts. Distance is oddio's `radius / max(distance, radius)` in [`maybraid-audio`](../../audio/). [`FireOnTrigger`](src/projectiles.rs) shots use the `PlayerWeapon` bus and briefly duck ambience / world weapons. Spatial clips are authored **mono** WAV. [`spawn_follow_camera`](../../player/camera/src/lib.rs) carries the listener pose; playgrounds that fire without that helper get one on the lone `Camera3d`.

Authoring (bone-space meshes, slots, armature tree): [`maybraid/art/items/guns/README.md`](../../art/items/guns/README.md).

```bash
cargo run -p items-playground
```

Blender sources: [`maybraid/art/items/guns/`](../../art/items/guns/). Runtime GLBs: `maybraid/assets/items/guns/`. Firearm SFX sources: [`maybraid/art/sound-effects/weapons/firearms/`](../../art/sound-effects/weapons/firearms/). Runtime WAVs: `maybraid/assets/sound-effects/weapons/firearms/` via [`scripts/sound-effects/sync.sh`](../../../scripts/sound-effects/sync.sh).
