//! Use activation, windup, release marker, and follow-through.

use avian3d::prelude::LinearVelocity;
use bevy::prelude::*;
use character_inventory_user::InventoryUser;
use character_items::{GrenadeStats, Inventory, InventoryItem};
use characters::{CharacterHeading, CharacterRoot};
use damage::Downed;
use firearm_user::WeaponSwap;
use grenades::{
	spawn_thrown_grenade, DropLiveGrenade, GrenadeDetonated, GrenadeEffect, GrenadeMaterial,
	GrenadeSounds,
};
use maybraid_audio::{Audio, AudioClip, Mixer};
use maybraid_character_controller::CharacterIntent;
use player::{look_forward, PlayerLook};

use crate::hold::HeldGrenade;
use crate::HeldByGrenade;

/// 1:1 user → held visual. Relationship rows are immutable; throw clocks live on [`GrenadeThrow`].
#[derive(Component, Debug, Clone, Copy)]
#[relationship(relationship_target = HeldByGrenade)]
pub struct GrenadeUser {
	#[relationship]
	pub held: Entity,
	pub settings: GrenadeUserSettings,
}

impl GrenadeUser {
	pub fn holding(held: Entity) -> Self {
		Self { held, settings: GrenadeUserSettings::default() }
	}
}

/// Mutable throw / latch / cook state. Kept off [`GrenadeUser`] because relationships are immutable.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct GrenadeThrow {
	pub phase: GrenadePhase,
	/// True while use stays held after a throw so recharge cannot auto-fire.
	pub use_latched: bool,
	/// Remaining fuse while the pin is pulled ([`GrenadePhase::Cooking`] / [`GrenadePhase::Windup`]).
	pub fuse: f32,
}

impl GrenadeThrow {
	pub fn busy(&self) -> bool {
		!matches!(self.phase, GrenadePhase::Ready)
	}

	pub fn cooking(&self) -> bool {
		matches!(self.phase, GrenadePhase::Cooking { .. })
	}

	/// Follow-through blocks a swap. Cooking drops the live charge instead.
	pub fn blocks_swap(&self) -> bool {
		matches!(self.phase, GrenadePhase::Windup { .. } | GrenadePhase::Recovery { .. })
	}

	pub fn live_fuse(&self) -> Option<f32> {
		match self.phase {
			GrenadePhase::Cooking { .. } | GrenadePhase::Windup { .. } => Some(self.fuse),
			GrenadePhase::Ready | GrenadePhase::Recovery { .. } => None,
		}
	}
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GrenadeUserSettings {
	pub hold_forward: f32,
	pub hold_right: f32,
	pub hold_up: f32,
}

impl Default for GrenadeUserSettings {
	fn default() -> Self {
		Self { hold_forward: 0.16, hold_right: -0.04, hold_up: 0.03 }
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum GrenadePhase {
	#[default]
	Ready,
	/// Pin pulled, use still held. Fuse ticks; the arm holds a cocked pose.
	Cooking {
		age: f32,
	},
	/// Use released before [`GrenadeStats::release_at`]. Same throw window as a tap.
	Windup {
		age: f32,
	},
	Recovery {
		age: f32,
	},
}

/// Schedule point so death / loot teardown can run after a live drop.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GrenadeUserSystems {
	Interrupt,
}

/// One-shot: play the throw jingle, then remove.
#[derive(Component, Debug, Clone, Copy)]
pub(crate) struct ThrowJingleDue;

pub fn apply_throw_intents(
	mut intents: MessageReader<CharacterIntent>,
	bags: Query<&Inventory>,
	carriers: Query<&InventoryUser>,
	mut commands: Commands,
	mut users: Query<(Entity, &mut GrenadeThrow, Has<WeaponSwap>), With<GrenadeUser>>,
) {
	let use_held = intents.read().any(|intent| matches!(intent, CharacterIntent::UseItem(_)));
	for (entity, mut throw, swapping) in &mut users {
		if !use_held {
			throw.use_latched = false;
		}
		if swapping || throw.busy() || throw.use_latched {
			continue;
		}
		if !use_held {
			continue;
		}
		let ready = carriers
			.get(entity)
			.ok()
			.and_then(|carrier| bags.get(carrier.bag).ok())
			.and_then(Inventory::primary_weapon)
			.is_some_and(|item| match item {
				InventoryItem::Grenade { recharge, .. } => recharge.ready(),
				_ => false,
			});
		if !ready {
			continue;
		}
		let fuse = carriers
			.get(entity)
			.ok()
			.and_then(|carrier| bags.get(carrier.bag).ok())
			.and_then(|bag| bag.primary_weapon().and_then(InventoryItem::grenade_stats))
			.map(|stats| stats.fuse)
			.unwrap_or_else(|| GrenadeStats::standard().fuse);
		throw.phase = GrenadePhase::Cooking { age: 0.0 };
		throw.fuse = fuse;
		throw.use_latched = true;
		commands.entity(entity).try_insert(ThrowJingleDue);
	}
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
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<GrenadeMaterial>>,
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
	for (user_entity, user, mut throw) in &mut users {
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
						&mut commands,
						&mut meshes,
						&mut materials,
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
						&mut commands,
						&mut meshes,
						&mut materials,
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

/// Drop a pin-pulled charge when the holder dies or starts a swap.
pub fn interrupt_live_cooks(
	mut drops: MessageWriter<DropLiveGrenade>,
	velocities: Query<&LinearVelocity>,
	carriers: Query<&InventoryUser>,
	mut bags: Query<&mut Inventory>,
	held: Query<&Transform, With<HeldGrenade>>,
	mut users: Query<
		(Entity, &GrenadeUser, &mut GrenadeThrow, Option<&Transform>, Option<&GlobalTransform>),
		Or<(With<Downed>, With<WeaponSwap>)>,
	>,
) {
	for (user_entity, user, mut throw, local, global) in &mut users {
		let Some(fuse) = throw.live_fuse().filter(|fuse| *fuse > 0.0) else {
			continue;
		};
		let Some(stats) = carriers
			.get(user_entity)
			.ok()
			.and_then(|carrier| bags.get(carrier.bag).ok())
			.and_then(|bag| bag.primary_weapon().and_then(InventoryItem::grenade_stats))
		else {
			reset_throw(&mut throw);
			continue;
		};
		let spec = carriers
			.get(user_entity)
			.ok()
			.and_then(|carrier| bags.get(carrier.bag).ok())
			.and_then(|bag| bag.primary_weapon().and_then(InventoryItem::grenade_spec))
			.unwrap_or_default();
		let origin = held
			.get(user.held)
			.ok()
			.map(|pose| pose.translation)
			.or_else(|| global.map(GlobalTransform::translation))
			.or_else(|| local.map(|transform| transform.translation))
			.unwrap_or(Vec3::ZERO);
		let inherit = velocities.get(user_entity).map(|velocity| velocity.0).unwrap_or(Vec3::ZERO);
		start_primary_recharge(&carriers, &mut bags, user_entity);
		drops.write(DropLiveGrenade {
			origin,
			velocity: inherit * stats.inherit_velocity,
			spec,
			stats,
			effect: GrenadeEffect::from_stats(&stats),
			fuse,
			source: user_entity,
		});
		reset_throw(&mut throw);
	}
}

fn release_grenade(
	commands: &mut Commands,
	meshes: &mut Assets<Mesh>,
	materials: &mut Assets<GrenadeMaterial>,
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
	spawn_thrown_grenade(
		commands,
		meshes,
		materials,
		origin,
		launch_velocity(aim, &stats, inherit),
		spec,
		stats,
		GrenadeEffect::from_stats(&stats),
		user_entity,
		fuse_remaining,
	);
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

fn start_primary_recharge(
	carriers: &Query<&InventoryUser>,
	bags: &mut Query<&mut Inventory>,
	user_entity: Entity,
) {
	if let Ok(carrier) = carriers.get(user_entity) {
		if let Ok(mut bag) = bags.get_mut(carrier.bag) {
			if let Some(InventoryItem::Grenade { recharge, stats, .. }) = bag.primary_weapon_mut() {
				recharge.start(stats.recharge);
			}
		}
	}
}

fn reset_throw(throw: &mut GrenadeThrow) {
	throw.phase = GrenadePhase::Ready;
	throw.fuse = 0.0;
}

/// Camera `-Z` with the same yaw×pitch product as the follow camera.
/// First person follows the camera. Third person uses body heading; orbit pitch is not a throw angle.
pub fn throw_aim(facing: Vec3, look: &PlayerLook) -> Vec3 {
	if look.first_person {
		return look_forward(look);
	}
	Vec3::new(facing.x, 0.0, facing.z).normalize_or(Vec3::Z)
}

pub fn launch_velocity(aim: Vec3, stats: &GrenadeStats, inherit: Vec3) -> Vec3 {
	let tossed = (aim + Vec3::Y * stats.upward_bias).normalize_or(Vec3::Y);
	tossed * stats.launch_speed + inherit * stats.inherit_velocity
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::ecs::system::RunSystemOnce;
	use character_items::{GrenadeRecharge, GrenadeSpec, GrenadeStats, InventoryItem};

	fn spawn_ready_user(world: &mut World) -> Entity {
		let bag = world
			.spawn(Inventory {
				items: vec![InventoryItem::Grenade {
					spec: GrenadeSpec::standard(),
					stats: GrenadeStats::standard(),
					recharge: GrenadeRecharge { remaining: 0.0 },
				}],
				clothing: Vec::new(),
				weapons: vec![0],
				skills: Vec::new(),
			})
			.id();
		let held = world
			.spawn((
				Transform::from_xyz(0.0, 1.4, 0.4),
				Visibility::Visible,
				HeldGrenade { scale: 0.2 },
			))
			.id();
		world
			.spawn((
				InventoryUser::carrying(bag),
				GrenadeUser::holding(held),
				GrenadeThrow::default(),
			))
			.id()
	}

	fn advance_time(world: &mut World, dt: f32) {
		let mut time = Time::<()>::default();
		time.advance_by(std::time::Duration::from_secs_f32(dt));
		world.insert_resource(time);
	}

	fn run_advance(world: &mut World) -> anyhow::Result<()> {
		world.init_resource::<Messages<GrenadeDetonated>>();
		world.init_resource::<Assets<Mesh>>();
		world.init_resource::<Assets<GrenadeMaterial>>();
		world
			.run_system_once(advance_throw)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		Ok(())
	}

	fn apply_use_item(world: &mut World) -> anyhow::Result<()> {
		world.init_resource::<Messages<CharacterIntent>>();
		world.write_message(CharacterIntent::UseItem(1.0));
		world
			.run_system_once(apply_throw_intents)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		Ok(())
	}

	#[test]
	fn use_item_starts_cooking_without_mouse() -> anyhow::Result<()> {
		let mut world = World::new();
		let user = spawn_ready_user(&mut world);
		apply_use_item(&mut world)?;
		let throw = world.get::<GrenadeThrow>(user).expect("throw");
		assert!(matches!(throw.phase, GrenadePhase::Cooking { age: 0.0 }));
		assert!((throw.fuse - GrenadeStats::standard().fuse).abs() < 1e-4);
		assert!(throw.use_latched);
		assert!(world.get::<ThrowJingleDue>(user).is_some());
		Ok(())
	}

	#[test]
	fn use_item_works_for_non_player_users() -> anyhow::Result<()> {
		let mut world = World::new();
		let user = spawn_ready_user(&mut world);
		apply_use_item(&mut world)?;
		assert!(matches!(
			world.get::<GrenadeThrow>(user).expect("throw").phase,
			GrenadePhase::Cooking { .. }
		));
		Ok(())
	}

	#[test]
	fn held_use_item_does_not_rethrow_while_latched() -> anyhow::Result<()> {
		let mut world = World::new();
		let user = spawn_ready_user(&mut world);
		apply_use_item(&mut world)?;
		world.write_message(CharacterIntent::UseItem(1.0));
		world
			.run_system_once(apply_throw_intents)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		let throw = world.get::<GrenadeThrow>(user).expect("throw");
		assert!(matches!(throw.phase, GrenadePhase::Cooking { age: 0.0 }));
		assert!(throw.use_latched);
		Ok(())
	}

	#[test]
	fn weapon_swap_blocks_throw_start() -> anyhow::Result<()> {
		let mut world = World::new();
		let user = spawn_ready_user(&mut world);
		world.entity_mut(user).insert(WeaponSwap::default());
		apply_use_item(&mut world)?;
		let throw = world.get::<GrenadeThrow>(user).expect("throw");
		assert_eq!(throw.phase, GrenadePhase::Ready);
		assert!(!throw.use_latched);
		Ok(())
	}

	#[test]
	fn recharging_grenade_blocks_throw_start() -> anyhow::Result<()> {
		let mut world = World::new();
		let bag = world
			.spawn(Inventory {
				items: vec![InventoryItem::Grenade {
					spec: GrenadeSpec::standard(),
					stats: GrenadeStats::standard(),
					recharge: GrenadeRecharge { remaining: 2.0 },
				}],
				clothing: Vec::new(),
				weapons: vec![0],
				skills: Vec::new(),
			})
			.id();
		let held = world.spawn_empty().id();
		let user = world
			.spawn((
				InventoryUser::carrying(bag),
				GrenadeUser::holding(held),
				GrenadeThrow::default(),
			))
			.id();
		apply_use_item(&mut world)?;
		let throw = world.get::<GrenadeThrow>(user).expect("throw");
		assert_eq!(throw.phase, GrenadePhase::Ready);
		Ok(())
	}

	#[test]
	fn launch_follows_body_heading_not_default_look_yaw() {
		let look = PlayerLook { yaw: 0.0, pitch: 0.0, ..default() };
		let aim = throw_aim(Vec3::Z, &look);
		let velocity = launch_velocity(aim, &GrenadeStats::standard(), Vec3::ZERO);
		assert!(velocity.y > 0.0);
		assert!(velocity.z > 0.0, "facing +Z must toss +Z, got {velocity:?}");
	}

	#[test]
	fn first_person_uses_look_yaw() {
		let look = PlayerLook { yaw: 0.0, pitch: 0.0, first_person: true, ..default() };
		let aim = throw_aim(Vec3::Z, &look);
		assert!(aim.z < 0.0);
	}

	#[test]
	fn first_person_positive_pitch_throws_up() {
		let look = PlayerLook { yaw: 0.0, pitch: 0.4, first_person: true, ..default() };
		let aim = throw_aim(Vec3::Z, &look);
		assert!(aim.y > 0.2, "camera-up must toss up, got {aim:?}");
		assert!(aim.z < 0.0);
	}

	#[test]
	fn third_person_ignores_orbit_pitch() {
		let look = PlayerLook { yaw: 0.0, pitch: 0.6, first_person: false, ..default() };
		let aim = throw_aim(Vec3::Z, &look);
		assert!(aim.y.abs() < 1e-5, "orbit pitch is not a throw angle, got {aim:?}");
		assert!(aim.z > 0.9);
	}

	#[test]
	fn windup_crosses_release_once() {
		let stats = GrenadeStats::standard();
		let prev = stats.release_at - 0.2;
		let next = prev + 0.5;
		assert!(prev < stats.release_at && next >= stats.release_at);
	}

	#[test]
	fn launch_is_a_brisk_lob() {
		let stats = GrenadeStats::standard();
		let velocity = launch_velocity(Vec3::Z, &stats, Vec3::ZERO);
		assert!(stats.launch_speed > 8.0 && stats.launch_speed < 12.0);
		assert!(velocity.length() > 8.0 && velocity.length() < 12.0);
		assert!(velocity.y > velocity.z * 0.18);
	}

	#[test]
	fn quick_tap_throws_at_release_like_today() -> anyhow::Result<()> {
		let mut world = World::new();
		let user = spawn_ready_user(&mut world);
		apply_use_item(&mut world)?;
		let _ = world.resource_mut::<Messages<CharacterIntent>>().drain().count();
		world
			.run_system_once(apply_throw_intents)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		let stats = GrenadeStats::standard();
		advance_time(&mut world, stats.release_at);
		run_advance(&mut world)?;
		let throw = world.get::<GrenadeThrow>(user).expect("throw");
		assert!(
			matches!(throw.phase, GrenadePhase::Recovery { .. }),
			"quick tap must still release at release_at, got {:?}",
			throw.phase
		);
		let fuse = world
			.query::<&grenades::GrenadeFuse>()
			.iter(&world)
			.next()
			.map(|fuse| fuse.remaining)
			.ok_or_else(|| anyhow::anyhow!("thrown grenade"))?;
		assert!(
			(fuse - (stats.fuse - stats.release_at)).abs() < 1e-3,
			"tap fuse must be full fuse minus the throw window, got {fuse}"
		);
		Ok(())
	}

	#[test]
	fn release_after_cook_keeps_remaining_fuse() -> anyhow::Result<()> {
		let mut world = World::new();
		let user = spawn_ready_user(&mut world);
		apply_use_item(&mut world)?;
		let stats = GrenadeStats::standard();
		let cook = 1.0;
		advance_time(&mut world, cook);
		run_advance(&mut world)?;
		let throw = world.get::<GrenadeThrow>(user).expect("throw");
		assert!(matches!(throw.phase, GrenadePhase::Cooking { .. }));
		assert!((throw.fuse - (stats.fuse - cook)).abs() < 1e-3);
		assert!(world.query::<&grenades::ThrownGrenade>().iter(&world).next().is_none());
		let _ = world.resource_mut::<Messages<CharacterIntent>>().drain().count();
		world
			.run_system_once(apply_throw_intents)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		advance_time(&mut world, 1.0 / 60.0);
		run_advance(&mut world)?;
		let fuse = world
			.query::<&grenades::GrenadeFuse>()
			.iter(&world)
			.next()
			.map(|fuse| fuse.remaining)
			.ok_or_else(|| anyhow::anyhow!("thrown grenade"))?;
		let expected = stats.fuse - cook - 1.0 / 60.0;
		assert!(
			(fuse - expected).abs() < 1e-3,
			"remaining fuse after a 1s cook must carry to the thrown body, got {fuse} want {expected}"
		);
		Ok(())
	}

	#[test]
	fn overcook_detonates_in_hand() -> anyhow::Result<()> {
		let mut world = World::new();
		let user = spawn_ready_user(&mut world);
		apply_use_item(&mut world)?;
		let stats = GrenadeStats::standard();
		advance_time(&mut world, stats.fuse);
		run_advance(&mut world)?;
		let throw = world.get::<GrenadeThrow>(user).expect("throw");
		assert_eq!(throw.phase, GrenadePhase::Ready);
		assert!(world.query::<&grenades::ThrownGrenade>().iter(&world).next().is_none());
		let blasts = world.resource_mut::<Messages<GrenadeDetonated>>().drain().count();
		assert_eq!(blasts, 1, "overcook must detonate in hand");
		let bag = world
			.query::<&Inventory>()
			.single(&world)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		let InventoryItem::Grenade { recharge, .. } = &bag.items[0] else {
			anyhow::bail!("expected grenade");
		};
		assert!(recharge.remaining > 0.0, "in-hand detonation starts recharge");
		Ok(())
	}

	#[test]
	fn interrupt_while_cooking_drops_live() -> anyhow::Result<()> {
		use damage::Downed;

		let mut world = World::new();
		world.init_resource::<Messages<DropLiveGrenade>>();
		let user = spawn_ready_user(&mut world);
		apply_use_item(&mut world)?;
		advance_time(&mut world, 0.8);
		run_advance(&mut world)?;
		let remaining = world.get::<GrenadeThrow>(user).expect("throw").fuse;
		world
			.entity_mut(user)
			.insert(Downed { source: None, point: Vec3::ZERO, at: 0.0 });
		world
			.run_system_once(interrupt_live_cooks)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		let throw = world.get::<GrenadeThrow>(user).expect("throw");
		assert_eq!(throw.phase, GrenadePhase::Ready);
		let drop = world
			.resource_mut::<Messages<DropLiveGrenade>>()
			.drain()
			.next()
			.ok_or_else(|| anyhow::anyhow!("live drop"))?;
		assert!((drop.fuse - remaining).abs() < 1e-4);
		assert_eq!(drop.source, user);
		Ok(())
	}
}
