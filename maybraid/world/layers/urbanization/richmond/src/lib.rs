//! Richmond development models: urbanization leaves, terrain pads, and a unified
//! generation path for single buildings, campuses, and neighborhoods.
//!
//! Developments generate on the shared HCSG runtime over [`RichmondGround`].
//! [`PaddedTerrain`] is the presented ground surface; [`Built`] hosts nest
//! building scenes.

pub mod artifact;
pub mod buildings_lod;
pub mod built;
pub mod cell;
pub mod compose;
pub mod config;
pub mod developments;
pub mod ground;
pub mod hcsg;
pub mod host;
pub mod layer;
pub mod layer_config;
pub mod pad;
pub mod padded;
pub mod place;
pub mod plugin;
pub mod storage;

pub use artifact::BuiltDevelopment;
pub use buildings_lod::{
	register_developments_buildings_lod_plugin, BuildingsBullseye, BuildingsCull,
	BuildingsSpotlight, DevelopmentsBuildingsLodPlugin,
};
pub use built::Built;
pub use cell::{
	cell_selected, DevelopmentExtent, BUILDING_INSET, DEFAULT_LIKELIHOOD,
	DEFAULT_SPATIAL_CORRELATION, DEVELOPMENT_CELL_SIZE,
};
pub use compose::PadComposable;
pub use config::{DevelopmentConfig, DevelopmentSites};
pub use developments::site::{
	select_kind, AuthoredCourtyard, AuthoredDevelopment, AuthoredDevelopments, DevelopmentKind,
	DevelopmentSite,
};
pub use developments::{DevelopmentCell, DevelopmentPad, RichmondDevelopment, SiteDevelopment};
pub use ground::{hydro_overlaps_xz, GroundCell, GroundCells, RichmondGround};
pub use hcsg::{
	BuiltPresentationPlugin, DevelopmentNeighborhood, RichmondPresentationPlugin, RichmondRoots,
};
pub use host::{DevelopmentHost, DevelopmentHosts};
pub use layer::Richmond;
pub use layer_config::{
	stream_radii_m, DevelopmentFocus, RichmondConfig, UrbanizationStreamSpec,
	DEFAULT_URBANIZATION_NOISE, DEFAULT_URBANIZATION_STREAM_RADIUS, PLAYGROUND_LIKELIHOOD,
};
pub use pad::{cell_bounds2, PadComplex, PadNode, PadPrimitive};
pub use padded::{PaddedTerrain, TerrainWithPads};
pub use place::{DiscoverablePlace, DiscoverablePlaceLabel, InteriorArea};
pub use plugin::{register_richmond_plugin, RichmondDevelopmentModelsPlugin};
pub use storage::column_bounds;
pub use urbanization_developments::{
	yaw_about_xz, Development, DevelopmentFinish, DevelopmentFinishRole, PadParams, SiteGround,
	Terrace, TerraceDevelopment, TerraceEnvelope, LES_HALLES_MAX_FOOTPRINT, PAD_BERM,
	PAD_EDGE_EASE, PAD_ROUND, RING_FORT_MAX_FOOTPRINT, RING_FORT_MIN_FOOTPRINT,
};
