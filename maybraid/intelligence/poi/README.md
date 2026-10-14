# POI intelligence

Shared point-of-interest discovery and retained knowledge.

- `Poi` carries a stable `PoiId`, semantic `PoiKind`, arrival radius, and salience.
- `LocalPoi` enters a Gimme-backed bounded index for nearby scans; `GlobalPoi`
  enters a sparse whole-map set for those scans.
- Distant xz-tile lookups use a `PoiOrigin` `GenerationScheme` (`PoiSet<S>`)
  via `get_or_generate_in`. World sources (groves, urban settings, places)
  implement `PoiOrigin` and reuse those grids' ids. There is no side POI index
  for this path.
- `PoiIntelligenceUser` scans both sets according to its interests and learning policy.
- `PoiKnowledge` unions source ownership, retains old findings, and accepts directed
  `PoiObservation` messages from any other discovery system.
- `PoiGoal` hands a selected destination to routing when available, otherwise directly
  to movement. `destination_salt` offsets that walk point on the arrival disk so
  pack-mates who share a `PoiId` do not all aim at the pin (`0` keeps the center).
  Plant / replace / salted live points use a 20–80 m disk so a small authored
  arrival radius does not stack everyone on the porch.
  `linger_secs` keeps the goal active after first arrival; leaving the
  disk resets that clock. `PoiGoalState` preserves generation-tagged completion
  state and `PoiGoalCompleted` notifies one-shot consumers once linger finishes.

`PoiVisitPolicy::Cycle` deliberately uses a roster and cursor. Once its roster reaches
the requested size, higher-order users repeat that exact sequence instead of trying to
approximate a loop with novelty and retention weights. `Weighted` still prefers unvisited
and off-cooldown POIs (new discoveries win immediately). Cooldown is a ranking bias, not
a freeze: if every known candidate is cooling, selection keeps the stalest one so a small
local cluster keeps circulating while the next scan can load more.
