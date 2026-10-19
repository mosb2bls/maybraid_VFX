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

## Stable attack / event contract

Downstream owners should code against this section:

- Bikanwarda authored-attack adapter ([#1230](https://github.com/ramate-io/maybraid/pull/1230), recipes on [#1119](https://github.com/ramate-io/maybraid/issues/1119))
- God Beam / sonic VFX ([#1150](https://github.com/ramate-io/maybraid/pull/1150) / [#1122](https://github.com/ramate-io/maybraid/issues/1122) / [#1123](https://github.com/ramate-io/maybraid/issues/1123))
- Spatial rumble ([#1146](https://github.com/ramate-io/maybraid/pull/1146))
- Epiphant Bray / sleep animation cues ([#1141](https://github.com/ramate-io/maybraid/pull/1141))

The crate rustdoc (`maybraid/attacks/src/lib.rs`) is the same contract.

### `BeginAttack`

Write one message per requested instance. The owner must already have
[`AttackUser`](src/instance.rs).

| Field | Role |
|---|---|
| `owner` | Caster. Must exist and carry `AttackUser`. |
| `definition` | Catalog [`AttackId`](src/definition.rs). Encounter and summoned owners share ids. |
| `target` | Optional explicit victim. Aim uses `CombatContact` memory after `CombatTargetingSystems::Rank`. Membership is not aimable. |
| `allies` | Extra friendly-fire exclusions, merged with `TargetSource::ALLY` members. |
| `emission` | World-space mouth / socket. Missing uses `emission_socket`, then owner origin. |
| `emission_socket` | Live socket sampled while the instance still tracks aim. |

Missing contact memory falls back to owner facing. The runtime never reads a
live target transform and never fabricates contacts.

### `AttackInstance`

One in-flight attack is its own entity. Presenters key effects by
[`AttackInstanceId`](src/instance.rs), not by owner.

Carries `id`, `definition`, `owner`, `stage`, `age` / `stage_age` /
`execute_age`, [`AimSnapshot`](src/aim.rs), `live_target`, `allies`, and hit
clocks. `AimSnapshot` stores three distinct world points: `origin` (caster),
`aim_point` (observed contact), `emission` (mouth / socket).
`HazardOrigin::Owner` and `HazardOrigin::AimPoint` are not the same.

### Stages

`Windup` → `Execute` → `Recover`, then the instance despawns.
[`AttackUser`](src/instance.rs) starts the per-`AttackId` cooldown **only on
`AttackEndReason::Completed`**. Cancel / death / sleep / retirement do not
start cooldown.

| Stage | Gameplay | Presentation |
|---|---|---|
| Windup | No Hits. Telegraph pose only. | `Stage { from: Windup, to: Windup }` on admit, then `Telegraph { Spawned, Updated }` |
| Execute | Pulses write `Hit`. | `Stage { Windup, Execute }`, `Hazard { Spawned, Updated }`, optional persisted telegraph |
| Recover | No Hits. Volumes already cleared. | `Stage { Execute, Recover }`, `Hazard` / `Telegraph` `Despawned` |

A hitch that spans several stages still visits Execute.

### `AttackPresentationEvent` kinds

Every event carries `instance`, `owner`, `definition`, and one
[`AttackPresentationKind`](src/presentation.rs):

| Kind | When |
|---|---|
| `Stage { from, to, snapshot }` | Lifecycle, including the admit echo `Windup → Windup` |
| `Telegraph { phase, pose, shape }` | Warning volume. Same shapes as hazards; no damage |
| `Hazard { index, phase, pose, shape, wave }` | Gameplay volume. `wave` is `Some` only for `HazardShape::Wavefront` |
| `Ended { reason }` | Always paired with gameplay [`AttackEnded`](src/command.rs) |

`PresentationPhase` is `Spawned` / `Updated` / `Despawned`. Spawn a presenter
on `Spawned`, follow `Updated`, despawn on `Despawned` or `Ended`. Tag
presenter entities with [`AttackPresentationBinding`](src/presentation.rs)
`{ instance, kind: Telegraph | Hazard { index, wave } }` so gameplay cleanup
can despawn them without knowing the VFX type.

#### Wave / hazard presentation

[`WavefrontPresentation`](src/presentation.rs) is authoritative. VFX follows
it and does **not** simulate motion:

| Field | Meaning |
|---|---|
| `origin` | Mouth / socket at emit time |
| `direction` | World-space travel heading (already flattened; optional pitch already applied) |
| `position` | Current ring center |
| `speed` / `range` | Authored travel |
| `radius` | Constant ring radius |
| `thickness` | Full thickness along travel (`2 * half_depth`) |
| `wave_index` | Which sequential ring |

`HazardShape::Wavefront` is a discrete ring / short disc, **not** a radially
expanding ground ring. Rings spawn at emission, travel a flattened heading,
keep constant size, emit `waves` at `interval`, and hit each target at most
once. Collision is swept Avian (`shape_hits_callback`) so a hitch still hits.

Stationary hazards (`Sphere`, `Cylinder`, `Box`, `Line`) pulse on Execute with
per-target `DamageSpec.cadence` (`<= 0` means once). Child colliders remount
onto parent `Health`.

Rumble ([#1146](https://github.com/ramate-io/maybraid/pull/1146)) and Bray
mouth-emit ([#1141](https://github.com/ramate-io/maybraid/pull/1141)) should
subscribe to these events rather than inventing a second motion clock.
`BeginAttack.emission` / `emission_socket` is the mouth pose `#1141` already
samples.

### Interruption events

| Trigger | `AttackEndReason` |
|---|---|
| [`CancelAttacks`](src/command.rs) | `Cancelled` (or the reason stamped on the message) |
| Owner `Downed` | `OwnerDowned` |
| [`AttackSuspended`](src/cleanup.rs) | `OwnerSuspended` |
| [`AttackRetired`](src/cleanup.rs) | `OwnerRetired` |
| Owner despawn | `OwnerDespawned` |
| Recover finished | `Completed` |

[`InterruptionPolicy`](src/definition.rs) gates cancel / downed / suspended /
retired. Owner despawn and completed always apply. Each teardown emits volume
`Despawned` events, `AttackPresentationKind::Ended`, and `AttackEnded`, then
despawns the instance entity and its bindings.

### Concurrency

`AttackDefinition.concurrent` defaults to `false`.

- **Exclusive** (`concurrent: false`, God Beam / Bray / Parade parent): at
  most one live instance per owner. Admission is authoritative for the
  current `begin_attacks` batch — deferred `commands.spawn` values count as
  in-flight. Two same-owner exclusive `BeginAttack`s in one system invocation
  spawn only the first that passes catalog / `AttackUser` / cooldown gates.
- **Concurrent** (`concurrent: true`, `bikanwarda.parade_blast`): always
  admit. Child explosions may overlap a live exclusive parent and may stack
  with each other in the same batch.
- The exclusive lock is per owner, not global. Cooldown is per `AttackId` on
  `AttackUser`, checked at admit, started only on `Completed`.

### Schedule

```text
CombatTargetingSystems::Rank
  → AttackSystems::Command   (cooldowns, BeginAttack admission)
  → ApplyDeferred
  → AttackSystems::Interrupt (cancel, Downed, Suspended, Retired, despawn)
  → ApplyDeferred
  → AttackSystems::Advance   (stages, Hits, presentation updates)
```

### Contract changes vs prior `#1147` head (`01fd4b4`)

No public type, field, or event-kind changes. Behavior only:

- Exclusive same-owner `BeginAttack`s in one `begin_attacks` invocation used
  to both pass because newly spawned instances are deferred. They now admit
  only the first success. Concurrent recipes, including Parade blasts, are
  unchanged.

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
