//! [`Development`]: every kind of development cell, one module each.
//!
//! A development is planned over a [`SiteGround`], which settles the pads it
//! stands on, then built: its buildings fitted to that plan. Most stand on one
//! [`crate::Terrace`] and only say how they fit it, as a
//! [`crate::TerraceDevelopment`].
//!
//! Buildings and helpers only one development uses are submodules of it.
//! Where another development reuses them (temple sanctums raise ring-fort
//! keeps; most developments place shepherds' houses and huts), it imports
//! them from the development that owns them.

pub mod les_halles;
pub mod old_city_market;
pub mod ring_fort;
pub mod shepherds_commune;
pub mod shepherds_village;
pub mod single_highrise;
pub mod skybridge_bazaar;
pub mod suburban_homes;
pub mod temple_complex;
pub mod wizards_tower;

pub use les_halles::{LesHalles, LES_HALLES_MAX_FOOTPRINT};
pub use old_city_market::OldCityMarket;
pub use ring_fort::{RingFort, RING_FORT_MAX_FOOTPRINT, RING_FORT_MIN_FOOTPRINT};
pub use shepherds_commune::ShepherdsCommune;
pub use shepherds_village::ShepherdsVillage;
pub use single_highrise::SingleHighrise;
pub use skybridge_bazaar::SkybridgeBazaar;
pub use suburban_homes::SuburbanHomes;
pub use temple_complex::TempleComplex;
pub use wizards_tower::WizardsTower;

use bevy_math::bounding::Aabb3d;
use procedural_common::{NoiseParams, SeededHash};

use crate::plan::cell_salt;
use crate::{PadPlan, SiteGround};

/// A development cell's kit assembly: planned over its ground, then built.
pub trait Development: Sized {
	/// What planning settles before any building is fitted.
	type Plan;

	/// The plan for `cell` and the pads it stands on, or `None` where this
	/// development does not fit the ground.
	fn plan(
		ground: &mut impl SiteGround,
		cell: Aabb3d,
		seed: u32,
	) -> Option<(Self::Plan, Vec<PadPlan>)>;

	/// The buildings fitted to `plan`.
	fn build(plan: &Self::Plan) -> Option<Self>;
}

/// Root draw for a layout fitted to confines on `cell`.
fn root_hash(cell: Aabb3d, noise: NoiseParams) -> SeededHash {
	SeededHash::new((noise.seed as u32).wrapping_add(cell_salt(cell)))
}

/// Lift every site to within `max_relief` of the highest.
fn raise_toward_peak(heights: &mut [Option<f32>], max_relief: f32) {
	let peak = heights.iter().flatten().copied().max_by(f32::total_cmp);
	let Some(peak) = peak else {
		return;
	};
	let floor = peak - max_relief.max(0.0);
	for height in heights.iter_mut().flatten() {
		*height = height.max(floor);
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn sites_stay_near_the_peak() -> anyhow::Result<()> {
		let mut heights = vec![Some(100.0), Some(72.0), Some(95.0), None];
		raise_toward_peak(&mut heights, 8.0);
		anyhow::ensure!(heights[0] == Some(100.0));
		anyhow::ensure!(heights[1] == Some(92.0));
		anyhow::ensure!(heights[2] == Some(95.0));
		anyhow::ensure!(heights[3].is_none());
		Ok(())
	}
}
