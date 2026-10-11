# Attacks

Reusable attack timing, hazards, and telegraphs. Encounter scripts and summoned
owners invoke the same [`AttackDefinition`](src/definition.rs) values; this crate
does not know about Bikanwarda, Self, or any other content cast.

Gameplay writes [`Hit`](../damage/src/lib.rs). Presentation is a subscriber:

```text
BeginAttack → Windup (telegraph) → Execute (hazard Hits) → Recover → cooldown
                 │                      │
                 └─ AttackPresentationEvent ─┘
```

Default damage, cooldown, interruption, and friendly-fire are authored in
[`AttackDefaults::AUTHORED`](src/defaults.rs). Callers copy or override those
fields; they are never implied at runtime.

```shell
cargo run -p attacks-playground
```

## Architecture

### Aim and origin

[`AimSnapshot`](src/aim.rs) stores three distinct world points:

| Field | Meaning |
|---|---|
| `origin` | Owner / caster pose |
| `aim_point` | Observed world aim from [`CombatContact::aim_point`](../intelligence/combat/targeting/src/contact.rs) |
| `emission` | Mouth / socket for traveling waves |

[`HazardOrigin`](src/hazard.rs) resolves against those fields. `Owner` is never
`AimPoint`. A target-position God Beam uses `HazardOrigin::AimPoint` so the
column sits on the observed point, not on the boss.

### Targeting knowledge

Membership (`CombatTargeting.active`) is not an aimable observation. Attacks
read [`CombatTargeting::contact`](../intelligence/combat/targeting/src/targeting.rs)
/ `best_contact` after `CombatTargetingSystems::Rank`, the same split firearm
combat uses. Missing memory falls back to owner facing. The runtime does not
read live target transforms and does not fabricate contacts.

### Collision

Execute pulses query Avian `SpatialQuery` (`shape_intersections`, and
`shape_hits_callback` for wavefront sweeps) against `Animated` colliders, then
remount child hulls onto parent [`Health`](../damage/src/lib.rs) with the existing
per-target cadence / once-per-wave clocks. Point-in-volume stays for telegraphs
and unit tests only. The host app provides `PhysicsPlugins`; this crate does not
register world physics.

### Wave contract

[`HazardShape::Wavefront`](src/hazard.rs) is a discrete ring / short disc,
**not** a radially expanding ground ring.

1. Spawn at the caller-supplied emission transform (`HazardOrigin::Emission`,
   `BeginAttack::emission` / `emission_socket`).
2. Travel in world space along a flattened horizontal heading. Optional `pitch`
   tilts that heading; default pitch is `0`.
3. Keep constant `radius` and `half_depth` (thickness = `2 * half_depth`) up to
   `range` at `speed`.
4. Emit `waves` sequential rings at `interval`. Each wave freezes its emission
   pose when it leaves the mouth.
5. Each wave hits a target at most once.

[`WavefrontPresentation`](src/presentation.rs) is authoritative. VFX follows it
and does not simulate motion:

| Field | Meaning |
|---|---|
| `origin` | Mouth / socket at emit time |
| `direction` | World-space travel heading |
| `position` | Current ring center |
| `speed` / `range` | Authored travel |
| `radius` | Constant ring radius |
| `thickness` | Full thickness along travel |
| `wave_index` | Which sequential ring |

VFX lanes ([#1122](https://github.com/ramate-io/maybraid/issues/1122),
[#1123](https://github.com/ramate-io/maybraid/issues/1123)) subscribe to
[`AttackPresentationEvent`](src/presentation.rs) and tag effects with
[`AttackPresentationBinding`](src/presentation.rs).

Software-Vulkan captures (side view of rings leaving a mouth-height emitter):

```shell
cargo build -p attacks-playground
xvfb-run -a -s "-screen 0 1280x720x24" maybraid/attacks/preview/capture.sh
```

Images land in [`preview/`](preview/).
