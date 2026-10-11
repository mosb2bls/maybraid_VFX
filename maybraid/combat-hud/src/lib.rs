//! Reusable combat feedback for player health, outgoing hits, and incoming damage.

mod plate;
mod score;
mod screen_pin;
mod vitals;

use bevy::prelude::*;
use damage::{DamageApplied, Downed, HeadshotBand, Health};
use player::{LocomotionCapsule, Npc, Player};
use score::{ingest_combat_score, spawn_combat_score, sync_combat_score};
use vitals::{spawn_player_vitals, sync_player_vitals, vitals_fonts};

pub use score::{CombatScore, LiveEnemies};
pub use screen_pin::ScreenPin;

/// When `false`, the player vitals plate stays hidden (pause / menu overlays).
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct CombatHudVisible(pub bool);

impl Default for CombatHudVisible {
	fn default() -> Self {
		Self(true)
	}
}

const BAR_WIDTH: f32 = 240.0;
const BAR_HEIGHT: f32 = 18.0;
const WORLD_BAR_WIDTH: f32 = 120.0;
const WORLD_BAR_HEIGHT: f32 = 8.0;
const INDICATOR_COUNT: usize = 8;
const INDICATOR_RADIUS: f32 = 148.0;
const INDICATOR_LIFE: f32 = 1.4;
const INDICATOR_GROUP: f32 = 0.5;
const HEAD_LIFT_PAD: f32 = 0.38;
const POPUP_LIFE: f32 = 0.9;
const POPUP_RISE: f32 = 48.0;
const HIT_POINTS: u8 = 1;
const HEAD_POINTS: u8 = 2;
const DOWN_POINTS: u8 = 5;

/// Independently configurable combat feedback categories.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CombatHudPlugin {
	pub health_bars: bool,
	pub hit_markers: bool,
	pub directional_damage: bool,
	/// Bottom-left name + pip bar. Independent of the top YOU/NPC [`health_bars`].
	pub player_vitals: bool,
	/// Top-right running score, shown while a [`CombatScore`] exists.
	pub score: bool,
}

impl Default for CombatHudPlugin {
	fn default() -> Self {
		Self {
			health_bars: true,
			hit_markers: true,
			directional_damage: true,
			player_vitals: false,
			score: false,
		}
	}
}

impl Plugin for CombatHudPlugin {
	fn build(&self, app: &mut App) {
		let any = self.health_bars
			|| self.hit_markers
			|| self.directional_damage
			|| self.player_vitals
			|| self.score;
		if !any {
			return;
		}
		app.insert_resource(CombatHudConfig(*self))
			.init_resource::<CombatHudVisible>()
			.add_systems(Startup, spawn_combat_hud);
		if self.health_bars {
			app.init_resource::<CombatHudOpponentTotal>().add_systems(
				Update,
				(ensure_world_health_bars, sync_health_hud, sync_world_health_bars)
					.in_set(CombatHudSystems::Health),
			);
		}
		if self.player_vitals {
			app.add_systems(Update, sync_player_vitals.in_set(CombatHudSystems::Health));
		}
		if self.score {
			app.add_systems(Update, sync_combat_score.in_set(CombatHudSystems::Health))
				.add_systems(
					PostUpdate,
					ingest_combat_score
						.after(damage::DamageSystems::Apply)
						.before(damage::DamageSystems::Down),
				);
		}
		if self.hit_markers {
			app.add_systems(Update, update_hit_markers)
				.add_systems(PostUpdate, ingest_hit_markers.after(damage::DamageSystems::Down));
		}
		if self.directional_damage {
			app.init_resource::<DamageTicks>()
				.add_systems(Update, update_directional_damage)
				.add_systems(
					PostUpdate,
					ingest_directional_damage.after(damage::DamageSystems::Down),
				);
		}
	}
}

#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CombatHudSystems {
	Health,
}

/// Optional total used to render an aggregate opponent health bar.
///
/// Leave this as `None` for a single opponent. Set it to the field size when
/// many NPCs represent one encounter.
#[derive(Resource, Default, Clone, Copy, Debug, PartialEq, Eq)]
pub struct CombatHudOpponentTotal(pub Option<usize>);

/// Optional color-coded status dot rendered above a world health bar.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct CombatHudStatusColor(pub Color);

#[derive(Resource, Clone, Copy)]
struct CombatHudConfig(CombatHudPlugin);

#[derive(Component)]
struct CombatHudRoot;

#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum CombatantHud {
	Player,
	Npc,
}

#[derive(Component)]
struct HudBarFill;

#[derive(Component)]
struct HudBarLabel;

#[derive(Component)]
struct WorldHudFill;

#[derive(Component)]
struct WorldHudLabel;

#[derive(Component)]
struct WorldHudStatus;

#[derive(Component)]
struct WorldHealthAnchor {
	target: Entity,
}

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
			GlobalZIndex(i32::MAX - 8),
			Pickable::IGNORE,
		))
		.with_children(|root| {
			if config.0.health_bars {
				spawn_screen_health_bars(root);
			}
			if config.0.directional_damage {
				spawn_directional_damage_ring(root);
			}
			let fonts = vitals_fonts(asset_server.as_deref());
			if config.0.player_vitals {
				spawn_player_vitals(root, &fonts);
			}
			if config.0.score {
				spawn_combat_score(root, &fonts);
			}
		});
}

fn spawn_screen_health_bars(root: &mut ChildSpawnerCommands) {
	root.spawn(Node {
		position_type: PositionType::Absolute,
		top: Val::Px(16.0),
		left: Val::Px(16.0),
		right: Val::Px(16.0),
		justify_content: JustifyContent::SpaceBetween,
		align_items: AlignItems::FlexStart,
		column_gap: Val::Px(24.0),
		..default()
	})
	.with_children(|row| {
		spawn_screen_bar(row, CombatantHud::Player, "YOU");
		spawn_screen_bar(row, CombatantHud::Npc, "NPC");
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

fn spawn_screen_bar(parent: &mut ChildSpawnerCommands, kind: CombatantHud, title: &'static str) {
	parent
		.spawn((
			Node {
				flex_direction: FlexDirection::Column,
				row_gap: Val::Px(4.0),
				padding: UiRect::all(Val::Px(8.0)),
				min_width: Val::Px(BAR_WIDTH + 16.0),
				border_radius: BorderRadius::all(Val::Px(4.0)),
				..default()
			},
			BackgroundColor(Color::srgba(0.04, 0.05, 0.07, 0.72)),
		))
		.with_children(|cluster| {
			cluster.spawn((
				kind,
				HudBarLabel,
				Text::new(format!("{title}  --")),
				TextFont { font_size: FontSize::Px(16.0), ..default() },
				TextColor(Color::srgb(0.95, 0.97, 1.0)),
			));
			cluster
				.spawn((
					Node {
						width: Val::Px(BAR_WIDTH),
						height: Val::Px(BAR_HEIGHT),
						padding: UiRect::all(Val::Px(2.0)),
						border_radius: BorderRadius::all(Val::Px(3.0)),
						..default()
					},
					BackgroundColor(Color::srgba(0.08, 0.09, 0.1, 0.95)),
				))
				.with_children(|track| {
					track.spawn((
						kind,
						HudBarFill,
						Node {
							width: Val::Percent(100.0),
							height: Val::Percent(100.0),
							border_radius: BorderRadius::all(Val::Px(2.0)),
							..default()
						},
						BackgroundColor(bar_color(1.0)),
					));
				});
		});
}

fn spawn_world_bar(parent: Entity, target: Entity, commands: &mut Commands) {
	commands.entity(parent).with_children(|root| {
		root.spawn((
			WorldHealthAnchor { target },
			Node {
				position_type: PositionType::Absolute,
				left: Val::Px(0.0),
				top: Val::Px(0.0),
				width: Val::Px(WORLD_BAR_WIDTH),
				height: Val::Px(WORLD_BAR_HEIGHT + 18.0),
				flex_direction: FlexDirection::Column,
				align_items: AlignItems::Center,
				row_gap: Val::Px(2.0),
				..default()
			},
			Visibility::Hidden,
			Pickable::IGNORE,
		))
		.with_children(|anchor| {
			anchor.spawn((
				WorldHudStatus,
				Node {
					width: Val::Px(10.0),
					height: Val::Px(10.0),
					border_radius: BorderRadius::all(Val::Px(10.0)),
					..default()
				},
				BackgroundColor(Color::WHITE),
				Visibility::Hidden,
				Pickable::IGNORE,
			));
			anchor.spawn((
				WorldHudLabel,
				Text::new(""),
				TextFont { font_size: FontSize::Px(12.0), ..default() },
				TextColor(Color::WHITE),
			));
			anchor
				.spawn((
					Node {
						width: Val::Percent(100.0),
						height: Val::Px(WORLD_BAR_HEIGHT),
						border_radius: BorderRadius::all(Val::Px(2.0)),
						..default()
					},
					BackgroundColor(Color::srgba(0.05, 0.05, 0.06, 0.85)),
				))
				.with_children(|track| {
					track.spawn((
						WorldHudFill,
						Node {
							width: Val::Percent(100.0),
							height: Val::Percent(100.0),
							border_radius: BorderRadius::all(Val::Px(2.0)),
							..default()
						},
						BackgroundColor(bar_color(1.0)),
					));
				});
		});
	});
}

type LiveNpcs<'w, 's> = Query<'w, 's, Entity, (With<Npc>, With<Health>, Without<Downed>)>;

fn ensure_world_health_bars(
	mut commands: Commands,
	hud: Query<Entity, With<CombatHudRoot>>,
	npcs: LiveNpcs,
	anchors: Query<(Entity, &WorldHealthAnchor)>,
) {
	let Ok(hud) = hud.single() else {
		return;
	};
	let live: Vec<Entity> = npcs.iter().collect();
	let existing: Vec<(Entity, Entity)> =
		anchors.iter().map(|(entity, anchor)| (entity, anchor.target)).collect();
	for (entity, target) in &existing {
		if !live.contains(target) {
			commands.entity(*entity).try_despawn();
		}
	}
	for npc in live {
		if !existing.iter().any(|(_, target)| *target == npc) {
			spawn_world_bar(hud, npc, &mut commands);
		}
	}
}

fn sync_health_hud(
	total: Res<CombatHudOpponentTotal>,
	players: Query<&Health, (With<Player>, Without<Downed>)>,
	npcs: Query<&Health, (With<Npc>, Without<Downed>)>,
	mut fills: Query<(&CombatantHud, &mut Node, &mut BackgroundColor), With<HudBarFill>>,
	mut labels: Query<(&CombatantHud, &mut Text), With<HudBarLabel>>,
) {
	let player = players.single().ok().copied();
	let alive = npcs.iter().count();
	let npc = npcs.single().ok().copied();
	let opponent_fraction = total
		.0
		.map(|total| alive as f32 / total.max(1) as f32)
		.unwrap_or_else(|| npc.map(Health::fraction).unwrap_or(0.0))
		.clamp(0.0, 1.0);
	for (kind, mut node, mut color) in &mut fills {
		let fraction = match *kind {
			CombatantHud::Player => player.map(Health::fraction).unwrap_or(0.0),
			CombatantHud::Npc => opponent_fraction,
		};
		node.width = Val::Percent(fraction * 100.0);
		color.0 = bar_color(fraction);
	}
	for (kind, mut text) in &mut labels {
		text.0 = match *kind {
			CombatantHud::Player => health_label("YOU", player),
			CombatantHud::Npc => total
				.0
				.map(|total| format!("FIELD  {alive}/{}", total.max(1)))
				.unwrap_or_else(|| health_label("NPC", npc)),
		};
	}
}

type WorldBarFills<'w, 's> = Query<
	'w,
	's,
	(&'static mut Node, &'static mut BackgroundColor),
	(With<WorldHudFill>, Without<WorldHealthAnchor>, Without<WorldHudStatus>),
>;
type WorldHealthNpcs<'w, 's> = Query<
	'w,
	's,
	(
		&'static GlobalTransform,
		&'static Health,
		Option<&'static LocomotionCapsule>,
		Option<&'static CombatHudStatusColor>,
	),
	(With<Npc>, Without<Downed>),
>;
type WorldBarAnchors<'w, 's> = Query<
	'w,
	's,
	(Entity, &'static WorldHealthAnchor, &'static mut Node, &'static mut Visibility),
	(Without<WorldHudFill>, Without<WorldHudStatus>),
>;
type WorldHudStatuses<'w, 's> = Query<
	'w,
	's,
	(&'static mut BackgroundColor, &'static mut Visibility),
	(With<WorldHudStatus>, Without<WorldHealthAnchor>, Without<WorldHudFill>),
>;

fn sync_world_health_bars(
	cameras: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
	npcs: WorldHealthNpcs,
	mut anchors: WorldBarAnchors,
	children: Query<&Children>,
	mut fills: WorldBarFills,
	mut labels: Query<&mut Text, With<WorldHudLabel>>,
	mut statuses: WorldHudStatuses,
) {
	let Ok((camera, camera_transform)) = cameras.single() else {
		return;
	};
	for (anchor_entity, anchor, mut node, mut visibility) in &mut anchors {
		let Ok((transform, health, hull, status)) = npcs.get(anchor.target) else {
			*visibility = Visibility::Hidden;
			continue;
		};
		let lift = hull.copied().unwrap_or_default().half_height() + HEAD_LIFT_PAD;
		let head = transform.translation() + Vec3::Y * lift;
		if let Ok(screen) = camera.world_to_viewport(camera_transform, head) {
			node.left = Val::Px(screen.x - WORLD_BAR_WIDTH * 0.5);
			node.top = Val::Px(screen.y - 26.0);
			*visibility = Visibility::Visible;
		} else {
			*visibility = Visibility::Hidden;
			continue;
		}
		let Ok(kids) = children.get(anchor_entity) else {
			continue;
		};
		for child in kids {
			if let Ok((mut color, mut status_visibility)) = statuses.get_mut(*child) {
				if let Some(status) = status {
					color.0 = status.0;
					*status_visibility = Visibility::Visible;
				} else {
					*status_visibility = Visibility::Hidden;
				}
			}
			if let Ok(mut text) = labels.get_mut(*child) {
				text.0 = format!("{:.0}", health.current);
			}
			let Ok(grandchildren) = children.get(*child) else {
				continue;
			};
			for grandchild in grandchildren {
				let Ok((mut fill, mut color)) = fills.get_mut(*grandchild) else {
					continue;
				};
				fill.width = Val::Percent(health.fraction() * 100.0);
				color.0 = bar_color(health.fraction());
			}
		}
	}
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

fn health_label(name: &str, health: Option<Health>) -> String {
	match health {
		Some(health) if !health.is_dead() => {
			format!("{name}  {:.0}/{:.0}", health.current, health.max)
		}
		_ => format!("{name}  DOWN"),
	}
}

fn bar_color(fraction: f32) -> Color {
	if fraction <= 0.0 {
		Color::srgb(0.28, 0.08, 0.08)
	} else if fraction < 0.35 {
		Color::srgb(0.86, 0.18, 0.14)
	} else if fraction < 0.7 {
		Color::srgb(0.92, 0.72, 0.18)
	} else {
		Color::srgb(0.28, 0.82, 0.38)
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

	#[test]
	fn categories_default_to_enabled() {
		assert_eq!(
			CombatHudPlugin::default(),
			CombatHudPlugin {
				health_bars: true,
				hit_markers: true,
				directional_damage: true,
				player_vitals: false,
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
		assert_eq!(world.query::<&HudBarFill>().iter(world).count(), 2);
		assert_eq!(world.query::<&DamageTick>().iter(world).count(), INDICATOR_COUNT);
		assert_eq!(world.query::<&vitals::PlayerVitalsRoot>().iter(world).count(), 0);
	}

	#[test]
	fn player_vitals_spawn_name_and_pips() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins).add_plugins(CombatHudPlugin {
			health_bars: false,
			hit_markers: false,
			directional_damage: false,
			player_vitals: true,
			score: false,
		});
		app.update();
		let world = app.world_mut();
		assert_eq!(world.query::<&vitals::PlayerVitalsRoot>().iter(world).count(), 1);
		assert_eq!(
			world.query::<&vitals::PlayerVitalsPip>().iter(world).count(),
			vitals::VITALS_PIPS
		);
	}

	fn vitals_app() -> App {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins).add_plugins(CombatHudPlugin {
			health_bars: false,
			hit_markers: false,
			directional_damage: false,
			player_vitals: true,
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
			health_bars: false,
			hit_markers: false,
			directional_damage: false,
			player_vitals: true,
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
		let lit = world
			.query::<(&vitals::PlayerVitalsPip, &BackgroundColor)>()
			.iter(world)
			.filter(|(_, color)| color.0 == vitals::vitals_fill_color(0.4))
			.count();
		assert_eq!(lit, 4);
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
				health_bars: false,
				hit_markers: false,
				directional_damage: false,
				player_vitals: false,
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
				health_bars: false,
				hit_markers: true,
				directional_damage: false,
				player_vitals: false,
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
}
