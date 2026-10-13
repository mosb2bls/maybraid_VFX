//! Foliage continuous forms.

use lod::scene::LodSceneLevel;

use crate::assets::{foliage as foliage_assets, AssetPath};
use crate::foliage::collection::{CheapBallCollection, FrondCollection, FrondKit};

/// Foliage footprint / construction.
///
/// Tessellated / multi-leaf forms (e.g. [`Self::FrondCollection`]) sit on the same
/// enum as single kits — one [`crate::FoliageNode`] and one foliage LOD probe, like
/// polyline partitions under a partition node.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum FoliageGeometry {
	/// Layered-ball kit: unit ball before placement scale (GLB under standard style).
	LayeredBall,
	/// Cheap-ball kit: lower-poly unit ball for dense packed clusters.
	#[default]
	CheapBall,
	/// Point tip kit (`straight_frond_001_*`); prefer [`Self::StraightFrondSegment`] for strands.
	StraightFrond,
	/// Square-ended frond segment (`straight_frond_segment_001_*`):
	/// \(Y \in [0, 1]\), \(X \in [-0.1, 0.1]\), \(Z\) negligible.
	StraightFrondSegment,
	/// Many placed frond kits under one LOD parent (merge thinning by distance).
	FrondCollection(FrondCollection),
	/// Many placed cheap-ball kits under one LOD parent (merge thinning by distance).
	CheapBallCollection(CheapBallCollection),
}

impl FoliageGeometry {
	pub fn layered_ball() -> Self {
		Self::LayeredBall
	}

	pub fn cheap_ball() -> Self {
		Self::CheapBall
	}

	pub fn straight_frond() -> Self {
		Self::StraightFrond
	}

	pub fn straight_frond_segment() -> Self {
		Self::StraightFrondSegment
	}

	pub fn frond_collection(collection: FrondCollection) -> Self {
		Self::FrondCollection(collection)
	}

	pub fn cheap_ball_collection(collection: CheapBallCollection) -> Self {
		Self::CheapBallCollection(collection)
	}

	pub fn is_layered_ball(&self) -> bool {
		matches!(self, Self::LayeredBall)
	}

	pub fn is_cheap_ball(&self) -> bool {
		matches!(self, Self::CheapBall)
	}

	pub fn is_frond_kit(&self) -> bool {
		matches!(self, Self::StraightFrond | Self::StraightFrondSegment)
	}

	pub fn is_frond_collection(&self) -> bool {
		matches!(self, Self::FrondCollection(_))
	}

	pub fn as_frond_collection(&self) -> Option<&FrondCollection> {
		match self {
			Self::FrondCollection(c) => Some(c),
			_ => None,
		}
	}

	pub fn as_cheap_ball_collection(&self) -> Option<&CheapBallCollection> {
		match self {
			Self::CheapBallCollection(c) => Some(c),
			_ => None,
		}
	}

	pub fn is_kit_collection(&self) -> bool {
		matches!(self, Self::FrondCollection(_) | Self::CheapBallCollection(_))
	}

	fn standard_triad_for_level(
		level: LodSceneLevel,
		high: AssetPath,
		mid: AssetPath,
		low: AssetPath,
	) -> AssetPath {
		match level {
			LodSceneLevel::High => high,
			LodSceneLevel::Medium => mid,
			LodSceneLevel::Low
			| LodSceneLevel::UltraLow
			| LodSceneLevel::Distance(_)
			| LodSceneLevel::Resolution(_) => low,
		}
	}

	/// Layered-ball GLB for `level`.
	pub fn layered_ball_glb_for_level(level: LodSceneLevel) -> AssetPath {
		Self::standard_triad_for_level(
			level,
			foliage_assets::standard::LAYERED_BALL_HIGH,
			foliage_assets::standard::LAYERED_BALL_MID,
			foliage_assets::standard::LAYERED_BALL_LOW,
		)
	}

	/// Cheap-ball GLB for `level`.
	pub fn cheap_ball_glb_for_level(level: LodSceneLevel) -> AssetPath {
		Self::standard_triad_for_level(
			level,
			foliage_assets::standard::CHEAP_BALL_HIGH,
			foliage_assets::standard::CHEAP_BALL_MID,
			foliage_assets::standard::CHEAP_BALL_LOW,
		)
	}

	/// Point-tip straight frond GLB (`straight_frond_001_*`).
	pub fn straight_frond_glb_for_level(level: LodSceneLevel) -> AssetPath {
		Self::standard_triad_for_level(
			level,
			foliage_assets::standard::STRAIGHT_FROND_HIGH,
			foliage_assets::standard::STRAIGHT_FROND_MID,
			foliage_assets::standard::STRAIGHT_FROND_LOW,
		)
	}

	/// Square-ended straight frond segment GLB (`straight_frond_segment_001_*`).
	pub fn straight_frond_segment_glb_for_level(level: LodSceneLevel) -> AssetPath {
		Self::standard_triad_for_level(
			level,
			foliage_assets::standard::STRAIGHT_FROND_SEGMENT_HIGH,
			foliage_assets::standard::STRAIGHT_FROND_SEGMENT_MID,
			foliage_assets::standard::STRAIGHT_FROND_SEGMENT_LOW,
		)
	}

	/// Kit GLB for one [`FrondKit`] member at `level`.
	pub fn frond_kit_glb_for_level(kit: FrondKit, level: LodSceneLevel) -> AssetPath {
		match kit {
			FrondKit::StraightFrond => Self::straight_frond_glb_for_level(level),
			FrondKit::StraightFrondSegment => Self::straight_frond_segment_glb_for_level(level),
		}
	}
}
