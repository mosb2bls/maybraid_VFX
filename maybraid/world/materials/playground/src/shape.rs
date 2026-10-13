//! Blast-style preview / shot hulls. A filled sphere stays available for comparison.

use std::f32::consts::{FRAC_PI_2, PI, TAU};

use bevy::prelude::*;
use clap::ValueEnum;

/// Geometry the playground puts the current [`material_ref::MaterialRef`] on.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
#[value(rename_all = "kebab-case")]
pub enum PreviewShape {
	/// Filled UV sphere — the original solid host.
	#[default]
	Sphere,
	/// Capsule along local Y (the original shot hull).
	Capsule,
	/// Tiny overbright orb. Skyrim / Potter cores.
	Core,
	/// Faceted energy hull, larger than the core.
	Shell,
	/// Flat shockwave annulus in XZ.
	Ring,
	/// Outward-facing flipbook cards around the origin.
	Cards,
	/// Core + shell + ring stacked at one origin.
	Blast,
}

impl PreviewShape {
	pub fn label(self) -> &'static str {
		match self {
			Self::Sphere => "sphere",
			Self::Capsule => "capsule",
			Self::Core => "core",
			Self::Shell => "shell",
			Self::Ring => "ring",
			Self::Cards => "cards",
			Self::Blast => "blast",
		}
	}

	pub fn is_composite(self) -> bool {
		matches!(self, Self::Cards | Self::Blast)
	}

	/// Shot flies along +X. Capsule Y is remapped onto that axis.
	pub fn shot_rotation(self) -> Quat {
		match self {
			Self::Capsule => Quat::from_rotation_z(-FRAC_PI_2),
			_ => Quat::IDENTITY,
		}
	}
}

const SPHERE_R: f32 = 0.55;
const CORE_R: f32 = 0.16;
const SHELL_R: f32 = 0.78;
const CAPSULE_R: f32 = 0.12;
const CAPSULE_LEN: f32 = 1.6;
const RING_INNER: f32 = 0.48;
const RING_OUTER: f32 = 0.72;
const CARD_W: f32 = 0.22;
const CARD_H: f32 = 0.48;
const CARD_RADIUS: f32 = 0.28;
const CARD_COUNT: u32 = 6;

/// Shared mesh handles for preview and shots.
#[derive(Resource, Clone)]
pub struct PlaygroundMeshes {
	pub sphere: Handle<Mesh>,
	pub capsule: Handle<Mesh>,
	pub core: Handle<Mesh>,
	pub shell: Handle<Mesh>,
	pub ring: Handle<Mesh>,
	pub card: Handle<Mesh>,
}

impl PlaygroundMeshes {
	pub fn build(meshes: &mut Assets<Mesh>) -> Self {
		Self {
			sphere: meshes.add(Sphere::new(SPHERE_R).mesh().uv(32, 18)),
			capsule: meshes.add(
				Capsule3d::new(CAPSULE_R, CAPSULE_LEN)
					.mesh()
					.latitudes(10)
					.longitudes(16)
					.rings(4)
					.build(),
			),
			core: meshes.add(Sphere::new(CORE_R).mesh().uv(24, 14)),
			shell: meshes.add(Sphere::new(SHELL_R).mesh().ico(2).expect("shell ico")),
			ring: meshes.add(Annulus::new(RING_INNER, RING_OUTER).mesh().resolution(48)),
			card: meshes.add(Rectangle::new(CARD_W, CARD_H)),
		}
	}

	pub fn single(&self, shape: PreviewShape) -> Option<Handle<Mesh>> {
		match shape {
			PreviewShape::Sphere => Some(self.sphere.clone()),
			PreviewShape::Capsule => Some(self.capsule.clone()),
			PreviewShape::Core => Some(self.core.clone()),
			PreviewShape::Shell => Some(self.shell.clone()),
			PreviewShape::Ring => Some(self.ring.clone()),
			PreviewShape::Cards | PreviewShape::Blast => None,
		}
	}

	/// Local pieces for composite hosts. Ring lies in XZ (shockwave).
	pub fn parts(&self, shape: PreviewShape) -> Vec<(Handle<Mesh>, Transform)> {
		match shape {
			PreviewShape::Cards => card_transforms()
				.into_iter()
				.map(|transform| (self.card.clone(), transform))
				.collect(),
			PreviewShape::Blast => vec![
				(self.core.clone(), Transform::IDENTITY),
				(self.shell.clone(), Transform::IDENTITY),
				(self.ring.clone(), Transform::from_rotation(Quat::from_rotation_x(-FRAC_PI_2))),
			],
			_ => self
				.single(shape)
				.map(|mesh| {
					let transform = match shape {
						PreviewShape::Ring => {
							Transform::from_rotation(Quat::from_rotation_x(-FRAC_PI_2))
						}
						_ => Transform::IDENTITY,
					};
					vec![(mesh, transform)]
				})
				.unwrap_or_default(),
		}
	}
}

fn card_transforms() -> Vec<Transform> {
	(0..CARD_COUNT)
		.map(|i| {
			let yaw = TAU * (i as f32) / (CARD_COUNT as f32);
			let outward = Vec3::new(yaw.cos(), 0.0, yaw.sin());
			Transform {
				translation: outward * CARD_RADIUS,
				rotation: Quat::from_rotation_y(-yaw + PI) * Quat::from_rotation_x(-0.18),
				scale: Vec3::ONE,
			}
		})
		.collect()
}

pub fn setup_meshes(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>) {
	commands.insert_resource(PlaygroundMeshes::build(&mut meshes));
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn blast_is_three_pieces() {
		assert!(PreviewShape::Blast.is_composite());
		assert_eq!(PreviewShape::Blast.label(), "blast");
		assert_eq!(PreviewShape::Capsule.shot_rotation(), Quat::from_rotation_z(-FRAC_PI_2));
	}
}
