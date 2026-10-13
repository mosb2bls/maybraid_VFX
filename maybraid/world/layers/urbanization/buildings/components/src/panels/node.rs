//! Panel IR node: style + geometry + placement — fine-phase [`LodScene`] host.

use bevy::math::bounding::Aabb3d;
use bevy::prelude::Component;
use bevy::scene::prelude::Scene;
use lod::lod_ref::LodRef;
use lod::scene::{LodScene, LodSceneCulls, LodSceneLevel, LodSceneStatus};
use lod::SceneChunk;
use material_ref::MaterialRef;

use crate::kit_merge::{scenes_from_kit_parts, KitPart};
use crate::layer::Layers;
use crate::lod_band::{placement_bounds, warm_mesh_lod_culls};
use crate::panels::geometry::{PanelGeometry, Rectangle, RightTriangle};
use crate::panels::lod::{
	panel_kit_scene_ref, PANEL_ULTRA_LOW_RECTANGLE, PANEL_ULTRA_LOW_RIGHT_TRIANGLE,
};
use crate::panels::style::PanelStyle;
use crate::parent_confines::ParentConfines;
use crate::placed::Placement;
use crate::scene_children::{pose, scene_children};

/// Authoring IR for a shared panel feature (rectangle / triangle tessellation).
///
/// [`Self::style`] picks the kit GLB path. [`Self::material`] is an optional
/// shader look ([`MaterialRef`]) stamped onto that kit after spawn.
#[derive(Debug, Clone, PartialEq, Component, Default)]
pub struct PanelNode {
	pub style: PanelStyle,
	pub geometry: PanelGeometry,
	pub placement: Placement,
	pub material: Option<MaterialRef>,
}

impl PanelNode {
	pub fn new(style: PanelStyle, geometry: PanelGeometry, placement: Placement) -> Self {
		Self { style, geometry, placement, material: None }
	}

	pub fn with_material(mut self, material: MaterialRef) -> Self {
		self.material = Some(material);
		self
	}

	pub fn rough_stone(geometry: PanelGeometry, placement: Placement) -> Self {
		Self::new(PanelStyle::RoughStonework, geometry, placement)
	}

	pub fn shepherds_thatch(geometry: PanelGeometry, placement: Placement) -> Self {
		Self::new(PanelStyle::ShepherdsThatch, geometry, placement)
	}

	fn probe(&self) -> crate::panels::lod::PanelLodProbe {
		crate::panels::lod::PanelLodProbe::from_placement(&self.placement)
	}

	pub(crate) fn kit_parts(&self, level: LodSceneLevel) -> Vec<KitPart> {
		self.geometry
			.flatten(self.style.kit_caps())
			.into_iter()
			.filter_map(|piece| {
				let transform = pose(self.placement) * pose(piece.placement);
				let scene = match piece.geom {
					PanelGeometry::Rectangle(Rectangle) => {
						let (high, mid, low) = self.style.rectangle_lod()?;
						Some(panel_kit_scene_ref(
							high.scene_ref(),
							mid.scene_ref(),
							low.scene_ref(),
							PANEL_ULTRA_LOW_RECTANGLE.scene_ref(),
							level,
						))
					}
					PanelGeometry::RightTriangle(RightTriangle { mirror }) => {
						let (high, mid, low) = self.style.right_triangle_lod()?;
						Some(panel_kit_scene_ref(
							high.scene_ref().with_mirror(mirror),
							mid.scene_ref().with_mirror(mirror),
							low.scene_ref().with_mirror(mirror),
							PANEL_ULTRA_LOW_RIGHT_TRIANGLE.scene_ref().with_mirror(mirror),
							level,
						))
					}
					_ => None,
				}?;
				Some(KitPart {
					scene,
					transform,
					material: self.material.clone(),
					confines: ParentConfines::External,
				})
			})
			.collect()
	}

	fn content_for_level(&self, level: LodSceneLevel) -> impl Scene + 'static {
		scene_children(scenes_from_kit_parts(self.kit_parts(level)))
	}
}

impl Layers<PanelNode> {
	/// Stamp a shader look onto every panel, leaving kit [`PanelStyle`] unchanged.
	pub fn with_material(self, material: MaterialRef) -> Self {
		self.map(|node| node.with_material(material.clone()))
	}
}

impl LodScene for PanelNode {
	fn scene_lod_level(&self, lod_ref: &LodRef) -> LodSceneLevel {
		self.probe().level_for(lod_ref.current_transform)
	}

	fn scene_lod_status(&self, lod_ref: &LodRef) -> LodSceneStatus {
		self.probe().status_for_lod_ref(lod_ref)
	}

	fn scene_lod_culls(&self, _lod_ref: &LodRef, current: LodSceneLevel) -> LodSceneCulls {
		warm_mesh_lod_culls(current)
	}

	fn scene_with_level(&self, _lod_ref: &LodRef, level: LodSceneLevel) -> impl Scene + 'static {
		self.content_for_level(level)
	}

	fn scene_chunks_with_level(&self, lod_ref: &LodRef, level: LodSceneLevel) -> SceneChunk {
		SceneChunk::primitive(self.scene_with_level(lod_ref, level))
	}

	fn scene_bounds(&self) -> Aabb3d {
		placement_bounds(&self.placement)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::panels::geometry::PanelGeometry;

	#[test]
	fn with_material_stamps_ref_without_changing_style() {
		let node =
			PanelNode::rough_stone(PanelGeometry::Rectangle(Rectangle), Placement::default())
				.with_material(MaterialRef::named("stucco"));
		assert_eq!(node.style, PanelStyle::RoughStonework);
		assert_eq!(
			node.material.as_ref().map(|m| &m.name),
			Some(&material_ref::MaterialId::named("stucco"))
		);
	}
}
