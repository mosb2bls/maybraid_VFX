//! Richmond developments: place building hosts (floors, shafts, roofs).
//!
//! Analogous to `chico-groves`: a development fits confines and emits flattened
//! hosts. Every kind of development cell is a [`Development`], one module each
//! under [`development`], with the buildings it assembles as its submodules.
//! The other modules here are what any development builds with: pads, ground
//! probes, scatter and connectivity layouts, and finishes.
//!
//! These are the developments as kit assemblies. Planning reads the ground
//! through a [`SiteGround`] probe and returns the [`PadPlan`]s a development
//! stands on; generating them over real terrain is Richmond's concern.

pub mod connected;
pub mod connectivity;
pub mod development;
pub mod finish;
pub mod ground;
mod math;
pub mod pad;
pub mod placed;
pub mod plan;
pub mod scatter;
pub mod terrace;

pub use connected::{ConnectedDevelopment, DevelopmentEdge};
pub use connectivity::{corridor_levels, ConnectivityCorridor, ConnectivityGraph};
pub use development::les_halles::{
	courtyard_well_side, MixedUseLesHallesDevelopment, MixedUseLesHallesHost,
};
pub use development::old_city_market::{
	OldCityMarketCorridor, OldCityMarketSite, OldCityMarketTerrace, OldCityMarketTier,
	MARKET_PLATFORM_HEIGHT,
};
pub use development::ring_fort::{
	CircularTower, CurtainRing, GalleryColonnade, GalleryTerrace, Keep, RingFortBuilding,
	RingFortHost, RingFortJoin, RingFortKeep, RingFortSite, RingFortTower, TrazaloidTower,
};
pub use development::shepherds_commune::{ShepherdsCommuneCorridor, ShepherdsCommuneSite};
pub use development::shepherds_village::{
	ShepherdsBuilding, ShepherdsFinish, ShepherdsHouse, ShepherdsHut, ShepherdsVillageBuilding,
	HOUSE_MAX_FOOTPRINT, HOUSE_MIN_FOOTPRINT, HOUSE_STOREY_HEIGHT, HUT_HEIGHT, HUT_MAX_FOOTPRINT,
	HUT_MIN_FOOTPRINT,
};
pub use development::skybridge_bazaar::Skybridge;
pub use development::temple_complex::{TempleSanctum, TempleSanctumComponents};
pub use development::wizards_tower::SolitaryWizardsTower;
pub use development::{
	Development, LesHalles, OldCityMarket, RingFort, ShepherdsCommune, ShepherdsVillage,
	SingleHighrise, SkybridgeBazaar, SuburbanHomes, TempleComplex, WizardsTower,
	LES_HALLES_MAX_FOOTPRINT, RING_FORT_MAX_FOOTPRINT, RING_FORT_MIN_FOOTPRINT,
};
pub use finish::{DevelopmentFinish, DevelopmentFinishRole};
pub use ground::SiteGround;
pub use pad::{PadForm, PadParams, PadPlan, PAD_BERM, PAD_EDGE_EASE, PAD_ROUND};
pub use placed::{BuildingFootprint, PlacedBuilding};
pub use plan::{
	cell_salt, inscribe_yawed_extents, sample_confines_yaw, yaw_about_xz, yawed_plan_aabb_extent,
};
pub use scatter::{bounds_intersect, ScatterCandidate, ScatterChoice, ScatterPlan, ScatterRecipe};
pub use terrace::{Terrace, TerraceDevelopment, TerraceEnvelope};
