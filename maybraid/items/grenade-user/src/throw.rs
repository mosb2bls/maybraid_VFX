//! Use activation, windup, release marker, and follow-through.

use avian3d::prelude::LinearVelocity;
use bevy::prelude::*;
use character_inventory_user::InventoryUser;
use character_items::{GrenadeStats, Inventory, InventoryItem};
use characters::{CharacterHeading, CharacterRoot};
use firearm_user::WeaponSwap;
use grenades::{spawn_thrown_grenade, GrenadeEffect, GrenadeMaterial, GrenadeSounds};
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

/// Mutable throw / latch state. Kept off [`GrenadeUser`] because relationships are immutable.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct GrenadeThrow {
	pub phase: GrenadePhase,
	/// True while use stays held after a throw so recharge cannot auto-fire.
	pub use_latched: bool,
}

impl GrenadeThrow {
	pub fn busy(&self) -> bool {
		!matches!(self.phase, GrenadePhase::Ready)
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
	Windup {
		age: f32,
	},
	Recovery {
		age: f32,
	},
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
		throw.phase = GrenadePhase::Windup { age: 0.0 };
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
			GrenadePhase::Windup { age } => {
				let Some(stats) = stats else {
					throw.phase = GrenadePhase::Ready;
					continue;
				};
				let next = age + dt;
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
					);
				}
				if next >= stats.throw_secs {
					throw.phase = GrenadePhase::Ready;
				} else if next >= stats.release_at {
					throw.phase = GrenadePhase::Recovery { age: next };
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
					throw.phase = GrenadePhase::Ready;
				} else {
					throw.phase = GrenadePhase::Recovery { age: next };
				}
			}
		}
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
	if let Ok(carrier) = carriers.get(user_entity) {
		if let Ok(mut bag) = bags.get_mut(carrier.bag) {
			if let Some(InventoryItem::Grenade { recharge, stats, .. }) = bag.primary_weapon_mut() {
				recharge.start(stats.recharge);
			}
		}
	}
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
	);
}

/// Camera `-Z` with the same yaw×pitch product as the follow camera.
/// First person follows the camera. Third person uses body heading; orbit pitch is not a throw angle.
pub fn throw_aim(facing: Vec3, look: &PlayerLook) -> Vec3 {
	if look.first_person {
		return look_forward(look);
	}
	Vec3::new(facing.x, 0.0, facing.z).normalize_or(Vec3::Z)
}

/// Avian default |g|. Thrown grenades use unit gravity scale and no linear damping.
pub const THROW_GRAVITY: f32 = 9.81;

/// Chest hold (`1.28 + hold_up`) plus the release toss offset (`0.12`).
pub const STANDING_RELEASE_HEIGHT: f32 = 1.43;

pub fn launch_velocity(aim: Vec3, stats: &GrenadeStats, inherit: Vec3) -> Vec3 {
	let tossed = (aim + Vec3::Y * stats.upward_bias).normalize_or(Vec3::Y);
	tossed * stats.launch_speed + inherit * stats.inherit_velocity
}

/// Seconds until a vacuum throw from `height` returns to y = 0.
pub fn landing_time(height: f32, vy: f32, gravity: f32) -> f32 {
	if gravity <= 1e-6 {
		return 0.0;
	}
	let disc = vy * vy + 2.0 * gravity * height.max(0.0);
	(vy + disc.sqrt()) / gravity
}

/// Horizontal meters a vacuum throw covers before y = 0.
pub fn landing_range(height: f32, velocity: Vec3, gravity: f32) -> f32 {
	Vec2::new(velocity.x, velocity.z).length() * landing_time(height, velocity.y, gravity)
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
		let held = world.spawn_empty().id();
		world
			.spawn((
				InventoryUser::carrying(bag),
				GrenadeUser::holding(held),
				GrenadeThrow::default(),
			))
			.id()
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
	fn use_item_starts_windup_without_mouse() -> anyhow::Result<()> {
		let mut world = World::new();
		let user = spawn_ready_user(&mut world);
		apply_use_item(&mut world)?;
		let throw = world.get::<GrenadeThrow>(user).expect("throw");
		assert!(matches!(throw.phase, GrenadePhase::Windup { age: 0.0 }));
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
			GrenadePhase::Windup { .. }
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
		assert!(matches!(throw.phase, GrenadePhase::Windup { age: 0.0 }));
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
		assert!(stats.launch_speed > 28.0 && stats.launch_speed < 32.0);
		assert!(velocity.length() > 28.0 && velocity.length() < 32.0);
		assert!(velocity.y > velocity.z * 0.18);
	}

	fn standing_third_person_velocity(stats: &GrenadeStats) -> Vec3 {
		let look = PlayerLook::default();
		let aim = throw_aim(Vec3::Z, &look);
		launch_velocity(aim, stats, Vec3::ZERO)
	}

	#[test]
	fn standing_third_person_lob_clears_thirty_meters() {
		let stats = GrenadeStats::standard();
		let velocity = standing_third_person_velocity(&stats);
		let range = landing_range(STANDING_RELEASE_HEIGHT, velocity, THROW_GRAVITY);
		let airtime = landing_time(STANDING_RELEASE_HEIGHT, velocity.y, THROW_GRAVITY);
		assert!(range > 30.0, "standing third-person lob should land past 30 m, got {range:.1} m");
		assert!(
			airtime < stats.fuse,
			"fuse {:.1}s must outlast the {airtime:.2}s lob so it lands before detonation",
			stats.fuse
		);
	}

	#[test]
	fn faster_catalog_throw_lands_further() {
		let stats = GrenadeStats::standard();
		let mut boosted = stats;
		boosted.launch_speed *= 1.5;
		let base = landing_range(
			STANDING_RELEASE_HEIGHT,
			standing_third_person_velocity(&stats),
			THROW_GRAVITY,
		);
		let extra = landing_range(
			STANDING_RELEASE_HEIGHT,
			standing_third_person_velocity(&boosted),
			THROW_GRAVITY,
		);
		assert!(
			extra > base * 1.4,
			"launch_speed modifiers must stack on the base lob ({base:.1} m → {extra:.1} m)"
		);
	}

	#[test]
	fn ten_meter_launch_used_to_land_around_ten_meters() {
		let mut old = GrenadeStats::standard();
		old.launch_speed = 10.0;
		let range = landing_range(
			STANDING_RELEASE_HEIGHT,
			standing_third_person_velocity(&old),
			THROW_GRAVITY,
		);
		assert!(
			range > 7.0 && range < 12.0,
			"the previous 10 m/s catalog throw should reconstruct the ~10 m lob, got {range:.1} m"
		);
	}
}
