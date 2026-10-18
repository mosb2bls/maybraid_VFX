//! Geneva: tiled languages and place names over a vegetated world.
//!
//! Large tiles compose grammar and lexicon bundles from seed and coordinates.
//! Guillotine small tiles pick a subset. Nearby features and places are named
//! by translating English kind labels in the chosen language. Names are
//! generated values like the features they name; Geneva does not change the
//! ground. [`GenevaPlugin`] runs it on the shared HCSG runtime.

mod bundle;
mod catalog;
mod english;
pub mod hcsg;
mod key;
mod name;
mod named;
mod places;
mod present;
mod tiles;

pub use bundle::{LanguageBundle, LexiconFamily};
pub use catalog::KindConceptUniverse;
pub use english::{
	color_term, compose_english, development_terms, geographic_terms, grove_kind_terms,
	layering_terms, named_forest_english, named_geographic_english, named_grove_english,
	named_place_english, named_region_english, named_urban_english, place_label_terms,
	urbanization_terms, with_color_name, PLACE_COLORS,
};
pub use hcsg::{
	GenevaPlugin, GenevaRoots, LanguageConfig, LanguageNeighborhood, LanguageWorldSeed,
	NAME_WINDOW_QUANT_M,
};
pub use key::{name_key_salt, NameKey};
pub use name::PlaceName;
pub use named::{
	AuthoredWaters, Forests, GeographicStamp, Groves, NameEntry, NameSource, Nameable, Named,
	Places, Regions, Speaks, Stamp, Urban, Waters,
};
pub use places::{DevelopmentPlace, DevelopmentPlaces, LanguageGround};
pub use present::{LanguageOverlay, LargeTileOverlay, NamedOverlay};
pub use tiles::{
	large_tile_aabb, large_tile_index, large_tile_origin, large_tiles_overlapping, LargeTile,
	SmallTile, LARGE_TILE, SMALL_STEP_MAX, SMALL_STEP_MIN,
};

#[cfg(test)]
mod tests;
