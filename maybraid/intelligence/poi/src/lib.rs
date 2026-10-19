//! Shared POI identity, spatial discovery, retained knowledge, and goal handoff.
//!
//! [`LocalPoi`] values are indexed through Gimme for bounded nearby scans.
//! [`GlobalPoi`] values form a deliberately sparse whole-map set for those
//! scans. Distant xz-tile lookups resolve through a [`PoiOrigin`]
//! [`lod::hcsg::GenerationScheme`] (`get_or_generate_in`), not a global list.

mod discovery;
mod goal;
mod hash;
mod kind;
mod knowledge;
mod marker;
mod nearby;
mod place;
mod plugin;
mod policy;
mod registry;
mod scheme;
mod source;
mod visit;

pub use discovery::{discover_pois, ingest_poi_observations, sync_poi_registry};
pub use goal::{
	begin_poi_goal, complete_poi_goals, drive_poi_goals, refresh_poi_goals, PoiGoal,
	PoiGoalCompleted, PoiGoalState, PoiGoalStatus,
};
pub use hash::{hashed_poi_id, mix as mix_seed};
pub use kind::{PoiId, PoiKind};
pub use knowledge::{KnownPoi, PoiIntelligenceUser, PoiKnowledge, PoiObservation};
pub use marker::{GlobalPoi, LocalPoi, Poi, MAX_POI_ARRIVAL_RADIUS};
pub use nearby::{NearbyChoice, NearbyFallback, NearbyQuery, DEFAULT_NEARBY_RADIUS};
pub use place::{place_nearby, place_nearby_among, ArrivalDisk, NearbyPlace, AGENT_SEPARATION};
pub use plugin::{PoiDiscoverLimits, PoiIntelligencePlugin, PoiSystems};
pub use policy::{PoiInterest, PoiInterests, PoiLearningPolicy, PoiVisitPolicy};
pub use registry::{PoiRecord, PoiRegistry};
pub use scheme::{
	collect_matching_in_xz_tile, xz_tile, xz_tile_region, Catalog, DistantTilePoiLookup,
	DistantTilePois, GridTiles, PoiCatalog, PoiEntry, PoiOrigin, PoiSet, GRID_KIND,
};
pub use source::PoiSource;
pub use visit::{choose_poi, PoiVisitState};

#[cfg(test)]
mod scheme_timing;
#[cfg(test)]
mod tests;
