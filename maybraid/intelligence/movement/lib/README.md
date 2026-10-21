# Movement intelligence

Install [`MovementIntelligence`](src/user.rs) on a capsule and register
[`MovementIntelligencePlugin`](src/plugin.rs) with a
[`MovementIntelligenceSurface`](src/surface.rs) `SystemParam`.

Writes [`MoveWish`](../../../player/lib/src/body.rs) as an XZ heading toward the next
waypoint. The capsule motor projects that onto the current walkable plane
while grounded; this crate does not own physics. Higher-order systems write
[`MovementObjective`](src/objective.rs) and insert
[`ReplanMovement`](src/user.rs) to rebuild the plan.

Per-character [`MovementAbility`](src/ability.rs) owns covering (budget, vantage standoffs)
and `path_segment` (walk-stride ceiling for local vias).
[`MovementIntelligenceLimits`](src/surface.rs) caps that budget for the app,
drains at most `max_replans_per_frame` markers, and spends at most
`max_walk_probes_per_frame` Avian walk attempts per `Update`. `Reach` /
`EdgeOf` keep one candidate (or a small ring) and may detour unless already
on the disk; nearby `VantageOn` / `FleeFrom` keep the fan. Viewer Far still
snaps. Leftover [`ReplanMovement`](src/user.rs) stays queued; do not timer
the replan set.
