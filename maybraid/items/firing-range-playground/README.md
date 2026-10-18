# Firing range

Braidman on a flat range, plus a standing NPC braidman down-left of the pad
body. Both hold a bullpup through [`firearm-user`](../firearm-user/). The
followed player fires from the pad. The NPC installs
[`spotting-intelligence`](../../intelligence/spotting/lib),
[`combat-targeting`](../../intelligence/combat/targeting), and
[`firearm-intelligence`](../../intelligence/combat/firearm). Live combatants
register one semantic character proxy. Avian broadphase discovers nearby
proxies, bounded eye probes remember visible contacts, and the combat algebra
ranks the active set by hostility, distance, firearm opportunity, and
engagement continuity. Fresh contacts can satisfy the spotting directive and
skip further discovery until they need refreshing.
Look tracks the inferred live capsule; fire needs both a fresh sighting and a
posed-muzzle trajectory accepted by the obstruction policy. Firearm movement
reads that same weighted target list, then
writes [`VantageOn`](../../intelligence/movement/lib/src/objective.rs)
into [`movement-intelligence`](../../intelligence/movement/lib) at an ~8 m
standoff (no close-range flee). Lost sightlines raise sightline weight so the
NPC does not glue to a cover crack. Firearm combat aims
[`PlayerLook`](../../player/src/identity.rs) from the stock (shoulder pivot), not
the muzzle, and holds the trigger after the first on-target acquire;
`trigger_happiness` is only the delay before that first pull.
Projectile sweeps include both fixed geometry and animated character capsules.
Collider size comes from the character recipe
[`LocomotionCapsule`](../../world/layers/mobs/characters/lib/src/components.rs) (Braidman is
the 0.4 / 1.0 humanoid hull). Walk uses the shared player
[`MotorTraction`](../../player/src/contact.rs) contact hooks so Les Halles
floors keep grip for props without dragging the capsule. Each character starts with 100 health and takes 25 damage per bolt contact
in `duel`. `free-for-all` bakes the rolled firearm's DPC, speed, range,
penetration, cadence, and recoil (plus clothing HP / outgoing damage) into the
live weapon. Health is shown on a persistent top HUD and as a bar above each
capsule; the followed player's gun stats sit in the bottom-right card. The
world reticle flashes when the followed player lands a hit, and `+1` / `+2` /
`+5` float at the impact point for a body hit, a headshot (upper half of the
top capsule hemisphere, 1.25× HP, light blue), or a down. Catalog recoil noisily
kicks the follow camera (and NPC look) along a short lerp to the hashed offset
(scaled by the rolled recoil value); lasers do not kick. NPC aim tracks at a
finite angular rate, uses 0.45 counter-recoil skill, and must recover its actual
bore inside the short alignment grace to keep firing. Connected pads rumble
on the followed player's fire and hit-confirm (faster / harder shots scale the
pulse; lasers stay a low constant tick). The
player also gets directional hit ticks around screen center. At 0 health the
gameplay capsule and held firearm retire after the character visual becomes a
persistent procedural ragdoll; player and NPC both return after two seconds.
Smoothed FPS and frame time are logged to the terminal every two seconds.
The NPC spots and aims during a ceasefire, but does not fire until the player
takes a shot. Player death (or switching mode) resets that ceasefire.

`free-for-all` is a generated-loadout benchmark: one rolled player (starter
clothing plus one gallery-style firearm from
[`character-items`](../../world/layers/mobs/characters/items)) and `--npcs` rolled
NPCs spread around the pad and on the upper storey. Each NPC discovers a
bounded set of character subjects inside an 80 m perception envelope. The live
enemyship roster supplies explicit spotting hints, while Avian broadphase can
still discover other matching subjects; both paths share the same visibility
executor and eight-subject candidate budget. The perception envelope is
independent of the shorter movement-planning horizon. Combat
still waits for the player's first shot.

`assault-free-for-all` (`affa`) keeps that armed fight and adds unarmed
civilians. Civilians install [`evasion-intelligence`](../../intelligence/evasion)
for assailant memory, then [`fleeing-intelligence`](../../intelligence/fleeing)
or [`hiding-intelligence`](../../intelligence/hiding) write movement from the
exclusive hide | flee signal. They are not combat targets: civilian [`SpotSubject`](../../intelligence/spotting/lib/src/subject.rs)
proxies use the `CIVILIAN` interest layer, so combat `CHARACTER` directives do
not discover them. A shot is a
`RECEIVED_FIRE` stimulus (last-known position plus decaying threat), not a
fabricated sighting. Occupancy counts live character subjects and hide claims
so civilians do not pile into the same pocket. In this mode, civilians who
flee more than 48 m xz from the pad origin despawn and return on the spawn
ring. Downed NPCs (armed or civilian) respawn after two seconds even while
the corpse is still on the field. A pink HUD dot marks a fleeing civilian; a
blue dot marks a hiding one.

Rolled guns keep gallery looks
(material and palette per slot) and sample projectile / cadence from the
session RNG rather than reseeding from spec identity, so a run is not locked
to one color or to lasers. As an application-level performance policy, the
playground samples discovery, respotting, and movement decisions at 8 Hz,
with up to eight initial sight probes per pass,
validates and checks fire control at roughly 30 Hz, uses bounded movement-search
budgets, caches unchanged aim trajectories briefly, and immediately retires
downed spotting/targeting users and subjects. It keeps the reusable intelligence
plugins cadence-neutral. `duel`
restores the 1v1 bullpup pad fight
(100 HP / 25 DPC). `test-dummy` (`dummy`) spawns the player with that same
bullpup and a stationary unarmed dummy at the NPC pad pose — no combat AI,
no return fire — so projectile contacts can be checked in isolation.
`--species spibmom` plants the 2× meerkat head. Dummy mode draws the motor
capsule (amber), the query-only head capsule (cyan, Y-stretched), and the
headshot plane (blue) so a miss on the crown is visible.

The standalone `firing-range` executable is retired. This crate is a library
([`FiringRangePlugin`](src/lib.rs), [`FreeForAllPlugin`](src/lib.rs)) with no
workspace `[[bin]]` and no current host in `maybraid` or Training Ground.

WASD / left stick move, mouse / right stick look, Space / A jump, click / RT fire. R3 toggles first person; right mouse / LT focuses through the optic FOV; middle mouse / left bumper / C ADS with iron FOV.
