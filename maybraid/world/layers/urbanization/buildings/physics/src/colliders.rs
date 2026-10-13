//! Cuboids from panel / floor / partition placement, triangle prisms, and stair ramps.
//!
//! Spawn queues [`BuildingWalkShapes`] on the host. [`attach_building_walk_colliders`]
//! then stamps **one** Fixed compound child after [`lod::LodSceneHost`] exists, so
//! `spawn_scene` / LOD cull are not racing hundreds of RigidBody children.

use avian3d::prelude::{Collider, Friction, RigidBody};
use bevy::prelude::*;
use bevy_math::Vec3;
use building_components::floors::FloorGeometry;
use building_components::panels::{
	rectangle_kit_hull, right_triangle_kit_hull, tessellated_triangle_kit_hull,
	to_centered_rect_placement, PanelGeometry, PANEL_KIT_MAX, PANEL_KIT_MIN,
};
use building_components::partitions::{
	PartitionGeometry, PartitionTile, PANEL_Y_HALF, SLICE_KIT_HEIGHT,
};
use building_components::placed::Placement;
use building_components::{BuildingComponents, FloorNode, PanelNode, PartitionNode};
use lod::scene::LodSceneLevel;
use lod::LodSceneHost;
use lod_avian::PhysicsInteractionLayer;

use crate::BuildingFrictionConfig;

/// Marks the single Fixed compound spawned from building IR (not a LOD Host volume).
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct BuildingWalkCollider;

/// Queued High-LOD walk shapes; consumed when the host's LOD scene is live.
#[derive(Component, Clone, Debug)]
pub struct BuildingWalkShapes {
	pub shapes: Vec<(Vec3, Quat, Collider)>,
	pub friction: Friction,
}

impl BuildingWalkShapes {
	/// `building`'s High-LOD walk shapes; `None` when it has nothing to stand on.
	pub fn of(building: &impl BuildingComponents, friction: Friction) -> Option<Self> {
		let shapes = walk_shapes(building);
		(!shapes.is_empty()).then_some(Self { shapes, friction })
	}
}

#[derive(Component, Debug, Clone, Copy, Default)]
pub(crate) struct BuildingWalkColliderAttached;

/// Panel GLBs: \(X \in [0, 1]\), \(Y \in [-0.2, 0.2]\), \(Z \in [-1, 0]\) (eave at \(Z = 0\)).
const KIT_MIN: Vec3 = PANEL_KIT_MIN;
const KIT_MAX: Vec3 = PANEL_KIT_MAX;

/// Queue walk colliders on `parent` from High-LOD domain nodes.
pub fn spawn_building_walk_colliders(
	commands: &mut Commands,
	parent: Entity,
	building: &impl BuildingComponents,
	friction: Friction,
) {
	if let Some(shapes) = BuildingWalkShapes::of(building, friction) {
		commands.entity(parent).insert(shapes);
	}
}

/// Stamp one compound child once [`LodSceneHost`] is on the parent.
pub(crate) fn attach_building_walk_colliders(
	mut commands: Commands,
	pending: Query<
		(Entity, &BuildingWalkShapes),
		(With<LodSceneHost>, Without<BuildingWalkColliderAttached>),
	>,
) {
	for (entity, spec) in &pending {
		let Ok(mut host) = commands.get_entity(entity) else {
			continue;
		};
		let shapes = spec.shapes.clone();
		let friction = spec.friction;
		host.insert(BuildingWalkColliderAttached);
		host.remove::<BuildingWalkShapes>();
		if shapes.is_empty() {
			continue;
		}
		commands.spawn((
			Name::new("building-walk-collider"),
			BuildingWalkCollider,
			ChildOf(entity),
			Transform::IDENTITY,
			Visibility::Hidden,
			RigidBody::Static,
			Collider::compound(shapes),
			PhysicsInteractionLayer::fixed_layers(),
			friction,
		));
	}
}

/// Convenience when the app has [`BuildingFrictionConfig`].
impl BuildingFrictionConfig {
	pub fn spawn_walk_colliders(
		self,
		commands: &mut Commands,
		parent: Entity,
		building: &impl BuildingComponents,
	) {
		spawn_building_walk_colliders(commands, parent, building, self.0);
	}
}

struct CuboidPose {
	translation: Vec3,
	rotation: Quat,
	size: Vec3,
}

struct WalkHull {
	translation: Vec3,
	rotation: Quat,
	points: Vec<Vec3>,
}

fn walk_shapes(building: &impl BuildingComponents) -> Vec<(Vec3, Quat, Collider)> {
	let level = LodSceneLevel::High;
	let mut shapes = Vec::new();
	for node in building.panel_nodes_for_level(level).flatten() {
		if let Some(hull) = panel_collider(&node) {
			if let Some(shape) = hull_shape(hull) {
				shapes.push(shape);
			}
		}
	}
	for node in building.floor_nodes_for_level(level).flatten() {
		if let Some(pose) = floor_cuboid(&node) {
			shapes.push(cuboid_shape(pose));
		}
		for (translation, rotation, points) in node.triangle_walk_hulls() {
			if let Some(shape) = hull_shape(WalkHull { translation, rotation, points }) {
				shapes.push(shape);
			}
		}
		for (translation, rotation, points) in node.inscribed_cap_walk_hulls() {
			if let Some(shape) = hull_shape(WalkHull { translation, rotation, points }) {
				shapes.push(shape);
			}
		}
	}
	for node in building.partition_nodes_for_level(level).flatten() {
		if let Some(pose) = partition_cuboid(&node) {
			shapes.push(cuboid_shape(pose));
		}
		shapes.extend(arc_partition_cuboids(&node).into_iter().map(cuboid_shape));
	}
	for node in building.stair_nodes_for_level(level).flatten() {
		for (translation, rotation, size) in node.walk_ramps() {
			shapes.push(cuboid_shape(CuboidPose { translation, rotation, size }));
		}
	}
	shapes
}

fn cuboid_shape(pose: CuboidPose) -> (Vec3, Quat, Collider) {
	let size = pose.size.max(Vec3::splat(0.05));
	(pose.translation, pose.rotation, Collider::cuboid(size.x, size.y, size.z))
}

fn hull_shape(hull: WalkHull) -> Option<(Vec3, Quat, Collider)> {
	if let Some(collider) = Collider::convex_hull(hull.points.clone()) {
		return Some((hull.translation, hull.rotation, collider));
	}
	// Thin / large kits can fail convex hull; keep an AABB so floors still collide.
	let mut min = Vec3::splat(f32::MAX);
	let mut max = Vec3::splat(f32::MIN);
	for p in &hull.points {
		min = min.min(*p);
		max = max.max(*p);
	}
	if !min.is_finite() || !max.is_finite() {
		return None;
	}
	let size = (max - min).max(Vec3::splat(0.05));
	let center = (min + max) * 0.5;
	Some(cuboid_shape(CuboidPose {
		translation: hull.translation + hull.rotation * center,
		rotation: hull.rotation,
		size,
	}))
}

fn oriented_kit_cuboid(placement: Placement, kit_min: Vec3, kit_max: Vec3) -> CuboidPose {
	let size = (kit_max - kit_min).abs().max(Vec3::splat(1e-3));
	let kit_center = (kit_min + kit_max) * 0.5;
	let scaled = kit_center * placement.scale;
	CuboidPose {
		translation: placement.translation + placement.rotation() * scaled,
		rotation: placement.rotation(),
		size: size * placement.scale.abs(),
	}
}

fn panel_collider(node: &PanelNode) -> Option<WalkHull> {
	// Hull points are local to the kit origin so the collider shares the mesh TRS
	// (`pose(placement)` scales the same corners). QuadPanel landings are
	// tessellated triangles, not rectangle leaves.
	match &node.geometry {
		PanelGeometry::Rectangle(_) => Some(WalkHull {
			translation: node.placement.translation,
			rotation: node.placement.rotation(),
			points: rectangle_kit_hull(node.placement.scale),
		}),
		PanelGeometry::RightTriangle(tri) => Some(WalkHull {
			translation: node.placement.translation,
			rotation: node.placement.rotation(),
			points: right_triangle_kit_hull(node.placement.scale, tri.mirror),
		}),
		PanelGeometry::TessellatedTriangle(tri) => Some(WalkHull {
			translation: node.placement.translation,
			rotation: node.placement.rotation(),
			points: tessellated_triangle_kit_hull(tri.a, tri.b, tri.c, node.placement.scale),
		}),
	}
}

fn floor_cuboid(node: &FloorNode) -> Option<CuboidPose> {
	match node.geometry {
		FloorGeometry::Rectangle(_) => {
			let centered = to_centered_rect_placement(node.placement);
			let size = Vec3::new(
				(centered.scale.x * 2.0).abs().max(0.2),
				(centered.scale.y.abs() * PANEL_Y_HALF * 2.0).max(0.08),
				(centered.scale.z * 2.0).abs().max(0.2),
			);
			Some(CuboidPose {
				translation: centered.translation,
				rotation: centered.rotation(),
				size,
			})
		}
		_ => None,
	}
}

fn partition_cuboid(node: &PartitionNode) -> Option<CuboidPose> {
	match node.geometry {
		PartitionGeometry::Linear(_) => Some(oriented_kit_cuboid(node.placement, KIT_MIN, KIT_MAX)),
		_ => None,
	}
}

/// Radial band of the rough-stone arc kits (unit ring; meshes span about 0.86–1.13).
const ARC_KIT_INNER: f32 = 0.87;
const ARC_KIT_OUTER: f32 = 1.12;
/// Chord width. At 15° a chord strays under 1% of the radius from the ring.
const ARC_CHORD_DEGREES: f32 = 15.0;

/// Chord cuboids along each placed arc kit, so ring walls (Wizard's Tower,
/// Ring Fort towers) block like the meshes. Door and window clips are gaps in
/// the solid sweeps, so they stay open.
fn arc_partition_cuboids(node: &PartitionNode) -> Vec<CuboidPose> {
	let height = match node.geometry {
		PartitionGeometry::Arc(_) => 1.0,
		PartitionGeometry::SliceArc(_) => SLICE_KIT_HEIGHT,
		_ => return Vec::new(),
	};
	let mut out = Vec::new();
	for tile in node.geometry.placed_tiles_for_style(node.style, node.placement) {
		let degrees = match tile.geom {
			PartitionTile::Arc180 | PartitionTile::SliceArc180 => 180.0,
			PartitionTile::Arc90 | PartitionTile::SliceArc90 => 90.0,
			PartitionTile::Arc15 | PartitionTile::SliceArc15 => 15.0,
			_ => continue,
		};
		out.extend(arc_kit_chords(tile.placement, degrees, height));
	}
	out
}

/// Kits sit on local +X and sweep toward +Z: kit angle θ is the direction
/// (cos θ, 0, sin θ).
fn arc_kit_chords(placement: Placement, degrees: f32, height: f32) -> Vec<CuboidPose> {
	let chords = (degrees / ARC_CHORD_DEGREES).ceil().max(1.0) as usize;
	let step = degrees.to_radians() / chords as f32;
	let rotation = placement.rotation();
	let scale = placement.scale;
	(0..chords)
		.map(|chord| {
			let mid = (chord as f32 + 0.5) * step;
			let (sin, cos) = mid.sin_cos();
			let radial = Vec3::new(cos, 0.0, sin);
			let tangent = Vec3::new(-sin, 0.0, cos) * scale;
			let length = 2.0 * ARC_KIT_OUTER * (0.5 * step).sin();
			let center =
				radial * (0.5 * (ARC_KIT_INNER + ARC_KIT_OUTER)) + Vec3::Y * (0.5 * height);
			CuboidPose {
				translation: placement.translation + rotation * (center * scale),
				rotation: rotation * Quat::from_rotation_y(f32::atan2(-tangent.z, tangent.x)),
				size: Vec3::new(
					length * tangent.length(),
					height * scale.y.abs(),
					(ARC_KIT_OUTER - ARC_KIT_INNER) * (radial * scale).length(),
				),
			}
		})
		.collect()
}

#[cfg(test)]
mod tests {
	use super::*;
	use building_components::placed::Placement;

	#[test]
	fn kit_cuboid_centers_on_neg_z_panel_space() -> anyhow::Result<()> {
		let pose = oriented_kit_cuboid(Placement::IDENTITY, KIT_MIN, KIT_MAX);
		assert!((pose.translation - Vec3::new(0.5, 0.0, -0.5)).length() < 1e-4);
		assert!((pose.size.x - 1.0).abs() < 1e-4);
		assert!((pose.size.z - 1.0).abs() < 1e-4);
		assert!((pose.size.y - PANEL_Y_HALF * 2.0).abs() < 1e-4);
		Ok(())
	}

	#[test]
	fn right_triangle_hull_keeps_the_origin_and_far_leg() -> anyhow::Result<()> {
		let pts = right_triangle_kit_hull(Vec3::ONE, None);
		assert_eq!(pts.len(), 6);
		assert!(pts.iter().any(|p| (p - Vec3::new(0.0, PANEL_Y_HALF, 0.0)).length() < 1e-4));
		assert!(pts.iter().any(|p| (p - Vec3::new(1.0, -PANEL_Y_HALF, 0.0)).length() < 1e-4));
		assert!(pts.iter().any(|p| (p - Vec3::new(0.0, -PANEL_Y_HALF, -1.0)).length() < 1e-4));
		assert!(Collider::convex_hull(pts).is_some());
		Ok(())
	}

	#[test]
	fn rectangle_panel_hull_matches_kit_and_is_convex() -> anyhow::Result<()> {
		let node = PanelNode::rough_stone(
			PanelGeometry::rectangle(),
			Placement::IDENTITY.with_scale(Vec3::new(4.0, 0.4, 2.0)),
		);
		let Some(hull) = panel_collider(&node) else {
			anyhow::bail!("rectangle panel should stamp a hull");
		};
		assert_eq!(hull.points.len(), 8);
		assert!(hull.translation.length() < 1e-6);
		assert!(Collider::convex_hull(hull.points).is_some());
		Ok(())
	}

	#[test]
	fn tessellated_panel_stamps_a_prism_hull() -> anyhow::Result<()> {
		use bevy_math::Vec2;
		use building_components::panels::TessellatedTriangle;
		let tri = TessellatedTriangle::new(Vec2::ZERO, Vec2::new(2.0, 0.0), Vec2::new(0.5, -1.5));
		let node =
			PanelNode::rough_stone(PanelGeometry::tessellated_triangle(tri), Placement::IDENTITY);
		let Some(hull) = panel_collider(&node) else {
			anyhow::bail!("tessellated panel should stamp a prism");
		};
		assert_eq!(hull.points.len(), 6);
		assert!(hull.points.iter().any(|p| (Vec2::new(p.x, p.z) - tri.c).length() < 1e-4));
		assert!(Collider::convex_hull(hull.points).is_some());
		Ok(())
	}

	#[test]
	fn inscribed_square_cap_stamps_a_convex_walk_hull() -> anyhow::Result<()> {
		use building_components::floors::{Floor, FloorNode};
		let node = FloorNode::rough_stone(
			Floor::circle_inscribed_square(),
			Placement::new(Vec3::ZERO, 0.0).with_scale(Vec3::new(4.0, 0.2, 4.0)),
		);
		let hulls = node.inscribed_cap_walk_hulls();
		assert_eq!(hulls.len(), 1);
		let (translation, _rotation, points) = &hulls[0];
		assert!(translation.length() < 1e-6);
		assert!(points.iter().any(|p| (p.z - 4.0).abs() < 1e-3 && p.x.abs() < 1e-3));
		assert!(Collider::convex_hull(points.clone()).is_some());
		Ok(())
	}

	fn in_cuboid(pose: &CuboidPose, at: Vec3) -> bool {
		let local = pose.rotation.inverse() * (at - pose.translation);
		local.abs().cmple(pose.size * 0.5 + Vec3::splat(1e-3)).all()
	}

	fn ring_node(geometry: PartitionGeometry, yaw: f32) -> PartitionNode {
		PartitionNode::new(
			building_components::partitions::PartitionStyle::RoughStonework,
			geometry,
			Placement::new(Vec3::new(5.0, 2.0, -3.0), yaw).with_scale(Vec3::new(10.0, 4.0, 10.0)),
		)
	}

	#[test]
	fn a_full_ring_wall_blocks_all_the_way_round() -> anyhow::Result<()> {
		let node = ring_node(PartitionGeometry::arc(360.0), 0.3);
		let chords = arc_partition_cuboids(&node);
		assert!(!chords.is_empty());
		for step in 0..72 {
			let angle = (step as f32 * 5.0).to_radians();
			let at = Vec3::new(5.0, 4.0, -3.0) + Vec3::new(angle.cos(), 0.0, angle.sin()) * 10.0;
			assert!(chords.iter().any(|pose| in_cuboid(pose, at)), "{at} is open");
		}
		let inside = Vec3::new(5.0, 4.0, -3.0) + Vec3::X * 7.0;
		assert!(chords.iter().all(|pose| !in_cuboid(pose, inside)), "the floor stays walkable");
		Ok(())
	}

	#[test]
	fn an_arc_collides_where_its_kit_sweeps() -> anyhow::Result<()> {
		use building_components::arc_ring_dir;
		let yaw = 1.0;
		let chords = arc_partition_cuboids(&ring_node(PartitionGeometry::arc(15.0), yaw));
		let at = |yaw: f32| {
			let dir = arc_ring_dir(yaw);
			Vec3::new(5.0 + 10.0 * dir.x, 3.0, -3.0 + 10.0 * dir.y)
		};
		let start = (Quat::from_rotation_y(yaw) * Vec3::X).xz();
		assert!((start - arc_ring_dir(yaw)).length() < 1e-4, "yaw places the kit start");
		assert!(chords.iter().any(|pose| in_cuboid(pose, at(yaw - 7.5_f32.to_radians()))));
		assert!(chords.iter().all(|pose| !in_cuboid(pose, at(yaw + 7.5_f32.to_radians()))));
		let slice = arc_partition_cuboids(&ring_node(PartitionGeometry::slice_arc(15.0), yaw));
		assert!(slice.iter().all(|pose| (pose.size.y - SLICE_KIT_HEIGHT * 4.0).abs() < 1e-4));
		Ok(())
	}

	#[test]
	fn attach_spawns_one_compound_after_lod_host_exists() {
		use crate::BUILDING_FRICTION;
		let mut app = App::new();
		app.add_systems(Update, attach_building_walk_colliders);
		let host = app
			.world_mut()
			.spawn((
				LodSceneHost,
				BuildingWalkShapes {
					shapes: vec![(Vec3::ZERO, Quat::IDENTITY, Collider::cuboid(1.0, 1.0, 1.0))],
					friction: BUILDING_FRICTION,
				},
			))
			.id();
		app.update();
		let world = app.world_mut();
		assert_eq!(world.query::<&BuildingWalkCollider>().iter(world).count(), 1);
		assert!(world.get::<BuildingWalkShapes>(host).is_none());
	}
}
