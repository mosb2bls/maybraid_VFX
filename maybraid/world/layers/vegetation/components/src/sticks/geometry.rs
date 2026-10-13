//! Stick continuous forms.

use lod::scene::LodSceneLevel;

use crate::assets::{sticks as stick_assets, AssetPath};

/// Stick footprint / role in the chain.
///
/// Geometry selects the GLB triad under `vegetation/sticks/standard/`
/// (`001_*` vs `trunk_001_*`) and the mesh-LOD extent policy (radius vs
/// length-dominated).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum StickGeometry {
	/// Generic branch / connector segment (`001_*` GLBs; radius-based mesh LOD).
	#[default]
	Segment,
	/// Primary trunk / long-lived woody member (`trunk_001_*` GLBs; length-biased mesh LOD).
	Trunk,
}

impl StickGeometry {
	pub fn segment() -> Self {
		Self::Segment
	}

	pub fn trunk() -> Self {
		Self::Trunk
	}

	/// Standard-kit GLB for this geometry at `level` (`None` = empty UltraLow).
	pub fn standard_glb_for_level(self, level: LodSceneLevel) -> Option<AssetPath> {
		match self {
			Self::Segment => match level {
				LodSceneLevel::High => Some(stick_assets::standard::HIGH),
				LodSceneLevel::Medium => Some(stick_assets::standard::MID),
				LodSceneLevel::Low | LodSceneLevel::Distance(_) | LodSceneLevel::Resolution(_) => {
					Some(stick_assets::standard::LOW)
				}
				LodSceneLevel::UltraLow => None,
			},
			Self::Trunk => match level {
				LodSceneLevel::High => Some(stick_assets::standard::trunk::HIGH),
				LodSceneLevel::Medium => Some(stick_assets::standard::trunk::MID),
				LodSceneLevel::Low | LodSceneLevel::Distance(_) | LodSceneLevel::Resolution(_) => {
					Some(stick_assets::standard::trunk::LOW)
				}
				LodSceneLevel::UltraLow => None,
			},
		}
	}
}
