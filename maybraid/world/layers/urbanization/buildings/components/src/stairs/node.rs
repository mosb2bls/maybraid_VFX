//! Stair IR node: style + geometry + placement — fine-phase [`LodScene`] host.

use bevy::math::bounding::Aabb3d;
use bevy::prelude::Component;
use bevy::scene::prelude::Scene;
use bevy_math::{Quat, Vec2, Vec3};
use lod::lod_ref::LodRef;
use lod::scene::{LodScene, LodSceneCulls, LodSceneLevel, LodSceneStatus};
use lod::SceneChunk;

use crate::assets::stairs::rough_stonework::TREAD;
use crate::kit_merge::with_optional_material;
use crate::layer::Layers;
use crate::lod_band::placement_bounds;
use crate::parent_confines::{confined_scene, ParentConfines};
use crate::placed::Placement;
use crate::scene_children::{pose, posed_glb, scene_children, with_pose};
use crate::stairs::geometry::StairGeometry;
use crate::stairs::style::StairStyle;
use crate::stairs::tessellate::StairKit;
use crate::stairs::{RoughStoneSpiralStair, RoughStoneStraightStair, WoodStraightStair};
use material_ref::MaterialRef;

/// Authoring IR for a stair feature.
///
/// [`Self::style`] picks the kit GLB path. [`Self::material`] is an optional
/// shader look ([`MaterialRef`]) stamped onto that kit after spawn.
#[derive(Debug, Clone, PartialEq, Component, Default)]
pub struct StairNode {
	pub style: StairStyle,
	pub geometry: StairGeometry,
	pub placement: Placement,
	/// External silhouette vs internal detail gating.
	pub confines: ParentConfines,
	pub material: Option<MaterialRef>,
}

impl StairNode {
	pub fn new(style: StairStyle, geometry: StairGeometry, placement: Placement) -> Self {
		Self { style, geometry, placement, confines: ParentConfines::External, material: None }
	}

	pub fn rough_stone(geometry: StairGeometry, placement: Placement) -> Self {
		Self::new(StairStyle::RoughStonework, geometry, placement)
	}

	pub fn wood(geometry: StairGeometry, placement: Placement) -> Self {
		Self::new(StairStyle::Wood, geometry, placement)
	}

	pub fn with_material(mut self, material: MaterialRef) -> Self {
		self.material = Some(material);
		self
	}

	pub fn with_confines(mut self, confines: ParentConfines) -> Self {
		self.confines = confines;
		self
	}

	/// Oriented cuboids for each walkable tread (center, rotation, full size).
	pub fn tread_cuboids(&self) -> Vec<(Vec3, Quat, Vec3)> {
		self.geometry
			.placed_kits(self.placement)
			.into_iter()
			.map(|piece| {
				let p = piece.placement;
				let size = Vec3::new(
					(2.0 * p.scale.x).abs().max(0.05),
					(2.0 * p.scale.y).abs().max(0.05),
					(2.0 * p.scale.z).abs().max(0.05),
				);
				(p.translation, p.rotation(), size)
			})
			.collect()
	}

	/// Walk colliders: one sloped slab per straight flight, short ramps between spiral treads.
	pub fn walk_ramps(&self) -> Vec<(Vec3, Quat, Vec3)> {
		match &self.geometry {
			StairGeometry::Straight(g) => vec![straight_ramp(self.placement, g)],
			StairGeometry::Spiral(_) => spiral_ramps(&self.tread_cuboids()),
		}
	}
}

const RAMP_THICKNESS: f32 = 0.14;

fn straight_ramp(
	placement: Placement,
	g: &crate::stairs::geometry::StraightStair,
) -> (Vec3, Quat, Vec3) {
	let going = g.going_per_tread();
	let tops = g.effective_tread_tops();
	let y1 = tops.last().copied().unwrap_or(g.height);
	let x0 = -0.5 * going;
	let x1 = g.length - 0.5 * going;
	// Meet the supporting floor at the first tread's trailing edge. Connecting
	// the tread tops directly leaves a vertical first-riser collision face.
	ramp_from_local_run(placement, Vec3::new(x0, 0.0, 0.0), Vec3::new(x1, y1, 0.0), g.width)
}

fn spiral_ramps(treads: &[(Vec3, Quat, Vec3)]) -> Vec<(Vec3, Quat, Vec3)> {
	let mut ramps = Vec::new();
	for pair in treads.windows(2) {
		let (a, _, sa) = pair[0];
		let (b, _, sb) = pair[1];
		let width = sa.z.max(sb.z).max(0.2);
		let start = Vec3::new(a.x, a.y + sa.y * 0.5, a.z);
		let end = Vec3::new(b.x, b.y + sb.y * 0.5, b.z);
		ramps.push(ramp_from_world_run(start, end, width));
	}
	if ramps.is_empty() {
		return treads.to_vec();
	}
	ramps
}

fn ramp_from_local_run(
	placement: Placement,
	start: Vec3,
	end: Vec3,
	width: f32,
) -> (Vec3, Quat, Vec3) {
	let delta = end - start;
	let horiz = Vec2::new(delta.x, delta.z).length().max(1e-4);
	let theta = delta.y.atan2(horiz);
	let tilt = Quat::from_rotation_z(theta);
	let rotation = placement.rotation() * tilt;
	let mid = (start + end) * 0.5;
	let diag = delta.length().max(0.1);
	let translation = placement.translation + placement.rotation() * mid
		- rotation * Vec3::Y * (RAMP_THICKNESS * 0.5);
	(translation, rotation, Vec3::new(diag, RAMP_THICKNESS, width.max(0.2)))
}

fn ramp_from_world_run(start: Vec3, end: Vec3, width: f32) -> (Vec3, Quat, Vec3) {
	let delta = end - start;
	let horiz = Vec3::new(delta.x, 0.0, delta.z);
	let yaw = if horiz.length_squared() < 1e-8 {
		Quat::IDENTITY
	} else {
		Quat::from_rotation_arc(Vec3::X, horiz.normalize())
	};
	let theta = delta.y.atan2(horiz.length().max(1e-4));
	let rotation = yaw * Quat::from_rotation_z(theta);
	let mid = (start + end) * 0.5;
	let diag = delta.length().max(0.1);
	let translation = mid - rotation * Vec3::Y * (RAMP_THICKNESS * 0.5);
	(translation, rotation, Vec3::new(diag, RAMP_THICKNESS, width.max(0.2)))
}

impl LodScene for StairNode {
	fn scene_lod_status(&self, _lod_ref: &LodRef) -> LodSceneStatus {
		LodSceneStatus::Unchanged
	}

	fn scene_lod_culls(&self, _lod_ref: &LodRef, _current: LodSceneLevel) -> LodSceneCulls {
		LodSceneCulls::None
	}

	fn scene_with_level(&self, lod_ref: &LodRef, level: LodSceneLevel) -> impl Scene + 'static {
		let children: Vec<Box<dyn Scene>> = self
			.geometry
			.placed_kits(self.placement)
			.into_iter()
			.map(|piece| {
				let transform = pose(piece.placement);
				match self.style {
					StairStyle::RoughStonework => match piece.geom {
						StairKit::Tread => Box::new(posed_glb(TREAD, transform)) as Box<dyn Scene>,
						StairKit::Spiral => Box::new(with_pose(
							transform,
							RoughStoneSpiralStair.scene_with_level(lod_ref, level),
						)) as Box<dyn Scene>,
						StairKit::Straight => Box::new(with_pose(
							transform,
							RoughStoneStraightStair.scene_with_level(lod_ref, level),
						)) as Box<dyn Scene>,
					},
					StairStyle::Wood => {
						let child: Box<dyn Scene> = match piece.geom {
							StairKit::Tread | StairKit::Spiral => {
								Box::new(RoughStoneSpiralStair.scene_with_level(lod_ref, level))
							}
							StairKit::Straight => {
								Box::new(WoodStraightStair.scene_with_level(lod_ref, level))
							}
						};
						Box::new(with_pose(transform, child)) as Box<dyn Scene>
					}
				}
			})
			.collect();
		with_optional_material(
			confined_scene(self.confines, scene_children(children)),
			self.material.clone(),
		)
	}

	fn scene_chunks_with_level(&self, lod_ref: &LodRef, level: LodSceneLevel) -> SceneChunk {
		SceneChunk::primitive(self.scene_with_level(lod_ref, level))
	}

	fn scene_bounds(&self) -> Aabb3d {
		placement_bounds(&self.placement)
	}
}

impl Layers<StairNode> {
	/// Stamp a shader look onto every stair, leaving kit [`StairStyle`] unchanged.
	pub fn with_material(self, material: MaterialRef) -> Self {
		self.map(|node| node.with_material(material.clone()))
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn with_material_stamps_ref_without_changing_style() {
		let node = StairNode::rough_stone(
			StairGeometry::straight_run(1.8, 3.6, 0.8, 0.36),
			Placement::new(Vec3::ZERO, 0.0),
		)
		.with_material(MaterialRef::named("stucco"));
		assert_eq!(node.style, StairStyle::RoughStonework);
		assert_eq!(
			node.material.as_ref().map(|m| &m.name),
			Some(&material_ref::MaterialId::named("stucco"))
		);
	}

	#[test]
	fn straight_ramp_is_one_slab_along_the_flight() -> anyhow::Result<()> {
		let node = StairNode::rough_stone(
			StairGeometry::straight_run(1.8, 3.6, 0.8, 0.36),
			Placement::new(Vec3::ZERO, 0.0),
		);
		let ramps = node.walk_ramps();
		assert_eq!(ramps.len(), 1);
		let (center, rot, size) = ramps[0];
		assert!(size.x > 3.0, "diagonal should cover the flight, got {}", size.x);
		assert!((size.y - RAMP_THICKNESS).abs() < 1e-4);
		let up = rot * Vec3::Y;
		assert!(up.y > 0.5, "ramp normal should point mostly up, got {up}");
		assert!(center.y > 0.2 && center.y < 1.6, "ramp mid height {}", center.y);
		Ok(())
	}

	#[test]
	fn straight_ramp_low_top_edge_meets_the_floor() -> anyhow::Result<()> {
		let geometry = StairGeometry::straight_run(1.8, 3.6, 0.8, 0.36);
		let StairGeometry::Straight(stair) = &geometry else {
			anyhow::bail!("expected straight stair");
		};
		let going = stair.going_per_tread();
		let node = StairNode::rough_stone(geometry, Placement::new(Vec3::ZERO, 0.0));
		let (center, rotation, size) = node.walk_ramps()[0];
		let low_top = center + rotation * Vec3::new(-size.x * 0.5, size.y * 0.5, 0.0);
		assert!(low_top.y.abs() < 1e-4, "low ramp edge should meet floor: {low_top}");
		assert!(
			(low_top.x + going * 0.5).abs() < 1e-4,
			"low ramp edge should meet first tread trailing edge: {low_top}"
		);
		Ok(())
	}
}
