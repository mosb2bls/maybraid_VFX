# Roster bind (later LodScene)

High scene data must stay **Entity-free**. Membership is still a roster slot plus
a live pointer; the pointer is a **patch** after spawn, not a field on
`SemanticLodScene`.

## Why not bake `Entity` into LodScene

`scene_with_level` builds Bevy `Scene` / BSN from `LodRef` and a band. Those
recipes cannot name a host `Entity` without a context argument such as
`scene_with_level(parent_chain, level)`. That would make every grove plant
depend on ECS identity. Vegetation and buildings do not do this.

Drain already knows the host (`LodChunkFulfillment::host`) and parents each
primitive under the High root. That is a **spawn-time fact in the world**, not
something the scene trait should take as input.

Do not:

- add `parent_chain: Vec<Entity>` to `SemanticLodScene`
- use `ChildOf` as gameplay membership (LodScene already owns that tree)
- bake a fixed parent depth
- rebind on every lod refresh
- send a `WantsParentRelationship` message as the membership graph
- `impl LodScene` on a per-member [`RosterRef`](src/roster_ref.rs) (one host per
  mob; slots are plant recipes, not groves)

`ChildOf` remains scaffolding: host → level-roots bag → `LodLevelRoot(High)` →
**stubs**. HORIZON already forbids `Query<&Transform, With<Npc>>` off that tree;
cull would break Occupy. Capsules are **not** transform children of the host:
`MobTravel` lerps the host `Transform`, and a parented capsule would slide with
no `MoveWish`.

## Wish in, live `Entity` out

This is the SceneRef shape: a path is not an entity yet; fulfill writes the
handle cache. A High stub's wish is a [`RosterRef`](src/roster_ref.rs) (Arc'd
character recipe + slot + local offset). The live body is spawned unparented;
the roster's `entity: Option<Entity>` is the resolved cache. [`RosterBinding`](src/roster_ref.rs)
on the stub points at that body for cull.

```text
LodScene High BSN (pure)
  RosterRef { recipe: Arc<T>, slot, offset }
  no host Entity, no CharacterSceneRecipe, no capsule

drain_chunk_lod_fulfill
  spawn_scene → ChildOf(High root)
  host Entity is known here, but stays out of the recipe

fulfill (after drain)
  world pose = host GlobalTransform × offset
  spawn recipe at that pose (unparented)
  stamp MobSlot + MobId on the body
  RosterBinding { body, host, slot } on the stub

MobSystems::Bind  (Added / missing MemberOf)
  resolve host by MobId
  MemberOf { mob, slot }
  roster[slot].entity = Some(body)
  Personality::install(..., tether: host) if mixer missing
  copy MobAffiliations; member POI interests stay role-split (not combined
  with the host pack table). Meander scores and live `PoiGoal` points take a
  per-slot salt. A host `MobTetherLock` also gives each slot a 6 / 8 / 10 m
  standoff around the shared subject.

High cull
  drain despawns the High root (stubs)
  write pose / health onto the roster
  despawn the body only if it still has matching MemberOf
  roster[slot].entity = None
  do **not** schedule [`MobMemberNeeded`](src/roster.rs)

Death
  `Downed` → replacement clock + `DespawnAfter` (even if the roster
  pointer is stale; a missed timer would leave a 0-HP plant that
  ignores further hits)
  writeback clears the pointer
  after delay, [`MobMemberNeeded`](src/roster.rs) at the host (full HP)
  replacement bodies are unparented and have no stub; leaving High
  despawns remaining MemberOf
```

Fulfill, cull, and death are the times the live link changes. Refresh must not
rebind. Trickle spawn is the chunk budget; `LodLazyPending` is not required for
the link.

## Two resolve paths

1. **Id bind (always works).** The plant carries `MobSlot` + `MobId`. Bind looks
   up the host with that id. Use this when the body is **not** a child of the
   host (High fulfill and death replacements).
2. **Ancestor bind (LodScene drain).** The plant carries `MobSlot` only. Bind
   walks `ChildOf` to the nearest ancestor [`Mob`](src/host.rs). Walk to `Mob`,
   **not** `LodSceneHost` — a character host on the NPC would win first.

Prefer an explicit `MobId` when both are present. Production High plants always
stamp `MobId` so bind does not walk the stub tree.

Playground `Commands` that already hold `host: Entity` may stamp `MemberOf` in
the same batch. That path never needed LodScene context. The bind system exists
so serialized High content can stay as dumb as every other grove plant.

## What stays on the host while plants are gone

The mob brain runs off the roster: spec, last pose, health, cheap summaries.
Occupying, relocating the tether, and pack antagonism cannot require live NPC
queries. Death respawn is a [`MobMemberNeeded`](src/roster.rs) message after the
corpse despawns; the app spawns the body and stamps the wish again. High cull
is the other vacancy: clear the pointer and wait for fulfill.

Journeying / `MobTravel` move the **host** (the tether). Members follow through
tether intelligence, not by parenting. The host `Transform` still moves so
brains, journeying, and the local leash AABB stay on the host pose.

## Character trickle

NPCs can appear one-per-frame under the fulfill budget. Each High chunk is one
[`RosterRef`](src/roster_ref.rs) stub; the app spawns the unparented body from
that recipe. Bind is incremental: each new `MobSlot` without `MemberOf` is one
plant. No extra relationship sync and no parent-wish dirty flag.
