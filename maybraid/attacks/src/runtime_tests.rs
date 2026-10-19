use std::time::Duration;

use anyhow::{anyhow, Result};
use avian3d::prelude::{Collider, PhysicsPlugins, Sensor};
use bevy::asset::AssetPlugin;
use bevy::mesh::MeshPlugin;
use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use combat_targeting::{CombatContact, CombatTargeting, TargetSource};
use damage::{apply_hits, DamageApplied, Died, Downed, Health, Hit};
use lod_avian::PhysicsInteractionLayer;

use crate::{
	placeholder_svg, AimPolicy, AttackCatalog, AttackDefinition, AttackEndReason, AttackEnded,
	AttackId, AttackInstance, AttackPlaceholderPresentationPlugin, AttackPresentationBinding,
	AttackPresentationEvent, AttackRetired, AttackStage, AttackSuspended, AttackUser,
	AttacksPlugin, BeginAttack, CancelAttacks, HazardOrigin, HazardPose, HazardShape, HazardSpec,
	TelegraphSpec,
};

const COLUMN: AttackId = AttackId("column");
const WAVE: AttackId = AttackId("wave");

fn column_def() -> AttackDefinition {
	AttackDefinition::with_authored_defaults(COLUMN, 0.4, 0.8, 0.2, AimPolicy::TargetAtWindup)
		.with_telegraph(TelegraphSpec {
			shape: HazardShape::Sphere { radius: 1.5 },
			origin: HazardOrigin::AimPoint,
			persist_into_execute: false,
		})
		.authored_hazard(HazardShape::Cylinder { radius: 1.5, height: 7.0 }, HazardOrigin::AimPoint)
}

fn wave_def() -> AttackDefinition {
	AttackDefinition::with_authored_defaults(WAVE, 0.3, 1.0, 0.2, AimPolicy::TrackUntilExecute)
		.with_telegraph(TelegraphSpec {
			shape: HazardShape::Line { length: 8.0, half_width: 0.4 },
			origin: HazardOrigin::AimPoint,
			persist_into_execute: true,
		})
		.with_hazard(HazardSpec {
			shape: HazardShape::Wavefront(
				crate::WavefrontSpec::new(1.25, 8.0, 8.0).with_waves(2, 0.35).with_half_depth(0.3),
			),
			origin: HazardOrigin::Emission,
			damage: crate::DamageSpec::authored(),
		})
}

fn attack_app(dt: f32) -> App {
	let mut app = App::new();
	app.add_plugins((
		MinimalPlugins,
		TransformPlugin,
		PhysicsPlugins::default(),
		AssetPlugin::default(),
		MeshPlugin,
		AttacksPlugin,
	));
	app.finish();
	app
		.add_message::<DamageApplied>()
		.add_message::<Died>()
		.add_systems(PostUpdate, apply_hits)
		.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(dt)));
	app.world_mut()
		.resource_mut::<Time<Virtual>>()
		.set_max_delta(Duration::from_secs(30));
	app.world_mut()
		.resource_mut::<AttackCatalog>()
		.insert(column_def())
		.insert(wave_def());
	app.update();
	app
}

fn step(app: &mut App, dt: f32) {
	app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(dt)));
	app.update();
}

fn spawn_owner(app: &mut App, translation: Vec3) -> Entity {
	app.world_mut()
		.spawn((
			AttackUser::default(),
			Transform::from_translation(translation),
			GlobalTransform::from_translation(translation),
		))
		.id()
}

fn spawn_target(app: &mut App, translation: Vec3, health: f32) -> Entity {
	app.world_mut()
		.spawn((
			Health::from_max(health),
			Transform::from_translation(translation),
			GlobalTransform::from_translation(translation),
			Collider::capsule(0.4, 1.0),
			Sensor,
			PhysicsInteractionLayer::animated_layers(),
		))
		.id()
}

fn observe(app: &mut App, owner: Entity, target: Entity, point: Vec3) {
	let mut targeting = app.world().get::<CombatTargeting>(owner).cloned().unwrap_or_default();
	targeting.upsert_contact(CombatContact {
		subject: target,
		position: point,
		movement_vector: Vec3::ZERO,
		visible_point: point,
		visible_head: None,
		last_spotted_at: 0.0,
	});
	targeting.rebalance(0.0);
	app.world_mut().entity_mut(owner).insert(targeting);
}

fn cast(app: &mut App, owner: Entity, definition: AttackId, target: Entity, point: Vec3) {
	observe(app, owner, target, point);
	app.world_mut().write_message(BeginAttack::new(owner, definition).with_target(target));
}

fn live_instances(app: &mut App) -> usize {
	let mut query = app.world_mut().query::<&AttackInstance>();
	query.iter(app.world()).count()
}

fn first_instance(app: &mut App) -> Option<AttackInstance> {
	let mut query = app.world_mut().query::<&AttackInstance>();
	query.iter(app.world()).next().cloned()
}

fn drain_hits(app: &mut App) -> Vec<Hit> {
	app.world_mut().resource_mut::<Messages<Hit>>().drain().collect()
}

fn drain_ended(app: &mut App) -> Vec<AttackEnded> {
	app.world_mut().resource_mut::<Messages<AttackEnded>>().drain().collect()
}

fn drain_presented(app: &mut App) -> Vec<AttackPresentationEvent> {
	app.world_mut().resource_mut::<Messages<AttackPresentationEvent>>().drain().collect()
}

#[test]
fn attack_applies_hit_without_a_presenter() -> Result<()> {
	let mut app = attack_app(0.1);
	let owner = spawn_owner(&mut app, Vec3::ZERO);
	let target = spawn_target(&mut app, Vec3::new(0.0, 1.0, -1.0), 100.0);
	cast(&mut app, owner, COLUMN, target, Vec3::new(0.0, 1.0, -1.0));
	step(&mut app, 0.1);
	step(&mut app, 0.4);
	let hits = drain_hits(&mut app);
	if hits.is_empty() {
		return Err(anyhow!("disabled VFX must still write Hit"));
	}
	assert_eq!(hits[0].target, target);
	assert_eq!(hits[0].source, Some(owner));
	assert!((hits[0].amount - crate::AttackDefaults::AUTHORED.damage).abs() < 1e-4);
	Ok(())
}

#[test]
fn stage_boundaries_spawn_and_clear_volumes() -> Result<()> {
	let mut app = attack_app(0.1);
	let owner = spawn_owner(&mut app, Vec3::ZERO);
	let target = spawn_target(&mut app, Vec3::new(0.0, 1.0, -1.0), 100.0);
	cast(&mut app, owner, COLUMN, target, Vec3::new(0.0, 1.0, -1.0));
	step(&mut app, 0.1);
	let instance =
		first_instance(&mut app).ok_or_else(|| anyhow!("expected a live instance in windup"))?;
	if instance.stage != AttackStage::Windup {
		return Err(anyhow!("expected windup, got {:?}", instance.stage));
	}
	if instance.age <= 0.0 {
		return Err(anyhow!("tick should advance age, got {}", instance.age));
	}
	step(&mut app, 0.4);
	let instance = first_instance(&mut app).ok_or_else(|| anyhow!("expected execute"))?;
	if instance.stage != AttackStage::Execute {
		return Err(anyhow!("expected execute, got {:?}", instance.stage));
	}
	step(&mut app, 0.8);
	let instance = first_instance(&mut app).ok_or_else(|| anyhow!("expected recover"))?;
	if instance.stage != AttackStage::Recover {
		return Err(anyhow!("expected recover, got {:?}", instance.stage));
	}
	step(&mut app, 0.3);
	if live_instances(&mut app) != 0 {
		return Err(anyhow!("instance should complete after recover"));
	}
	Ok(())
}

#[test]
fn cancel_removes_hazards_and_warning_bindings() -> Result<()> {
	let mut app = attack_app(0.1);
	let owner = spawn_owner(&mut app, Vec3::ZERO);
	let target = spawn_target(&mut app, Vec3::new(0.0, 1.0, -1.0), 100.0);
	let binding = app
		.world_mut()
		.spawn(AttackPresentationBinding {
			instance: crate::AttackInstanceId(1),
			kind: crate::AttackPresentationBindingKind::Telegraph,
		})
		.id();
	cast(&mut app, owner, COLUMN, target, Vec3::new(0.0, 1.0, -1.0));
	step(&mut app, 0.1);
	let instance_id = first_instance(&mut app)
		.map(|instance| instance.id)
		.ok_or_else(|| anyhow!("missing instance"))?;
	app.world_mut().entity_mut(binding).insert(AttackPresentationBinding {
		instance: instance_id,
		kind: crate::AttackPresentationBindingKind::Hazard { index: 0, wave: 0 },
	});
	app.world_mut().write_message(CancelAttacks::all(owner));
	step(&mut app, 0.1);
	if live_instances(&mut app) != 0 {
		return Err(anyhow!("cancel should despawn the instance"));
	}
	if app.world().get_entity(binding).is_ok() {
		return Err(anyhow!("cancel should despawn presentation bindings"));
	}
	let ended = drain_ended(&mut app);
	if !ended.iter().any(|event| event.reason == AttackEndReason::Cancelled) {
		return Err(anyhow!("expected cancelled end"));
	}
	Ok(())
}

#[test]
fn multiple_colliders_on_one_target_make_one_hit() -> Result<()> {
	let mut app = attack_app(0.1);
	let owner = spawn_owner(&mut app, Vec3::ZERO);
	let body = spawn_target(&mut app, Vec3::new(0.0, 1.0, 0.0), 100.0);
	app.world_mut().spawn((
		ChildOf(body),
		Transform::from_translation(Vec3::new(0.2, 1.0, 0.0)),
		GlobalTransform::from_translation(Vec3::new(0.2, 1.0, 0.0)),
		Collider::capsule(0.3, 0.6),
		Sensor,
		PhysicsInteractionLayer::animated_layers(),
	));
	app.world_mut().spawn((
		ChildOf(body),
		Transform::from_translation(Vec3::new(-0.2, 1.0, 0.0)),
		GlobalTransform::from_translation(Vec3::new(-0.2, 1.0, 0.0)),
		Collider::capsule(0.3, 0.6),
		Sensor,
		PhysicsInteractionLayer::animated_layers(),
	));
	cast(&mut app, owner, COLUMN, body, Vec3::new(0.0, 1.0, 0.0));
	step(&mut app, 0.1);
	step(&mut app, 0.4);
	let hits: Vec<_> = drain_hits(&mut app).into_iter().filter(|hit| hit.target == body).collect();
	if hits.len() != 1 {
		return Err(anyhow!("expected one remounted hit, got {}", hits.len()));
	}
	Ok(())
}

#[test]
fn frame_rate_variation_does_not_dump_or_skip_execute() -> Result<()> {
	let mut fine = attack_app(1.0 / 60.0);
	let owner = spawn_owner(&mut fine, Vec3::ZERO);
	let target = spawn_target(&mut fine, Vec3::new(0.0, 1.0, 0.0), 100.0);
	cast(&mut fine, owner, COLUMN, target, Vec3::new(0.0, 1.0, 0.0));
	for _ in 0..24 {
		step(&mut fine, 1.0 / 60.0);
	}
	let fine_hits = drain_hits(&mut fine).len();

	let mut hitch = attack_app(2.0);
	let owner = spawn_owner(&mut hitch, Vec3::ZERO);
	let target = spawn_target(&mut hitch, Vec3::new(0.0, 1.0, 0.0), 100.0);
	cast(&mut hitch, owner, COLUMN, target, Vec3::new(0.0, 1.0, 0.0));
	step(&mut hitch, 2.0);
	let hitch_hits = drain_hits(&mut hitch).len();
	if hitch_hits == 0 {
		return Err(anyhow!("a hitch must still pulse execute"));
	}
	if hitch_hits > fine_hits {
		return Err(anyhow!("hitch dumped extra hits: hitch={hitch_hits} fine={fine_hits}"));
	}
	Ok(())
}

#[test]
fn target_loss_keeps_last_snapshot() -> Result<()> {
	let mut app = attack_app(0.1);
	let owner = spawn_owner(&mut app, Vec3::ZERO);
	let target = spawn_target(&mut app, Vec3::new(0.0, 0.0, -4.0), 100.0);
	cast(&mut app, owner, WAVE, target, Vec3::new(0.0, 0.0, -4.0));
	step(&mut app, 0.1);
	let before = first_instance(&mut app)
		.map(|instance| instance.snapshot)
		.ok_or_else(|| anyhow!("missing snapshot"))?;
	app.world_mut().entity_mut(target).despawn();
	step(&mut app, 0.1);
	let after =
		first_instance(&mut app).ok_or_else(|| anyhow!("instance should survive target loss"))?;
	if after.snapshot.aim_point != before.aim_point {
		return Err(anyhow!("aim must stay on the last observed point, not a live transform"));
	}
	if after.snapshot.direction != before.direction {
		return Err(anyhow!("snapshot should stay at last known aim"));
	}
	Ok(())
}

#[test]
fn owner_despawn_cleans_instance_and_bindings() -> Result<()> {
	let mut app = attack_app(0.1);
	let owner = spawn_owner(&mut app, Vec3::ZERO);
	let target = spawn_target(&mut app, Vec3::new(0.0, 1.0, -1.0), 100.0);
	cast(&mut app, owner, COLUMN, target, Vec3::new(0.0, 1.0, -1.0));
	step(&mut app, 0.1);
	let instance_id = first_instance(&mut app)
		.map(|instance| instance.id)
		.ok_or_else(|| anyhow!("missing instance"))?;
	let binding = app
		.world_mut()
		.spawn(AttackPresentationBinding {
			instance: instance_id,
			kind: crate::AttackPresentationBindingKind::Telegraph,
		})
		.id();
	app.world_mut().entity_mut(owner).despawn();
	step(&mut app, 0.1);
	if live_instances(&mut app) != 0 {
		return Err(anyhow!("owner despawn should retire the instance"));
	}
	if app.world().get_entity(binding).is_ok() {
		return Err(anyhow!("owner despawn should clear bindings"));
	}
	Ok(())
}

#[test]
fn sleep_and_retirement_interrupt_live_attacks() -> Result<()> {
	let mut app = attack_app(0.1);
	let sleeper = spawn_owner(&mut app, Vec3::ZERO);
	let retired = spawn_owner(&mut app, Vec3::new(6.0, 0.0, 0.0));
	let target = spawn_target(&mut app, Vec3::new(0.0, 1.0, -1.0), 100.0);
	cast(&mut app, sleeper, COLUMN, target, Vec3::new(0.0, 1.0, -1.0));
	cast(&mut app, retired, COLUMN, target, Vec3::new(0.0, 1.0, -1.0));
	step(&mut app, 0.1);
	if live_instances(&mut app) != 2 {
		return Err(anyhow!("both owners should share the same definition"));
	}
	app.world_mut().entity_mut(sleeper).insert(AttackSuspended);
	app.world_mut().entity_mut(retired).insert(AttackRetired);
	step(&mut app, 0.1);
	if live_instances(&mut app) != 0 {
		return Err(anyhow!("sleep and retirement must clean up"));
	}
	let ended = drain_ended(&mut app);
	assert!(ended.iter().any(|event| event.reason == AttackEndReason::OwnerSuspended));
	assert!(ended.iter().any(|event| event.reason == AttackEndReason::OwnerRetired));
	Ok(())
}

#[test]
fn death_interrupts_live_attacks() -> Result<()> {
	let mut app = attack_app(0.1);
	let owner = spawn_owner(&mut app, Vec3::ZERO);
	let target = spawn_target(&mut app, Vec3::new(0.0, 1.0, -1.0), 100.0);
	cast(&mut app, owner, COLUMN, target, Vec3::new(0.0, 1.0, -1.0));
	step(&mut app, 0.1);
	app.world_mut()
		.entity_mut(owner)
		.insert(Downed { source: None, point: Vec3::ZERO, at: 0.0 });
	step(&mut app, 0.1);
	if live_instances(&mut app) != 0 {
		return Err(anyhow!("death must clean up hazards and telegraphs"));
	}
	let ended = drain_ended(&mut app);
	if !ended.iter().any(|event| event.reason == AttackEndReason::OwnerDowned) {
		return Err(anyhow!("expected owner-downed end"));
	}
	Ok(())
}

#[test]
fn targeting_membership_is_used_without_fabricating_contacts() -> Result<()> {
	let mut app = attack_app(0.1);
	let owner = spawn_owner(&mut app, Vec3::ZERO);
	let target = spawn_target(&mut app, Vec3::new(0.0, 1.0, -1.0), 100.0);
	let mut targeting = CombatTargeting::default();
	targeting.include(target, TargetSource::OBJECTIVE);
	targeting.rebalance(0.0);
	app.world_mut().entity_mut(owner).insert(targeting);
	app.world_mut().write_message(BeginAttack::new(owner, COLUMN));
	step(&mut app, 0.1);
	let instance = first_instance(&mut app)
		.ok_or_else(|| anyhow!("expected instance even without observed knowledge"))?;
	if instance.live_target.is_some() {
		return Err(anyhow!("membership without contact must not aim at a live transform"));
	}
	if (instance.snapshot.aim_point - instance.snapshot.origin).length() > 1e-4 {
		return Err(anyhow!("without contact, AimPoint must stay on the owner"));
	}
	if app
		.world()
		.get::<CombatTargeting>(owner)
		.and_then(|targeting| targeting.contact(target))
		.is_some()
	{
		return Err(anyhow!("attack runtime must not fabricate contacts"));
	}
	Ok(())
}

#[test]
fn placeholder_preview_svg_records_telegraph_and_hazard() -> Result<()> {
	let telegraph = (
		HazardPose::new(Vec3::ZERO, Quat::IDENTITY),
		HazardShape::Line { length: 8.0, half_width: 0.35 },
	);
	let hazard = (
		HazardPose::new(Vec3::new(0.0, 0.0, 2.0), Quat::IDENTITY),
		HazardShape::Wavefront(crate::WavefrontSpec::new(1.2, 4.0, 8.0)),
	);
	let svg = placeholder_svg(telegraph, hazard);
	let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
		.join("preview/placeholder-telegraph-hazard.svg");
	if let Some(parent) = path.parent() {
		std::fs::create_dir_all(parent)?;
	}
	std::fs::write(&path, svg)?;
	let written = std::fs::read_to_string(path)?;
	if !written.contains("#f0c12a") || !written.contains("#c42b24") {
		return Err(anyhow!("preview svg missing placeholder colors"));
	}
	Ok(())
}

#[test]
fn wavefront_hits_once_per_wave_and_exposes_kinematics() -> Result<()> {
	let mut app = attack_app(0.1);
	let wave = AttackId::new("wave-once");
	app.world_mut().resource_mut::<AttackCatalog>().insert(
		AttackDefinition::with_authored_defaults(wave, 0.2, 1.2, 0.2, AimPolicy::TargetAtWindup)
			.with_hazard(HazardSpec {
				shape: HazardShape::Wavefront(
					crate::WavefrontSpec::new(1.4, 8.0, 8.0).with_waves(2, 0.4).with_half_depth(0.3),
				),
				origin: HazardOrigin::Emission,
				damage: crate::DamageSpec::once(20.0),
			}),
	);
	let owner = spawn_owner(&mut app, Vec3::ZERO);
	let target = spawn_target(&mut app, Vec3::new(0.0, 0.0, -2.0), 100.0);
	let mouth = Vec3::new(0.0, 0.0, -0.2);
	observe(&mut app, owner, target, Vec3::new(0.0, 0.0, -2.0));
	app.world_mut().write_message(
		BeginAttack::new(owner, wave).with_target(target).with_emission(mouth),
	);
	step(&mut app, 0.2);
	step(&mut app, 0.3);
	let first = drain_hits(&mut app);
	if first.len() != 1 {
		return Err(anyhow!("wave 0 should hit once, got {}", first.len()));
	}
	let presented = drain_presented(&mut app);
	let Some(wavefront) = presented.iter().find_map(|event| match &event.kind {
		crate::AttackPresentationKind::Hazard { wave: Some(wave), .. } => Some(*wave),
		_ => None,
	}) else {
		return Err(anyhow!("expected wavefront presentation kinematics"));
	};
	if (wavefront.origin - mouth).length() > 1e-3 {
		return Err(anyhow!("presenter origin should be the emission, got {:?}", wavefront.origin));
	}
	if (wavefront.direction - Vec3::NEG_Z).length() > 1e-3 {
		return Err(anyhow!("presenter direction should be flattened heading"));
	}
	if (wavefront.speed - 8.0).abs() > 1e-4 || (wavefront.range - 8.0).abs() > 1e-4 {
		return Err(anyhow!("presenter missing speed/range"));
	}
	if (wavefront.radius - 1.4).abs() > 1e-4 {
		return Err(anyhow!("presenter missing ring radius"));
	}
	if (wavefront.thickness - 0.6).abs() > 1e-4 {
		return Err(anyhow!("presenter missing ring thickness"));
	}
	if !presented.iter().any(|event| matches!(
		&event.kind,
		crate::AttackPresentationKind::Hazard {
			wave: Some(wave),
			phase: crate::PresentationPhase::Updated,
			..
		} if (wave.position - wave.origin).length() > 0.2
	)) {
		return Err(anyhow!("updated wave should carry a current position away from the mouth"));
	}
	step(&mut app, 0.4);
	let second = drain_hits(&mut app);
	if second.len() != 1 {
		return Err(anyhow!("wave 1 should hit the same target once, got {}", second.len()));
	}
	let later = drain_presented(&mut app);
	if !later.iter().any(|event| matches!(
		&event.kind,
		crate::AttackPresentationKind::Hazard { wave: Some(wave), .. } if wave.wave_index == 1
	)) {
		return Err(anyhow!("second wave should announce wave_index 1"));
	}
	Ok(())
}

#[test]
fn aim_point_is_observed_and_distinct_from_owner() -> Result<()> {
	let mut app = attack_app(0.1);
	let owner = spawn_owner(&mut app, Vec3::new(0.0, 1.0, 4.0));
	let seen = Vec3::new(6.0, 2.0, -5.0);
	let target = spawn_target(&mut app, seen, 100.0);
	cast(&mut app, owner, COLUMN, target, seen);
	step(&mut app, 0.1);
	let instance = first_instance(&mut app).ok_or_else(|| anyhow!("missing instance"))?;
	if (instance.snapshot.origin - Vec3::new(0.0, 1.0, 4.0)).length() > 1e-3 {
		return Err(anyhow!("owner origin should stay on the caster"));
	}
	if (instance.snapshot.aim_point - seen).length() > 1e-3 {
		return Err(anyhow!("AimPoint must be the observed world point, got {:?}", instance.snapshot.aim_point));
	}
	if (HazardOrigin::AimPoint.resolve(instance.snapshot) - seen).length() > 1e-3 {
		return Err(anyhow!("God Beam style AimPoint resolved to the attacker"));
	}
	if (HazardOrigin::Owner.resolve(instance.snapshot) - instance.snapshot.origin).length() > 1e-3 {
		return Err(anyhow!("Owner origin should remain the caster"));
	}
	Ok(())
}

#[test]
fn hazard_hits_collider_extent_not_just_origin() -> Result<()> {
	let mut app = attack_app(0.1);
	let id = AttackId::new("close-sphere");
	app.world_mut().resource_mut::<AttackCatalog>().insert(
		AttackDefinition::with_authored_defaults(id, 0.1, 0.4, 0.1, AimPolicy::TargetAtWindup)
			.authored_hazard(HazardShape::Sphere { radius: 1.0 }, HazardOrigin::Owner),
	);
	let owner = spawn_owner(&mut app, Vec3::ZERO);
	let target = spawn_target(&mut app, Vec3::new(1.25, 0.0, 0.0), 100.0);
	cast(&mut app, owner, id, target, Vec3::new(1.25, 0.0, 0.0));
	step(&mut app, 0.1);
	step(&mut app, 0.2);
	let hits = drain_hits(&mut app);
	if hits.is_empty() {
		return Err(anyhow!("capsule intersecting the sphere must hit even if origin is outside"));
	}
	Ok(())
}

#[test]
fn placeholder_plugin_is_optional() {
	let _ = AttackPlaceholderPresentationPlugin;
	let _ = std::any::type_name::<AttackPresentationEvent>();
}

const PARADE: AttackId = AttackId("parade");
const PARADE_BLAST: AttackId = AttackId("parade_blast");

fn parade_parent() -> AttackDefinition {
	AttackDefinition::with_authored_defaults(PARADE, 0.2, 1.0, 0.2, AimPolicy::TrackUntilExecute)
}

fn parade_blast() -> AttackDefinition {
	AttackDefinition::with_authored_defaults(PARADE_BLAST, 0.0, 0.3, 0.0, AimPolicy::OwnerPoseAtWindup)
		.with_concurrent(true)
		.authored_hazard(HazardShape::Sphere { radius: 1.2 }, HazardOrigin::Emission)
}

fn insert_parade_catalog(app: &mut App) {
	app.world_mut()
		.resource_mut::<AttackCatalog>()
		.insert(parade_parent())
		.insert(parade_blast());
}

fn live_definitions(app: &mut App) -> Vec<AttackId> {
	let mut query = app.world_mut().query::<&AttackInstance>();
	query.iter(app.world()).map(|instance| instance.definition).collect()
}

fn live_owners(app: &mut App) -> Vec<Entity> {
	let mut query = app.world_mut().query::<&AttackInstance>();
	query.iter(app.world()).map(|instance| instance.owner).collect()
}

#[test]
fn same_frame_duplicate_nonconcurrent_admits_one() -> Result<()> {
	let mut app = attack_app(0.1);
	let owner = spawn_owner(&mut app, Vec3::ZERO);
	let target = spawn_target(&mut app, Vec3::new(0.0, 1.0, -1.0), 100.0);
	observe(&mut app, owner, target, Vec3::new(0.0, 1.0, -1.0));
	app.world_mut().write_message(BeginAttack::new(owner, COLUMN).with_target(target));
	app.world_mut().write_message(BeginAttack::new(owner, WAVE).with_target(target));
	step(&mut app, 0.1);
	let live = live_definitions(&mut app);
	if live != [COLUMN] {
		return Err(anyhow!(
			"same-batch exclusive begins must admit only the first, got {live:?}"
		));
	}
	Ok(())
}

#[test]
fn later_frame_exclusive_begin_still_refused_while_live() -> Result<()> {
	let mut app = attack_app(0.1);
	let owner = spawn_owner(&mut app, Vec3::ZERO);
	let target = spawn_target(&mut app, Vec3::new(0.0, 1.0, -1.0), 100.0);
	cast(&mut app, owner, COLUMN, target, Vec3::new(0.0, 1.0, -1.0));
	step(&mut app, 0.1);
	app.world_mut().write_message(BeginAttack::new(owner, WAVE).with_target(target));
	step(&mut app, 0.1);
	if live_instances(&mut app) != 1 {
		return Err(anyhow!("a live exclusive instance must still block later exclusive begins"));
	}
	if live_definitions(&mut app) != [COLUMN] {
		return Err(anyhow!("the original exclusive instance must remain the only live one"));
	}
	Ok(())
}

#[test]
fn concurrent_parade_blasts_admit_beside_parent_in_one_batch() -> Result<()> {
	let mut app = attack_app(0.1);
	insert_parade_catalog(&mut app);
	let owner = spawn_owner(&mut app, Vec3::ZERO);
	let target = spawn_target(&mut app, Vec3::new(0.0, 1.0, -2.0), 100.0);
	observe(&mut app, owner, target, Vec3::new(0.0, 1.0, -2.0));
	app.world_mut().write_message(BeginAttack::new(owner, PARADE).with_target(target));
	app.world_mut().write_message(
		BeginAttack::new(owner, PARADE_BLAST)
			.with_target(target)
			.with_emission(Vec3::new(0.0, 0.0, -2.0)),
	);
	app.world_mut().write_message(
		BeginAttack::new(owner, PARADE_BLAST)
			.with_target(target)
			.with_emission(Vec3::new(0.0, 0.0, -4.0)),
	);
	app.world_mut().write_message(BeginAttack::new(owner, PARADE).with_target(target));
	step(&mut app, 0.1);
	let mut live = live_definitions(&mut app);
	live.sort_by_key(|id| id.as_str());
	if live != [PARADE, PARADE_BLAST, PARADE_BLAST] {
		return Err(anyhow!(
			"parent plus two concurrent blasts must admit; duplicate parent must not, got {live:?}"
		));
	}
	Ok(())
}

#[test]
fn concurrent_blasts_admit_after_parent_is_already_live() -> Result<()> {
	let mut app = attack_app(0.1);
	insert_parade_catalog(&mut app);
	let owner = spawn_owner(&mut app, Vec3::ZERO);
	let target = spawn_target(&mut app, Vec3::new(0.0, 1.0, -2.0), 100.0);
	cast(&mut app, owner, PARADE, target, Vec3::new(0.0, 1.0, -2.0));
	step(&mut app, 0.1);
	app.world_mut().write_message(
		BeginAttack::new(owner, PARADE_BLAST)
			.with_target(target)
			.with_emission(Vec3::new(0.0, 0.0, -2.0)),
	);
	app.world_mut().write_message(
		BeginAttack::new(owner, PARADE_BLAST)
			.with_target(target)
			.with_emission(Vec3::new(0.0, 0.0, -4.0)),
	);
	step(&mut app, 0.1);
	if live_instances(&mut app) != 3 {
		return Err(anyhow!(
			"live parent must not block concurrent child explosions, got {}",
			live_instances(&mut app)
		));
	}
	Ok(())
}

#[test]
fn cancel_clears_parent_and_concurrent_children() -> Result<()> {
	let mut app = attack_app(0.1);
	insert_parade_catalog(&mut app);
	let owner = spawn_owner(&mut app, Vec3::ZERO);
	let target = spawn_target(&mut app, Vec3::new(0.0, 1.0, -2.0), 100.0);
	observe(&mut app, owner, target, Vec3::new(0.0, 1.0, -2.0));
	app.world_mut().write_message(BeginAttack::new(owner, PARADE).with_target(target));
	app.world_mut().write_message(
		BeginAttack::new(owner, PARADE_BLAST)
			.with_target(target)
			.with_emission(Vec3::new(0.0, 0.0, -2.0)),
	);
	step(&mut app, 0.1);
	if live_instances(&mut app) != 2 {
		return Err(anyhow!("expected parent and blast before cancel"));
	}
	app.world_mut().write_message(CancelAttacks::all(owner));
	step(&mut app, 0.1);
	if live_instances(&mut app) != 0 {
		return Err(anyhow!("cancel must retire parent and concurrent children"));
	}
	let ended = drain_ended(&mut app);
	if ended.iter().filter(|event| event.reason == AttackEndReason::Cancelled).count() != 2 {
		return Err(anyhow!("expected cancelled ends for parent and blast, got {ended:?}"));
	}
	Ok(())
}

#[test]
fn owner_death_clears_parent_and_concurrent_children() -> Result<()> {
	let mut app = attack_app(0.1);
	insert_parade_catalog(&mut app);
	let owner = spawn_owner(&mut app, Vec3::ZERO);
	let target = spawn_target(&mut app, Vec3::new(0.0, 1.0, -2.0), 100.0);
	observe(&mut app, owner, target, Vec3::new(0.0, 1.0, -2.0));
	app.world_mut().write_message(BeginAttack::new(owner, PARADE).with_target(target));
	app.world_mut().write_message(
		BeginAttack::new(owner, PARADE_BLAST)
			.with_target(target)
			.with_emission(Vec3::new(0.0, 0.0, -2.0)),
	);
	step(&mut app, 0.1);
	app.world_mut()
		.entity_mut(owner)
		.insert(Downed { source: None, point: Vec3::ZERO, at: 0.0 });
	step(&mut app, 0.1);
	if live_instances(&mut app) != 0 {
		return Err(anyhow!("owner death must clean parent and concurrent children"));
	}
	if !drain_ended(&mut app)
		.iter()
		.all(|event| event.reason == AttackEndReason::OwnerDowned)
	{
		return Err(anyhow!("expected owner-downed ends for every live instance"));
	}
	Ok(())
}

#[test]
fn exclusive_admission_is_per_owner() -> Result<()> {
	let mut app = attack_app(0.1);
	let first = spawn_owner(&mut app, Vec3::ZERO);
	let second = spawn_owner(&mut app, Vec3::new(8.0, 0.0, 0.0));
	let target = spawn_target(&mut app, Vec3::new(0.0, 1.0, -1.0), 100.0);
	observe(&mut app, first, target, Vec3::new(0.0, 1.0, -1.0));
	observe(&mut app, second, target, Vec3::new(0.0, 1.0, -1.0));
	app.world_mut().write_message(BeginAttack::new(first, COLUMN).with_target(target));
	app.world_mut().write_message(BeginAttack::new(second, COLUMN).with_target(target));
	step(&mut app, 0.1);
	let owners = live_owners(&mut app);
	if owners.len() != 2 || !owners.contains(&first) || !owners.contains(&second) {
		return Err(anyhow!("exclusive lock must not leak across owners, got {owners:?}"));
	}
	Ok(())
}
