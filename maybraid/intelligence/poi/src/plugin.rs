use bevy::prelude::*;
use intelligence_lod::IntelligencePriority;
use movement_intelligence::MovementIntelligenceSystems;
use routing_intelligence::RoutingSystems;

use crate::{
	complete_poi_goals, discover_pois, drive_poi_goals, ingest_poi_observations, refresh_poi_goals,
	sync_poi_registry, DistantTilePoiLookup, PoiGoalCompleted, PoiObservation, PoiRegistry,
};

/// How many due learners may run a local / global scan on one Discover tick.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct PoiDiscoverLimits {
	pub max_scans_per_tick: usize,
}

impl Default for PoiDiscoverLimits {
	fn default() -> Self {
		Self { max_scans_per_tick: 8 }
	}
}

/// Shared ordering points for discovery and higher-order POI users.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PoiSystems {
	Index,
	Ingest,
	Discover,
	RefreshGoal,
	Complete,
	/// Meandering, journeying, and other goal selectors run here.
	Select,
	Drive,
}

/// Registers POI indexing, discovery, inbox, retention, and goal handoff.
pub struct PoiIntelligencePlugin;

impl Plugin for PoiIntelligencePlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<PoiRegistry>()
			.init_resource::<DistantTilePoiLookup>()
			.init_resource::<IntelligencePriority>()
			.init_resource::<PoiDiscoverLimits>()
			.add_message::<PoiObservation>()
			.add_message::<PoiGoalCompleted>()
			.configure_sets(
				Update,
				(
					PoiSystems::Index,
					PoiSystems::Ingest,
					PoiSystems::Discover,
					PoiSystems::RefreshGoal,
					PoiSystems::Complete,
					PoiSystems::Select,
					PoiSystems::Drive,
				)
					.chain(),
			)
			.configure_sets(
				Update,
				PoiSystems::Drive
					.before(RoutingSystems::Plan)
					.before(MovementIntelligenceSystems::Replan),
			)
			.add_systems(Update, sync_poi_registry.in_set(PoiSystems::Index))
			.add_systems(Update, ingest_poi_observations.in_set(PoiSystems::Ingest))
			.add_systems(Update, discover_pois.in_set(PoiSystems::Discover))
			.add_systems(Update, refresh_poi_goals.in_set(PoiSystems::RefreshGoal))
			.add_systems(Update, complete_poi_goals.in_set(PoiSystems::Complete))
			.add_systems(Update, drive_poi_goals.in_set(PoiSystems::Drive));
	}
}
