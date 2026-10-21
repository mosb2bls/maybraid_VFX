//! Cook / windup clocks and throw / detonate execution.

use avian3d::prelude::LinearVelocity;
use bevy::prelude::*;
use character_inventory_user::InventoryUser;
use character_items::{GrenadeStats, Inventory, InventoryItem};
use characters::{CharacterHeading, CharacterRoot};
use grenades::{GrenadeDetonated, GrenadeEffect, GrenadeSounds, SpawnThrownGrenade};
use maybraid_audio::{Audio, AudioClip, Mixer};
use maybraid_controllers_core::{InputRouter, InputSource};
use player::PlayerLook;

use super::lifecycle::start_primary_recharge;
use super::spawn::{launch_velocity, throw_aim};
use super::{GrenadePhase, GrenadeThrow, GrenadeUser, ThrowJingleDue};
use crate::hold::HeldGrenade;

fn clocks_frozen(router: Option<&InputRouter>, entity: Entity) -> bool {
	let Some(router) = router else {
		return false;
	};
	let source = InputSource::local_pad();
	router.is_suspended(source) && router.actor(source).is_some_and(|actor| actor.matches(entity))
}

/// Jingle at the held charge when the throw windup starts, before release.
pub fn play_throw_jingles(
	mut commands: Commands,
	sounds: Option<Res<GrenadeSounds>>,
	clips: Option<Res<Assets<AudioClip>>>,
	audio: Option<Res<Audio>>,
	mixer: Option<ResMut<Mixer>>,
	listeners: Query<&GlobalTransform, With<SpatialListener>>,
	due: Query<(Entity, &GrenadeUser, Option<&GlobalTransform>), With<ThrowJingleDue>>,
	held: Query<&GlobalTransform, With<HeldGrenade>>,
) {
	let (Some(sounds), Some(clips), Some(audio), Some(mut mixer), Some(listener)) =
		(sounds.as_deref(), clips.as_deref(), audio.as_deref(), mixer, listeners.iter().next())
	else {
		return;
	};
	for (entity, user, user_xf) in &due {
		let world = held
			.get(user.held)
			.ok()
			.map(GlobalTransform::translation)
			.or_else(|| user_xf.map(GlobalTransform::translation))
			.unwrap_or(Vec3::ZERO);
		sounds.play_throw(&mut commands, clips, audio, &mut mixer, listener, world);
		commands.entity(entity).remove::<ThrowJingleDue>();
	}
}

pub fn advance_throw(
	time: Res<Time>,
	router: Option<Res<InputRouter>>,
	mut throws: MessageWriter<SpawnThrownGrenade>,
	mut detonations: MessageWriter<GrenadeDetonated>,
	looks: Query<&PlayerLook>,
	velocities: Query<&LinearVelocity>,
	headings: Query<(&CharacterHeading, &ChildOf), With<CharacterRoot>>,
	carriers: Query<&InventoryUser>,
	mut bags: Query<&mut Inventory>,
	mut users: Query<(Entity, &GrenadeUser, &mut GrenadeThrow)>,
	mut held: Query<(&Transform, &mut Visibility), With<HeldGrenade>>,
) {
	let dt = time.delta_secs();
	let router = router.as_deref();
	for (user_entity, user, mut throw) in &mut users {
		if clocks_frozen(router, user_entity) {
			continue;
		}
		let stats = carriers
			.get(user_entity)
			.ok()
			.and_then(|carrier| bags.get(carrier.bag).ok())
			.and_then(|bag| bag.primary_weapon().and_then(InventoryItem::grenade_stats));
		match throw.phase {
			GrenadePhase::Ready => {}
			GrenadePhase::Cooking { age } => {
				let Some(stats) = stats else {
					reset_throw(&mut throw);
					continue;
				};
				let next_age = age + dt;
				let next_fuse = throw.fuse - dt;
				if next_fuse <= 0.0 {
					detonate_in_hand(
						&mut detonations,
						&carriers,
						&mut bags,
						&mut held,
						user_entity,
						user.held,
						stats,
					);
					reset_throw(&mut throw);
					continue;
				}
				throw.fuse = next_fuse;
				if throw.use_latched {
					throw.phase = GrenadePhase::Cooking { age: next_age };
				} else if next_age >= stats.release_at {
					release_grenade(
						&mut throws,
						&looks,
						&velocities,
						&headings,
						&carriers,
						&mut bags,
						&mut held,
						user_entity,
						user.held,
						stats,
						next_fuse,
					);
					throw.phase = GrenadePhase::Recovery { age: stats.release_at };
					throw.fuse = 0.0;
				} else {
					throw.phase = GrenadePhase::Windup { age: next_age };
				}
			}
			GrenadePhase::Windup { age } => {
				let Some(stats) = stats else {
					reset_throw(&mut throw);
					continue;
				};
				let next = age + dt;
				let next_fuse = throw.fuse - dt;
				if next_fuse <= 0.0 {
					detonate_in_hand(
						&mut detonations,
						&carriers,
						&mut bags,
						&mut held,
						user_entity,
						user.held,
						stats,
					);
					reset_throw(&mut throw);
					continue;
				}
				throw.fuse = next_fuse;
				if age < stats.release_at && next >= stats.release_at {
					release_grenade(
						&mut throws,
						&looks,
						&velocities,
						&headings,
						&carriers,
						&mut bags,
						&mut held,
						user_entity,
						user.held,
						stats,
						next_fuse,
					);
				}
				if next >= stats.throw_secs {
					reset_throw(&mut throw);
				} else if next >= stats.release_at {
					throw.phase = GrenadePhase::Recovery { age: next };
					throw.fuse = 0.0;
				} else {
					throw.phase = GrenadePhase::Windup { age: next };
				}
			}
			GrenadePhase::Recovery { age } => {
				let throw_secs = stats
					.map(|stats| stats.throw_secs)
					.unwrap_or(GrenadeStats::standard().throw_secs);
				let next = age + dt;
				if next >= throw_secs {
					reset_throw(&mut throw);
				} else {
					throw.phase = GrenadePhase::Recovery { age: next };
				}
			}
		}
	}
}

fn release_grenade(
	throws: &mut MessageWriter<SpawnThrownGrenade>,
	looks: &Query<&PlayerLook>,
	velocities: &Query<&LinearVelocity>,
	headings: &Query<(&CharacterHeading, &ChildOf), With<CharacterRoot>>,
	carriers: &Query<&InventoryUser>,
	bags: &mut Query<&mut Inventory>,
	held: &mut Query<(&Transform, &mut Visibility), With<HeldGrenade>>,
	user_entity: Entity,
	held_entity: Entity,
	stats: GrenadeStats,
	fuse_remaining: f32,
) {
	let look = looks.get(user_entity).copied().unwrap_or_default();
	let inherit = velocities.get(user_entity).map(|v| v.0).unwrap_or(Vec3::ZERO);
	let facing = headings
		.iter()
		.find(|(_, child)| child.parent() == user_entity)
		.map(|(heading, _)| heading.0)
		.unwrap_or(Vec3::Z);
	let Ok((pose, mut visibility)) = held.get_mut(held_entity) else {
		return;
	};
	*visibility = Visibility::Hidden;
	let aim = throw_aim(facing, &look);
	let origin = pose.translation + aim * 0.55 + Vec3::Y * 0.12;
	let spec = carriers
		.get(user_entity)
		.ok()
		.and_then(|carrier| bags.get(carrier.bag).ok())
		.and_then(|bag| bag.primary_weapon().and_then(InventoryItem::grenade_spec))
		.unwrap_or_default();
	start_primary_recharge(carriers, bags, user_entity);
	throws.write(SpawnThrownGrenade::new(
		origin,
		launch_velocity(aim, &stats, inherit),
		spec,
		stats,
		GrenadeEffect::from_stats(&stats),
		user_entity,
		fuse_remaining,
	));
}

fn detonate_in_hand(
	detonations: &mut MessageWriter<GrenadeDetonated>,
	carriers: &Query<&InventoryUser>,
	bags: &mut Query<&mut Inventory>,
	held: &mut Query<(&Transform, &mut Visibility), With<HeldGrenade>>,
	user_entity: Entity,
	held_entity: Entity,
	stats: GrenadeStats,
) {
	let position = held.get(held_entity).map(|(pose, _)| pose.translation).unwrap_or(Vec3::ZERO);
	if let Ok((_, mut visibility)) = held.get_mut(held_entity) {
		*visibility = Visibility::Hidden;
	}
	start_primary_recharge(carriers, bags, user_entity);
	detonations.write(GrenadeDetonated {
		position,
		source: user_entity,
		effect: GrenadeEffect::from_stats(&stats),
		radius: stats.blast_radius,
		damage: stats.blast_damage,
	});
}

fn reset_throw(throw: &mut GrenadeThrow) {
	throw.phase = GrenadePhase::Ready;
	throw.fuse = 0.0;
}
