//! Old City Market stall tiers and the stone terraces stalls cluster on.

use bevy_math::bounding::{Aabb2d, Aabb3d};
use bevy_math::{Vec2, Vec3};
use building_components::panels::{PanelNode, PanelStyle};
use building_components::{BuildingComponents, JointNode, Layers};
use buildings::{Openings, RectFloor, RectFloorParams, RectFloorSlab};
use lod::scene::LodSceneLevel;

use crate::BuildingFootprint;

pub const MARKET_PLATFORM_HEIGHT: f32 = 0.35;

/// Stall-count tier assigned from graph topology.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OldCityMarketTier {
	Dense,
	Medium,
	Sparse,
}

/// One shared stone platform underneath a cluster of market buildings.
#[derive(Debug, Clone, PartialEq)]
pub struct OldCityMarketTerrace {
	pub bounds: Aabb3d,
	shell: RectFloor,
}

impl OldCityMarketTerrace {
	pub fn new(center: Vec2, footprint: Vec2, elevation: f32) -> Self {
		let bounds = Aabb3d::from_min_max(
			Vec3::new(center.x - footprint.x * 0.5, elevation, center.y - footprint.y * 0.5),
			Vec3::new(
				center.x + footprint.x * 0.5,
				elevation + MARKET_PLATFORM_HEIGHT,
				center.y + footprint.y * 0.5,
			),
		);
		let shell = RectFloorParams::new(
			Vec3::new(center.x, elevation, center.y),
			footprint,
			MARKET_PLATFORM_HEIGHT,
		)
		.floor(RectFloorSlab::Solid)
		.ceiling(RectFloorSlab::Solid)
		.style(PanelStyle::RoughStonework)
		.openings(Openings::new())
		.build();
		Self { bounds, shell }
	}
}

impl BuildingFootprint for OldCityMarketTerrace {
	fn footprint_rects(&self) -> Vec<Aabb2d> {
		vec![Aabb2d {
			min: Vec2::new(self.bounds.min.x, self.bounds.min.z),
			max: Vec2::new(self.bounds.max.x, self.bounds.max.z),
		}]
	}
}

impl BuildingComponents for OldCityMarketTerrace {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		self.shell.panel_nodes_for_level(level)
	}

	fn joint_nodes_for_level(&self, level: LodSceneLevel) -> Layers<JointNode> {
		self.shell.joint_nodes_for_level(level)
	}
}
#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn terrace_owns_a_renderable_shared_footprint() -> anyhow::Result<()> {
		let terrace = OldCityMarketTerrace::new(Vec2::new(12.0, 18.0), Vec2::new(30.0, 24.0), 7.0);
		let footprint = terrace
			.footprint_rects()
			.into_iter()
			.next()
			.ok_or_else(|| anyhow::anyhow!("terrace should have a footprint"))?;
		assert_eq!(footprint.min, Vec2::new(-3.0, 6.0));
		assert_eq!(footprint.max, Vec2::new(27.0, 30.0));
		assert!(!terrace.panel_nodes_for_level(LodSceneLevel::High).is_empty());
		Ok(())
	}
}
