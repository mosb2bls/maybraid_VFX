//! [`RingFort`]: one walled ring fort on a yawed terrace.

mod building;
mod curtain_ring;
mod keep;

pub use building::{
	GalleryColonnade, GalleryTerrace, RingFortBuilding, RingFortHost, RingFortJoin, RingFortSite,
	RingFortTower,
};
pub use curtain_ring::CurtainRing;
pub use keep::{CircularTower, Keep, RingFortKeep, TrazaloidTower, TOWER_STOREY_HEIGHT};

use buildings::Fit;
use procedural_common::SeededHash;

use crate::{DevelopmentFinish, PlacedBuilding, Terrace, TerraceDevelopment, TerraceEnvelope};

/// Ring fort courtyard + corner keeps need a wide curtain wall inside the cell.
pub const RING_FORT_MAX_FOOTPRINT: f32 = 240.0;
/// Minimum ring-fort plan so a keep-bearing gallery and courtyard still fit.
pub const RING_FORT_MIN_FOOTPRINT: f32 = 120.0;
/// Confines height for the courtyard ring only (2–4 storeys at 3 m).
const MIN_CONFINES_HEIGHT: f32 = 8.0;
const MAX_CONFINES_HEIGHT: f32 = 13.0;

/// One walled ring fort on a yawed terrace.
pub type RingFort = PlacedBuilding<RingFortBuilding>;

impl TerraceDevelopment for RingFort {
	fn envelope() -> TerraceEnvelope {
		TerraceEnvelope {
			min_footprint: RING_FORT_MIN_FOOTPRINT,
			max_footprint: RING_FORT_MAX_FOOTPRINT,
			min_height: MIN_CONFINES_HEIGHT,
			max_height: MAX_CONFINES_HEIGHT,
			rotates: true,
		}
	}

	fn finish(hash: SeededHash) -> DevelopmentFinish {
		DevelopmentFinish::pick(hash)
	}

	fn fit_terrace(terrace: &Terrace) -> Option<Self> {
		let (placed, _) = Self::fit_to_confines(&terrace.confines(), terrace.noise()).ok()?;
		let finish = terrace.finish.clone();
		Some(placed.map(|fort| fort.with_finish(finish.wall, finish.roof)))
	}
}
