//! Shared woody grove LOD router. Authored bands and canopy policy stay on the grove.

use bevy::prelude::Vec3;
use bevy::scene::prelude::Scene;
use lod::lod_ref::LodRef;
use lod::scene::LodSceneLevel;
use lod::SceneChunk;
use vegetation_components::{
	flattened_canopy_proxy_chunks, FoliageNode, Layers, StickNode, StructuralLod,
	VegetationComponents,
};

use super::vc_compose::{
	foliage_ultra_low_merged_balls, grove_detail_level, grove_detail_level_keep_low,
	layers_from_nodes, trained_proxy_stick_nodes_for_level, CanopyProxySite,
	ULTRA_LOW_CANOPY_BIN_METERS,
};

/// Tile canopy policy. High / Medium / Low numbers stay on [`WoodyGroveLod`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WoodyCanopyPolicy {
	/// High/Medium nest plants. Low = one ball per site. UltraLow bins.
	Ordinary,
	/// High/Medium/Low nest plants (palm Low star). UltraLow bins.
	KeepLowPlants,
	/// Like [`Ordinary`] but UltraLow does not bin (sparse groves).
	SkipUltraLowBins,
}

/// Authored woody tile bands plus canopy policy.
///
/// Opening a grove file should still show the numbers and the policy constructor
/// (`ordinary` / `keep_low_plants` / `skip_ultralow_bins` / `rory_trunk`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WoodyGroveLod {
	pub high: f32,
	pub medium: f32,
	pub low: f32,
	pub policy: WoodyCanopyPolicy,
	pub rory_trunks: bool,
}

impl WoodyGroveLod {
	pub const fn ordinary(high: f32, medium: f32, low: f32) -> Self {
		Self { high, medium, low, policy: WoodyCanopyPolicy::Ordinary, rory_trunks: false }
	}

	pub const fn keep_low_plants(high: f32, medium: f32, low: f32) -> Self {
		Self { high, medium, low, policy: WoodyCanopyPolicy::KeepLowPlants, rory_trunks: false }
	}

	pub const fn skip_ultralow_bins(high: f32, medium: f32, low: f32) -> Self {
		Self { high, medium, low, policy: WoodyCanopyPolicy::SkipUltraLowBins, rory_trunks: false }
	}

	/// Ordinary foliage plus trained Low / UltraLow trunk sticks.
	pub const fn rory_trunk(high: f32, medium: f32, low: f32) -> Self {
		Self { high, medium, low, policy: WoodyCanopyPolicy::Ordinary, rory_trunks: true }
	}

	pub const fn with_rory_trunks(self) -> Self {
		Self { rory_trunks: true, ..self }
	}

	pub fn structural_lod(self, center: Vec3, radius: f32) -> StructuralLod {
		StructuralLod::new(center, radius)
			.with_factors(self.high, self.medium, self.low)
			.with_preserve_ultra_low(true)
	}

	pub fn nest_plant_level(self, level: LodSceneLevel) -> Option<LodSceneLevel> {
		match self.policy {
			WoodyCanopyPolicy::KeepLowPlants => grove_detail_level_keep_low(level),
			WoodyCanopyPolicy::Ordinary | WoodyCanopyPolicy::SkipUltraLowBins => {
				grove_detail_level(level)
			}
		}
	}

	pub fn stick_nodes(
		self,
		level: LodSceneLevel,
		trunks: impl IntoIterator<Item = StickNode>,
	) -> Layers<StickNode> {
		if self.rory_trunks {
			trained_proxy_stick_nodes_for_level(level, trunks)
		} else {
			Layers::new()
		}
	}

	pub fn foliage_nodes(
		self,
		level: LodSceneLevel,
		sites: &[CanopyProxySite],
		low_nodes: Vec<FoliageNode>,
	) -> Layers<FoliageNode> {
		match (self.policy, level) {
			(_, LodSceneLevel::High | LodSceneLevel::Medium) => Layers::new(),
			(WoodyCanopyPolicy::KeepLowPlants, LodSceneLevel::Low) => Layers::new(),
			(
				WoodyCanopyPolicy::Ordinary | WoodyCanopyPolicy::SkipUltraLowBins,
				LodSceneLevel::Low,
			) => layers_from_nodes(low_nodes),
			(
				WoodyCanopyPolicy::SkipUltraLowBins,
				LodSceneLevel::UltraLow | LodSceneLevel::Distance(_) | LodSceneLevel::Resolution(_),
			) => layers_from_nodes(low_nodes),
			(
				WoodyCanopyPolicy::Ordinary | WoodyCanopyPolicy::KeepLowPlants,
				LodSceneLevel::UltraLow | LodSceneLevel::Distance(_) | LodSceneLevel::Resolution(_),
			) => layers_from_nodes(foliage_ultra_low_merged_balls(
				sites,
				ULTRA_LOW_CANOPY_BIN_METERS,
			)),
		}
	}

	pub fn scene_chunks(
		self,
		level: LodSceneLevel,
		lod_ref: &LodRef,
		plant_chunks: Vec<SceneChunk>,
		vegetation: &impl VegetationComponents,
	) -> SceneChunk {
		match self.nest_plant_level(level) {
			Some(_) => {
				if plant_chunks.is_empty() {
					SceneChunk::primitive(vegetation_components::scene_children(Vec::new()))
				} else {
					SceneChunk::chunks(plant_chunks)
				}
			}
			None => flattened_canopy_proxy_chunks(vegetation, lod_ref, level),
		}
	}

	pub fn scene_with_level(
		self,
		vegetation: &impl VegetationComponents,
		lod_ref: &LodRef,
		level: LodSceneLevel,
	) -> impl Scene + 'static {
		match self.nest_plant_level(level) {
			Some(_) => vegetation_components::scene_children(Vec::new()),
			None => {
				let mut children: Vec<Box<dyn Scene>> = Vec::new();
				vegetation_components::append_component_scenes(
					vegetation,
					lod_ref,
					level,
					&mut children,
				);
				vegetation_components::scene_children(children)
			}
		}
	}
}

/// Mechanical [`VegetationComponents`] + [`LodScene`] for a woody grove.
///
/// `$lod` is a [`WoodyGroveLod`] value that stays next to the grove's HIGH / MEDIUM / LOW
/// constants. Optional `trunks` / `low_nodes` use inherent `proxy_trunks` / `foliage_low_nodes`.
#[macro_export]
macro_rules! impl_woody_grove_lod {
	($Grove:ty, $lod:expr) => {
		$crate::impl_woody_grove_lod!(@emit $Grove, $lod, empty, default_low);
	};
	($Grove:ty, $lod:expr, trunks) => {
		$crate::impl_woody_grove_lod!(@emit $Grove, $lod, trunks, default_low);
	};
	($Grove:ty, $lod:expr, low_nodes) => {
		$crate::impl_woody_grove_lod!(@emit $Grove, $lod, empty, low_nodes);
	};
	($Grove:ty, $lod:expr, trunks, low_nodes) => {
		$crate::impl_woody_grove_lod!(@emit $Grove, $lod, trunks, low_nodes);
	};
	(@emit $Grove:ty, $lod:expr, $trunks:ident, $low:ident) => {
		impl vegetation_components::VegetationComponents for $Grove {
			fn stick_nodes_for_level(
				&self,
				level: lod::scene::LodSceneLevel,
			) -> vegetation_components::Layers<vegetation_components::StickNode> {
				$crate::impl_woody_grove_lod!(@trunks $lod, level, $trunks, self)
			}

			fn foliage_nodes_for_level(
				&self,
				level: lod::scene::LodSceneLevel,
			) -> vegetation_components::Layers<vegetation_components::FoliageNode> {
				$crate::impl_woody_grove_lod!(@low $lod, level, $low, self)
			}

			fn structural_lod(&self) -> Option<vegetation_components::StructuralLod> {
				Some(($lod).structural_lod(self.structural_center, self.footprint_radius))
			}
		}

		impl lod::scene::LodScene for $Grove {
			fn scene_lod_level(&self, lod_ref: &lod::lod_ref::LodRef) -> lod::scene::LodSceneLevel {
				self.structural_lod()
					.map(|band| $crate::grove::grove_lod_level(band, lod_ref))
					.unwrap_or(lod::scene::LodSceneLevel::High)
			}

			fn scene_lod_status(&self, lod_ref: &lod::lod_ref::LodRef) -> lod::scene::LodSceneStatus {
				self.structural_lod()
					.map(|band| $crate::grove::grove_lod_status(band, lod_ref))
					.unwrap_or(lod::scene::LodSceneStatus::Unchanged)
			}

			fn scene_lod_culls(
				&self,
				lod_ref: &lod::lod_ref::LodRef,
				_current: lod::scene::LodSceneLevel,
			) -> lod::scene::LodSceneCulls {
				self.structural_lod()
					.map(|band| $crate::grove::grove_lod_culls(band, lod_ref))
					.unwrap_or(lod::scene::LodSceneCulls::None)
			}

			fn scene_with_level(
				&self,
				lod_ref: &lod::lod_ref::LodRef,
				level: lod::scene::LodSceneLevel,
			) -> impl bevy::scene::prelude::Scene + 'static {
				($lod).scene_with_level(self, lod_ref, level)
			}

			fn scene_chunks_with_level(
				&self,
				lod_ref: &lod::lod_ref::LodRef,
				level: lod::scene::LodSceneLevel,
			) -> lod::SceneChunk {
				($lod).scene_chunks(level, lod_ref, self.nest_plant_chunks(lod_ref, level), self)
			}

			fn scene_bounds(&self) -> bevy::math::bounding::Aabb3d {
				self.structural_lod()
					.map(|p| p.footprint_aabb())
					.unwrap_or_else(|| vegetation_components::vegetation_bounds(self))
			}

			fn scene_with_lod(
				&self,
				lod_ref: &lod::lod_ref::LodRef,
			) -> impl bevy::scene::prelude::Scene + 'static {
				lod::lod_host_scene_pending(self.scene_lod_level(lod_ref), self.scene_bounds())
			}
		}
	};
	(@trunks $lod:expr, $level:ident, empty, $this:ident) => {
		($lod).stick_nodes($level, Vec::new())
	};
	(@trunks $lod:expr, $level:ident, trunks, $this:ident) => {
		($lod).stick_nodes($level, $this.proxy_trunks())
	};
	(@low $lod:expr, $level:ident, default_low, $this:ident) => {
		($lod).foliage_nodes(
			$level,
			&$this.canopy_sites(),
			$crate::grove::foliage_low_canopy_balls($this.canopy_sites()),
		)
	};
	(@low $lod:expr, $level:ident, low_nodes, $this:ident) => {
		($lod).foliage_nodes($level, &$this.canopy_sites(), $this.foliage_low_nodes())
	};
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn ordinary_nests_high_medium_only() {
		let lod = WoodyGroveLod::ordinary(2.0, 5.0, 12.0);
		assert_eq!(lod.nest_plant_level(LodSceneLevel::High), Some(LodSceneLevel::High));
		assert_eq!(lod.nest_plant_level(LodSceneLevel::Medium), Some(LodSceneLevel::Medium));
		assert_eq!(lod.nest_plant_level(LodSceneLevel::Low), None);
		assert_eq!(lod.high, 2.0);
		assert_eq!(lod.medium, 5.0);
		assert_eq!(lod.low, 12.0);
		assert!(!lod.rory_trunks);
		assert!(lod.structural_lod(Vec3::ZERO, 1.0).preserve_ultra_low);
	}

	#[test]
	fn keep_low_nests_through_low() {
		let lod = WoodyGroveLod::keep_low_plants(5.0, 20.0, 30.0);
		assert_eq!(lod.nest_plant_level(LodSceneLevel::Low), Some(LodSceneLevel::Low));
		assert_eq!(lod.nest_plant_level(LodSceneLevel::UltraLow), None);
	}

	#[test]
	fn rory_trunk_is_ordinary_plus_trunks() {
		let lod = WoodyGroveLod::rory_trunk(2.0, 5.0, 20.0);
		assert_eq!(lod.policy, WoodyCanopyPolicy::Ordinary);
		assert!(lod.rory_trunks);
		assert_eq!(lod.nest_plant_level(LodSceneLevel::Low), None);
	}

	#[test]
	fn skip_ultralow_can_add_rory_trunks() {
		let lod = WoodyGroveLod::skip_ultralow_bins(5.0, 20.0, 25.0).with_rory_trunks();
		assert_eq!(lod.policy, WoodyCanopyPolicy::SkipUltraLowBins);
		assert!(lod.rory_trunks);
	}
}
