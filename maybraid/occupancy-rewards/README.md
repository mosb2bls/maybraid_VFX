# Occupancy rewards

Semantic cell occupancy, periodic timers, and a generic reward-rule framework
([#1111](https://github.com/ramate-io/maybraid/issues/1111)).

Resolve an actor from world position onto the forest / grove / planting /
urban keys used by HCSG generation. Cached metadata is published into a
Gimme-backed store; rules never scan vegetation and never read rendered
meshes. Periodic evaluation emits [`OccupancyAward`](src/occupancy.rs) values
that a thin adapter turns into [`LubikanChargeRequest`](../lubikans/src/change.rs)
(`LubikanChargeReason::OCCUPANCY`) using the shared
[`lubikans`](../lubikans) types from
[#1107](https://github.com/ramate-io/maybraid/issues/1107) /
[PR #1142](https://github.com/ramate-io/maybraid/pull/1142).
[`LubikanSystems::Award`](../lubikans/src/award.rs)
([#1108](https://github.com/ramate-io/maybraid/issues/1108) /
[PR #1154](https://github.com/ramate-io/maybraid/pull/1154)) applies each
request once and emits one `LubikanChargeChanged`.

```text
GlobalTransform
    → OccupancyIdentity (1600 m forest, 100 m grove, optional planting / urban)
    → enter / exit + timers
    → RewardRule match against CellMetadataStore
    → OccupancyAward
    → LubikanChargeRequest   (account, OCCUPANCY, signed f32, origin)
    → LubikanSystems::Award  → LubikanChargeChanged
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
(Self when nothing external is equipped). Occupancy writes
`LubikanChargeRequest` and does not mutate ledgers or emit
`LubikanChargeChanged`.

Evaluation runs in PostUpdate after [`LubikanSystems::Owner`](../lubikans/src/plugin.rs)
(PreUpdate attach/restore) and before [`LubikanSystems::Award`](../lubikans/src/award.rs).
Actors without a ledger that frame are skipped.

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
`ChicoForest` / `ChicoGrove` cell facts — not vegetation meshes — and inserts
only `OccupancyRewardsUser`. Ledgers come from [`LubikansPlugin`](../lubikans);
evaluation skips a player until a ledger is present.
