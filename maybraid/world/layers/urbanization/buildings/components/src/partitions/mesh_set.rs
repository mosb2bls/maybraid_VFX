//! Partition mesh path sets (resolution policy — no host BSN).
//!
//! Three GLB paths today (high / mid / low). A fourth **ultra-low** path is planned;
//! until then UltraLow banding shares [`PartitionMeshTier::Low`].

use bevy::scene::prelude::Scene;

use crate::assets::AssetPath;

/// Which resolution [`SceneRef`](scene_ref::SceneRef) is selected for a [`LodSceneLevel`](lod::scene::LodSceneLevel).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PartitionMeshTier {
	/// Shared by Low and UltraLow until a dedicated ultra-low GLB exists.
	Low,
	Mid,
	High,
}

/// High / mid / low GLB set for one kit piece (may repeat the same path).
///
/// Add `ultra_low` when those assets are authored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PartitionMeshSet {
	pub high: AssetPath,
	pub mid: AssetPath,
	pub low: AssetPath,
}

impl PartitionMeshSet {
	pub const fn new(high: AssetPath, mid: AssetPath, low: AssetPath) -> Self {
		Self { high, mid, low }
	}

	pub const fn uniform(asset: AssetPath) -> Self {
		Self { high: asset, mid: asset, low: asset }
	}

	pub fn for_tier(self, tier: PartitionMeshTier) -> AssetPath {
		match tier {
			PartitionMeshTier::High => self.high,
			PartitionMeshTier::Mid => self.mid,
			PartitionMeshTier::Low => self.low,
		}
	}
}

pub fn mesh_child(asset: AssetPath) -> Box<dyn Scene> {
	Box::new(asset.scene_ref().scene())
}
