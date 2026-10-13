//! Height operators produced by landform stamps.

pub mod affine;
pub mod bowl;
pub mod cell_domain;
pub mod grading;
pub mod multi_polyline_band;
pub mod polyline_grading;

pub use affine::RegionAffineModulation;
pub use bowl::RegionBowlModulation;
pub use cell_domain::{CellDomainMask, CELL_DOMAIN_EASE_FRAC, DEFAULT_CELL_DOMAIN_EASE};
pub use grading::RegionGradingModulation;
pub use multi_polyline_band::{
	soft_voronoi_weights, MultiPolylineBandModulation, MultiPolylineBandPart,
	MultiPolylineOffsetModulation, MultiPolylineOffsetPart,
};
pub use polyline_grading::{PolylineGradeMode, RegionPolylineGradingModulation};

use procedural_common::Bounds2;

/// All Stamp height operators — preferred consumer surface for durham compose.
#[derive(Debug, Clone)]
pub enum StampModulation {
	Affine(RegionAffineModulation),
	Grading(RegionGradingModulation),
	Bowl(RegionBowlModulation),
	PolylineGrading(RegionPolylineGradingModulation),
	MultiPolylineBand(MultiPolylineBandModulation),
	MultiPolylineOffset(MultiPolylineOffsetModulation),
	/// Construction op bound to a cell / leaf domain (hard-clip + edge ease).
	CellBound {
		domain: CellDomainMask,
		inner: Box<StampModulation>,
	},
}

impl StampModulation {
	pub fn modify_elevation(&self, elevation: f32, x: f32, z: f32) -> f32 {
		match self {
			Self::Affine(m) => m.modify_elevation(elevation, x, z),
			Self::Grading(m) => m.modify_elevation(elevation, x, z),
			Self::Bowl(m) => m.modify_elevation(elevation, x, z),
			Self::PolylineGrading(m) => m.modify_elevation(elevation, x, z),
			Self::MultiPolylineBand(m) => m.modify_elevation(elevation, x, z),
			Self::MultiPolylineOffset(m) => m.modify_elevation(elevation, x, z),
			Self::CellBound { domain, inner } => {
				let w = domain.weight(x, z);
				if w <= 0.0 {
					return elevation;
				}
				let y = inner.modify_elevation(elevation, x, z);
				if w >= 1.0 {
					return y;
				}
				elevation + w * (y - elevation)
			}
		}
	}

	/// Wrap this op in a cell-domain mask (hard exterior clip + interior ease).
	pub fn bind_to_cell(self, bounds: Bounds2) -> Self {
		Self::CellBound { domain: CellDomainMask::for_bounds(bounds), inner: Box::new(self) }
	}

	/// Bind every modulation to `bounds` so support is exactly identity outside.
	pub fn bind_all(modulations: Vec<Self>, bounds: Bounds2) -> Vec<Self> {
		modulations.into_iter().map(|m| m.bind_to_cell(bounds)).collect()
	}
}
