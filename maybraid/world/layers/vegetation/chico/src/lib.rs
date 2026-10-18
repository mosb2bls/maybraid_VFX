//! Cellular forests for Chico vegetation ([RFC-183 §3.5]).
//!
//! Forests and groves generate on the shared HCSG runtime; each forest cell is
//! 1600 m with Hopscotch layer selection and 100 m grove tiles.

mod assemble;
mod blend;
mod bump_out;
mod chico;
mod config;
mod extent;
mod forest;
mod generation;
mod ground;
mod grove;
pub mod hcsg;
pub(crate) mod hopscotch;
mod host;
mod kind;
mod layer;
pub mod layerings;
mod material;
mod plugin;
mod recipe;
mod stick_physics;
mod stream;
mod view;

pub use assemble::{
	assemble, assemble_isolated, grow_tile, presenting_recipes, AssembledForest, ForestGroveTile,
	NeighborLayers,
};
pub use blend::{
	GROVE_BLEND_INFLUENCE, GROVE_BLEND_NOISE, GROVE_BLEND_RADIUS, GROVE_BLEND_TEMPERATURE,
};
pub use bump_out::{
	blend_selection_neighborhood, blend_selection_on_bounds, bump_out_cell_bounds,
	bump_out_cells_overlapping, bump_out_chebyshev_xz, bump_out_from_cell, bump_out_in_inner_hole,
	bump_out_noise, medium_bump_out_in_band, selection_sample_at, BumpOutSelection,
	BumpOutSelectionSample, CanopyBumpOut, MediumCanopyBumpOut, BUMP_OUT_CELL_XZ,
	BUMP_OUT_INNER_RADIUS_M, BUMP_OUT_OUTER_RADIUS_M, MEDIUM_BUMP_OUT_ANCHOR_STEP_M,
	MEDIUM_BUMP_OUT_CELL_XZ, MEDIUM_BUMP_OUT_INNER_RADIUS_M, MEDIUM_BUMP_OUT_OUTER_RADIUS_M,
};
pub use chico::{chico_hopscotch, select_cell, select_layering, DEFAULT_HOP_BUDGET};
pub use config::{
	stream_radii_m, ChicoConfig, ForestSelection, ForestStreamSpec, DEFAULT_FOREST_NOISE,
	DEFAULT_FOREST_STREAM_RADIUS,
};
pub use extent::{ForestExtent, DEFAULT_FOREST_EXTENT_XZ, DEFAULT_FOREST_GROVE_TILE_XZ};
pub use forest::{neighbor_layers, ChicoForest};
pub use generation::{GROVE_GENERATE_RADIUS_M, GROVE_PRESENT_RADIUS_M};
pub use ground::{fine_overlay_size, overlay_chunk_ref};
pub use grove::{grove_from_id, grove_id, ChicoGrove};
pub use hcsg::{
	BumpOutPresentationPlugin, BumpOutRing, BumpedOut, CanopyProxy, ChicoPresentationPlugin,
	ChicoRoots, ForestGround, GroundSurface, GroveNeighborhood, GrownGrove,
};
pub use hopscotch::{select as hopscotch_select, HopscotchNode};
pub use host::ChicoGroveHost;
pub use kind::{
	ForestGroveKind, ForestLayer, ForestLayering, LayerDropOut, LayeringKind, SelectedLayers,
	WeightedGrove, TUFT_DROP_MIN_HEIGHT_M,
};
pub use layer::{select_layers, throw_layer};
pub use material::{VegetationOnTerrainMaterialLib, VegetationOnTerrainMaterialRefPlugin};
pub use plugin::{register_vegetation_view, VegetationViewPlugin};
pub use recipe::{ForestGroveRecipe, WORLD_FOREST_TREE_VARIANTS};
pub use stream::parse_layering_kind;
pub use view::{
	VegetationBullseye, VegetationCull, VegetationLodRefreshPlugin, VegetationRefresh,
	VegetationSpotlight,
};
