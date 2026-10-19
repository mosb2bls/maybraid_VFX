//! Shape intersection against target colliders. Point-in-volume stays in
//! [`crate::hazard`] for telegraphs and unit tests.

use avian3d::prelude::{Collider, ShapeCastConfig, SpatialQuery, SpatialQueryFilter};
use bevy::prelude::*;
use damage::Health;
use lod_avian::PhysicsInteractionLayer;

use crate::{
	aim::owner_point, HazardPose, HazardShape, HazardSpec, HitCandidate, WavefrontSpec,
	WavefrontSweep,
};

pub fn overlap_filter(owner: Entity) -> SpatialQueryFilter {
	PhysicsInteractionLayer::Animated.query_filter().with_excluded_entities([owner])
}

pub fn query_shape(shape: HazardShape, pose: HazardPose) -> (Collider, Vec3, Quat) {
	match shape {
		HazardShape::Sphere { radius } => (Collider::sphere(radius.max(0.05)), pose.origin, pose.rotation),
		HazardShape::Cylinder { radius, height } => (
			Collider::cylinder(radius.max(0.05), height.max(0.05)),
			pose.origin + pose.rotation * Vec3::Y * (height.max(0.05) * 0.5),
			pose.rotation,
		),
		HazardShape::Box { half_extents } => (
			Collider::cuboid(
				half_extents.x.max(0.05) * 2.0,
				half_extents.y.max(0.05) * 2.0,
				half_extents.z.max(0.05) * 2.0,
			),
			pose.origin,
			pose.rotation,
		),
		HazardShape::Line { length, half_width } => (
			Collider::cuboid(
				half_width.max(0.05) * 2.0,
				half_width.max(0.05) * 2.0,
				length.max(0.05),
			),
			pose.origin + pose.rotation * Vec3::Z * (length.max(0.05) * 0.5),
			pose.rotation,
		),
		HazardShape::Wavefront(wave) => wavefront_collider(wave, pose),
	}
}

fn wavefront_collider(wave: WavefrontSpec, pose: HazardPose) -> (Collider, Vec3, Quat) {
	(
		Collider::cylinder(wave.radius.max(0.05), wave.half_depth.max(0.05) * 2.0),
		pose.origin,
		pose.rotation * Quat::from_rotation_arc(Vec3::Y, Vec3::Z),
	)
}

#[allow(clippy::too_many_arguments)]
pub fn gather_overlaps(
	spatial: &SpatialQuery,
	spec: HazardSpec,
	pose: HazardPose,
	sweep: Option<WavefrontSweep>,
	owner: Entity,
	health: &Query<(), With<Health>>,
	parents: &Query<&ChildOf>,
	transforms: &Query<(&Transform, Option<&GlobalTransform>)>,
) -> Vec<HitCandidate> {
	let filter = overlap_filter(owner);
	let mut entities = spatial_entities(spatial, spec.shape, pose, sweep, &filter);
	entities.sort();
	entities.dedup();
	entities
		.into_iter()
		.filter_map(|entity| {
			let parent = parents.get(entity).ok().map(ChildOf::parent);
			let has_health = health.contains(entity);
			let parent_has_health = parent.is_some_and(|parent| health.contains(parent));
			if !has_health && !parent_has_health {
				return None;
			}
			let point = transforms
				.get(entity)
				.ok()
				.map(|(transform, global)| owner_point(transform, global))
				.or_else(|| {
					parent.and_then(|parent| {
						transforms
							.get(parent)
							.ok()
							.map(|(transform, global)| owner_point(transform, global))
					})
				})?;
			Some(HitCandidate {
				entity,
				parent,
				has_health,
				parent_has_health,
				point,
				confirmed: true,
			})
		})
		.collect()
}

fn spatial_entities(
	spatial: &SpatialQuery,
	shape: HazardShape,
	pose: HazardPose,
	sweep: Option<WavefrontSweep>,
	filter: &SpatialQueryFilter,
) -> Vec<Entity> {
	if let (HazardShape::Wavefront(wave), Some(sweep)) = (shape, sweep) {
		return wavefront_entities(spatial, wave, sweep, filter);
	}
	let (collider, origin, rotation) = query_shape(shape, pose);
	spatial.shape_intersections(&collider, origin, rotation, filter)
}

fn wavefront_entities(
	spatial: &SpatialQuery,
	wave: WavefrontSpec,
	sweep: WavefrontSweep,
	filter: &SpatialQueryFilter,
) -> Vec<Entity> {
	let pose = wave.pose(sweep.emission, sweep.heading, sweep.to);
	let (collider, origin, rotation) = wavefront_collider(wave, pose);
	let mut entities = spatial.shape_intersections(&collider, origin, rotation, filter);
	let start = wave.pose(sweep.emission, sweep.heading, sweep.from);
	let (start_collider, start_origin, start_rotation) = wavefront_collider(wave, start);
	entities.extend(spatial.shape_intersections(&start_collider, start_origin, start_rotation, filter));
	let travel = sweep.to - sweep.from;
	if travel > 1e-3 {
		if let Ok(dir) = Dir3::new(sweep.heading) {
			spatial.shape_hits_callback(
				&collider,
				sweep.emission + sweep.heading * sweep.from,
				rotation,
				dir,
				&ShapeCastConfig::from_max_distance(travel),
				filter,
				|hit| {
					entities.push(hit.entity);
					true
				},
			);
		}
	}
	entities
}
