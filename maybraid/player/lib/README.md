# Player

Capsule body, character visual, and **handoff slots** for camera and pose.

The collider size comes from the character recipe's
[`LocomotionCapsule`](../../world/layers/mobs/characters/lib/src/components.rs), not from this
crate. Spawn uses the humanoid default until a visual is attached; attaching a
recipe reapplies that hull (`Collider`, `ShapeCaster`, component). This crate
does not know about firearms or melee. Item-user crates write the
slots; [`player/camera`](../camera/README.md) and overlay systems read them. [`Npc`](src/identity.rs)
reuses the capsule, [`PlayerLook`](src/identity.rs), and locomotion clips
without pad input, `CameraFollow`, or `PlayerVisual` (so first-person face hide
stays on the followed body). Insert [`CharacterLocomotion`](src/body.rs) before
[`PlayerPlugin`] to cap the walkable slope (default ~81°; Durham uses ~70°).
Grounded wish follows this frame's walkable contact plane so hillside
heading is along the slope, not world XZ into the mesh. Walk is the same
target-speed motor as the vegetation capsule: 6 m/s jog / 10.5 m/s L3 sprint
along the plane at 40 m/s² accel (50 m/s² idle brake), with 0.25× air control.
Wish length is the stick throttle (capped at 1), so a half-deflected stick jogs
at half the cap. A player [`Sprinting`](src/body.rs) marker (from `StartSprint`
/ `StopSprint`) selects the cap; left-trigger / RMB focus clears it. Iron ADS
may stay on. [`CharacterStance`](src/stance.rs) squat / prone shrinks the motor
hull through [`apply_locomotion_capsule`](src/body.rs), scales wish speed, and
prefers held clips. Sprint stands first. **B** tap / hold writes the stance.
Speed is framed in `dt`, not a per-frame multiply. Idle grounded motion brakes to rest along that
plane so walkable grades do not slide when solver friction is zero.
[`MotorTraction`](src/contact.rs) plus Avian
[`MotorTractionHooks`](src/contact.rs) own that policy: floor materials keep
high grip for props and ragdolls; motor contacts only block penetration.
Apps must register physics through
[`register_motor_traction_physics`](src/contact.rs) before any other Avian
plugin. Last plane is only a
[`Grounded`](src/body.rs) snap when the caster missed. Off the ground, gravity
owns Y (XZ heading only). A jump is takeoff (impulse delayed) → air → land
recovery; only air is XZ-only. Ordinary locomotion, Tanz, and NPC drive declare
[`MotorIntent`](src/motor.rs) contributions. [`compose_motor`](src/motor.rs)
selects them into [`ResolvedMotorRequest`](src/motor.rs);
[`apply_motor`](src/motor.rs) is the only path that writes
[`MoveWish`](src/body.rs) / [`JumpWish`](src/body.rs) / sprint / speed cap
and does not inspect whether the producer was Tanz. Overlapping [`Npc`](src/identity.rs) capsules declare
a kinematic XZ [`SoftBump`](src/separation.rs) modifier so pack-mates start
steering apart before capsule contacts shove them. Animated movers contact Fixed geometry and each other; restitution
on the capsule stays zero. Pronograde recipes keep that vertical motor
hull and add a query-only horizontal [`HitCapsule`](../../world/layers/mobs/characters/lib/src/components.rs)
child (`Sensor`, Animated layer) so projectiles can hit the body and tail.
Hit radius follows rest-pose shoulder / hip / torso bone scales, not the motor
radius. Bipeds keep a single vertical motor capsule sized from rest-pose legs,
spine, neck, and shoulders. Oversized heads (Spibmom, cartoon whelps) add a
query-only [`HeadCapsule`](../../world/layers/mobs/characters/lib/src/components.rs) that
stretches on Y (ears / crown) without fattening XZ. The child follows visual
yaw; `Health`
stays on the body.

```text
CharacterIntent ─► MotorIntent (locomotion) ─► compose ─► apply_motor ─► wish / jump
               ─► look / POV            (player/camera)
               ─► UseItem               (firearm-user, melee-user)

PlayerLook          camera → item users read this, not Camera3d
PlayerCameraAim     item users → camera follow blends toward pose
PlayerYawOwner      Wish (locomotion face) vs Look (first-person cone)
PlayerUse           which driver currently claims extra pose/camera
```

System sets other crates add to:

| Set | When | Writers |
|---|---|---|
| `PlayerSystems::Intent` | after Bind, before Motor | item users (fire / throw) |
| `PlayerSystems::Body` | `ControllerPipeline::Motor` | `compose_motor` + `apply_motor` then capsule physics |
| `PlayerPoseSystems::Item` | after body, before locomotion | held-mesh pose |
| `PlayerSystems::Locomotion` | before `CharacterMotionSystems::Anim` | walk/run clips |
| `PlayerPoseSystems::Overlay` | after Anim | IK / melee overlay |
