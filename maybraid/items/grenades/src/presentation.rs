//! Thrown-body mesh, material, and drop fulfillment.

use avian3d::prelude::*;
use bevy::prelude::*;
use character_items::{GrenadeSpec, GrenadeStats};
use std::f32::consts::PI;

use crate::material::{grenade_material, GrenadeMaterial, GrenadeMaterialPlugin};
use crate::runtime::{
	tick_grenade_fuses, DropLiveGrenade, GrenadeEffect, GrenadeFuse, SpawnThrownGrenade,
	ThrownGrenade, ThrownGrenadeSpawn,
};
use crate::sound;

pub fn grenade_mesh() -> Mesh {
	Sphere::new(0.5).mesh().ico(3).expect("grenade sphere")
}

impl ThrownGrenadeSpawn {
	/// Realize the physics body and mesh from a throw / drop request.
	pub fn realize(
		self,
		commands: &mut Commands,
		meshes: &mut Assets<Mesh>,
		materials: &mut Assets<GrenadeMaterial>,
	) -> Entity {
		let radius = self.stats.radius.max(0.08);
		let collider = Collider::sphere(radius);
		let volume = (4.0 / 3.0) * PI * radius * radius * radius;
		let density = (self.stats.mass / volume.max(1e-6)).max(0.05);
		let tumble = self.velocity.cross(Vec3::Y).normalize_or(Vec3::X) * 10.0;
		commands
			.spawn((
				(
					Name::new("thrown-grenade"),
					Transform {
						translation: self.origin,
						rotation: Quat::from_rotation_arc(
							Vec3::Y,
							self.velocity.normalize_or(Vec3::Y),
						),
						scale: Vec3::splat(radius * 2.0),
					},
					Visibility::default(),
					Mesh3d(meshes.add(grenade_mesh())),
					MeshMaterial3d(materials.add(grenade_material())),
					RigidBody::Dynamic,
					MassPropertiesBundle::from_shape(&collider, density),
					collider,
					crate::runtime::grenade_layers(),
				),
				(
					LinearVelocity(self.velocity),
					AngularVelocity(tumble),
					// Unit gravity, no linear damping / max-speed cap: catalog launch_speed
					// is the live throw. A 22 m/s median lob lands around 30 m.
					GravityScale(1.0),
					Restitution {
						coefficient: self.stats.restitution,
						combine_rule: CoefficientCombine::Average,
					},
					Friction {
						dynamic_coefficient: self.stats.friction,
						static_coefficient: self.stats.friction,
						combine_rule: CoefficientCombine::Average,
					},
					ThrownGrenade {
						spec: self.spec,
						stats: self.stats,
						effect: self.effect,
						source: self.source,
						detonated: false,
					},
					GrenadeFuse { remaining: self.fuse.max(0.0) },
					CollidingEntities::default(),
					sound::GrenadeBounceClock { next_at: 0.0, touching: Vec::new() },
				),
			))
			.id()
	}
}

pub fn spawn_thrown_grenade(
	commands: &mut Commands,
	meshes: &mut Assets<Mesh>,
	materials: &mut Assets<GrenadeMaterial>,
	origin: Vec3,
	velocity: Vec3,
	spec: GrenadeSpec,
	stats: GrenadeStats,
	effect: GrenadeEffect,
	source: Entity,
	fuse_remaining: f32,
) -> Entity {
	ThrownGrenadeSpawn::from_throw(origin, velocity, spec, stats, effect, source, fuse_remaining)
		.realize(commands, meshes, materials)
}

pub fn spawn_dropped_live_grenades(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<GrenadeMaterial>>,
	mut drops: MessageReader<DropLiveGrenade>,
) {
	for drop in drops.read() {
		drop.spawn().realize(&mut commands, &mut meshes, &mut materials);
	}
}

pub fn spawn_thrown_grenades(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<GrenadeMaterial>>,
	mut throws: MessageReader<SpawnThrownGrenade>,
) {
	for throw in throws.read() {
		throw.spawn().realize(&mut commands, &mut meshes, &mut materials);
	}
}

#[derive(Resource, Default)]
struct GrenadePresentation;

/// Mesh / material / audio for thrown bodies. Fulfills throw and drop requests.
pub fn register_grenade_presentation(app: &mut App) {
	if app.world().contains_resource::<GrenadePresentation>() {
		return;
	}
	if !app.is_plugin_added::<GrenadeMaterialPlugin>() {
		app.add_plugins(GrenadeMaterialPlugin);
	}
	sound::configure_grenade_audio(app);
	app.init_resource::<GrenadePresentation>().add_systems(
		Update,
		(spawn_thrown_grenades, spawn_dropped_live_grenades)
			.chain()
			.after(tick_grenade_fuses),
	);
}

#[cfg(test)]
mod tests {
	use bevy::ecs::system::RunSystemOnce;
	use character_items::GrenadeSpec;

	use super::*;
	use crate::runtime::GrenadeFuse;

	#[test]
	fn thrown_body_keeps_the_requested_fuse() {
		let mut world = World::new();
		world.init_resource::<Assets<Mesh>>();
		world.init_resource::<Assets<GrenadeMaterial>>();
		let stats = GrenadeStats::standard();
		let id = world
			.run_system_once(
				move |mut commands: Commands,
				      mut meshes: ResMut<Assets<Mesh>>,
				      mut materials: ResMut<Assets<GrenadeMaterial>>| {
					ThrownGrenadeSpawn::from_throw(
						Vec3::ZERO,
						Vec3::Z,
						GrenadeSpec::standard(),
						stats,
						GrenadeEffect::from_stats(&stats),
						Entity::PLACEHOLDER,
						1.25,
					)
					.realize(&mut commands, &mut meshes, &mut materials)
				},
			)
			.expect("spawn");
		let fuse = world.get::<GrenadeFuse>(id).expect("fuse");
		assert!((fuse.remaining - 1.25).abs() < 1e-4);
	}
}
