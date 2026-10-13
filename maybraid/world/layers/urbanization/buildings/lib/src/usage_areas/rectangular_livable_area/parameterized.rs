//! Parameterized strategies for [`super::RectangularLivableArea`].

use procedural_common::{NoiseConfig, NoiseParams};

use crate::fit::{Confines, FitError};

/// How a rectangular livable area subdivides.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RectLivableStrategy {
	/// Try [`AllOpen`] → [`SingleClosed`] → [`SpineHall`] → [`GuillotineSplit`].
	CaseAttempt,
	/// Entire rect is open circulation / common quarters (no closed rooms).
	AllOpen,
	/// Single closed quarter; legal only with exactly one passage and small area.
	SingleClosed,
	/// ≥1 m open spine connecting all passage faces; closed rooms abut the spine.
	SpineHall,
	/// Bipartition; recurse two child areas with a shared passage on the cut.
	GuillotineSplit,
}

/// Knobs for fitting a rectangular livable area.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RectangularLivableAreaParameterized {
	pub strategy: RectLivableStrategy,
	/// Minimum clear width (m) for open access / hall bands.
	pub min_hall: f32,
	/// Max footprint area (m²) for the single-closed shortcut.
	pub closed_max_area: f32,
}

pub const SCOPE: &str = "rectangular_livable_area";
pub const DEFAULT_MIN_HALL: f32 = 1.5;
pub const DEFAULT_CLOSED_MAX_AREA: f32 = 36.0;

impl Default for RectangularLivableAreaParameterized {
	fn default() -> Self {
		Self {
			strategy: RectLivableStrategy::CaseAttempt,
			min_hall: DEFAULT_MIN_HALL,
			closed_max_area: DEFAULT_CLOSED_MAX_AREA,
		}
	}
}

impl RectangularLivableAreaParameterized {
	pub fn sample(confines: &Confines, noise: NoiseParams) -> Result<Self, FitError> {
		let cfg = NoiseConfig::new(noise);
		let c = confines.center();
		// Mostly CaseAttempt; occasionally force a concrete strategy for variety.
		let pick = cfg.sample_range_f32_4d(0.0, 1.0, c.x, c.y, c.z, 71.0);
		let strategy = if pick < 0.72 {
			RectLivableStrategy::CaseAttempt
		} else if pick < 0.82 {
			RectLivableStrategy::AllOpen
		} else if pick < 0.90 {
			RectLivableStrategy::SpineHall
		} else if pick < 0.96 {
			RectLivableStrategy::GuillotineSplit
		} else {
			RectLivableStrategy::SingleClosed
		};
		Ok(Self { strategy, min_hall: DEFAULT_MIN_HALL, closed_max_area: DEFAULT_CLOSED_MAX_AREA })
	}
}

/// Chosen layout description after a successful strategy.
#[derive(Debug, Clone, PartialEq)]
pub struct RectangularLivableAreaPlan {
	pub parameterized: RectangularLivableAreaParameterized,
	/// Concrete strategy that produced the layout (never `CaseAttempt`).
	pub chosen: RectLivableStrategy,
	pub hall_bands: Vec<bevy_math::bounding::Aabb2d>,
}
