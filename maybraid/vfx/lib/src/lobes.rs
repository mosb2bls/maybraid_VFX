//! Lobe mesh, sampled variation, and age-driven transforms.

use bevy::prelude::*;

use crate::composition::LobeSpec;
use crate::lobe_material::LobeMaterial;
use crate::seed::{signed_salted, stream};

/// Low-poly icosphere with vertex colors holding object-space positions.
pub fn rounded_lobe_mesh() -> Mesh {
	let mut mesh = Sphere::new(0.5).mesh().ico(2).expect("lobe icosphere");
	let Some(positions) = mesh.attribute(Mesh::ATTRIBUTE_POSITION).and_then(|a| a.as_float3())
	else {
		return mesh;
	};
	let colors: Vec<[f32; 4]> = positions.iter().map(|p| [p[0], p[1], p[2], 1.0]).collect();
	mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
	mesh
}

/// Fast initial swell, slower later motion. Shared so lobes read as one blast.
pub fn expand_amount(age: f32, duration: f32) -> f32 {
	let t = if duration > 1e-4 { (age / duration).clamp(0.0, 1.0) } else { 1.0 };
	1.0 - (1.0 - t).powf(2.35)
}

/// Modest per-instance offsets. Call once at spawn and store the result.
pub fn vary_lobe(spec: LobeSpec, seed: u64, layer: u32, index: u32) -> LobeSpec {
	let s = stream(seed, layer, index);
	LobeSpec {
		offset: spec.offset
			+ Vec3::new(signed_salted(s, 1), signed_salted(s, 2), signed_salted(s, 3)) * 0.04,
		scale: spec.scale * (1.0 + signed_salted(s, 4) * 0.12),
		euler: spec.euler
			+ Vec3::new(signed_salted(s, 5), signed_salted(s, 6), signed_salted(s, 7)) * 0.22,
		expand: spec.expand * (1.0 + signed_salted(s, 8) * 0.08),
		rise: spec.rise * (1.0 + signed_salted(s, 9) * 0.12),
		roll: spec.roll * (1.0 + signed_salted(s, 10) * 0.15),
		duration: (spec.duration * (1.0 + signed_salted(s, 11) * 0.05)).max(0.05),
	}
}

pub fn lobe_transform(spec: &LobeSpec, age: f32) -> Transform {
	let expand = expand_amount(age, spec.duration);
	let rest = Quat::from_euler(EulerRot::YXZ, spec.euler.y, spec.euler.x, spec.euler.z);
	let spin = Quat::from_axis_angle(rest * Vec3::X, spec.roll * expand);
	let stretch = Vec3::new(1.0, 1.0 + expand * 0.18, 1.0);
	Transform {
		translation: spec.offset * (1.0 + expand * 0.38) + Vec3::Y * spec.rise * expand,
		rotation: spin * rest,
		scale: spec.scale * stretch * (1.0 + spec.expand * expand),
	}
}

/// One animated mesh lobe. `spec` is the sampled instance, not the authored default.
#[derive(Component, Debug)]
pub struct VfxLobe {
	pub age: f32,
	pub spec: LobeSpec,
	pub playback: f32,
}

/// Temporary holder so [`stamp_lobe_materials`] can add the asset after spawn.
#[derive(Component)]
pub struct LobeMaterialSlot(pub LobeMaterial);

pub fn stamp_lobe_materials(
	mut commands: Commands,
	mut materials: ResMut<Assets<LobeMaterial>>,
	pending: Query<(Entity, &LobeMaterialSlot)>,
) {
	for (entity, slot) in &pending {
		let handle = materials.add(slot.0.clone());
		commands
			.entity(entity)
			.insert(MeshMaterial3d(handle))
			.remove::<LobeMaterialSlot>();
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn expansion_is_fast_then_slow() {
		assert!(expand_amount(0.1, 1.0) > 0.2);
		assert!(expand_amount(0.1, 1.0) < expand_amount(0.5, 1.0));
		assert!((expand_amount(1.0, 1.0) - 1.0).abs() < 1e-4);
	}

	#[test]
	fn variation_is_stored_and_seed_stable() {
		let authored = LobeSpec::new(Vec3::ZERO, Vec3::new(0.7, 0.4, 0.5))
			.with_euler(Vec3::new(0.2, 0.4, -0.1))
			.with_expand(0.8)
			.with_rise(0.2)
			.with_roll(1.4)
			.with_duration(1.0);
		let a = vary_lobe(authored, 11, crate::seed::LAYER_FIRE, 0);
		let b = vary_lobe(authored, 11, crate::seed::LAYER_FIRE, 0);
		let c = vary_lobe(authored, 12, crate::seed::LAYER_FIRE, 0);
		assert_eq!(a.offset, b.offset);
		assert!(a.offset.distance(c.offset) > 1e-5);
		assert!((a.duration - authored.duration).abs() < 0.08);
	}

	#[test]
	fn lobes_rotate_as_they_expand() {
		let spec = LobeSpec::new(Vec3::ZERO, Vec3::new(0.7, 0.4, 0.5))
			.with_euler(Vec3::new(0.2, 0.4, -0.1))
			.with_expand(0.8)
			.with_rise(0.2)
			.with_roll(1.4)
			.with_duration(1.0);
		let early = lobe_transform(&spec, 0.05);
		let late = lobe_transform(&spec, 0.8);
		assert!(early.rotation.angle_between(late.rotation) > 0.2);
		assert!((early.scale.x - early.scale.y).abs() > 0.05);
	}
}
