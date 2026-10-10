//! Application-owned budgets and cadence for world NPC intelligence.

use std::time::Duration;

use bevy::prelude::*;
use bevy::time::common_conditions::on_timer;
use characters::CharacterMotionSystems;
use evasion_intelligence::{EvasionPlugin, EvasionSystems};
use firearm_intelligence::{FirearmIntelligencePlugin, FirearmIntelligenceSystems};
use firearm_user::FirearmUserPlugin;
use firearms::FirearmWeaponsPlugin;
use fleeing_intelligence::{FleeingPlugin, FleeingSystems};
use grenade_user::GrenadeUserPlugin;
use hiding_intelligence::{HidingPlugin, HidingSystems};
use intelligence_lod::{
	look_near_m, look_promotes, IntelligenceBakeKey, IntelligenceBand, IntelligenceFocus,
	IntelligenceFocusSample, IntelligenceLod, IntelligenceLook, IntelligenceLookFrame,
	IntelligencePriority, LOOK_APPLY_FOV_INSET, LOOK_FOV_INSET,
};
use lod::LodViewer;
use meandering_intelligence::MeanderingIntelligencePlugin;
use mob_scenes::player_affiliations;
use movement_intelligence::{
	CandidateBudget, MovementIntelligenceLimits, MovementIntelligencePlugin,
};
use movement_intelligence_avian::AvianMovementSurface;
use movement_realization::MovementRealizationPlugin;
use npc_intelligence::NpcIntelligencePlugin;
use player::LocomotionCapsule;
use player::PlayerCameraAim;
use player_camera::{CameraController, CameraPov, FollowCamera};
use poi_intelligence::{PoiDiscoverLimits, PoiSystems};
use routing_intelligence::{RoutingPlugin, RoutingSystems};
use spotting_intelligence::{
	InterestLayers, SpotBounds, SpotSubject, SpottingObserveLimits, SpottingSystems,
};
use tether_intelligence::TetherSystems;
use threat_intelligence::{
	Affiliations, ThreatDiscoverLimits, ThreatId, ThreatIntelligencePlugin, ThreatSubject,
	ThreatSystems,
};
use threat_intelligence_damage::ThreatIntelligenceDamagePlugin;
use threat_management_intelligence::{
	ThreatManagementIntelligence, ThreatManagementPlugin, ThreatTactic,
};
use world_player::Player as VegetationPlayer;

const INTELLIGENCE_LOD_REFRESH_INTERVAL: Duration = Duration::from_millis(250);

const WORLD_SPOTTING_LIMITS: SpottingObserveLimits =
	SpottingObserveLimits { max_observers_per_tick: 8 };

const WORLD_POI_LIMITS: PoiDiscoverLimits = PoiDiscoverLimits { max_scans_per_tick: 8 };

const WORLD_THREAT_LIMITS: ThreatDiscoverLimits = ThreatDiscoverLimits { max_scans_per_tick: 8 };

const WORLD_MOVEMENT_LIMITS: MovementIntelligenceLimits = MovementIntelligenceLimits {
	max_budget: CandidateBudget { max_candidates: 8, max_steps: 3, horizon: 28.0 },
	max_replans_per_frame: 4,
	max_walk_probes_per_frame: 8,
};

type WorldPlayers<'w, 's> = Query<
	'w,
	's,
	(
		Entity,
		Option<&'static SpotSubject>,
		Option<&'static ThreatSubject>,
		Option<&'static Affiliations>,
		Option<&'static LocomotionCapsule>,
	),
	With<VegetationPlayer>,
>;

pub struct WorldIntelligencePlugin;

impl Plugin for WorldIntelligencePlugin {
	fn build(&self, app: &mut App) {
		app.insert_resource(WORLD_MOVEMENT_LIMITS)
			.insert_resource(WORLD_SPOTTING_LIMITS)
			.insert_resource(WORLD_POI_LIMITS)
			.insert_resource(WORLD_THREAT_LIMITS)
			.init_resource::<IntelligencePriority>()
			.init_resource::<IntelligenceFocus>()
			.init_resource::<IntelligenceLookFrame>();
		if !app.is_plugin_added::<FirearmWeaponsPlugin>() {
			app.add_plugins(FirearmWeaponsPlugin);
		}
		if !app.is_plugin_added::<FirearmUserPlugin>() {
			app.add_plugins(FirearmUserPlugin);
		}
		if !app.is_plugin_added::<GrenadeUserPlugin>() {
			app.add_plugins(GrenadeUserPlugin);
		}
		if !app.is_plugin_added::<MovementIntelligencePlugin<AvianMovementSurface<'_, '_>>>() {
			app.add_plugins(MovementIntelligencePlugin::<AvianMovementSurface<'_, '_>>::default());
		}
		if !app.is_plugin_added::<FirearmIntelligencePlugin>() {
			app.add_plugins(FirearmIntelligencePlugin);
		}
		if !app.is_plugin_added::<ThreatIntelligencePlugin>() {
			app.add_plugins(ThreatIntelligencePlugin);
		}
		if !app.is_plugin_added::<ThreatIntelligenceDamagePlugin>() {
			app.add_plugins(ThreatIntelligenceDamagePlugin);
		}
		if !app.is_plugin_added::<ThreatManagementPlugin>() {
			app.add_plugins(ThreatManagementPlugin);
		}
		if !app.is_plugin_added::<EvasionPlugin>() {
			app.add_plugins(EvasionPlugin);
		}
		if !app.is_plugin_added::<FleeingPlugin>() {
			app.add_plugins(FleeingPlugin);
		}
		if !app.is_plugin_added::<HidingPlugin>() {
			app.add_plugins(HidingPlugin);
		}
		if !app.is_plugin_added::<MeanderingIntelligencePlugin>() {
			app.add_plugins(MeanderingIntelligencePlugin);
		}
		if !app.is_plugin_added::<RoutingPlugin>() {
			app.add_plugins(RoutingPlugin);
		}
		if !app.is_plugin_added::<MovementRealizationPlugin>() {
			app.add_plugins(MovementRealizationPlugin);
		}
		if !app.is_plugin_added::<NpcIntelligencePlugin>() {
			app.add_plugins(NpcIntelligencePlugin);
		}
		app.add_systems(Update, sync_world_player_threat_actor.in_set(ThreatSystems::Prepare))
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
			.configure_sets(
				Update,
				(
					TetherSystems::Write.run_if(on_timer(Duration::from_millis(250))),
					RoutingSystems::Plan.run_if(on_timer(Duration::from_millis(250))),
				),
			)
			.add_systems(
				Update,
				(publish_intelligence_look, sync_intelligence_focus)
					.chain()
					.before(CharacterMotionSystems::Anim),
			)
			.add_systems(
				Update,
				bake_intelligence_lod
					.after(sync_intelligence_focus)
					.run_if(on_timer(INTELLIGENCE_LOD_REFRESH_INTERVAL)),
			)
			.configure_sets(
				PostUpdate,
				FirearmIntelligenceSystems::ValidateAim.run_if(on_timer(Duration::from_millis(33))),
			)
			.configure_sets(
				PostUpdate,
				FirearmIntelligenceSystems::Fire.run_if(on_timer(Duration::from_millis(33))),
			);
	}
}

/// Copy ADS / bore poses into the focus mailbox. Firearm control stays out of the bake.
fn hip_fov(follow: &FollowCamera, controller: &CameraController) -> f32 {
	match controller.pov {
		CameraPov::ThirdPerson | CameraPov::Map => follow.third_person_fov,
		CameraPov::FirstPerson => follow.first_person_fov,
	}
}

fn publish_intelligence_look(
	mut frame: ResMut<IntelligenceLookFrame>,
	cameras: Query<
		(&GlobalTransform, &Projection, &FollowCamera, &CameraController),
		With<LodViewer>,
	>,
) {
	frame.look = cameras.iter().find_map(|(transform, projection, follow, controller)| {
		let Projection::Perspective(perspective) = projection else {
			return None;
		};
		Some(IntelligenceLook::from_perspective_inset(
			transform,
			perspective,
			hip_fov(follow, controller),
			LOOK_APPLY_FOV_INSET,
		))
	});
}

fn sync_intelligence_focus(
	mut focus: ResMut<IntelligenceFocus>,
	aims: Query<&PlayerCameraAim>,
	cameras: Query<(&Projection, &FollowCamera, &CameraController), With<LodViewer>>,
) {
	focus.clear();
	let look_near = cameras.iter().find_map(|(projection, follow, controller)| {
		let Projection::Perspective(perspective) = projection else {
			return None;
		};
		Some(look_near_m(hip_fov(follow, controller), perspective.fov))
	});
	for aim in &aims {
		if aim.focus <= 1e-4 {
			continue;
		}
		let Some(pose) = aim.pose else {
			continue;
		};
		let dir = (pose.rotation * Vec3::NEG_Z).normalize_or_zero();
		if dir == Vec3::ZERO {
			continue;
		}
		let fov = aim.sight_fov.unwrap_or(50_f32.to_radians());
		focus.push(IntelligenceFocusSample {
			origin: pose.translation,
			dir,
			max_range: look_near.unwrap_or(IntelligenceBand::NEAR_M),
			half_angle: fov * 0.5 * LOOK_FOV_INSET,
		});
	}
}

fn bake_intelligence_lod(
	player: Query<&GlobalTransform, With<VegetationPlayer>>,
	cameras: Query<
		(&GlobalTransform, &Projection, &FollowCamera, &CameraController),
		With<LodViewer>,
	>,
	focus: Res<IntelligenceFocus>,
	mut plants: Query<(
		Entity,
		&GlobalTransform,
		&mut IntelligenceLod,
		&ThreatManagementIntelligence,
	)>,
	mut priority: ResMut<IntelligencePriority>,
	mut rank_scratch: Local<Vec<(Entity, IntelligenceBakeKey)>>,
) {
	let Ok(player) = player.single() else {
		return;
	};
	let look = cameras.iter().find_map(|(transform, projection, follow, controller)| {
		let Projection::Perspective(perspective) = projection else {
			return None;
		};
		Some(IntelligenceLook::from_perspective(
			transform,
			perspective,
			hip_fov(follow, controller),
		))
	});
	let player_xz = player.translation().xz();
	rank_scratch.clear();
	rank_scratch.reserve(plants.iter().len());
	for (entity, transform, mut lod, management) in &mut plants {
		let at = transform.translation();
		let dist = at.xz().distance(player_xz);
		let look = look_promotes(at, look, &focus);
		lod.band =
			IntelligenceBand::from_viewer(dist, management.tactic != ThreatTactic::Ignore, look);
		lod.skips = lod.skips.saturating_add(1);
		rank_scratch.push((entity, lod.bake_key(entity)));
	}
	rank_scratch.sort_by_key(|&(_, key)| key);
	priority.rank.clear();
	for (i, &(entity, _)) in rank_scratch.iter().enumerate() {
		priority.rank.insert(entity, i as u32);
	}
}

fn sync_world_player_threat_actor(mut commands: Commands, players: WorldPlayers) {
	for (entity, current_spot, current_subject, current_affiliations, hull) in &players {
		let hull = hull.copied().unwrap_or(LocomotionCapsule::HUMANOID);
		let spot = SpotSubject::new(
			InterestLayers::CHARACTER,
			SpotBounds::capsule(hull.radius, hull.half_height()),
		);
		let id = ThreatId(entity.to_bits());
		let subject = ThreatSubject::new(id);
		let mut entity_commands = commands.entity(entity);
		if current_spot != Some(&spot) {
			entity_commands.insert(spot);
		}
		if current_subject != Some(&subject) {
			entity_commands.insert(subject);
		}
		if current_affiliations.is_none() {
			entity_commands.insert(player_affiliations(id));
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use damage::DamageApplied;
	use mob_scenes::{MobBrain, MobKind, FFA_GROUP, PLAYER_GROUP};
	use threat_intelligence::{ThreatIntelligenceUser, ThreatKnowledge};

	fn bake_app() -> App {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.init_resource::<IntelligencePriority>()
			.init_resource::<IntelligenceFocus>()
			.add_systems(Update, bake_intelligence_lod);
		app
	}

	fn spawn_look_camera(app: &mut App, looking_at: Vec3, current_fov: f32, base_fov: f32) {
		let transform = Transform::from_translation(Vec3::Y * 1.6).looking_at(looking_at, Vec3::Y);
		app.world_mut().spawn((
			LodViewer,
			FollowCamera { first_person_fov: base_fov, third_person_fov: base_fov, ..default() },
			CameraController { pov: CameraPov::FirstPerson, ..default() },
			Projection::Perspective(PerspectiveProjection {
				fov: current_fov,
				aspect_ratio: 16.0 / 9.0,
				..default()
			}),
			GlobalTransform::from(transform),
		));
	}

	#[test]
	fn bake_orders_near_before_far_and_promotes_high_skips() {
		let mut app = bake_app();
		app.world_mut()
			.spawn((VegetationPlayer, Transform::default(), GlobalTransform::default()));
		let near = spawn_lod_plant(&mut app, Vec3::X * 40.0, ThreatTactic::Ignore, 0);
		let mid = spawn_lod_plant(&mut app, Vec3::X * 120.0, ThreatTactic::Ignore, 0);
		let far_fresh = spawn_lod_plant(&mut app, Vec3::X * 300.0, ThreatTactic::Ignore, 0);
		let far_waiting = spawn_lod_plant(
			&mut app,
			Vec3::X * 320.0,
			ThreatTactic::Ignore,
			IntelligenceLod::FAIRNESS_CAP,
		);
		let combat = spawn_lod_plant(&mut app, Vec3::X * 300.0, ThreatTactic::Combat, 0);

		app.update();

		let world = app.world();
		assert_eq!(
			world.get::<IntelligenceLod>(near).map(|lod| lod.band),
			Some(IntelligenceBand::Near)
		);
		assert_eq!(
			world.get::<IntelligenceLod>(mid).map(|lod| lod.band),
			Some(IntelligenceBand::Mid)
		);
		assert_eq!(
			world.get::<IntelligenceLod>(far_fresh).map(|lod| lod.band),
			Some(IntelligenceBand::Far)
		);
		assert_eq!(
			world.get::<IntelligenceLod>(combat).map(|lod| lod.band),
			Some(IntelligenceBand::Near)
		);
		assert_eq!(world.get::<IntelligenceLod>(near).map(|lod| lod.skips), Some(1));
		let rank = &world.resource::<IntelligencePriority>().rank;
		assert!(rank[&near] < rank[&mid]);
		assert!(rank[&mid] < rank[&far_waiting]);
		assert!(rank[&far_waiting] < rank[&far_fresh]);
		assert!(rank[&combat] < rank[&mid]);
	}

	#[test]
	fn bake_in_frustum_ignore_at_ninety_stays_mid() {
		let mut app = bake_app();
		app.world_mut()
			.spawn((VegetationPlayer, Transform::default(), GlobalTransform::default()));
		let hip = 75_f32.to_radians();
		spawn_look_camera(&mut app, Vec3::new(90.0, 1.6, 0.0), hip, hip);
		let grazer = spawn_lod_plant(&mut app, Vec3::X * 90.0, ThreatTactic::Ignore, 0);

		app.update();

		assert_eq!(
			app.world().get::<IntelligenceLod>(grazer).map(|lod| lod.band),
			Some(IntelligenceBand::Mid)
		);
	}

	#[test]
	fn bake_hip_look_does_not_extend_near() {
		let mut app = bake_app();
		app.world_mut()
			.spawn((VegetationPlayer, Transform::default(), GlobalTransform::default()));
		let hip = 75_f32.to_radians();
		spawn_look_camera(&mut app, Vec3::new(120.0, 1.6, 0.0), hip, hip);
		let ahead = spawn_lod_plant(&mut app, Vec3::X * 120.0, ThreatTactic::Ignore, 0);

		app.update();

		assert_eq!(
			app.world().get::<IntelligenceLod>(ahead).map(|lod| lod.band),
			Some(IntelligenceBand::Mid)
		);
	}

	#[test]
	fn bake_zoom_promotes_on_axis_mid_and_not_off_axis() {
		let mut app = bake_app();
		app.world_mut()
			.spawn((VegetationPlayer, Transform::default(), GlobalTransform::default()));
		spawn_look_camera(
			&mut app,
			Vec3::new(180.0, 1.6, 0.0),
			15_f32.to_radians(),
			75_f32.to_radians(),
		);
		let ahead = spawn_lod_plant(&mut app, Vec3::X * 180.0, ThreatTactic::Ignore, 0);
		let side = spawn_lod_plant(&mut app, Vec3::new(180.0, 0.0, 80.0), ThreatTactic::Ignore, 0);

		app.update();

		assert_eq!(
			app.world().get::<IntelligenceLod>(ahead).map(|lod| lod.band),
			Some(IntelligenceBand::Near)
		);
		assert_eq!(
			app.world().get::<IntelligenceLod>(side).map(|lod| lod.band),
			Some(IntelligenceBand::Mid)
		);
	}

	#[test]
	fn publish_look_uses_apply_inset() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.init_resource::<IntelligenceLookFrame>()
			.add_systems(Update, publish_intelligence_look);
		let hip = 75_f32.to_radians();
		spawn_look_camera(&mut app, Vec3::new(40.0, 1.6, 0.0), hip, hip);
		app.update();
		let look = app.world().resource::<IntelligenceLookFrame>().look.expect("look");
		assert!(look.in_frustum(Vec3::new(40.0, 1.6, 40.0)));
	}

	#[test]
	fn bake_focus_sample_promotes_without_camera() {
		let mut app = bake_app();
		app.world_mut()
			.spawn((VegetationPlayer, Transform::default(), GlobalTransform::default()));
		let at = Vec3::new(80.0, 0.0, 80.0);
		app.world_mut()
			.resource_mut::<IntelligenceFocus>()
			.push(IntelligenceFocusSample {
				origin: Vec3::Y,
				dir: (at - Vec3::Y).normalize(),
				max_range: f32::MAX,
				half_angle: 8_f32.to_radians(),
			});
		let side = spawn_lod_plant(&mut app, at, ThreatTactic::Ignore, 0);

		app.update();

		assert_eq!(
			app.world().get::<IntelligenceLod>(side).map(|lod| lod.band),
			Some(IntelligenceBand::Near)
		);
	}

	#[test]
	fn missing_lod_is_treated_as_near() {
		assert_eq!(IntelligenceLod::band_or_near(None), IntelligenceBand::Near);
	}

	fn spawn_lod_plant(app: &mut App, at: Vec3, tactic: ThreatTactic, skips: u8) -> Entity {
		let mut management = ThreatManagementIntelligence::default();
		management.tactic = tactic;
		app.world_mut()
			.spawn((
				GlobalTransform::from_translation(at),
				IntelligenceLod { band: IntelligenceBand::Near, skips },
				management,
			))
			.id()
	}

	#[test]
	fn world_replans_drain_instead_of_resolving_every_marker() {
		assert_eq!(WORLD_MOVEMENT_LIMITS.max_replans_per_frame, 4);
		assert_eq!(WORLD_MOVEMENT_LIMITS.max_walk_probes_per_frame, 8);
		assert_eq!(WORLD_SPOTTING_LIMITS.max_observers_per_tick, 8);
		assert_eq!(WORLD_POI_LIMITS.max_scans_per_tick, 8);
		assert_eq!(WORLD_THREAT_LIMITS.max_scans_per_tick, 8);
		assert!(
			WORLD_MOVEMENT_LIMITS.max_replans_per_frame
				< MovementIntelligenceLimits::default().max_replans_per_frame
		);
	}

	#[test]
	fn world_player_is_registered_for_spotting_and_threats() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.add_systems(Update, sync_world_player_threat_actor);
		let player = app.world_mut().spawn((VegetationPlayer, Transform::default())).id();

		app.update();

		let entity = app.world().entity(player);
		assert!(entity.get::<SpotSubject>().is_some());
		assert_eq!(
			entity.get::<ThreatSubject>().map(|subject| subject.id),
			Some(ThreatId(player.to_bits()))
		);
		let Some(affiliations) = entity.get::<Affiliations>() else {
			panic!("player affiliations were not installed");
		};
		assert!(affiliations.memberships.contains_key(&PLAYER_GROUP));
		assert!(affiliations.memberships.contains_key(&FFA_GROUP));
		assert!(affiliations.known_antagonists.contains_key(&FFA_GROUP));
	}

	#[test]
	fn generated_mob_discovers_nearby_world_player() {
		let mut app = threat_app();
		let player = spawn_world_player(&mut app);
		let mob_id = ThreatId(11);
		let affiliations = MobBrain::for_kind(MobKind::Herd).affiliations.for_member(mob_id);
		let mob = app
			.world_mut()
			.spawn((
				ThreatSubject::new(mob_id),
				affiliations,
				ThreatIntelligenceUser::default(),
				ThreatKnowledge::default(),
				Transform::from_xyz(4.0, 0.0, 0.0),
				GlobalTransform::from_xyz(4.0, 0.0, 0.0),
			))
			.id();

		app.update();

		let player_id = ThreatId(player.to_bits());
		assert!(app
			.world()
			.get::<ThreatKnowledge>(mob)
			.is_some_and(|knowledge| { knowledge.get(player_id).is_some() }));
	}

	#[test]
	fn damage_from_world_player_enters_victim_threat_knowledge() {
		let mut app = threat_app();
		let player = spawn_world_player(&mut app);
		let victim_id = ThreatId(13);
		let victim = app
			.world_mut()
			.spawn((
				ThreatSubject::new(victim_id),
				Affiliations::with_self(victim_id),
				ThreatIntelligenceUser::default(),
				ThreatKnowledge::default(),
				Transform::from_xyz(4.0, 0.0, 0.0),
				GlobalTransform::from_xyz(4.0, 0.0, 0.0),
			))
			.id();
		app.update();
		assert!(app
			.world()
			.get::<ThreatKnowledge>(victim)
			.is_some_and(ThreatKnowledge::is_empty));

		app.world_mut().write_message(DamageApplied {
			target: victim,
			source: Some(player),
			amount: 10.0,
			remaining: 90.0,
			point: Vec3::ZERO,
		});
		app.update();
		app.update();

		let player_id = ThreatId(player.to_bits());
		assert!(app
			.world()
			.get::<ThreatKnowledge>(victim)
			.is_some_and(|knowledge| { knowledge.get(player_id).is_some() }));
	}

	fn threat_app() -> App {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, TransformPlugin, ThreatIntelligenceDamagePlugin))
			.add_message::<DamageApplied>()
			.add_systems(Update, sync_world_player_threat_actor.in_set(ThreatSystems::Prepare));
		app
	}

	fn spawn_world_player(app: &mut App) -> Entity {
		app.world_mut()
			.spawn((VegetationPlayer, Transform::default(), GlobalTransform::default()))
			.id()
	}

	#[test]
	#[ignore]
	fn bake_lod_rank_scratch_timing() {
		const PLANT_COUNT: usize = 256;
		const BAKES: usize = 10_000;

		let mut plants: Vec<(Entity, IntelligenceLod)> = Vec::with_capacity(PLANT_COUNT);
		for index in 0..PLANT_COUNT {
			let entity = Entity::from_bits(index as u64 + 1);
			let band = match index % 3 {
				0 => IntelligenceBand::Near,
				1 => IntelligenceBand::Mid,
				_ => IntelligenceBand::Far,
			};
			let skips = (index % (IntelligenceLod::FAIRNESS_CAP as usize + 1)) as u8;
			plants.push((entity, IntelligenceLod { band, skips }));
		}

		let clone_start = std::time::Instant::now();
		for _ in 0..BAKES {
			let mut rows: Vec<(Entity, IntelligenceLod)> = Vec::with_capacity(PLANT_COUNT);
			for &(entity, lod) in &plants {
				rows.push((entity, lod));
			}
			rows.sort_by_key(|&(entity, lod)| lod.bake_key(entity));
			std::hint::black_box(&rows);
		}
		let clone_elapsed = clone_start.elapsed();

		let mut scratch: Vec<(Entity, IntelligenceBakeKey)> = Vec::with_capacity(PLANT_COUNT);
		let key_start = std::time::Instant::now();
		for _ in 0..BAKES {
			scratch.clear();
			scratch.reserve(PLANT_COUNT);
			for &(entity, lod) in &plants {
				scratch.push((entity, lod.bake_key(entity)));
			}
			scratch.sort_by_key(|&(_, key)| key);
			std::hint::black_box(&scratch);
		}
		let key_elapsed = key_start.elapsed();

		eprintln!(
			"bake_lod_rank_scratch_timing: {PLANT_COUNT} plants × {BAKES} bakes — clone lod rows {:?}, key scratch {:?}",
			clone_elapsed, key_elapsed,
		);
	}
}
