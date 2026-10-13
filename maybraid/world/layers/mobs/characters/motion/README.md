# Character motion

Per-frame articulation for characters. Recipes in `characters`
**stamp** host identity; this crate **syncs** host motion markers from the shown
LOD band and **realizes** clips / pitch. Motion does not implement `LodScene` or
species recipes.

## Crate graph

```
ground                    # ElevationProbe, GroundHit
ground-avian              # AvianElevationProbe (SpatialQuery)

character-animations           # Animation { apply_for, effects_for, apply }
character-rigs

character-motion   # this crate
        ↑
characters         # recipes; host() / fixed spawn stamps initial markers
        ↑
playgrounds
```

## Dataflow (one frame, High band)

```
WASD / Space
  → physics capsule (Avian)
  → drive_player_locomotion writes AnimRefRoot on the body host

sync_motion_markers
  → shown LodLevelRoot (else desired / High; fixed assemblies have no host) → motion_policy
  → clamp Mid/Far thinking LOD unless the look frustum is presenting the plant
  → insert/remove AnimateBones / AnimateEffects on body host
  → insert/remove ApplyTerrainPitch on character root

tick_anim_mailbox          # every body: advance clip time, or [`AnimProgress`]
apply_anim_mailbox         # With<AnimateBones|AnimateEffects>: sample + write
apply_terrain_pitch        # With<ApplyTerrainPitch>: Avian rays along girdle axis → visual rotation + support offset
draw_terrain_pitch_probes  # PostUpdate gizmos: lime front, orange hind, yellow sample axis, cyan mesh +Z
```

UltraLow: sync strips markers → far hosts still tick time, but bone writes and
rays are archetype-filtered out.

`SuspendAnimation` leaves mailbox time intact while an external pose driver,
such as `character-ragdoll`, owns the bones.

## LOD: host markers

**Do not stamp motion markers on level-content children.** Chunk fulfill only
spawns nested rig/part hosts. Runtime truth is on the host:

| Where | What |
|---|---|
| `RigNode::host` (body) | `AnimRefRoot`; initial `AnimateBones` / `AnimateEffects` from `motion_policy` |
| Character spawn | Initial `ApplyTerrainPitch` from `motion_policy` |
| `sync_motion_markers` | Keeps those host markers aligned with the **shown** band |

| Level | bones | effects | pitch |
|---|---|---|---|
| High | yes | yes | yes |
| Medium | no | yes | yes |
| Low | no | yes | no |
| UltraLow / distance / resolution | no | no | no |

This is the default linear ramp in [`motion_policy`](src/policy.rs), not a
per-recipe regime. Sync a different map yourself to differ.

## Bevy systems

### In `characters` (`CharacterComponentsPlugin`)

Structural realize only. Sets: `Membership → InvalidateRefs → BoneMap → Fulfill → Pose`.

No clip sampling. No rays.

### In this crate (`CharacterMotionPlugin`)

| System | Set | Does |
|---|---|---|
| `sync_motion_markers` | `Anim` | Shown band → host marker insert/remove |
| `prepare_anim_mailbox` | `Anim` | Typed rig + `AnimMailbox` once the bone map is ready |
| `tick_anim_mailbox` | `Anim` | Advance time on every body mailbox |
| `apply_anim_mailbox` | `Anim` | Sample/write only `With<AnimateBones\|AnimateEffects>` |
| `apply_terrain_pitch<P>` | `Elevation` | **Not registered here** — app adds with a concrete probe; filters `With<ApplyTerrainPitch>`; capsule children ray from the parent origin |
| `draw_terrain_pitch_probes` | `PostUpdate` | Sample gizmos when [`DrawTerrainPitchProbes`](src/elevation.rs) is true (default) |

Order `CharacterMotionSystems::Anim` after `CharacterHostSystems::Pose`.
Order elevation after physics / locomotion.

### In playgrounds

| System | Does |
|---|---|
| Player physics / `ShapeCaster` | Capsule ground, jump |
| `drive_player_locomotion` | Wish → explicit `CharacterHeading` + `AnimRefRoot` clip |
| `prepare_character_terrain_pitch` | Measure girdles, insert `TerrainPitch` |
| `sync_suspend_terrain_pitch` | `Jumping` → `SuspendTerrainPitch` on the body |
| `apply_terrain_pitch::<AvianElevationProbe>` | Probe colliders, write visual rotation |

## How to implement behaviors

### Play a clip

Insert `AnimRefRoot(AnimRef::new(AnimClip::walk()))` on the **body** rig host.
The mailbox transitions on `AnimId` (variant), not knob values.

### Gate work by LOD

Host markers + `With<>` on expensive systems. [`motion_policy`](src/policy.rs) is
the shared table `sync_motion_markers` applies. To add a capability, extend
`MotionPolicy` and sync a new host marker.

### Stand on colliders (not a heightfield)

Implement `ground::ElevationProbe` (or use `AvianElevationProbe`). Wrap the
generic apply loop in a concrete system. Capsule children sample from the
parent origin so support Y cannot lift the next ray; world-placed hosts still
use the visual `GlobalTransform`. Rest wheelbase locks on the first good
girdle measure; live shoulder–hip only steers the sample axis and gizmos.
`AvianElevationProbe` masks
`PhysicsInteractionLayer::Fixed`, walks past near-start / canopy hits, and
keeps the lowest standable collider so grove Host AABBs and tree sticks do
not steal the terrain trimesh. Side rays run only when
`TerrainPitch.roll_weight > 0`. Quadruped front/hind rays follow the live
shoulder–hip axis (`TerrainPitch.sagittal`); gizmos: lime/orange ray hits, yellow sample axis, cyan
explicit heading, magenta bone dots, teal/gold girdle midpoints (magenta chord). A pink
ring on the origin means girdles were found but the XZ run was too short.
Insert `DrawTerrainPitchProbes(false)` to hide. Locomotion, look, and combat
write `CharacterHeading`; terrain pitch reads that source instead of recovering
yaw from the tilted transform it wrote last frame. Pitch, roll, and support Y
continuously follow valid probe targets with exponential smoothing and a rate
cap. If center, coarse, or final support rays miss, apply holds the last pose
instead of substituting capsule Y as terrain height.

### Jump / airborne

Insert `SuspendTerrainPitch` on the visual or any ancestor (typically the
physics capsule). Pitch apply targets zero tilt and support offset while that
marker is on the ancestor chain. Local Y `support_offset` is applied only when
the visual is a near-origin child of a body, not when local translation is
world-sized under a group root.

## What this crate is not

- Species recipes or `impl LodScene`
- Socket / skin / proportion pose (those stay in `characters`)
- “Which hosts overlap this AABB?” (`lod::LodSceneRegionIndex`)
