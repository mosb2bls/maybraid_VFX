//! Reusable combat feedback for player health, outgoing hits, and incoming damage.

mod continuous_health;
mod cycle_cue;
mod lubikan_charge;
mod lubikan_points;
mod plate;
mod score;
mod screen_pin;
mod vitals;

use bevy::prelude::*;
use continuous_health::{
	trigger_completion_flash, update_continuous_health_bar_content, ContinuousHealthConfig,
};
use damage::{DamageApplied, HeadshotBand};
use lubikan_charge::{
	ingest_lubikan_activation_finished_pulse, spawn_lubikan_charge, sync_lubikan_charge,
	LubikanActivationFinishedHudPulse,
};
use lubikan_points::{ingest_lubikan_point_flyers, update_lubikan_point_flyers};
use lubikans::LubikanSystems;
use player::Player;
use score::{ingest_combat_score, spawn_combat_score, sync_combat_score};
use vitals::{spawn_player_vitals, sync_player_vitals, vitals_fonts};

pub use continuous_health::{HealthFill, HealthTrail};
pub use cycle_cue::CombatHudInventoryCycleCategory;
pub use lubikan_charge::LubikanChargeFlyTarget;
pub use score::{CombatScore, LiveEnemies};
pub use screen_pin::ScreenPin;
pub use vitals::PlayerVitalsHealthBar;

/// When `false`, the player vitals plate stays hidden (pause / menu overlays).
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct CombatHudVisible(pub bool);

impl Default for CombatHudVisible {
	fn default() -> Self {
		Self(true)
	}
}

const INDICATOR_COUNT: usize = 8;
const INDICATOR_RADIUS: f32 = 148.0;
const INDICATOR_LIFE: f32 = 1.4;
const INDICATOR_GROUP: f32 = 0.5;
const POPUP_LIFE: f32 = 0.9;
const POPUP_RISE: f32 = 48.0;
const HIT_POINTS: u8 = 1;
const HEAD_POINTS: u8 = 2;
const DOWN_POINTS: u8 = 5;

/// Independently configurable combat feedback categories.
///
/// Health presentation is the persistent player vitals plate only. World and
/// NPC bars are not spawned.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CombatHudPlugin {
	pub hit_markers: bool,
	pub directional_damage: bool,
	/// Bottom-left name + health meter. Hidden when there is no live player.
	pub player_vitals: bool,
	/// Lubikan charge row above [`player_vitals`]. Reads [`lubikans::LubikanLedger`] only.
	pub lubikan_charge: bool,
	/// Animate [`lubikans::LubikanChargeChanged`] toward the charge anchor.
	pub lubikan_point_feedback: bool,
	/// Top-right running score, shown while a [`CombatScore`] exists.
	pub score: bool,
}

impl Default for CombatHudPlugin {
	fn default() -> Self {
		Self {
			hit_markers: true,
			directional_damage: true,
			player_vitals: false,
			lubikan_charge: false,
			lubikan_point_feedback: false,
			score: false,
		}
	}
}

impl Plugin for CombatHudPlugin {
	fn build(&self, app: &mut App) {
		let any = self.hit_markers
			|| self.directional_damage
			|| self.player_vitals
			|| self.lubikan_charge
			|| self.lubikan_point_feedback
			|| self.score;
		if !any {
			return;
		}
		if self.player_vitals || self.lubikan_charge {
			app.add_message::<lubikans::LubikanActivationFinished>();
		}
		app.insert_resource(CombatHudConfig(*self))
			.init_resource::<CombatHudVisible>()
			.init_resource::<CombatHudInventoryCycleCategory>();

		// Order inside PostUpdate only. A same-named set in Update does not
		// constrain these systems relative to Heal / Award.
		app.configure_sets(
			PostUpdate,
			CombatHudSystems::Health
				.after(LubikanSystems::Heal)
				.after(LubikanSystems::Award),
		)
		.add_systems(Startup, spawn_combat_hud)
		.add_systems(Update, sync_combat_hud_root_visibility);

		if self.player_vitals {
			configure_health_content(app);
			configure_player_vitals(app);
		}
		if self.lubikan_charge {
			configure_lubikan_charge(app);
		}
		if self.lubikan_point_feedback {
			configure_lubikan_point_feedback(app);
		}
		if self.score {
			configure_score(app);
		}
		if self.hit_markers {
			configure_hit_markers(app);
		}
		if self.directional_damage {
			configure_directional_damage(app);
		}
	}
}

/// Shared meter content for the persistent player health widget.
fn configure_health_content(app: &mut App) {
	app.init_resource::<ContinuousHealthConfig>().add_systems(
			PostUpdate,
			(update_continuous_health_bar_content, trigger_completion_flash)
				.chain()
				.in_set(CombatHudSystems::Health)
				.after(sync_player_vitals),
		);
}

/// Bottom-left player name + health bar (reuses continuous health bar).
fn configure_player_vitals(app: &mut App) {
	app.add_systems(PostUpdate, sync_player_vitals.in_set(CombatHudSystems::Health));
}

/// Lubikan charge plate above player vitals.
fn configure_lubikan_charge(app: &mut App) {
	app.init_resource::<LubikanActivationFinishedHudPulse>();
	app.add_systems(
		PostUpdate,
		(ingest_lubikan_activation_finished_pulse, sync_lubikan_charge)
			.chain()
			.in_set(CombatHudSystems::Health),
	);
}

/// Animated point feedback flying toward charge anchor.
fn configure_lubikan_point_feedback(app: &mut App) {
	app.add_systems(
		PostUpdate,
		(ingest_lubikan_point_flyers, update_lubikan_point_flyers)
			.chain()
			.in_set(CombatHudSystems::Health),
	);
}

/// Top-right running score plate.
fn configure_score(app: &mut App) {
	app.add_systems(PostUpdate, sync_combat_score.in_set(CombatHudSystems::Health))
		.add_systems(
			PostUpdate,
			ingest_combat_score
				.after(damage::DamageSystems::Apply)
				.before(damage::DamageSystems::Down),
		);
}

/// Popup +N markers at hit points.
fn configure_hit_markers(app: &mut App) {
	app.add_systems(Update, update_hit_markers)
		.add_systems(PostUpdate, ingest_hit_markers.after(damage::DamageSystems::Down));
}

/// Directional damage indicators around screen center.
fn configure_directional_damage(app: &mut App) {
	app.init_resource::<DamageTicks>()
		.add_systems(Update, update_directional_damage)
		.add_systems(PostUpdate, ingest_directional_damage.after(damage::DamageSystems::Down));
}

#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CombatHudSystems {
	Health,
}

#[derive(Resource, Clone, Copy)]
struct CombatHudConfig(CombatHudPlugin);

#[derive(Component)]
struct CombatHudRoot;

#[derive(Component)]
struct DamageTick {
	slot: usize,
}

#[derive(Component)]
struct HitMarker {
	world: Vec3,
	born: f32,
	points: u8,
}

#[derive(Clone, Copy)]
struct LiveTick {
	origin: Vec3,
	born: f32,
}

#[derive(Resource, Default)]
struct DamageTicks([Option<LiveTick>; INDICATOR_COUNT]);

fn sync_combat_hud_root_visibility(
	hud: Res<CombatHudVisible>,
	mut roots: Query<&mut Visibility, With<CombatHudRoot>>,
) {
	let next = if hud.0 { Visibility::Inherited } else { Visibility::Hidden };
	for mut visibility in &mut roots {
		if *visibility != next {
			*visibility = next;
		}
	}
}

fn spawn_combat_hud(
	mut commands: Commands,
	config: Res<CombatHudConfig>,
	asset_server: Option<Res<AssetServer>>,
) {
	commands
		.spawn((
			Name::new("combat-hud"),
			CombatHudRoot,
			Node {
				position_type: PositionType::Absolute,
				left: Val::Px(0.0),
				top: Val::Px(0.0),
				width: Val::Percent(100.0),
				height: Val::Percent(100.0),
				..default()
			},
			Visibility::Inherited,
			GlobalZIndex(i32::MAX - 8),
			Pickable::IGNORE,
		))
		.with_children(|root| {
			if config.0.directional_damage {
				spawn_directional_damage_ring(root);
			}
			let fonts = vitals_fonts(asset_server.as_deref());
			let health_config = ContinuousHealthConfig::default();
			if config.0.lubikan_charge {
				spawn_lubikan_charge(root, &fonts, &health_config);
			}
			if config.0.player_vitals {
				spawn_player_vitals(root, &fonts, &health_config, Entity::PLACEHOLDER);
			}
			if config.0.score {
				spawn_combat_score(root, &fonts);
			}
		});
}

fn spawn_directional_damage_ring(root: &mut ChildSpawnerCommands) {
	root.spawn((
		Node {
			position_type: PositionType::Absolute,
			left: Val::Percent(50.0),
			top: Val::Percent(50.0),
			width: Val::Px(0.0),
			height: Val::Px(0.0),
			..default()
		},
		Pickable::IGNORE,
	))
	.with_children(|ring| {
		for slot in 0..INDICATOR_COUNT {
			ring.spawn((
				DamageTick { slot },
				Node {
					position_type: PositionType::Absolute,
					left: Val::Px(-7.0),
					top: Val::Px(-22.0),
					width: Val::Px(14.0),
					height: Val::Px(28.0),
					border: UiRect::all(Val::Px(2.0)),
					border_radius: BorderRadius::all(Val::Px(2.0)),
					..default()
				},
				BackgroundColor(Color::srgba(1.0, 0.22, 0.16, 0.0)),
				BorderColor::all(Color::srgba(1.0, 0.85, 0.7, 0.0)),
				UiTransform::IDENTITY,
				Visibility::Hidden,
				Pickable::IGNORE,
			));
		}
	});
}

fn ingest_directional_damage(
	time: Res<Time>,
	mut hits: MessageReader<DamageApplied>,
	players: Query<(), With<Player>>,
	transforms: Query<&GlobalTransform>,
	cameras: Query<&GlobalTransform, With<Camera3d>>,
	mut ticks: ResMut<DamageTicks>,
) {
	let Ok(camera) = cameras.single() else {
		return;
	};
	let now = time.elapsed_secs();
	for hit in hits.read() {
		if players.get(hit.target).is_err() {
			continue;
		}
		let origin = hit
			.source
			.and_then(|source| transforms.get(source).ok())
			.map(GlobalTransform::translation)
			.unwrap_or(hit.point);
		assign_tick(&mut ticks.0, camera, origin, now);
	}
}

fn update_directional_damage(
	time: Res<Time>,
	cameras: Query<&GlobalTransform, With<Camera3d>>,
	mut ticks: ResMut<DamageTicks>,
	mut nodes: Query<(
		&DamageTick,
		&mut UiTransform,
		&mut BackgroundColor,
		&mut BorderColor,
		&mut Visibility,
	)>,
) {
	let now = time.elapsed_secs();
	for slot in &mut ticks.0 {
		if slot.is_some_and(|tick| now >= tick.born + INDICATOR_LIFE) {
			*slot = None;
		}
	}
	let Ok(camera) = cameras.single() else {
		for (_, _, _, _, mut visibility) in &mut nodes {
			*visibility = Visibility::Hidden;
		}
		return;
	};
	for (tick, mut transform, mut fill, mut border, mut visibility) in &mut nodes {
		let Some(live) = ticks.0[tick.slot] else {
			*visibility = Visibility::Hidden;
			continue;
		};
		let age = now - live.born;
		let alpha = indicator_alpha(age);
		let yaw = incoming_yaw(camera, live.origin);
		let offset = indicator_offset(yaw, INDICATOR_RADIUS);
		transform.translation = Val2::px(offset.x, offset.y);
		transform.rotation = Rot2::radians(yaw);
		fill.0 = Color::srgba(1.0, 0.22, 0.16, alpha);
		*border = BorderColor::all(Color::srgba(1.0, 0.9, 0.75, alpha * 0.9));
		*visibility = Visibility::Visible;
	}
}

fn ingest_hit_markers(
	time: Res<Time>,
	mut hits: MessageReader<DamageApplied>,
	players: Query<Entity, With<Player>>,
	targets: Query<(&GlobalTransform, Option<&HeadshotBand>)>,
	hud: Query<Entity, With<CombatHudRoot>>,
	mut commands: Commands,
) {
	let Ok(player) = players.single() else {
		return;
	};
	let Ok(hud) = hud.single() else {
		return;
	};
	let now = time.elapsed_secs();
	for hit in hits.read() {
		if hit.source != Some(player) {
			continue;
		}
		let points = hit_points(targets.get(hit.target).ok(), hit.point);
		spawn_hit_marker(&mut commands, hud, hit.point, now, points);
		if hit.remaining <= 0.0 {
			spawn_hit_marker(&mut commands, hud, hit.point + Vec3::Y * 0.16, now, DOWN_POINTS);
		}
	}
}

/// Headshot points inside the target's [`HeadshotBand`], hit points elsewhere.
fn hit_points(target: Option<(&GlobalTransform, Option<&HeadshotBand>)>, point: Vec3) -> u8 {
	let head = target
		.is_some_and(|(transform, band)| band.is_some_and(|band| band.contains(transform, point)));
	if head {
		HEAD_POINTS
	} else {
		HIT_POINTS
	}
}

fn spawn_hit_marker(commands: &mut Commands, hud: Entity, world: Vec3, born: f32, points: u8) {
	let (fill, size) = hit_marker_style(points);
	commands.entity(hud).with_children(|root| {
		root.spawn((
			Name::new("hit-marker"),
			HitMarker { world, born, points },
			Node {
				position_type: PositionType::Absolute,
				left: Val::Px(0.0),
				top: Val::Px(0.0),
				..default()
			},
			Text::new(hit_marker_label(points)),
			TextFont { font_size: FontSize::Px(size), ..default() },
			TextColor(fill),
			Pickable::IGNORE,
		));
	});
}

fn update_hit_markers(
	time: Res<Time>,
	cameras: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
	mut markers: Query<(Entity, &HitMarker, &mut Node, &mut TextColor, &mut Visibility)>,
	mut commands: Commands,
) {
	let now = time.elapsed_secs();
	let Ok((camera, camera_transform)) = cameras.single() else {
		for (_, _, _, _, mut visibility) in &mut markers {
			*visibility = Visibility::Hidden;
		}
		return;
	};
	for (entity, marker, mut node, mut color, mut visibility) in &mut markers {
		let age = now - marker.born;
		if age >= POPUP_LIFE {
			commands.entity(entity).try_despawn();
			continue;
		}
		let Ok(screen) = camera.world_to_viewport(camera_transform, marker.world) else {
			*visibility = Visibility::Hidden;
			continue;
		};
		let rise = age / POPUP_LIFE * POPUP_RISE;
		node.left = Val::Px(screen.x - 16.0);
		node.top = Val::Px(screen.y - 18.0 - rise);
		color.0 = hit_marker_style(marker.points).0.with_alpha(marker_alpha(age));
		*visibility = Visibility::Visible;
	}
}

fn hit_marker_label(points: u8) -> String {
	format!("+{points}")
}

fn hit_marker_style(points: u8) -> (Color, f32) {
	if points >= DOWN_POINTS {
		(Color::srgb(1.0, 0.82, 0.28), 28.0)
	} else if points >= HEAD_POINTS {
		(Color::srgb(0.42, 0.86, 1.0), 24.0)
	} else {
		(Color::srgb(0.92, 0.98, 1.0), 22.0)
	}
}

fn marker_alpha(age: f32) -> f32 {
	(1.0 - age / POPUP_LIFE).clamp(0.0, 1.0)
}

fn assign_tick(
	slots: &mut [Option<LiveTick>; INDICATOR_COUNT],
	camera: &GlobalTransform,
	origin: Vec3,
	now: f32,
) {
	let yaw = incoming_yaw(camera, origin);
	let mut best_empty = None;
	let mut oldest = 0;
	let mut oldest_born = f32::MAX;
	for (index, slot) in slots.iter_mut().enumerate() {
		match slot {
			Some(live)
				if angle_delta(incoming_yaw(camera, live.origin), yaw) <= INDICATOR_GROUP =>
			{
				live.origin = origin;
				live.born = now;
				return;
			}
			Some(live) if live.born < oldest_born => {
				oldest_born = live.born;
				oldest = index;
			}
			None if best_empty.is_none() => best_empty = Some(index),
			_ => {}
		}
	}
	let index = best_empty.unwrap_or(oldest);
	slots[index] = Some(LiveTick { origin, born: now });
}

/// Camera-local yaw: 0 is forward / screen-up, positive is clockwise (right).
fn incoming_yaw(camera: &GlobalTransform, origin: Vec3) -> f32 {
	let local = camera.rotation().inverse() * (origin - camera.translation());
	local.x.atan2(-local.z)
}

fn indicator_offset(yaw: f32, radius: f32) -> Vec2 {
	Vec2::new(yaw.sin() * radius, -yaw.cos() * radius)
}

fn indicator_alpha(age: f32) -> f32 {
	if age <= 0.2 {
		1.0
	} else {
		(1.0 - (age - 0.2) / (INDICATOR_LIFE - 0.2)).clamp(0.0, 1.0)
	}
}

fn angle_delta(a: f32, b: f32) -> f32 {
	let mut delta = (a - b).abs();
	if delta > std::f32::consts::PI {
		delta = std::f32::consts::TAU - delta;
	}
	delta
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::continuous_health::ContinuousHealthBar;
	use crate::vitals::PlayerVitalsHealthBar;
	use damage::Health;

	#[test]
	fn categories_default_to_enabled() {
		assert_eq!(
			CombatHudPlugin::default(),
			CombatHudPlugin {
				hit_markers: true,
				directional_damage: true,
				player_vitals: false,
				lubikan_charge: false,
				lubikan_point_feedback: false,
				score: false,
			}
		);
	}

	#[test]
	fn enabled_categories_spawn_their_hud_elements() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.add_message::<DamageApplied>()
			.add_plugins(CombatHudPlugin::default());
		app.update();
		let world = app.world_mut();
		assert_eq!(world.query::<&CombatHudRoot>().iter(world).count(), 1);
		assert_eq!(world.query::<&DamageTick>().iter(world).count(), INDICATOR_COUNT);
		assert_eq!(world.query::<&vitals::PlayerVitalsRoot>().iter(world).count(), 0);
		assert_eq!(
			world.query::<&ContinuousHealthBar>().iter(world).count(),
			0,
			"default combat HUD must not spawn world or NPC health bars"
		);
	}

	#[test]
	fn player_vitals_spawn_name_and_health_bar() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins).add_plugins(CombatHudPlugin {
			hit_markers: false,
			directional_damage: false,
			player_vitals: true,
			lubikan_charge: false,
			lubikan_point_feedback: false,
			score: false,
		});
		app.update();
		let world = app.world_mut();
		assert_eq!(world.query::<&vitals::PlayerVitalsRoot>().iter(world).count(), 1);
		// Check continuous health bar spawned
		assert_eq!(
			world.query::<&ContinuousHealthBar>().iter(world).count(),
			1,
			"should have one continuous health bar"
		);
	}

	fn vitals_app() -> App {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins).add_plugins(CombatHudPlugin {
			hit_markers: false,
			directional_damage: false,
			player_vitals: true,
			lubikan_charge: false,
			lubikan_point_feedback: false,
			score: false,
		});
		app
	}

	fn vitals_visibility(world: &mut World) -> anyhow::Result<Visibility> {
		world
			.query_filtered::<&Visibility, With<vitals::PlayerVitalsRoot>>()
			.iter(world)
			.next()
			.copied()
			.ok_or_else(|| anyhow::anyhow!("vitals root"))
	}

	#[test]
	fn player_vitals_hide_when_the_hud_is_closed() -> anyhow::Result<()> {
		let mut app = vitals_app();
		app.update();
		app.world_mut()
			.spawn((Player, Health { current: 40.0, max: 100.0 }, Name::new("Ada")));
		app.world_mut().insert_resource(CombatHudVisible(false));
		app.update();
		assert_eq!(vitals_visibility(app.world_mut())?, Visibility::Hidden);
		Ok(())
	}

	#[test]
	fn player_vitals_show_when_playing() -> anyhow::Result<()> {
		let mut app = vitals_app();
		app.update();
		app.world_mut()
			.spawn((Player, Health { current: 40.0, max: 100.0 }, Name::new("Ada")));
		app.world_mut().insert_resource(CombatHudVisible(true));
		app.update();
		assert_eq!(vitals_visibility(app.world_mut())?, Visibility::Visible);
		Ok(())
	}

	#[test]
	fn player_vitals_stay_hidden_without_a_player() -> anyhow::Result<()> {
		let mut app = vitals_app();
		app.world_mut().insert_resource(CombatHudVisible(true));
		app.update();
		assert_eq!(vitals_visibility(app.world_mut())?, Visibility::Hidden);
		Ok(())
	}

	#[test]
	fn player_vitals_follow_the_named_player() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins).add_plugins(CombatHudPlugin {
			hit_markers: false,
			directional_damage: false,
			player_vitals: true,
			lubikan_charge: false,
			lubikan_point_feedback: false,
			score: false,
		});
		app.update();
		app.world_mut()
			.spawn((Player, Health { current: 40.0, max: 100.0 }, Name::new("Ada")));
		app.update();
		let world = app.world_mut();
		let name = world
			.query::<&Text>()
			.iter(world)
			.find(|text| text.0 == "Ada")
			.expect("character name");
		assert_eq!(name.0, "Ada");
		// Check continuous health bar exists
		let bar_count = world.query::<&ContinuousHealthBar>().iter(world).count();
		assert_eq!(bar_count, 1, "should have one continuous health bar");
	}

	#[test]
	fn player_vitals_rebind_across_respawn_without_world_bars() -> anyhow::Result<()> {
		let mut app = vitals_app();
		app.update();
		let first = app
			.world_mut()
			.spawn((Player, Health { current: 40.0, max: 100.0 }, Name::new("Ada")))
			.id();
		let npc = app
			.world_mut()
			.spawn((player::Npc, Health { current: 80.0, max: 100.0 }, Name::new("Pack")))
			.id();
		app.update();
		let bar = {
			let world = app.world_mut();
			assert_eq!(
				world.query::<&ContinuousHealthBar>().iter(world).count(),
				1,
				"NPCs must not receive health bars"
			);
			world
				.query_filtered::<(Entity, &ContinuousHealthBar), With<PlayerVitalsHealthBar>>()
				.iter(world)
				.next()
				.map(|(entity, bar)| (entity, bar.target))
				.ok_or_else(|| anyhow::anyhow!("player vitals bar"))?
		};
		assert_eq!(bar.1, first);
		assert!(app.world().get_entity(npc).is_ok());

		app.world_mut().entity_mut(first).remove::<Player>();
		app.update();
		assert_eq!(vitals_visibility(app.world_mut())?, Visibility::Hidden);
		let second = app
			.world_mut()
			.spawn((Player, Health { current: 100.0, max: 100.0 }, Name::new("Ada")))
			.id();
		app.update();
		assert_eq!(vitals_visibility(app.world_mut())?, Visibility::Visible);
		let rebound = {
			let world = app.world_mut();
			assert_eq!(world.query::<&ContinuousHealthBar>().iter(world).count(), 1);
			world
				.query_filtered::<(Entity, &ContinuousHealthBar), With<PlayerVitalsHealthBar>>()
				.iter(world)
				.next()
				.map(|(entity, bar)| (entity, bar.target))
				.ok_or_else(|| anyhow::anyhow!("rebound vitals bar"))?
		};
		assert_eq!(rebound.0, bar.0, "the player widget is persistent");
		assert_eq!(rebound.1, second);
		Ok(())
	}

	#[test]
	fn forward_damage_sits_above_center() {
		let camera = GlobalTransform::from(
			Transform::from_translation(Vec3::ZERO).looking_to(Dir3::NEG_Z, Dir3::Y),
		);
		let offset = indicator_offset(incoming_yaw(&camera, Vec3::NEG_Z * 8.0), 100.0);
		assert!(offset.y < -90.0, "{offset:?}");
		assert!(offset.x.abs() < 8.0, "{offset:?}");
	}

	fn score_app() -> App {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins).add_message::<DamageApplied>().add_plugins(
			CombatHudPlugin {
				hit_markers: false,
				directional_damage: false,
				player_vitals: false,
				lubikan_charge: false,
				lubikan_point_feedback: false,
				score: true,
			},
		);
		app.update();
		app
	}

	fn applied(target: Entity, source: Option<Entity>, remaining: f32) -> DamageApplied {
		DamageApplied { target, source, amount: 10.0, applied: 10.0, remaining, point: Vec3::ZERO }
	}

	#[test]
	fn score_counts_the_players_hits_downs_and_deaths() -> anyhow::Result<()> {
		let mut app = score_app();
		app.world_mut().insert_resource(CombatScore::default());
		let player = app.world_mut().spawn(Player).id();
		let brawler = app.world_mut().spawn(GlobalTransform::default()).id();
		let rival = app.world_mut().spawn(GlobalTransform::default()).id();
		for hit in [
			applied(brawler, Some(player), 20.0),
			applied(brawler, Some(player), 0.0),
			applied(rival, Some(brawler), 0.0),
			applied(player, Some(rival), 0.0),
		] {
			app.world_mut().write_message(hit);
		}
		app.update();
		let score = *app.world().resource::<CombatScore>();
		assert_eq!((score.downs, score.deaths), (1, 1));
		assert_eq!((score.streak, score.best_streak), (0, 1));

		app.update();
		let world = app.world_mut();
		let shown = world
			.query_filtered::<&Visibility, With<score::CombatScoreRoot>>()
			.iter(world)
			.next()
			.copied()
			.ok_or_else(|| anyhow::anyhow!("score root"))?;
		assert_eq!(shown, Visibility::Visible);
		assert!(world.query::<&Text>().iter(world).any(|text| text.0 == "Downs  1"));
		Ok(())
	}

	#[test]
	fn score_stays_hidden_and_idle_without_a_tally() -> anyhow::Result<()> {
		let mut app = score_app();
		let player = app.world_mut().spawn(Player).id();
		let brawler = app.world_mut().spawn(GlobalTransform::default()).id();
		app.world_mut().write_message(applied(brawler, Some(player), 0.0));
		app.update();
		let world = app.world_mut();
		assert!(world.get_resource::<CombatScore>().is_none());
		let shown = world
			.query_filtered::<&Visibility, With<score::CombatScoreRoot>>()
			.iter(world)
			.next()
			.copied()
			.ok_or_else(|| anyhow::anyhow!("score root"))?;
		assert_eq!(shown, Visibility::Hidden);
		Ok(())
	}

	#[test]
	fn hit_head_and_down_markers_keep_ffa_scores() {
		assert_eq!(hit_marker_label(HIT_POINTS), "+1");
		assert_eq!(hit_marker_label(HEAD_POINTS), "+2");
		assert_eq!(hit_marker_label(DOWN_POINTS), "+5");
	}

	#[test]
	fn player_sourced_blast_spawns_a_hit_marker() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins).add_message::<DamageApplied>().add_plugins(
			CombatHudPlugin {
				hit_markers: true,
				directional_damage: false,
				player_vitals: false,
				lubikan_charge: false,
				lubikan_point_feedback: false,
				score: false,
			},
		);
		app.update();
		let player = app.world_mut().spawn(Player).id();
		let target = app.world_mut().spawn(GlobalTransform::default()).id();
		app.world_mut().write_message(DamageApplied {
			target,
			source: Some(player),
			amount: 40.0,
			applied: 40.0,
			remaining: 60.0,
			point: Vec3::new(1.0, 1.2, 0.5),
		});
		app.update();
		let world = app.world_mut();
		assert!(
			world.query::<&Text>().iter(world).any(|text| text.0 == "+1"),
			"player-thrown blast must raise a hit marker"
		);
	}

	fn lubikan_app() -> App {
		let mut app = App::new();
		lubikans::fixtures::install_interface(&mut app);
		app.add_plugins(MinimalPlugins).add_plugins(CombatHudPlugin {
			hit_markers: false,
			directional_damage: false,
			player_vitals: true,
			lubikan_charge: true,
			lubikan_point_feedback: false,
			score: false,
		});
		app
	}

	#[test]
	fn lubikan_charge_shares_vitals_visibility() -> anyhow::Result<()> {
		let mut app = lubikan_app();
		app.update();
		let owner = lubikans::fixtures::spawn_self_owner(app.world_mut());
		app.world_mut()
			.entity_mut(owner)
			.insert((Player, Health { current: 50.0, max: 100.0 }));
		app.world_mut().insert_resource(CombatHudVisible(false));
		app.update();
		let world = app.world_mut();
		let hidden = world
			.query_filtered::<&Visibility, With<lubikan_charge::LubikanChargeRoot>>()
			.iter(world)
			.next()
			.copied()
			.ok_or_else(|| anyhow::anyhow!("lubikan root"))?;
		assert_eq!(hidden, Visibility::Hidden);
		Ok(())
	}

	#[test]
	fn lubikan_charge_reads_ledger_without_mutating_charge() -> anyhow::Result<()> {
		let mut app = lubikan_app();
		app.update();
		let owner = lubikans::fixtures::spawn_self_owner(app.world_mut());
		app.world_mut().entity_mut(owner).insert((Player, Health::default()));
		{
			let mut ledger = app
				.world_mut()
				.get_mut::<lubikans::LubikanLedger>(owner)
				.ok_or_else(|| anyhow::anyhow!("ledger"))?;
			ledger.apply_to_equipped(80.0);
		}
		app.world_mut().insert_resource(CombatHudVisible(true));
		app.update();
		let world = app.world_mut();
		let stored = world
			.get::<lubikans::LubikanLedger>(owner)
			.map(|ledger| ledger.equipped().charge().current());
		assert_eq!(stored, Some(80.0));
		assert!(world.query::<&Text>().iter(world).any(|text| text.0 == "80"));
		Ok(())
	}

	#[test]
	fn lubikan_charge_catches_up_after_hud_was_hidden() -> anyhow::Result<()> {
		let mut app = lubikan_app();
		app.update();
		let owner = lubikans::fixtures::spawn_self_owner(app.world_mut());
		app.world_mut().entity_mut(owner).insert((Player, Health::default()));
		{
			let mut ledger = app
				.world_mut()
				.get_mut::<lubikans::LubikanLedger>(owner)
				.ok_or_else(|| anyhow::anyhow!("ledger"))?;
			ledger.apply_to_equipped(12.0);
		}
		app.world_mut().insert_resource(CombatHudVisible(false));
		app.update();
		{
			let mut ledger = app
				.world_mut()
				.get_mut::<lubikans::LubikanLedger>(owner)
				.ok_or_else(|| anyhow::anyhow!("ledger"))?;
			ledger.apply_to_equipped(33.0);
		}
		app.world_mut().insert_resource(CombatHudVisible(true));
		app.update();
		let world = app.world_mut();
		assert_eq!(
			world
				.get::<lubikans::LubikanLedger>(owner)
				.map(|ledger| ledger.equipped().charge().current()),
			Some(45.0)
		);
		assert!(
			world.query::<&Text>().iter(world).any(|text| text.0 == "45"),
			"charge plate must reflect awards applied while the HUD was hidden"
		);
		Ok(())
	}

	#[test]
	fn lubikan_charge_shows_banked_count_when_stock_exceeds_one() -> anyhow::Result<()> {
		let mut app = lubikan_app();
		app.update();
		let owner = lubikans::fixtures::spawn_self_owner(app.world_mut());
		app.world_mut().entity_mut(owner).insert((Player, Health::default()));
		let extra = lubikans::LubikanDefinition::new(
			lubikans::LubikanDefinitionId::from_raw(9),
			25.0,
			25.0,
		)
		.with_max_stock(3);
		{
			app.world_mut().resource_mut::<lubikans::LubikanCatalog>().register(extra);
			let mut ledger = app
				.world_mut()
				.get_mut::<lubikans::LubikanLedger>(owner)
				.ok_or_else(|| anyhow::anyhow!("ledger"))?;
			let other = ledger.ensure_owned(extra);
			ledger.apply_to(other, 50.0);
			if !ledger.select(Some(other)) {
				return Err(anyhow::anyhow!("select stacked account"));
			}
		}
		app.world_mut().insert_resource(CombatHudVisible(true));
		app.update();
		let world = app.world_mut();
		assert!(
			world.query::<&Text>().iter(world).any(|text| text.0 == "Lubikan 9  2/3"),
			"multi-stock charge row must show banked / max"
		);
		assert_eq!(
			world
				.get::<lubikans::LubikanLedger>(owner)
				.map(|ledger| ledger.equipped().charge().current()),
			Some(50.0),
			"HUD must not award or clamp charge"
		);
		Ok(())
	}
}
