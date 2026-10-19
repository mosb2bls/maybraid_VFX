//! Emitter readiness, instance clocks, animation, and cleanup.

use bevy::prelude::*;
use bevy_hanabi::prelude::{CompiledParticleEffect, EffectSpawner, EffectSystems, SpawnerSettings};

use crate::lobe_material::LobeMaterial;
use crate::lobes::{lobe_transform, stamp_lobe_materials, VfxLobe};
use crate::membership::{VfxInstanceMembers, VfxMemberOf};
use crate::spawn::{
	realize_layer, VfxEmitter, VfxEmitterArmed, VfxEmitterBurst, VfxFlash, VfxInstance,
	VfxLayerLife, VfxPendingLayers,
};

/// Mark GPU-ready emitters. Burst is deferred to [`gate_vfx_instances`].
pub fn arm_vfx_emitters(
	mut commands: Commands,
	emitters: Query<
		(Entity, &CompiledParticleEffect),
		(With<VfxEmitter>, Without<VfxEmitterArmed>),
	>,
) {
	for (entity, compiled) in &emitters {
		if compiled.is_ready() {
			commands.entity(entity).insert(VfxEmitterArmed);
		}
	}
}

/// Common start: once every immediate emitter is ready, burst and release the clocks.
/// Already-armed instances also burst late layers as they become ready.
pub fn gate_vfx_instances(
	mut commands: Commands,
	emitters: Query<(&VfxEmitter, Has<VfxEmitterArmed>, Has<VfxEmitterBurst>)>,
	mut spawners: Query<Option<&mut EffectSpawner>, With<VfxEmitter>>,
	mut instances: Query<(Entity, &mut VfxInstance, &VfxInstanceMembers)>,
) {
	for (_entity, mut instance, members) in &mut instances {
		if !instance.armed {
			if !members.emitters_ready(&emitters) {
				continue;
			}
			instance.armed = true;
			instance.age = 0.0;
		}
		burst_instance_emitters(members, &emitters, &mut spawners, &mut commands);
	}
}

pub fn tick_vfx_instances(
	mut commands: Commands,
	time: Res<Time>,
	lives: Query<&VfxLayerLife>,
	mut instances: Query<(Entity, &mut VfxInstance, &mut VfxPendingLayers, &VfxInstanceMembers)>,
) {
	let dt = time.delta_secs();
	for (entity, mut instance, mut pending, members) in &mut instances {
		if !instance.armed {
			continue;
		}
		instance.age += dt * instance.playback;
		let spawn = pending.spawn.clone();
		let mut remain = Vec::new();
		for layer in pending.layers.drain(..) {
			if instance.age + 1e-4 >= layer.delay {
				let actual_start = instance.age;
				instance.duration = instance.duration.max(actual_start + layer.part_lifetime());
				realize_layer(&mut commands, entity, &layer, &spawn);
			} else {
				remain.push(layer);
			}
		}
		pending.layers = remain;
		if pending.layers.is_empty() && members.layers_finished(&lives) {
			if instance.age >= instance.duration {
				commands.entity(entity).try_despawn();
			}
		}
	}
}

pub fn tick_vfx_flashes(
	mut commands: Commands,
	time: Res<Time>,
	instances: Query<&VfxInstance>,
	mut flashes: Query<(
		Entity,
		&VfxMemberOf,
		&mut VfxFlash,
		&mut PointLight,
		Option<&mut VfxLayerLife>,
	)>,
) {
	let dt = time.delta_secs();
	for (entity, member, mut flash, mut light, life) in &mut flashes {
		if !member.instance_armed(&instances) {
			light.intensity = 0.0;
			continue;
		}
		flash.age += dt * flash.playback;
		if let Some(mut life) = life {
			life.age = flash.age;
		}
		let t = if flash.fade > 1e-4 { (flash.age / flash.fade).clamp(0.0, 1.0) } else { 1.0 };
		light.intensity = flash.peak * (1.0 - t);
		if t >= 1.0 {
			commands.entity(entity).try_despawn();
		}
	}
}

pub fn tick_vfx_lobes(
	time: Res<Time>,
	instances: Query<&VfxInstance>,
	mut materials: ResMut<Assets<LobeMaterial>>,
	mut lobes: Query<(
		Entity,
		&VfxMemberOf,
		&mut VfxLobe,
		&mut Transform,
		&mut Visibility,
		Option<&MeshMaterial3d<LobeMaterial>>,
		Option<&mut VfxLayerLife>,
	)>,
) {
	let dt = time.delta_secs();
	for (_entity, member, mut lobe, mut transform, mut visibility, material, life) in &mut lobes {
		if !member.instance_armed(&instances) {
			*visibility = Visibility::Hidden;
			continue;
		}
		*visibility = Visibility::Inherited;
		lobe.age += dt * lobe.playback;
		if let Some(mut life) = life {
			life.age = lobe.age;
		}
		*transform = lobe_transform(&lobe.spec, lobe.age);
		if let Some(handle) = material {
			if let Some(mut material) = materials.get_mut(&handle.0) {
				material.set_age(lobe.age);
			}
		}
	}
}

pub fn tick_vfx_layer_lives(
	time: Res<Time>,
	armed: Query<Has<VfxEmitterArmed>, With<VfxEmitter>>,
	mut lives: Query<(Entity, &mut VfxLayerLife), With<VfxEmitter>>,
) {
	let dt = time.delta_secs();
	for (entity, mut life) in &mut lives {
		if life.waiting_for_emitter {
			if armed.get(entity).ok() != Some(true) {
				continue;
			}
			life.waiting_for_emitter = false;
			life.age = 0.0;
		}
		life.age += dt * life.playback;
	}
}

pub fn vfx_lifecycle_plugin(app: &mut App) {
	app.add_systems(
		PostUpdate,
		(arm_vfx_emitters, gate_vfx_instances)
			.chain()
			.before(EffectSystems::TickSpawners),
	)
	.add_systems(
		Update,
		(
			stamp_lobe_materials,
			tick_vfx_instances,
			tick_vfx_flashes,
			tick_vfx_lobes,
			tick_vfx_layer_lives,
		)
			.chain(),
	);
}

fn burst_instance_emitters(
	members: &VfxInstanceMembers,
	emitters: &Query<(&VfxEmitter, Has<VfxEmitterArmed>, Has<VfxEmitterBurst>)>,
	spawners: &mut Query<Option<&mut EffectSpawner>, With<VfxEmitter>>,
	commands: &mut Commands,
) {
	for member in members.iter() {
		let Ok((emitter, armed, burst)) = emitters.get(member) else {
			continue;
		};
		if !armed || burst {
			continue;
		}
		let settings = SpawnerSettings::once(emitter.count.into());
		if let Ok(Some(mut spawner)) = spawners.get_mut(member) {
			spawner.settings = settings;
			spawner.reset();
			spawner.active = true;
		} else {
			commands.entity(member).insert(EffectSpawner::new(&settings));
		}
		commands.entity(member).insert(VfxEmitterBurst);
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::membership::{VfxInstanceMembers, VfxMemberOf};

	fn spawn_instance(world: &mut World) -> Entity {
		world
			.spawn((
				VfxInstance {
					name: "test".into(),
					age: 0.0,
					duration: 1.0,
					playback: 1.0,
					seed: 0,
					armed: false,
				},
				VfxPendingLayers { layers: Vec::new(), spawn: Default::default() },
			))
			.id()
	}

	#[test]
	fn deep_member_respects_instance_armed() -> Result<(), String> {
		let mut world = World::new();
		let root = spawn_instance(&mut world);
		let mid = world.spawn(ChildOf(root)).id();
		world.spawn((ChildOf(mid), VfxMemberOf(root)));
		world.flush();

		let deep = world
			.query_filtered::<Entity, With<VfxMemberOf>>()
			.single(&world)
			.map_err(|_| "missing deep member")?;
		let member = world.get::<VfxMemberOf>(deep).ok_or("missing VfxMemberOf")?;
		assert!(
			!member.instance_armed_in_world(&world),
			"disarmed instance should gate deep member"
		);

		world.entity_mut(root).get_mut::<VfxInstance>().ok_or("missing instance")?.armed = true;
		let member = world.get::<VfxMemberOf>(deep).ok_or("missing VfxMemberOf")?;
		assert!(
			member.instance_armed_in_world(&world),
			"armed instance should release deep member"
		);
		Ok(())
	}

	#[test]
	fn membership_emitters_ready_ignores_depth() -> Result<(), String> {
		let mut world = World::new();
		let root = spawn_instance(&mut world);
		let mid = world.spawn(ChildOf(root)).id();
		world.spawn((ChildOf(mid), VfxMemberOf(root), VfxEmitter { count: 1.0 }));
		world.flush();

		let members =
			world.get::<VfxInstanceMembers>(root).ok_or("root missing VfxInstanceMembers")?;
		assert!(!members.emitters_ready_in_world(&world), "unarmed emitter should block readiness");

		let emitter = members.iter().next().ok_or("missing nested emitter")?;
		world.entity_mut(emitter).insert(VfxEmitterArmed);
		let members =
			world.get::<VfxInstanceMembers>(root).ok_or("root missing VfxInstanceMembers")?;
		assert!(
			members.emitters_ready_in_world(&world),
			"armed nested emitter should satisfy readiness"
		);
		Ok(())
	}

	#[test]
	fn membership_layers_finished_ignores_depth() -> Result<(), String> {
		let mut world = World::new();
		let root = spawn_instance(&mut world);
		let cluster = world.spawn((ChildOf(root), VfxMemberOf(root))).id();
		world.spawn((
			ChildOf(cluster),
			VfxMemberOf(root),
			VfxLayerLife { age: 0.5, duration: 0.5, playback: 1.0, waiting_for_emitter: false },
		));
		world.spawn((
			ChildOf(cluster),
			VfxMemberOf(root),
			VfxLayerLife { age: 0.2, duration: 0.5, playback: 1.0, waiting_for_emitter: false },
		));
		world.flush();

		let members =
			world.get::<VfxInstanceMembers>(root).ok_or("root missing VfxInstanceMembers")?;
		assert!(
			!members.layers_finished_in_world(&world),
			"unfinished deep lobe should block cleanup"
		);

		for member in members.iter().collect::<Vec<_>>() {
			let Some(life) = world.get::<VfxLayerLife>(member).cloned() else {
				continue;
			};
			world.entity_mut(member).insert(VfxLayerLife { age: life.duration, ..life });
		}
		let members =
			world.get::<VfxInstanceMembers>(root).ok_or("root missing VfxInstanceMembers")?;
		assert!(
			members.layers_finished_in_world(&world),
			"finished deep lobes should allow cleanup"
		);
		Ok(())
	}
}
