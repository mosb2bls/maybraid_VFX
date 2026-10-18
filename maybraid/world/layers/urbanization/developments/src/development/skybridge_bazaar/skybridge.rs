//! [`Skybridge`]: an enclosed lane between two bazaar towers.

use bevy_math::bounding::{Aabb2d, Aabb3d};
use bevy_math::Vec2;
use building_components::joints::JointNode;
use building_components::panels::PanelNode;
use building_components::{
	building_bounds, BuildingComponents, BuildingStructuralLodProbe, Layers,
};
use buildings::{CardinalFace, ConnectingHall, MappedOpening};
use lod::scene::LodSceneLevel;
use material_ref::MaterialRef;

use crate::BuildingFootprint;

/// Enclosed elevated lane joining two bazaar towers.
#[derive(Debug, Clone, PartialEq)]
pub struct Skybridge {
	pub bounds: Aabb3d,
	pub hall: ConnectingHall,
	pub storey: usize,
	pub directions: [CardinalFace; 2],
	material: Option<MaterialRef>,
}

impl Skybridge {
	pub fn new(hall: ConnectingHall, storey: usize, directions: [CardinalFace; 2]) -> Self {
		let bounds = building_bounds(&hall);
		Self { bounds, hall, storey, directions, material: None }
	}

	pub fn with_material(mut self, material: MaterialRef) -> Self {
		self.material = Some(material);
		self
	}

	pub fn endpoints(&self) -> (MappedOpening, MappedOpening) {
		self.hall.endpoints()
	}

	pub fn material(&self) -> Option<&MaterialRef> {
		self.material.as_ref()
	}
}

impl BuildingFootprint for Skybridge {
	fn footprint_rects(&self) -> Vec<Aabb2d> {
		vec![Aabb2d {
			min: Vec2::new(self.bounds.min.x, self.bounds.min.z),
			max: Vec2::new(self.bounds.max.x, self.bounds.max.z),
		}]
	}
}

impl BuildingComponents for Skybridge {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		let panels = self.hall.panel_nodes_for_level(level);
		match &self.material {
			Some(material) => panels.with_material(material.clone()),
			None => panels,
		}
	}

	fn joint_nodes_for_level(&self, level: LodSceneLevel) -> Layers<JointNode> {
		self.hall.joint_nodes_for_level(level)
	}

	fn structural_lod(&self) -> Option<BuildingStructuralLodProbe> {
		let mut probe = BuildingStructuralLodProbe::from_aabb3d_xz(
			bevy_math::Vec3::from(self.bounds.min),
			bevy_math::Vec3::from(self.bounds.max),
		);
		if let Some(material) = &self.material {
			for volume in &mut probe.volumes {
				volume.material = Some(material.clone());
			}
		}
		Some(probe)
	}
}
#[cfg(test)]
mod tests {
	use super::*;
	use bevy_math::{Vec2, Vec3};
	use building_components::BuildingComponents;
	use buildings::{MappedOpening, MappedOpeningQuad};

	#[test]
	fn skybridge_uses_connecting_hall_geometry_without_floor_nodes() {
		let a = MappedOpening::new(
			MappedOpeningQuad::new(
				Vec3::new(-8.0, 20.0, -1.5),
				Vec3::new(-8.0, 20.0, 1.5),
				Vec3::new(-8.0, 23.0, -1.5),
				Vec3::new(-8.0, 23.0, 1.5),
			),
			Vec2::X,
		);
		let b = MappedOpening::new(
			MappedOpeningQuad::new(
				Vec3::new(8.0, 20.0, 1.5),
				Vec3::new(8.0, 20.0, -1.5),
				Vec3::new(8.0, 23.0, 1.5),
				Vec3::new(8.0, 23.0, -1.5),
			),
			-Vec2::X,
		);
		let bridge = Skybridge::new(
			ConnectingHall::rough_stone(a, b),
			6,
			[CardinalFace::East, CardinalFace::West],
		);
		assert!(!bridge.panel_nodes_for_level(LodSceneLevel::High).is_empty());
		assert!(bridge.floor_nodes_for_level(LodSceneLevel::High).is_empty());
		assert_eq!(bridge.endpoints(), (a, b));
		assert_eq!(bridge.storey, 6);
	}
}
