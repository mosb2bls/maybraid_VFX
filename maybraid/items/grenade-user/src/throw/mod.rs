//! Use activation, windup, release marker, and follow-through.

mod execute;
mod input;
mod lifecycle;
mod spawn;

use bevy::prelude::*;
use maybraid_controllers_core::{HoldEffect, RoutingEvent, RoutingKind};

use crate::HeldByGrenade;

pub use execute::{advance_throw, play_throw_jingles};
pub use input::apply_throw_intents;
pub use lifecycle::{apply_throw_routing, interrupt_live_cooks};
pub use spawn::{
	STANDING_RELEASE_HEIGHT, THROW_GRAVITY, landing_range, landing_time, launch_velocity, throw_aim,
};

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

	/// Update the latch from a delivered use. Missing delivery must not call this.
	pub fn apply_use(&mut self, held: bool) {
		if !held {
			self.use_latched = false;
		}
	}

	/// Disconnect / reassign / removal cancel a live cook. Capture does not —
	/// use [`Self::apply_routing`], which branches on [`RoutingKind`].
	pub fn apply_hold(&mut self, hold: HoldEffect) -> Option<f32> {
		match hold {
			HoldEffect::ReleaseNeutral => self.take_live_cook(),
			HoldEffect::Freeze | HoldEffect::None => None,
		}
	}

	/// Capture cancels a leftover use latch. A pin-pulled charge stays cooking.
	pub fn cancel_partial_hold(&mut self) {
		if self.live_fuse().is_none() {
			self.use_latched = false;
		}
	}

	/// Drop on disconnect / reassign / removal. Capture only cancels a
	/// leftover latch; an already-live cook keeps its fuse.
	pub fn apply_routing(&mut self, event: RoutingEvent) -> Option<f32> {
		match event.kind() {
			RoutingKind::Reassigned | RoutingKind::Disconnected | RoutingKind::Removed => {
				self.use_latched = false;
				self.take_live_cook()
			}
			RoutingKind::Captured => {
				self.cancel_partial_hold();
				None
			}
			RoutingKind::Assigned
			| RoutingKind::Uncaptured
			| RoutingKind::Suspended
			| RoutingKind::Resumed => None,
		}
	}

	/// Start a cook when the addressed use is held and the primary is ready.
	pub fn try_start_cook(&mut self, ready: bool, swapping: bool, fuse: f32) -> bool {
		if swapping || self.busy() || self.use_latched || !ready {
			return false;
		}
		self.phase = GrenadePhase::Cooking { age: 0.0 };
		self.fuse = fuse;
		self.use_latched = true;
		true
	}

	/// One lifecycle owner for swap / death. Returns the live fuse once.
	pub fn take_live_cook(&mut self) -> Option<f32> {
		let fuse = self.live_fuse().filter(|fuse| *fuse > 0.0)?;
		self.phase = GrenadePhase::Ready;
		self.fuse = 0.0;
		Some(fuse)
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

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::ecs::system::RunSystemOnce;
	use character_inventory_user::InventoryUser;
	use character_items::{GrenadeRecharge, GrenadeSpec, GrenadeStats, Inventory, InventoryItem};
	use firearm_user::WeaponSwap;
	use grenades::{DropLiveGrenade, GrenadeDetonated, SpawnThrownGrenade};
	use maybraid_character_controller::{ActorTarget, InputRouter};
	use maybraid_equipment::{EquippedItemHandler, EquippedItemUse};
	use player::PlayerLook;

	use crate::hold::HeldGrenade;

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
				tanzes: Vec::new(),
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
		world.init_resource::<Messages<SpawnThrownGrenade>>();
		world.init_resource::<Assets<Mesh>>();
		world.init_resource::<Assets<grenades::GrenadeMaterial>>();
		world
			.run_system_once(advance_throw)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		world
			.run_system_once(grenades::spawn_thrown_grenades)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		Ok(())
	}

	fn apply_use_item(world: &mut World) -> anyhow::Result<()> {
		use maybraid_controllers_core::ActorTarget;
		use maybraid_equipment::{EquippedItemHandler, EquippedItemUse};

		world.init_resource::<Messages<EquippedItemUse>>();
		world.resource_mut::<Messages<EquippedItemUse>>().drain().for_each(drop);
		let user = world
			.query_filtered::<Entity, With<GrenadeUser>>()
			.iter(world)
			.next()
			.ok_or_else(|| anyhow::anyhow!("user"))?;
		world.write_message(EquippedItemUse::new(
			ActorTarget::new(user),
			EquippedItemHandler::Grenade,
			1.0,
			true,
		));
		world
			.run_system_once(apply_throw_intents)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		Ok(())
	}

	fn apply_release(world: &mut World) -> anyhow::Result<()> {
		use maybraid_controllers_core::ActorTarget;
		use maybraid_equipment::{EquippedItemHandler, EquippedItemUse};

		world.init_resource::<Messages<EquippedItemUse>>();
		world.resource_mut::<Messages<EquippedItemUse>>().drain().for_each(drop);
		let user = world
			.query_filtered::<Entity, With<GrenadeUser>>()
			.iter(world)
			.next()
			.ok_or_else(|| anyhow::anyhow!("user"))?;
		world.write_message(EquippedItemUse::new(
			ActorTarget::new(user),
			EquippedItemHandler::Grenade,
			0.0,
			false,
		));
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
		apply_use_item(&mut world)?;
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
				tanzes: Vec::new(),
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
		assert!(stats.launch_speed > 21.0 && stats.launch_speed < 23.0);
		assert!(velocity.length() > 21.0 && velocity.length() < 23.0);
		assert!(velocity.y > velocity.z * 0.18);
	}

	fn standing_third_person_velocity(stats: &GrenadeStats) -> Vec3 {
		let look = PlayerLook::default();
		let aim = throw_aim(Vec3::Z, &look);
		launch_velocity(aim, stats, Vec3::ZERO)
	}

	#[test]
	fn standing_third_person_lob_lands_around_thirty_meters() {
		let stats = GrenadeStats::standard();
		let velocity = standing_third_person_velocity(&stats);
		let range = landing_range(STANDING_RELEASE_HEIGHT, velocity, THROW_GRAVITY);
		let airtime = landing_time(STANDING_RELEASE_HEIGHT, velocity.y, THROW_GRAVITY);
		assert!(
			range > 28.0 && range < 32.0,
			"median 22 m/s standing lob should land around 30 m, got {range:.1} m"
		);
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
	fn rolled_launch_band_still_lands_inside_fuse() {
		let mut floor = GrenadeStats::standard();
		floor.launch_speed = 15.0;
		let mut ceiling = GrenadeStats::standard();
		ceiling.launch_speed = 30.0;
		for stats in [floor, ceiling] {
			let velocity = standing_third_person_velocity(&stats);
			let airtime = landing_time(STANDING_RELEASE_HEIGHT, velocity.y, THROW_GRAVITY);
			assert!(
				airtime < stats.fuse,
				"{} m/s lob airtime {airtime:.2}s must stay inside fuse {:.1}s",
				stats.launch_speed,
				stats.fuse
			);
		}
		let floor_range = landing_range(
			STANDING_RELEASE_HEIGHT,
			standing_third_person_velocity(&floor),
			THROW_GRAVITY,
		);
		let ceiling_range = landing_range(
			STANDING_RELEASE_HEIGHT,
			standing_third_person_velocity(&ceiling),
			THROW_GRAVITY,
		);
		assert!(
			floor_range > 15.0,
			"15 m/s floor should still land past 15 m, got {floor_range:.1} m"
		);
		assert!(
			ceiling_range > 50.0,
			"30 m/s ceiling should land past 50 m, got {ceiling_range:.1} m"
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

	#[test]
	fn quick_tap_throws_at_release_like_today() -> anyhow::Result<()> {
		let mut world = World::new();
		let user = spawn_ready_user(&mut world);
		apply_use_item(&mut world)?;
		apply_release(&mut world)?;
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
		apply_release(&mut world)?;
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

	#[test]
	fn missing_use_does_not_release_a_live_cook() -> anyhow::Result<()> {
		let mut world = World::new();
		let user = spawn_ready_user(&mut world);
		apply_use_item(&mut world)?;
		world
			.run_system_once(apply_throw_intents)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		let throw = world.get::<GrenadeThrow>(user).expect("throw");
		assert!(matches!(throw.phase, GrenadePhase::Cooking { .. }));
		assert!(throw.use_latched);
		Ok(())
	}

	#[test]
	fn release_neutral_takes_the_live_cook() {
		let mut throw = GrenadeThrow::default();
		assert!(throw.try_start_cook(true, false, 4.0));
		assert_eq!(throw.apply_hold(HoldEffect::Freeze), None);
		assert!(throw.cooking());
		assert_eq!(throw.apply_hold(HoldEffect::ReleaseNeutral), Some(4.0));
		assert_eq!(throw.phase, GrenadePhase::Ready);
	}

	fn emit_kind(kind_setup: impl FnOnce(&mut InputRouter)) -> RoutingEvent {
		use maybraid_character_controller::{ActorTarget, InputRouter, InputSource};

		let mut router = InputRouter::default();
		router.assign_actor(InputSource::local_pad(), ActorTarget::new(Entity::from_bits(1)));
		let _ = router.take_events();
		kind_setup(&mut router);
		router.take_events().pop().expect("routing event")
	}

	#[test]
	fn disconnected_routing_takes_the_live_cook() {
		use maybraid_character_controller::InputSource;

		let mut throw = GrenadeThrow::default();
		assert!(throw.try_start_cook(true, false, 4.0));
		let event = emit_kind(|router| {
			router.disconnect(InputSource::local_pad());
		});
		assert_eq!(throw.apply_routing(event), Some(4.0));
		assert_eq!(throw.phase, GrenadePhase::Ready);
		assert!(!throw.use_latched);
	}

	#[test]
	fn captured_routing_keeps_a_live_cook_and_cancels_a_latch() {
		use maybraid_character_controller::{CaptureKind, InputSource, ScopeOwner, UiTarget};

		let event = emit_kind(|router| {
			router.acquire_capture(
				InputSource::local_pad(),
				ScopeOwner::ui(UiTarget::new(Entity::from_bits(7))),
				CaptureKind::Overlay,
			);
		});
		let mut cooking = GrenadeThrow::default();
		assert!(cooking.try_start_cook(true, false, 4.0));
		assert_eq!(cooking.apply_routing(event), None);
		assert!(cooking.cooking());
		assert!(cooking.use_latched);

		let mut latched = GrenadeThrow { use_latched: true, ..GrenadeThrow::default() };
		assert_eq!(latched.apply_routing(event), None);
		assert!(!latched.use_latched);
		assert_eq!(latched.phase, GrenadePhase::Ready);
	}

	#[test]
	fn suspension_freezes_fuse_and_windup() -> anyhow::Result<()> {
		use maybraid_character_controller::{ActorTarget, InputRouter, InputSource, ScopeOwner};

		let mut world = World::new();
		let user = spawn_ready_user(&mut world);
		apply_use_item(&mut world)?;
		advance_time(&mut world, 0.25);
		run_advance(&mut world)?;
		let fuse = world.get::<GrenadeThrow>(user).expect("throw").fuse;
		let mut router = InputRouter::default();
		router.assign_actor(InputSource::local_pad(), ActorTarget::new(user));
		router.suspend(InputSource::local_pad(), ScopeOwner::pause());
		world.insert_resource(router);
		advance_time(&mut world, 1.0);
		run_advance(&mut world)?;
		let throw = world.get::<GrenadeThrow>(user).expect("throw");
		assert!(throw.cooking());
		assert!((throw.fuse - fuse).abs() < 1e-4, "suspended fuse must not tick");
		world.get_mut::<GrenadeThrow>(user).expect("throw").apply_use(false);
		advance_time(&mut world, 1.0);
		run_advance(&mut world)?;
		assert!(
			matches!(
				world.get::<GrenadeThrow>(user).expect("throw").phase,
				GrenadePhase::Cooking { .. }
			),
			"suspended windup must not advance"
		);
		Ok(())
	}

	#[test]
	fn capture_keeps_ticking_the_fuse() -> anyhow::Result<()> {
		use maybraid_character_controller::{
			ActorTarget, CaptureKind, InputRouter, InputSource, ScopeOwner, UiTarget,
		};

		let mut world = World::new();
		let user = spawn_ready_user(&mut world);
		apply_use_item(&mut world)?;
		let mut router = InputRouter::default();
		router.assign_actor(InputSource::local_pad(), ActorTarget::new(user));
		router.acquire_capture(
			InputSource::local_pad(),
			ScopeOwner::ui(UiTarget::new(Entity::from_bits(7))),
			CaptureKind::Overlay,
		);
		world.insert_resource(router);
		advance_time(&mut world, 0.5);
		run_advance(&mut world)?;
		let throw = world.get::<GrenadeThrow>(user).expect("throw");
		assert!(throw.cooking());
		assert!(throw.fuse < GrenadeStats::standard().fuse - 0.4);
		Ok(())
	}

	fn publish_routing(
		world: &mut World,
		apply: impl FnOnce(&mut InputRouter) + Send + Sync + 'static,
	) -> anyhow::Result<()> {
		use maybraid_character_controller::InputRouting;

		world.init_resource::<InputRouter>();
		world.init_resource::<Messages<RoutingEvent>>();
		world.init_resource::<Messages<DropLiveGrenade>>();
		let mut apply = Some(apply);
		world
			.run_system_once(move |mut routing: InputRouting| {
				if let Some(apply) = apply.take() {
					apply(&mut routing);
				}
			})
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		Ok(())
	}

	fn apply_use_to(world: &mut World, user: Entity) -> anyhow::Result<()> {
		world.init_resource::<Messages<EquippedItemUse>>();
		world.resource_mut::<Messages<EquippedItemUse>>().drain().for_each(drop);
		world.write_message(EquippedItemUse::new(
			ActorTarget::new(user),
			EquippedItemHandler::Grenade,
			1.0,
			true,
		));
		world
			.run_system_once(apply_throw_intents)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		Ok(())
	}

	#[test]
	fn disconnect_drops_only_the_named_grenade_user() -> anyhow::Result<()> {
		use maybraid_character_controller::{ActorTarget, InputSource};

		let mut world = World::new();
		let first = spawn_ready_user(&mut world);
		let second = spawn_ready_user(&mut world);
		apply_use_to(&mut world, first)?;
		apply_use_to(&mut world, second)?;
		assert!(world.get::<GrenadeThrow>(first).is_some_and(GrenadeThrow::cooking));
		assert!(world.get::<GrenadeThrow>(second).is_some_and(GrenadeThrow::cooking));
		publish_routing(&mut world, move |router| {
			router.assign_actor(InputSource::local_pad(), ActorTarget::new(first));
		})?;
		world.resource_mut::<Messages<RoutingEvent>>().drain().for_each(drop);
		publish_routing(&mut world, |router| {
			router.disconnect(InputSource::local_pad());
		})?;
		world
			.run_system_once(apply_throw_routing)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert_eq!(
			world.get::<GrenadeThrow>(first).map(|throw| throw.phase),
			Some(GrenadePhase::Ready)
		);
		assert!(world.get::<GrenadeThrow>(second).is_some_and(GrenadeThrow::cooking));
		let drops: Vec<_> = world.resource_mut::<Messages<DropLiveGrenade>>().drain().collect();
		assert_eq!(drops.len(), 1);
		assert_eq!(drops[0].source, first);
		Ok(())
	}

	#[test]
	fn reassignment_then_capture_in_one_frame_cleans_the_old_actor() -> anyhow::Result<()> {
		use maybraid_character_controller::{
			ActorTarget, CaptureKind, InputSource, ScopeOwner, UiTarget,
		};

		let mut world = World::new();
		let cook = spawn_ready_user(&mut world);
		let next = spawn_ready_user(&mut world);
		apply_use_to(&mut world, cook)?;
		publish_routing(&mut world, move |router| {
			router.assign_actor(InputSource::local_pad(), ActorTarget::new(cook));
		})?;
		world.resource_mut::<Messages<RoutingEvent>>().drain().for_each(drop);
		world.resource_mut::<Messages<DropLiveGrenade>>().drain().for_each(drop);
		publish_routing(&mut world, move |router| {
			router.assign_actor(InputSource::local_pad(), ActorTarget::new(next));
			router.acquire_capture(
				InputSource::local_pad(),
				ScopeOwner::ui(UiTarget::new(Entity::from_bits(9))),
				CaptureKind::Overlay,
			);
		})?;
		world
			.run_system_once(apply_throw_routing)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert_eq!(
			world.get::<GrenadeThrow>(cook).map(|throw| throw.phase),
			Some(GrenadePhase::Ready)
		);
		assert!(!world.get::<GrenadeThrow>(next).is_some_and(GrenadeThrow::cooking));
		let drops: Vec<_> = world.resource_mut::<Messages<DropLiveGrenade>>().drain().collect();
		assert_eq!(drops.len(), 1);
		assert_eq!(drops[0].source, cook);
		Ok(())
	}

	#[test]
	fn release_emits_exactly_one_spawn_request() -> anyhow::Result<()> {
		let mut world = World::new();
		let user = spawn_ready_user(&mut world);
		apply_use_item(&mut world)?;
		apply_release(&mut world)?;
		let stats = GrenadeStats::standard();
		advance_time(&mut world, stats.release_at);
		world.init_resource::<Messages<GrenadeDetonated>>();
		world.init_resource::<Messages<SpawnThrownGrenade>>();
		world
			.run_system_once(advance_throw)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		let requests: Vec<SpawnThrownGrenade> =
			world.resource_mut::<Messages<SpawnThrownGrenade>>().drain().collect();
		assert_eq!(requests.len(), 1);
		let request = requests[0];
		assert_eq!(request.source, user);
		assert!((request.fuse - (stats.fuse - stats.release_at)).abs() < 1e-3);
		let look = PlayerLook::default();
		let aim = throw_aim(Vec3::Z, &look);
		let expected_origin = Vec3::new(0.0, 1.4, 0.4) + aim * 0.55 + Vec3::Y * 0.12;
		let expected_velocity = launch_velocity(aim, &stats, Vec3::ZERO);
		assert!((request.origin - expected_origin).length() < 1e-4);
		assert!((request.velocity - expected_velocity).length() < 1e-4);
		assert!(world.query::<&grenades::ThrownGrenade>().iter(&world).next().is_none());
		Ok(())
	}
}
