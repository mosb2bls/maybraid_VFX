//! 400 m square pad of packs. Fly toward a pack; the white public capsule
//! follows the camera look-at so distance drives Ignore | Evade | Combat.

mod camera;
mod mobs;
mod scene;

use std::time::Duration;

use avian3d::prelude::{LinearVelocity, PhysicsPlugins, PhysicsSchedulePlugin};
use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;
use bevy::time::common_conditions::on_timer;
use camera::{camera_controller, ground_look_at, release_modifiers_on_focus_change, setup_camera};
use evasion_intelligence::{EvasionPlugin, EvasionSystems};
use firearm_intelligence::{FirearmIntelligencePlugin, FirearmIntelligenceSystems};
use fleeing_intelligence::{FleeingPlugin, FleeingSystems};
use hiding_intelligence::{HidingPlugin, HidingSystems};
use maybraid_character_controller::CharacterControllerPlugin;
use maybraid_locomotion_controller::LocomotionControllerPlugin;
use meandering_intelligence::MeanderingIntelligencePlugin;
use mob_intelligence::{MemberOf, Mob, MobIdAlloc, MobIntelligencePlugin};
use mobs::{clamp_to_pad, spawn_mobs, spawn_needed_members, spawn_presence, PublicPresence};
use movement_intelligence::{
	CandidateBudget, MovementIntelligenceLimits, MovementIntelligencePlugin,
};
use movement_intelligence_avian::AvianMovementSurface;
use movement_realization::MovementRealizationPlugin;
use npc_intelligence::NpcIntelligencePlugin;
use player::{LocomotionCapsule, Npc, PlayerPlugin};
use poi_intelligence::{PoiIntelligencePlugin, PoiSystems};
use scene::{setup_cover, setup_ground, setup_lighting, setup_pois, PAD_EXTENT};
use spotting_intelligence::SpottingSystems;
use tether_intelligence::TetherPlugin;
use threat_intelligence::ThreatIntelligencePlugin;
use threat_management_intelligence::{
	ThreatManagementIntelligence, ThreatManagementPlugin, ThreatTactic,
};

pub use camera::CameraController;
pub use mobs::{member_count, recipes, MobKind, MobRecipe};
pub use scene::{HIGH_RING, PAD_SIDE, SPOTTING_RING};

#[derive(Component)]
struct StatusText;

pub struct PersonalitiesPlaygroundPlugin;

impl Plugin for PersonalitiesPlaygroundPlugin {
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
			.add_plugins(MeanderingIntelligencePlugin)
			.add_plugins(TetherPlugin)
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
			.configure_sets(Update, PoiSystems::Select.run_if(on_timer(Duration::from_millis(250))))
			.add_systems(
				Startup,
				(
					setup_camera,
					setup_lighting,
					setup_ground,
					setup_cover,
					setup_pois,
					spawn_scene_actors,
					setup_hud,
				)
					.chain(),
			)
			.add_systems(Update, spawn_needed_members.after(mob_intelligence::MobSystems::Respawn))
			.add_systems(
				Update,
				(
					release_modifiers_on_focus_change.before(camera_controller),
					camera_controller,
					sync_presence.after(camera_controller),
				),
			)
			.add_systems(Update, draw_debug_world)
			.add_systems(Update, update_status_text);
	}
}

fn spawn_scene_actors(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
	mut ids: ResMut<MobIdAlloc>,
) {
	spawn_presence(&mut commands, &mut meshes, &mut materials);
	spawn_mobs(&mut commands, &mut meshes, &mut materials, &mut ids);
}

fn setup_hud(mut commands: Commands) {
	commands
		.spawn((
			Node {
				position_type: PositionType::Absolute,
				top: Val::Px(12.0),
				left: Val::Px(12.0),
				padding: UiRect::all(Val::Px(12.0)),
				max_width: Val::Px(520.0),
				..default()
			},
			BackgroundColor(Color::srgba(0.015, 0.02, 0.035, 0.9)),
		))
		.with_children(|parent| {
			parent.spawn((
				Text::new("personalities"),
				TextFont { font_size: bevy::text::FontSize::Px(15.0), ..default() },
				TextColor(Color::WHITE),
				StatusText,
			));
		});
}

type PresenceBody<'w, 's> = Query<
	'w,
	's,
	(&'static mut Transform, Option<&'static mut LinearVelocity>),
	(With<PublicPresence>, Without<Camera3d>),
>;

fn sync_presence(cameras: Query<&Transform, With<Camera3d>>, mut presence: PresenceBody) {
	let Ok(camera) = cameras.single() else {
		return;
	};
	let Some(hit) = ground_look_at(camera) else {
		return;
	};
	let xz = clamp_to_pad(hit.xz());
	let hull = LocomotionCapsule::HUMANOID;
	let Ok((mut transform, velocity)) = presence.single_mut() else {
		return;
	};
	transform.translation = Vec3::new(xz.x, hull.spawn_height(), xz.y);
	if let Some(mut velocity) = velocity {
		**velocity = Vec3::ZERO;
	}
}

fn draw_debug_world(
	mut gizmos: Gizmos,
	presence: Query<&GlobalTransform, With<PublicPresence>>,
	mobs: Query<(&Mob, &GlobalTransform)>,
	members: Query<(&GlobalTransform, &ThreatManagementIntelligence), With<Npc>>,
) {
	let edge = PAD_EXTENT;
	let grid = Color::srgba(0.35, 0.42, 0.5, 0.18);
	let mut x = -edge;
	while x <= edge + 0.5 {
		gizmos.line(Vec3::new(x, 0.2, -edge), Vec3::new(x, 0.2, edge), grid);
		gizmos.line(Vec3::new(-edge, 0.2, x), Vec3::new(edge, 0.2, x), grid);
		x += 50.0;
	}

	if let Ok(public) = presence.single() {
		let at = public.translation();
		xz_ring(&mut gizmos, at, SPOTTING_RING, Color::srgb(0.25, 0.85, 1.0));
		xz_ring(&mut gizmos, at, HIGH_RING, Color::srgb(0.95, 0.35, 0.85));
	}

	for (mob, transform) in &mobs {
		xz_ring(&mut gizmos, transform.translation(), mob.leash, Color::srgba(1.0, 1.0, 1.0, 0.28));
	}

	for (transform, management) in &members {
		let color = match management.tactic {
			ThreatTactic::Ignore => Color::srgb(0.35, 0.9, 0.45),
			ThreatTactic::Evade => Color::srgb(0.95, 0.85, 0.2),
			ThreatTactic::Combat => Color::srgb(1.0, 0.25, 0.2),
		};
		gizmos.sphere(
			Isometry3d::from_translation(transform.translation() + Vec3::Y * 2.1),
			0.28,
			color,
		);
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

type MemberStatus<'a> = (&'a MemberOf, &'a ThreatManagementIntelligence, &'a GlobalTransform);

fn update_status_text(
	diagnostics: Res<DiagnosticsStore>,
	presence: Query<&GlobalTransform, With<PublicPresence>>,
	mobs: Query<(Entity, &MobKind, &Name)>,
	members: Query<MemberStatus<'_>>,
	mut text: Query<&mut Text, With<StatusText>>,
) {
	let Ok(mut text) = text.single_mut() else {
		return;
	};
	let fps = diagnostics
		.get(&FrameTimeDiagnosticsPlugin::FPS)
		.and_then(|d| d.smoothed())
		.unwrap_or(0.0);
	let public = presence
		.single()
		.ok()
		.map(|transform| transform.translation())
		.unwrap_or(Vec3::ZERO);
	let mut status = format!(
		"personalities   {PAD_SIDE:.0} m square   fps {fps:.0}\n\
		 WASD fly  mouse look  Space/Shift up/down  Ctrl sprint\n\
		 white capsule = public (follows look-at)\n\
		 cyan ring = spotting 80 m   magenta = High 200 m\n\
		 dots: green Ignore  yellow Evade  red Combat\n\
		 public @ {0:.0},{1:.0}\n\n",
		public.x, public.z
	);

	let mut rows: Vec<_> = mobs.iter().collect();
	rows.sort_by_key(|(entity, ..)| entity.to_bits());
	for (entity, kind, name) in rows {
		let mut ignore = 0;
		let mut evade = 0;
		let mut combat = 0;
		let mut nearest = f32::MAX;
		for (member, management, transform) in &members {
			if member.mob != entity {
				continue;
			}
			match management.tactic {
				ThreatTactic::Ignore => ignore += 1,
				ThreatTactic::Evade => evade += 1,
				ThreatTactic::Combat => combat += 1,
			}
			nearest = nearest.min(transform.translation().xz().distance(public.xz()));
		}
		let nearest = if nearest.is_finite() { nearest } else { 0.0 };
		status.push_str(&format!(
			"{:<7} {:>2}  d {:>5.0}  I {ignore}  E {evade}  C {combat}  {kind:?}\n",
			name.as_str(),
			ignore + evade + combat,
			nearest,
		));
	}
	*text = Text::new(status);
}
