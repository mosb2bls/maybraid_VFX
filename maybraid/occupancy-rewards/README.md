# Occupancy rewards

Semantic cell occupancy, periodic timers, and a generic reward-rule framework
([#1111](https://github.com/ramate-io/maybraid/issues/1111)).

Resolve an actor from world position onto the forest / grove / planting /
urban keys used by HCSG generation. Cached metadata is published into a
Gimme-backed store; rules never scan vegetation and never read rendered
meshes. Periodic evaluation emits [`OccupancyAward`](src/occupancy.rs) values
that a thin adapter turns into [`OccupancyChargeRequest`](src/charge.rs)
using the shared [`lubikans`](../lubikans) types from
[#1107](https://github.com/ramate-io/maybraid/issues/1107) /
[PR #1142](https://github.com/ramate-io/maybraid/pull/1142).

```text
GlobalTransform
    → OccupancyIdentity (1600 m forest, 100 m grove, optional planting / urban)
    → enter / exit + timers
    → RewardRule match against CellMetadataStore
    → OccupancyAward
    → OccupancyChargeRequest   (LubikanAccountId, OCCUPANCY, signed f32, origin)
    → LubikanLedger::apply_to  → LubikanChargeMutation   (#1108 will own this)
    → LubikanChargeChanged     (#1108; not emitted here)
```

## Matching granularity

`ConstructionScope` is explicit on each rule:

| Scope | Meaning |
|-------|---------|
| `ForestCell` | Any published construction in the actor's 1600 m forest cell |
| `GroveTile` | Construction in the actor's 100 m grove tile (default) |
| `PlantingCell` | Construction in the planting sub-cell that owns the actor |

"Forest cell with a construction" is **not** implied. A Self demonstration
rule that should require a construction in the occupied grove tile must set
`ConstructionScope::GroveTile`.

## Charge attachment

This crate depends on `lubikans` and does not invent a parallel account
model. Awards snapshot [`LubikanSelection::resolved`](../lubikans/src/selection.rs)
(Self when nothing external is equipped). The adapter stops at
`OccupancyChargeRequest`. Applying that request produces a
`LubikanChargeMutation`. Authoritative `LubikanChargeChanged` routing is
[#1108](https://github.com/ramate-io/maybraid/issues/1108) and is not
emitted here.

## Pause, re-entry, streaming

- [`OccupancyRewardsPaused`](src/plugin.rs) and `OccupancyRewardsUser.enabled`
  freeze timers; identity still updates so leaving while paused is recorded.
- Leaving a geometric cell drops Reset timers. Persist keeps leftover
  progress for the same `(rule, cell)` key.
- Metadata eviction does **not** drop timers. Republishing the same cell
  after streaming cannot restart an interval or duplicate an award.

Self is the production default Lubikan. This crate has no Bikanwarda
dependency.

## World registration

The ordinary world (`maybraid-world`) registers this plugin through
`WorldOccupancyPlugin`. That publisher fills `CellMetadataStore` from HCSG
`ChicoForest` / `ChicoGrove` cell facts — not vegetation meshes — and attaches
Self plus `OccupancyRewardsUser` to the player.
