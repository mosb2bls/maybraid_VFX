# Intelligence

Higher-order brains write objectives; lower-order crates field them.

## Movement

- [`movement-intelligence`](movement/lib) — [`MovementIntelligence`](movement/lib/src/user.rs) on a capsule. It fields [`MovementObjective`](movement/lib/src/objective.rs) and writes [`MoveWish`](../player/src/body.rs). It does not lock onto other entities.
- [`movement-intelligence-avian`](movement/avian) — collider-backed surface over Avian `Fixed` geometry.
- [`movement-intelligence-richmond`](movement/richmond) — composes the Avian surface with Les Halles storey / stairwell IR.

A higher-order system writes the objective and inserts [`ReplanMovement`](movement/lib/src/user.rs) when it wants a new plan. [`MovementIntelligenceLimits`](movement/lib/src/surface.rs) caps per-query sampling, how many markers start, and how many walk probes run in one frame. `Reach` / `EdgeOf` keep one candidate and may detour unless already on the disk; covering queries LOD by distance. Walk via length is leftover work capped by [`MovementAbility`](movement/lib/src/ability.rs) `path_segment`, not `max_step`. Budget and vantage *sampling* live on that sheet. Hide / sightline *policy* belongs on the writer (firearm movement, etc.).

Walk colliders for Richmond IR live in [`building-physics`](../world/layers/urbanization/buildings/physics).

## Spotting

- [`spotting-intelligence`](spotting/lib) — semantic subjects, persistent interests and
  explicit subject hints, bounded discovery / respotting policy, and per-user visibility memory.
- [`spotting-intelligence-avian`](spotting/avian) — `Animated` broadphase discovery and Fixed-only sightline probes.

Spotting deliberately resolves a known subject's exact live location at probe time. Its memory records when visibility last succeeded and when another attempt is due; fresh contacts can satisfy a directive and skip discovery work. Position uncertainty is deferred to a higher-fidelity model.

## Threats

- [`threat-intelligence`](threat) — weighted group and individual affiliations
  (antagonism and mitigation), Gimme-backed local discovery, directed findings
  inboxes, retained threat memory, and source-owned spotting-hint export.
- [`threat-intelligence-damage`](threat/damage) — maps applied injury onto
  decaying individual antagonism and a directed `RECEIVED_DAMAGE` observation.
- [`threat-management-intelligence`](threat-management) — exclusive Ignore |
  Evade | Combat grant over retained knowledge. Combat and evade populate
  `ENEMYSHIP` membership; ignore retracts both so spotting, POI, and meander
  can still drive.

Threat classification is directional: a recipient's antagonist memory is matched
against a subject's memberships, then mitigating beliefs are subtracted. Threat
knowledge proposes spotting candidates; only spotting can establish visual contact.
Management then grants an exclusive Ignore | Evade | Combat tactic over that set.

## NPC

- [`npc-intelligence`](npc) — one mixer (threat → tether → meander) and named
  personality constructors. Personalities stamp coefficients and which actuators
  exist; they do not re-score tactics. [Horizon](npc/HORIZON.md): groups over
  mobs like forests over groves; High-band NPCs, mob brain always on the host.
- [`personalities-playground`](personalities) — 400 m square pad of proto-mobs.
  Fly the survey camera; public follows look-at so distance drives tactics.
  `cargo run -p personalities-playground --release`.

## Combat

- [`combat-targeting`](combat/targeting) — combat contact memory, source-owned active-set membership, factor algebra, decaying influences, continuity, and cached weight ranking.
- [`firearm-intelligence`](combat/firearm) — adapts spotted character contacts into combat targets, contributes firearm opportunity, writes movement / look, validates posed-muzzle aim trajectories, and gates the actual trigger through per-combatant [`FirearmEngagement`](combat/firearm/src/engagement.rs) (`Hold` | `ReturnFire` | `WeaponsFree`).

The layers form `(semantic broadphase + explicit hints) → visual contact memory → combat contact and weighted target set → firearm trajectory choice`. Applications own cadence; the reusable plugins remain cadence-neutral.

## Evasion

- [`evasion-intelligence`](evasion) — assailant memory, source-owned membership, and an exclusive hide | flee signal. This is the civilian analogue of combat targeting, not a movement mixer.
- [`fleeing-intelligence`](fleeing) — writes `FleeFrom` while the signal is flee.
- [`hiding-intelligence`](hiding) — writes `Reach` to a nearby low-vantage, low-occupancy pocket while the signal is hide.

The layers form `(semantic broadphase + explicit hints) → visual contact memory → assailant rank + signal → hide | flee → movement objective`. Combat `CHARACTER` subjects and civilian subjects stay on distinct interest layers so firearm targeting does not discover bystanders.

## Points of interest

- [`poi-intelligence`](poi) — stable POI identity, Gimme-backed local scans, sparse
  whole-map scans, source-owned retained knowledge, an external findings inbox, and
  entity-bound goal completion messages.
- [`meandering-intelligence`](meandering) — selects a learned POI in the immediate
  radius and hands it to POI goal routing.
- [`journeying-intelligence`](journeying) — probes distant tiles for learned POIs,
  then routes to one when available.
- [`poi-playground`](poi-playground) — flat visual comparison of weighted and
  fixed-cycle meandering and journeying. Run with `cargo run -p poi-playground --release`.

POI visit policy is explicit: exploration delays revisits, while cycling fills a
fixed-size roster and advances a cursor through it. Discovery cadence, acquisition
rate, retention, candidate budget, and memory capacity remain independent controls.

## Routing

- [`routing-intelligence`](routing) — hierarchical long-range corridors. Band segment lengths are per-user policy. Coarse chords are probed for buildings and cliffs; finer bands search along the committed corridor. The current fine hop is written as `Reach` for movement intelligence.
- [`tether-intelligence`](tether) — stay inside a leash or within a two-radius stalking annulus around a live entity. The user is the installed brain (`enabled` is the higher-order grant); [`TetherMemory`](tether/src/memory.rs) survives uninstall. Close remaining work writes `Reach` / `EdgeOf`; far remaining work routes to the nearest allowed boundary.