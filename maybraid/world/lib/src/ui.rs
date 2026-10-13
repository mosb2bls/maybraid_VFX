use crate::WorldFurnished;
use bevy::prelude::*;
use bevy::text::FontSize;
use combat_hud::ScreenPin;
use damage::Downed;
use evasion_intelligence::{EvasionActuator, EvasionIntelligenceUser};
use game_commands::ui::{GameCommandStatusText, GameCommandUiConfig};
use mob_scenes::{MobKind, MobScene, DEFAULT_MOB_HIGH_RADIUS};
use player::Npc;
use threat_management_intelligence::{ThreatManagementIntelligence, ThreatTactic};

const HUD_PIN_COUNT: usize = 8;
pub(crate) const HUD_MARGIN: f32 = 22.0;
const HUD_PIN_WIDTH: f32 = 118.0;
const HUD_PIN_WORLD_HEIGHT: f32 = 4.0;

pub fn ui_config() -> GameCommandUiConfig {
	GameCommandUiConfig {
		title: "World — character on Durham + forest + urbanization + sky — / for commands — Y or F1 drawer"
			.into(),
		empty_console_text: "Console: `mode free`, `set-character`, `stats mesh`, `stats vsync`, `help`".into(),
		root_background: Color::srgba(0.08, 0.16, 0.22, 0.82),
		controls_hint:
			"L-stick move — R-stick look — M / D-Pad up overhead map — D-Pad down close — hold RB / V and flick a stick to throw the skill map — L3 / Shift sprint — R3 POV — A jump — B tap squat / hold prone — LT optic — LB irons — D-Pad left/right / [ ] cycle map — RT use — X / E pick up — Tab drop — RT+X power — / commands"
				.into(),
	}
}

/// Edge-clamped host pins plus colored poles. Off in the game until Settings.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorldMobHudEnabled(pub bool);

impl Default for WorldMobHudEnabled {
	fn default() -> Self {
		Self(false)
	}
}

impl WorldMobHudEnabled {
	pub fn from_debug_chrome(debug_chrome: bool) -> Self {
		Self(debug_chrome)
	}
}

#[derive(Component)]
pub(crate) struct MobDebugHud;

#[derive(Component)]
pub(crate) struct MobDebugPin {
	target: Entity,
}

#[derive(Bundle)]
struct MobDebugPinBundle {
	name: Name,
	pin: MobDebugPin,
	node: Node,
	background: BackgroundColor,
	text: Text,
	font: TextFont,
	color: TextColor,
	pickable: Pickable,
	visibility: Visibility,
}

impl MobDebugHud {
	fn spawn(commands: &mut Commands) {
		commands.spawn((
			Name::new("mob-debug-hud"),
			Self,
			Node {
				position_type: PositionType::Absolute,
				width: Val::Percent(100.0),
				height: Val::Percent(100.0),
				..default()
			},
			Pickable::IGNORE,
		));
	}
}

pub(crate) fn spawn_mob_debug_hud(mut commands: Commands) {
	MobDebugHud::spawn(&mut commands);
}

pub(crate) fn sync_mob_debug_hud_presence(
	mut commands: Commands,
	enabled: Res<WorldMobHudEnabled>,
	hud: Query<Entity, With<MobDebugHud>>,
) {
	if enabled.0 {
		if hud.is_empty() {
			MobDebugHud::spawn(&mut commands);
		}
		return;
	}
	for entity in &hud {
		commands.entity(entity).despawn();
	}
}

pub(crate) fn sync_command_status_text(
	mut status: ResMut<GameCommandStatusText>,
	camera: Query<&GlobalTransform, With<Camera3d>>,
	hosts: Query<(&MobScene, &GlobalTransform)>,
	furniture: Query<&lod::hcsg::HcsgNode<WorldFurnished>>,
) {
	let mut nearest = ranked_hosts(&camera, &hosts);
	nearest.truncate(4);
	let presented = hosts.iter().count();
	let nearest_line = if nearest.is_empty() {
		"none in present ring".into()
	} else {
		nearest
			.iter()
			.map(|host| format!("{:?} {:.0}m", host.kind, host.distance))
			.collect::<Vec<_>>()
			.join("   ")
	};
	let furniture_cells = furniture.iter().count();
	let furniture_items: usize = furniture.iter().map(|node| node.value.cell.slots.len()).sum();
	status.0 = format!(
		"world  character  forest hopscotch  urbanization hopscotch  grove 1 km  bump-outs 1–5 km\n\
		 mobs {presented} presented   nearest {nearest_line}\n\
		 furniture  {furniture_cells} cells   {furniture_items} items\n\
		 HUD pins = 8 nearest (edge-clamped)   colored pole = host   plants only inside {DEFAULT_MOB_HIGH_RADIUS:.0} m\n\
		 NPC behavior: gray circle = ignore   amber arrow = flee   blue square = hide   red cross = combat"
	);
}

pub(crate) fn draw_mob_debug_gizmos(
	mut gizmos: Gizmos,
	hosts: Query<(&MobScene, &GlobalTransform)>,
) {
	for (scene, transform) in &hosts {
		let at = transform.translation();
		let color = kind_color(scene.mob.kind);
		gizmos.line(at, at + Vec3::Y * 18.0, color);
		gizmos.sphere(Isometry3d::from_translation(at + Vec3::Y * 18.0), 1.4, color);
		xz_ring(&mut gizmos, at, 6.0, color.with_alpha(0.7));
		xz_ring(&mut gizmos, at, scene.high_radius, color.with_alpha(0.18));
	}
}

type NpcBehaviorComponents<'a> = (
	&'a GlobalTransform,
	Option<&'a ThreatManagementIntelligence>,
	Option<&'a EvasionIntelligenceUser>,
);
type ActiveNpc = (With<Npc>, Without<Downed>);

pub(crate) fn draw_npc_behavior_gizmos(
	mut gizmos: Gizmos,
	npcs: Query<NpcBehaviorComponents<'_>, ActiveNpc>,
) {
	for (transform, intelligence, evasion) in &npcs {
		let tactic = intelligence.map(|intelligence| intelligence.tactic).unwrap_or_default();
		let behavior = npc_behavior(tactic, evasion.map(|evasion| evasion.signal.actuator));
		let color = behavior_color(behavior);
		let body = transform.translation();
		let marker = body + Vec3::Y * 3.0;
		gizmos.line(body + Vec3::Y, marker, color);
		draw_behavior_glyph(&mut gizmos, marker, behavior, color);
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NpcBehavior {
	Ignore,
	EvadePending,
	Flee,
	Hide,
	Combat,
}

fn npc_behavior(tactic: ThreatTactic, evasion: Option<EvasionActuator>) -> NpcBehavior {
	match tactic {
		ThreatTactic::Ignore => NpcBehavior::Ignore,
		ThreatTactic::Combat => NpcBehavior::Combat,
		ThreatTactic::Evade => match evasion {
			Some(EvasionActuator::Flee) => NpcBehavior::Flee,
			Some(EvasionActuator::Hide) => NpcBehavior::Hide,
			Some(EvasionActuator::Idle) | None => NpcBehavior::EvadePending,
		},
	}
}

fn draw_behavior_glyph(gizmos: &mut Gizmos, at: Vec3, behavior: NpcBehavior, color: Color) {
	const RADIUS: f32 = 0.38;
	match behavior {
		NpcBehavior::Ignore => {
			gizmos.sphere(Isometry3d::from_translation(at), RADIUS, color);
		}
		NpcBehavior::EvadePending => {
			gizmos.linestrip(
				[
					at + Vec3::Y * RADIUS,
					at + Vec3::X * RADIUS,
					at - Vec3::Y * RADIUS,
					at - Vec3::X * RADIUS,
					at + Vec3::Y * RADIUS,
				],
				color,
			);
		}
		NpcBehavior::Flee => {
			gizmos.line(at - Vec3::X * RADIUS, at + Vec3::X * RADIUS, color);
			gizmos.line(at + Vec3::X * RADIUS, at + Vec3::new(-RADIUS * 0.15, RADIUS, 0.0), color);
			gizmos.line(at + Vec3::X * RADIUS, at + Vec3::new(-RADIUS * 0.15, -RADIUS, 0.0), color);
		}
		NpcBehavior::Hide => {
			gizmos.linestrip(
				[
					at + Vec3::new(-RADIUS, -RADIUS, 0.0),
					at + Vec3::new(RADIUS, -RADIUS, 0.0),
					at + Vec3::new(RADIUS, RADIUS, 0.0),
					at + Vec3::new(-RADIUS, RADIUS, 0.0),
					at + Vec3::new(-RADIUS, -RADIUS, 0.0),
				],
				color,
			);
		}
		NpcBehavior::Combat => {
			let diagonal = Vec3::new(RADIUS, RADIUS, 0.0);
			let counter = Vec3::new(RADIUS, -RADIUS, 0.0);
			gizmos.line(at - diagonal, at + diagonal, color);
			gizmos.line(at - counter, at + counter, color);
		}
	}
}

fn behavior_color(behavior: NpcBehavior) -> Color {
	match behavior {
		NpcBehavior::Ignore => Color::srgb(0.55, 0.62, 0.7),
		NpcBehavior::EvadePending | NpcBehavior::Flee => Color::srgb(1.0, 0.68, 0.08),
		NpcBehavior::Hide => Color::srgb(0.15, 0.55, 1.0),
		NpcBehavior::Combat => Color::srgb(1.0, 0.12, 0.08),
	}
}

pub(crate) fn sync_mob_debug_pins(
	mut commands: Commands,
	camera: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
	hud: Query<Entity, With<MobDebugHud>>,
	hosts: Query<(Entity, &MobScene, &GlobalTransform)>,
	mut pins: Query<(
		Entity,
		&MobDebugPin,
		&mut Node,
		&mut BackgroundColor,
		&mut Text,
		&mut Visibility,
	)>,
) {
	let Ok(hud) = hud.single() else {
		return;
	};
	let Ok((camera, camera_transform)) = camera.single() else {
		for (_, _, _, _, _, mut visibility) in &mut pins {
			*visibility = Visibility::Hidden;
		}
		return;
	};
	let ranked = ranked_hosts_with_entity(camera_transform, &hosts);
	let wanted: Vec<_> = ranked.into_iter().take(HUD_PIN_COUNT).collect();
	let mut assigned = Vec::new();
	for (pin_entity, pin, mut node, mut background, mut text, mut visibility) in &mut pins {
		let Some(host) = wanted.iter().find(|host| host.entity == pin.target) else {
			commands.entity(pin_entity).despawn();
			continue;
		};
		let Some((screen, on_screen)) =
			ScreenPin::project(camera, camera_transform, mob_pin_anchor(host.at), HUD_MARGIN)
		else {
			*visibility = Visibility::Hidden;
			continue;
		};
		place_pin(&mut node, screen);
		background.0 = kind_color(host.kind).with_alpha(if on_screen { 0.72 } else { 0.92 });
		text.0 = format!("{:?} {:.0}m", host.kind, host.distance);
		*visibility = Visibility::Visible;
		assigned.push(host.entity);
	}
	for host in wanted {
		if assigned.contains(&host.entity) {
			continue;
		}
		let Some((screen, on_screen)) =
			ScreenPin::project(camera, camera_transform, mob_pin_anchor(host.at), HUD_MARGIN)
		else {
			continue;
		};
		commands.entity(hud).with_children(|root| {
			root.spawn(MobDebugPinBundle {
				name: Name::new("mob-debug-pin"),
				pin: MobDebugPin { target: host.entity },
				node: pin_node(screen),
				background: BackgroundColor(kind_color(host.kind).with_alpha(if on_screen {
					0.72
				} else {
					0.92
				})),
				text: Text::new(format!("{:?} {:.0}m", host.kind, host.distance)),
				font: TextFont { font_size: FontSize::Px(13.0), ..default() },
				color: TextColor(Color::WHITE),
				pickable: Pickable::IGNORE,
				visibility: Visibility::Visible,
			});
		});
	}
}

struct RankedHost {
	entity: Entity,
	kind: MobKind,
	at: Vec3,
	distance: f32,
}

fn ranked_hosts(
	camera: &Query<&GlobalTransform, With<Camera3d>>,
	hosts: &Query<(&MobScene, &GlobalTransform)>,
) -> Vec<RankedHost> {
	let Ok(camera) = camera.single() else {
		return Vec::new();
	};
	let origin = camera.translation();
	let mut ranked: Vec<_> = hosts
		.iter()
		.map(|(scene, transform)| RankedHost {
			entity: Entity::PLACEHOLDER,
			kind: scene.mob.kind,
			at: transform.translation(),
			distance: transform.translation().xz().distance(origin.xz()),
		})
		.collect();
	ranked.sort_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap_or(std::cmp::Ordering::Equal));
	ranked
}

fn ranked_hosts_with_entity(
	camera: &GlobalTransform,
	hosts: &Query<(Entity, &MobScene, &GlobalTransform)>,
) -> Vec<RankedHost> {
	let origin = camera.translation();
	let mut ranked: Vec<_> = hosts
		.iter()
		.map(|(entity, scene, transform)| RankedHost {
			entity,
			kind: scene.mob.kind,
			at: transform.translation(),
			distance: transform.translation().xz().distance(origin.xz()),
		})
		.collect();
	ranked.sort_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap_or(std::cmp::Ordering::Equal));
	ranked
}

fn mob_pin_anchor(host: Vec3) -> Vec3 {
	host + Vec3::Y * HUD_PIN_WORLD_HEIGHT
}

pub(crate) fn pin_node(screen: Vec2) -> Node {
	Node {
		position_type: PositionType::Absolute,
		left: Val::Px(screen.x - HUD_PIN_WIDTH * 0.5),
		top: Val::Px(screen.y - 12.0),
		width: Val::Px(HUD_PIN_WIDTH),
		padding: UiRect::axes(Val::Px(6.0), Val::Px(3.0)),
		justify_content: JustifyContent::Center,
		..default()
	}
}

pub(crate) fn place_pin(node: &mut Node, screen: Vec2) {
	node.left = Val::Px(screen.x - HUD_PIN_WIDTH * 0.5);
	node.top = Val::Px(screen.y - 12.0);
}

fn kind_color(kind: MobKind) -> Color {
	match kind {
		MobKind::Herd => Color::srgb(0.35, 0.9, 0.45),
		MobKind::Pack => Color::srgb(0.95, 0.55, 0.2),
		MobKind::Raider => Color::srgb(1.0, 0.32, 0.28),
		MobKind::Guard => Color::srgb(0.35, 0.55, 1.0),
		MobKind::Pleb => Color::srgb(0.95, 0.85, 0.3),
		MobKind::Rambles => Color::srgb(0.4, 0.85, 1.0),
		MobKind::Brawler => Color::srgb(0.95, 0.4, 0.85),
	}
}

fn xz_ring(gizmos: &mut Gizmos, center: Vec3, radius: f32, color: Color) {
	let mut points = Vec::with_capacity(49);
	for index in 0..=48 {
		let angle = index as f32 / 48.0 * std::f32::consts::TAU;
		points.push(Vec3::new(
			center.x + angle.cos() * radius,
			center.y + 0.4,
			center.z + angle.sin() * radius,
		));
	}
	gizmos.linestrip(points, color);
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn mob_pin_anchor_is_lifted_above_the_surface() {
		let host = Vec3::new(4.0, 12.0, -3.0);
		assert_eq!(mob_pin_anchor(host), Vec3::new(4.0, 16.0, -3.0));
	}

	#[test]
	fn behavior_modes_have_distinct_colors() {
		let ignore = behavior_color(NpcBehavior::Ignore);
		let evade = behavior_color(NpcBehavior::Flee);
		let hide = behavior_color(NpcBehavior::Hide);
		let combat = behavior_color(NpcBehavior::Combat);
		assert_ne!(ignore, evade);
		assert_ne!(evade, hide);
		assert_ne!(hide, combat);
		assert_ne!(evade, combat);
		assert_ne!(combat, ignore);
	}

	#[test]
	fn debug_chrome_turns_the_mob_hud_on() {
		assert!(WorldMobHudEnabled::from_debug_chrome(true).0);
		assert!(!WorldMobHudEnabled::from_debug_chrome(false).0);
		assert!(!WorldMobHudEnabled::default().0);
	}

	#[test]
	fn evade_signal_selects_flee_or_hide_label() {
		assert_eq!(
			npc_behavior(ThreatTactic::Evade, Some(EvasionActuator::Flee)),
			NpcBehavior::Flee
		);
		assert_eq!(
			npc_behavior(ThreatTactic::Evade, Some(EvasionActuator::Hide)),
			NpcBehavior::Hide
		);
		assert_eq!(npc_behavior(ThreatTactic::Evade, None), NpcBehavior::EvadePending);
	}
}
