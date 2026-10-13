//! External vs internal geometry gating relative to a parent volume.
//!
//! Used as an IR field on building-component nodes (`FloorNode`, `PartitionNode`, …).
//! Prefer **floor-wise / room-wise** compartments so a simple ball works. Use
//! [`InternalShape::Capsule`] only for long non-compartmentalized regions
//! (e.g. a continuous vertical spire).
//!
//! Internal volumes stay hidden until the viewer is within
//! [`INTERNAL_REVEAL_FACTOR`] × the confine radius (distance to ball center or
//! capsule segment), then child mesh LOD uses normal external banding.

use bevy::math::Vec3;
use bevy::prelude::{Children, Component, Transform, Visibility};
use bevy::scene::prelude::{bsn, template_value, Scene};

/// Reveal internal confines when viewer distance ≤ this × confine radius.
pub const INTERNAL_REVEAL_FACTOR: f32 = 5.0;

/// Geometry of an [`ParentConfines::Internal`] volume.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InternalShape {
	/// Floor- or room-compartment ball. Prefer authoring at this grain.
	Ball { center: Vec3, radius: f32 },
	/// Segment + radius for tall / long open volumes that should not be split
	/// into floor balls. Distance is to the medial segment `a`→`b`.
	Capsule { a: Vec3, b: Vec3, radius: f32 },
}

impl InternalShape {
	pub fn viewer_allowed(self, viewer: &Transform) -> bool {
		let p = viewer.translation;
		match self {
			Self::Ball { center, radius } => p.distance(center) <= radius * INTERNAL_REVEAL_FACTOR,
			Self::Capsule { a, b, radius } => {
				distance_to_segment(p, a, b) <= radius * INTERNAL_REVEAL_FACTOR
			}
		}
	}
}

/// Whether a node is part of the external silhouette or internal detail.
#[derive(Debug, Clone, Copy, PartialEq, Component, Default)]
pub enum ParentConfines {
	/// Liberal footprint / extent banding — typically always a LOD candidate.
	#[default]
	External,
	/// Hidden until within [`INTERNAL_REVEAL_FACTOR`] × the shape radius.
	/// Author per floor / room compartment — not the whole building.
	Internal(InternalShape),
}

impl ParentConfines {
	pub fn internal(center: Vec3, radius: f32) -> Self {
		Self::Internal(InternalShape::Ball { center, radius: radius.max(1e-4) })
	}

	pub fn capsule(a: Vec3, b: Vec3, radius: f32) -> Self {
		Self::Internal(InternalShape::Capsule { a, b, radius: radius.max(1e-4) })
	}

	/// Whether the viewer may activate this node's detail for this confine.
	pub fn viewer_allowed(&self, viewer: &Transform) -> bool {
		match self {
			Self::External => true,
			Self::Internal(shape) => shape.viewer_allowed(viewer),
		}
	}
}

/// Closest distance from `p` to the line segment `a`→`b`.
pub fn distance_to_segment(p: Vec3, a: Vec3, b: Vec3) -> f32 {
	let ab = b - a;
	let len_sq = ab.length_squared();
	if len_sq < 1e-12 {
		return p.distance(a);
	}
	let t = ((p - a).dot(ab) / len_sq).clamp(0.0, 1.0);
	p.distance(a + ab * t)
}

/// Wrap `content` with a [`ParentConfines`] root so fine-phase visibility can gate it.
pub fn confined_scene(
	confines: ParentConfines,
	content: impl Scene + 'static,
) -> impl Scene + 'static {
	let children: Vec<Box<dyn Scene>> = vec![Box::new(content)];
	bsn! {
		template_value(confines)
		Transform::default()
		Visibility::Inherited
		Children [ {children} ]
	}
}

/// Fine-phase: hide entities whose [`ParentConfines`] reject the camera.
pub fn apply_parent_confines(
	viewer: bevy::prelude::Query<&Transform, bevy::prelude::With<bevy::prelude::Camera3d>>,
	mut hosts: bevy::prelude::Query<(&ParentConfines, &mut Visibility)>,
) {
	let Ok(viewer_tf) = viewer.single() else {
		return;
	};
	let t0 = std::time::Instant::now();
	let mut n = 0u32;
	for (confines, mut visibility) in &mut hosts {
		n += 1;
		*visibility = if confines.viewer_allowed(viewer_tf) {
			Visibility::Inherited
		} else {
			Visibility::Hidden
		};
	}
	let elapsed_ms = t0.elapsed().as_secs_f64() * 1000.0;
	if elapsed_ms >= 0.5 {
		bevy::log::info!("[lod.confines] apply_parent_confines: hosts={n} in {elapsed_ms:.2}ms");
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::prelude::Transform;

	#[test]
	fn internal_confine_reveals_at_five_x_parent_ball() -> anyhow::Result<()> {
		let c = ParentConfines::internal(Vec3::ZERO, 5.0);
		// Parent ball R=5 → reveal out to 25.
		assert!(c.viewer_allowed(&Transform::from_xyz(24.0, 0.0, 0.0)));
		assert!(!c.viewer_allowed(&Transform::from_xyz(26.0, 0.0, 0.0)));
		assert!(ParentConfines::External.viewer_allowed(&Transform::from_xyz(100.0, 0.0, 0.0)));
		Ok(())
	}

	#[test]
	fn capsule_uses_distance_to_segment() -> anyhow::Result<()> {
		let c = ParentConfines::capsule(Vec3::ZERO, Vec3::Y * 10.0, 2.0);
		// Beside the mid-segment: radial dist 8 ≤ 2*5.
		assert!(c.viewer_allowed(&Transform::from_xyz(8.0, 5.0, 0.0)));
		assert!(!c.viewer_allowed(&Transform::from_xyz(11.0, 5.0, 0.0)));
		// Beyond an end-cap along the axis: still within 5×R of the endpoint.
		assert!(c.viewer_allowed(&Transform::from_xyz(0.0, 18.0, 0.0)));
		assert!(!c.viewer_allowed(&Transform::from_xyz(0.0, 21.0, 0.0)));
		Ok(())
	}
}
