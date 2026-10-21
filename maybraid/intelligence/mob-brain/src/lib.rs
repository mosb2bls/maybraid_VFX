//! Stationary occupy/watch, a roaming herd, and a hunt that tracks that herd.

mod camera;
mod packs;
mod scene;

use std::time::Duration;

use avian3d::prelude::{PhysicsPlugins, PhysicsSchedulePlugin};
use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;
use bevy::time::common_conditions::on_timer;
use camera::{camera_controller, release_modifiers_on_focus_change, setup_camera};
use evasion_intelligence::{EvasionPlugin, EvasionSystems};
use firearm_intelligence::{FirearmIntelligencePlugin, FirearmIntelligenceSystems};
use fleeing_intelligence::{FleeingPlugin, FleeingSystems};
use hiding_intelligence::{HidingPlugin, HidingSystems};
use journeying_intelligence::JourneyingIntelligencePlugin;
use maybraid_character_controller::CharacterControllerPlugin;
use maybraid_locomotion_controller::LocomotionControllerPlugin;
use meandering_intelligence::MeanderingIntelligencePlugin;
use mob_intelligence::{
	MemberOf, Mob, MobIdAlloc, MobIntelligencePlugin, MobSystems, MobTetherLock,
};
use movement_intelligence::{
	CandidateBudget, MovementIntelligenceLimits, MovementIntelligencePlugin,
};
use movement_intelligence_avian::AvianMovementSurface;
use movement_realization::MovementRealizationPlugin;
use npc_intelligence::NpcIntelligencePlugin;
use packs::{hunt_tracks_herd, spawn_needed_members, spawn_packs, start_hunt_browse, PackKind};
use player::{Npc, PlayerPlugin};
use poi_intelligence::{PoiGoal, PoiIntelligencePlugin, PoiSystems};
use routing_intelligence::RoutingPlugin;
use scene::{
	setup_ground, setup_lighting, setup_local_pois, setup_waypoints, PAD_EXTENT, PAD_SIDE,
};
use spotting_intelligence::SpottingSystems;
use tether_intelligence::TetherPlugin;
use threat_intelligence::ThreatIntelligencePlugin;
use threat_management_intelligence::{
	ThreatManagementIntelligence, ThreatManagementPlugin, ThreatTactic,
};

pub use camera::CameraController;
pub use packs::recipes;

#[derive(Component)]
struct StatusText;

pub struct MobBrainPlaygroundPlugin;

impl Plugin for MobBrainPlaygroundPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<PhysicsSchedulePlugin>() {
			app.add_plugins(PhysicsPlugins::default());
		}
		if !app.is_plugin_added::<FrameTimeDiagnosticsPlugin>() {
			app.add_plugins(FrameTimeDiagnosticsPlugin::default());
		}
		app.insert_resource(ClearColor(Color::srgb(0.03, 0.04, 0.055)))
			.insert_resource(MovementIntelligenceLimits {
				max_budget: CandidateBudget { max_candidates: 8, max_steps: 3, horizon: 28.0 },
				..default()
			})
			.add_plugins(CharacterControllerPlugin)
			.add_plugins(LocomotionControllerPlugin)
			.add_plugins(PlayerPlugin)
			.add_plugins(MovementIntelligencePlugin::<AvianMovementSurface<'_, '_>>::default())
			.add_plugins(FirearmIntelligencePlugin)
			.add_plugins(ThreatIntelligencePlugin)
			.add_plugins(ThreatManagementPlugin)
			.add_plugins(NpcIntelligencePlugin)
			.add_plugins(MobIntelligencePlugin)
			.add_plugins(damage::DamagePlugin)
			.add_plugins(EvasionPlugin)
			.add_plugins(FleeingPlugin)
			.add_plugins(HidingPlugin)
			.add_plugins(PoiIntelligencePlugin)
			.add_plugins(JourneyingIntelligencePlugin)
			.add_plugins(MeanderingIntelligencePlugin)
			.add_plugins(TetherPlugin)
			.add_plugins(RoutingPlugin)
			.add_plugins(MovementRealizationPlugin)
			.configure_sets(
				Update,
				SpottingSystems::Observe.run_if(on_timer(Duration::from_millis(125))),
			)
			.configure_sets(
				Update,
				FirearmIntelligenceSystems::Spotting.run_if(on_timer(Duration::from_millis(125))),
			)
			.configure_sets(
				Update,
				FirearmIntelligenceSystems::Movement.run_if(on_timer(Duration::from_millis(125))),
			)
			.configure_sets(
				Update,
				(EvasionSystems::Ingest, EvasionSystems::Rank)
					.chain()
					.after(SpottingSystems::Observe)
					.run_if(on_timer(Duration::from_millis(125))),
			)
			.configure_sets(
				Update,
				(FleeingSystems::Write, HidingSystems::Write)
					.chain()
					.after(EvasionSystems::Rank)
					.run_if(on_timer(Duration::from_millis(125))),
			)
			.configure_sets(Update, PoiSystems::Select.run_if(on_timer(Duration::from_millis(200))))
			.add_systems(
				Startup,
				(
					setup_camera,
					setup_lighting,
					setup_ground,
					setup_waypoints,
					setup_local_scene_pois,
					spawn_scene_actors,
					setup_hud,
				)
					.chain(),
			)
			.add_systems(
				Update,
				(release_modifiers_on_focus_change.before(camera_controller), camera_controller),
			)
			.add_systems(Update, spawn_needed_members.after(MobSystems::Respawn))
			.add_systems(
				Update,
				(start_hunt_browse, hunt_tracks_herd).chain().before(MobSystems::Travel),
			)
			.add_systems(Update, draw_debug_world)
			.add_systems(Update, update_status_text);
	}
}

fn setup_local_scene_pois(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
) {
	setup_local_pois(&mut commands, &mut meshes, &mut materials, packs::poi_placements());
}

fn spawn_scene_actors(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
	mut ids: ResMut<MobIdAlloc>,
) {
	spawn_packs(&mut commands, &mut meshes, &mut materials, &mut ids);
}

fn setup_hud(mut commands: Commands) {
	commands
		.spawn((
			Node {
				position_type: PositionType::Absolute,
				top: Val::Px(12.0),
				left: Val::Px(12.0),
				padding: UiRect::all(Val::Px(12.0)),
				max_width: Val::Px(640.0),
				..default()
			},
			BackgroundColor(Color::srgba(0.015, 0.02, 0.035, 0.9)),
		))
		.with_children(|parent| {
			parent.spawn((
				Text::new("mob-brain"),
				TextFont { font_size: bevy::text::FontSize::Px(15.0), ..default() },
				TextColor(Color::WHITE),
				StatusText,
			));
		});
}

type DebugHost<'a> = (
	Entity,
	&'a Mob,
	&'a PackKind,
	&'a GlobalTransform,
	Option<&'a PoiGoal>,
	Option<&'a MobTetherLock>,
);
type DebugMember<'a> =
	(&'a MemberOf, &'a GlobalTransform, Option<&'a PoiGoal>, &'a ThreatManagementIntelligence);

fn draw_debug_world(
	mut gizmos: Gizmos,
	hosts: Query<DebugHost<'_>>,
	members: Query<DebugMember<'_>, With<Npc>>,
) {
	let edge = PAD_EXTENT;
	let grid = Color::srgba(0.35, 0.42, 0.5, 0.18);
	let mut x = -edge;
	while x <= edge + 0.5 {
		gizmos.line(Vec3::new(x, 0.2, -edge), Vec3::new(x, 0.2, edge), grid);
		gizmos.line(Vec3::new(-edge, 0.2, x), Vec3::new(edge, 0.2, x), grid);
		x += 50.0;
	}

	let mut roam_at = None;
	let mut hunt_at = None;
	for (host, mob, kind, transform, goal, lock) in &hosts {
		let at = transform.translation();
		match kind {
			PackKind::Roam => roam_at = Some(at),
			PackKind::Hunt => hunt_at = Some(at),
			_ => {}
		}
		let ring = match kind {
			PackKind::Occupy => Color::srgb(0.55, 0.95, 0.4),
			PackKind::Watch => Color::srgb(0.95, 0.62, 0.2),
			PackKind::Roam => Color::srgb(0.4, 0.75, 1.0),
			PackKind::Hunt => Color::srgb(1.0, 0.35, 0.28),
		};
		xz_ring(&mut gizmos, at, mob.leash, ring.with_alpha(0.45));
		if let Some(goal) = goal {
			gizmos.line(
				at + Vec3::Y * 2.2,
				goal.location.point + Vec3::Y * 1.2,
				Color::srgb(1.0, 0.85, 0.2),
			);
		}
		if lock.is_some() {
			xz_ring(&mut gizmos, at, 3.0, Color::srgb(0.95, 0.35, 0.85).with_alpha(0.8));
		}
		for (membership, member, local, management) in &members {
			if membership.mob != host {
				continue;
			}
			gizmos.line(
				at + Vec3::Y * 1.6,
				member.translation() + Vec3::Y * 1.2,
				Color::srgba(1.0, 1.0, 1.0, 0.28),
			);
			if let Some(local) = local {
				gizmos.line(
					member.translation() + Vec3::Y * 1.2,
					local.location.point + Vec3::Y * 0.9,
					Color::srgb(0.35, 0.95, 0.55),
				);
			}
			let tactic = match management.tactic {
				ThreatTactic::Ignore => Color::srgb(0.35, 0.9, 0.45),
				ThreatTactic::Evade => Color::srgb(0.95, 0.85, 0.2),
				ThreatTactic::Combat => Color::srgb(1.0, 0.25, 0.2),
			};
			gizmos.sphere(
				Isometry3d::from_translation(member.translation() + Vec3::Y * 2.1),
				0.28,
				tactic,
			);
		}
	}
	if let (Some(hunt), Some(roam)) = (hunt_at, roam_at) {
		gizmos.line(hunt + Vec3::Y * 2.6, roam + Vec3::Y * 2.6, Color::srgb(0.95, 0.35, 0.85));
	}
}

fn xz_ring(gizmos: &mut Gizmos, center: Vec3, radius: f32, color: Color) {
	let mut points = Vec::with_capacity(65);
	for index in 0..=64 {
		let angle = index as f32 / 64.0 * std::f32::consts::TAU;
		points.push(Vec3::new(
			center.x + angle.cos() * radius,
			0.25,
			center.z + angle.sin() * radius,
		));
	}
	gizmos.linestrip(points, color);
}

type HostStatus<'a> = (
	Entity,
	&'a PackKind,
	&'a Name,
	&'a GlobalTransform,
	Option<&'a PoiGoal>,
	Option<&'a MobTetherLock>,
);
type MemberStatus<'a> =
	(&'a MemberOf, &'a GlobalTransform, Has<PoiGoal>, &'a ThreatManagementIntelligence);

fn update_status_text(
	diagnostics: Res<DiagnosticsStore>,
	hosts: Query<HostStatus<'_>>,
	members: Query<MemberStatus<'_>, With<Npc>>,
	mut text: Query<&mut Text, With<StatusText>>,
) {
	let Ok(mut text) = text.single_mut() else {
		return;
	};
	let fps = diagnostics
		.get(&FrameTimeDiagnosticsPlugin::FPS)
		.and_then(|d| d.smoothed())
		.unwrap_or(0.0);
	let mut hunt_at = None;
	let mut roam_at = None;
	for (_, kind, _, transform, _, _) in &hosts {
		match kind {
			PackKind::Hunt => hunt_at = Some(transform.translation()),
			PackKind::Roam => roam_at = Some(transform.translation()),
			_ => {}
		}
	}
	let gap = hunt_at
		.zip(roam_at)
		.map(|(hunt, roam)| hunt.xz().distance(roam.xz()))
		.unwrap_or(0.0);
	let mut status = format!(
		"mob-brain   {PAD_SIDE:.0} m pad   fps {fps:.0}\n\
		 WASD fly  mouse look  Space/Shift up/down  Ctrl sprint\n\
		 magenta = hunt tracks herd   gap {gap:.0} m   magenta ring = tether lock\n\
		 dots: green Ignore  yellow Evade  red Combat\n\n"
	);
	let mut rows: Vec<_> = hosts.iter().collect();
	rows.sort_by_key(|(entity, ..)| entity.to_bits());
	for (entity, kind, name, transform, goal, lock) in rows {
		let at = transform.translation();
		let mut count = 0;
		let mut poi = 0;
		let mut ignore = 0;
		let mut evade = 0;
		let mut combat = 0;
		let mut farthest = 0.0_f32;
		for (membership, member, has_goal, management) in &members {
			if membership.mob != entity {
				continue;
			}
			count += 1;
			poi += usize::from(has_goal);
			farthest = farthest.max(member.translation().xz().distance(at.xz()));
			match management.tactic {
				ThreatTactic::Ignore => ignore += 1,
				ThreatTactic::Evade => evade += 1,
				ThreatTactic::Combat => combat += 1,
			}
		}
		let dest = goal
			.map(|goal| format!("{:.0},{:.0}", goal.location.point.x, goal.location.point.z))
			.unwrap_or_else(|| "idle".into());
		let phase = if lock.is_some() { "lock" } else { "    " };
		status.push_str(&format!(
			"{:<5} {kind:?} {phase}  host {:>6.0},{:>6.0}  dest {dest}  n {count}  I {ignore} E {evade} C {combat}  poi {poi}  stretch {farthest:.0}\n",
			name.as_str(),
			at.x,
			at.z,
		));
	}
	*text = Text::new(status);
}
