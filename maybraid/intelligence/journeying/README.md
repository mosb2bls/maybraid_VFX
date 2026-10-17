# Journeying intelligence

`JourneyingIntelligenceUser` probes deterministic xz tiles several cells away,
resolves POIs for that tile through a [`PoiOrigin`](../poi/src/scheme.rs)
`GenerationScheme` (`get_or_generate_in`), learns a relevant pin, and creates a
`PoiGoal` for routing or movement. Confirmed-empty tiles are cached briefly.
`linger_secs` is copied onto the goal (default zero: finish on first arrival).

World groves, urban settings, and discoverable places generate as POI schemes
on those tiles. A cycle policy first fills its explicit roster from distant
tiles, then repeats that roster independent of later tile probes.

If an entity has both meandering and journeying components, meandering owns POI
selection; journeying remains inactive so the two brains cannot race one goal.
