use std::collections::BTreeMap;

use avian3d::prelude::{SpatialQuery, SpatialQueryFilter};
use bevy::prelude::*;
use bevy::transform::helper::TransformHelper;
use intelligence_lod::{due_by_rank, IntelligenceBand, IntelligenceLod, IntelligencePriority};
use lod_avian::PhysicsInteractionLayer;
use spotting_intelligence::{SpotCandidate, SpotSubject, SpottingObserveLimits, SpottingUser};

use crate::candidate::ProbeCandidate;
use crate::discover::{discover_subjects, merge_due_contacts, next_discovery_interval};
use crate::probe::probe_candidate_visibility;

fn forget_invalid_contacts(
	now: f32,
	spotters: &mut Query<(Entity, &mut SpottingUser, Option<&mut IntelligenceLod>)>,
	subjects: &Query<(Entity, &SpotSubject, Option<&avian3d::prelude::LinearVelocity>)>,
) {
	for (_, mut user, _) in spotters.iter_mut() {
		user.forget_stale(now);
		user.contacts.retain(|entity, _| subjects.get(*entity).is_ok());
	}
}

fn due_spotters(
	now: f32,
	spotters: &mut Query<(Entity, &mut SpottingUser, Option<&mut IntelligenceLod>)>,
) -> Vec<Entity> {
	spotters
		.iter_mut()
		.filter_map(|(entity, user, _)| {
			let due = now >= user.next_discovery_at
				|| user.contacts.values().any(|contact| contact.is_due(now));
			due.then_some(entity)
		})
		.collect()
}

fn fair_far_spotter(
	due: &[Entity],
	spotters: &mut Query<(Entity, &mut SpottingUser, Option<&mut IntelligenceLod>)>,
) -> Option<Entity> {
	due.iter().copied().find(|entity| {
		spotters.get_mut(*entity).is_ok_and(|(_, _, lod)| {
			lod.as_deref().is_some_and(|lod| {
				lod.band != IntelligenceBand::Near && lod.skips >= IntelligenceLod::FAIRNESS_CAP
			})
		})
	})
}

fn schedule_discovery(
	now: f32,
	user: &mut SpottingUser,
	band: IntelligenceBand,
	spotter_entity: Entity,
	fair: Option<Entity>,
) -> (bool, f32, usize) {
	let discovery_due = now >= user.next_discovery_at;
	let next_interval = next_discovery_interval(user);
	let run_discovery =
		discovery_due && (band != IntelligenceBand::Far || Some(spotter_entity) == fair);
	let discovery_sample_cursor = if run_discovery { user.advance_sample_cursor() } else { 0 };
	(run_discovery, next_interval, discovery_sample_cursor)
}

fn advance_discovery_clock(
	now: f32,
	user: &mut SpottingUser,
	band: IntelligenceBand,
	run_discovery: bool,
	discovery_due: bool,
	next_interval: f32,
) {
	if run_discovery {
		// Fair Far uses the raw interval so the reserve is not immediately
		// stretched away; Mid / Near still use `interval_scale`.
		let scale = if band == IntelligenceBand::Far { 1.0 } else { band.interval_scale() };
		user.next_discovery_at = now + next_interval * scale;
	} else if discovery_due {
		user.next_discovery_at = now + next_interval * IntelligenceBand::Far.interval_scale();
	}
}

/// Discover, rank, and visibility-probe subjects. Far skips Animated broadphase
/// unless a fairness reserve is due. `forget_stale` still runs on everyone.
/// `skips` reset only when discovery ran, matching `replan_movement`.
pub fn observe_spotting(
	spatial: SpatialQuery,
	time: Res<Time>,
	priority: Res<IntelligencePriority>,
	limits: Res<SpottingObserveLimits>,
	mut spotters: Query<(Entity, &mut SpottingUser, Option<&mut IntelligenceLod>)>,
	subjects: Query<(Entity, &SpotSubject, Option<&avian3d::prelude::LinearVelocity>)>,
	parents: Query<&ChildOf>,
	transforms: TransformHelper,
) {
	let now = time.elapsed_secs();
	let animated_filter = SpatialQueryFilter::from_mask(PhysicsInteractionLayer::Animated);
	let fixed_filter = SpatialQueryFilter::from_mask(PhysicsInteractionLayer::Fixed);

	forget_invalid_contacts(now, &mut spotters, &subjects);

	let mut due = due_spotters(now, &mut spotters);
	due_by_rank(&mut due, &priority);
	let fair = fair_far_spotter(&due, &mut spotters);

	let mut remaining = limits.max_observers_per_tick;
	for spotter_entity in due {
		if remaining == 0 {
			break;
		}
		let Ok((_, mut user, mut lod)) = spotters.get_mut(spotter_entity) else {
			continue;
		};
		let band = IntelligenceLod::band_or_near(lod.as_deref());
		let Ok(spotter_transform) = transforms.compute_global_transform(spotter_entity) else {
			continue;
		};
		let observer =
			spotter_transform.translation() + spotter_transform.rotation() * user.eye_offset;
		let discovery_due = now >= user.next_discovery_at;
		let (run_discovery, next_interval, discovery_sample_cursor) =
			schedule_discovery(now, &mut user, band, spotter_entity, fair);
		let mut candidates = BTreeMap::<Entity, ProbeCandidate>::new();

		if run_discovery {
			remaining -= 1;
			discover_subjects(
				spotter_entity,
				&user,
				now,
				observer,
				&spatial,
				&animated_filter,
				&subjects,
				&parents,
				&transforms,
				&mut candidates,
			);
		}
		advance_discovery_clock(now, &mut user, band, run_discovery, discovery_due, next_interval);

		merge_due_contacts(&user, now, observer, &subjects, &transforms, &mut candidates);

		let mut ranked: Vec<SpotCandidate> =
			candidates.values().map(|candidate| candidate.rank).collect();
		SpotCandidate::rank(&mut ranked);
		let candidate_budget = band.scale_count(user.settings.candidate_budget);
		let vision_samples = band.scale_count(user.settings.vision_samples);
		SpotCandidate::apply_budget(&mut ranked, candidate_budget);
		let grants =
			SpotCandidate::allocate_sample_budget(&ranked, candidate_budget, vision_samples);

		probe_candidate_visibility(
			&mut user,
			now,
			observer,
			discovery_sample_cursor,
			&candidates,
			ranked,
			grants,
			&spatial,
			&fixed_filter,
			&subjects,
			&transforms,
		);

		// Reset only when discovery ran. A Far skip still stretches
		// `next_discovery_at` and still merges due contacts, but must not wipe
		// bake increments or `FAIRNESS_CAP` never fires in a Far-only due set.
		if run_discovery {
			if let Some(lod) = lod.as_deref_mut() {
				if lod.skips != 0 {
					lod.skips = 0;
				}
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::candidate::ProbeCandidate;
	use avian3d::prelude::{Collider, PhysicsPlugins, RigidBody};
	use intelligence_lod::{IntelligenceBand, IntelligenceLod, IntelligencePriority};
	use spotting_intelligence::{
		InterestLayers, SpotBounds, SpotDirective, SpottedContact, SpottingHint,
		SpottingObserveLimits, SpottingSettings,
	};

	fn observe_app() -> App {
		let mut app = App::new();
		app.add_plugins((
			MinimalPlugins,
			TransformPlugin,
			PhysicsPlugins::default(),
			bevy::asset::AssetPlugin::default(),
			bevy::mesh::MeshPlugin,
		));
		app.add_plugins(crate::SpottingAvianPlugin);
		app.finish();
		app
	}

	fn character_user() -> SpottingUser {
		SpottingUser::new(Vec3::Y * 1.6, [SpotDirective::new(InterestLayers::CHARACTER, 20.0)])
			.with_settings(SpottingSettings::new(4, 4, 2.0))
	}

	fn spawn_subject(app: &mut App, at: Vec3) -> Entity {
		app.world_mut()
			.spawn((
				Transform::from_translation(at),
				SpotSubject::new(InterestLayers::CHARACTER, SpotBounds::capsule(0.4, 0.9)),
			))
			.id()
	}

	fn spawn_animated_subject(app: &mut App, at: Vec3) -> Entity {
		let entity = app
			.world_mut()
			.spawn((
				Transform::from_translation(at),
				GlobalTransform::from_translation(at),
				RigidBody::Static,
				Collider::sphere(0.5),
				PhysicsInteractionLayer::animated_layers(),
				SpotSubject::new(InterestLayers::CHARACTER, SpotBounds::capsule(0.4, 0.9)),
			))
			.id();
		app.update();
		entity
	}

	#[test]
	fn merge_uses_highest_priority_policy() -> anyhow::Result<()> {
		let subject = Entity::from_bits(1);
		let low = SpotDirective {
			priority: 1,
			respot_interval_secs: 0.5,
			max_samples_per_subject: 9,
			..SpotDirective::default()
		};
		let high = SpotDirective {
			priority: 3,
			respot_interval_secs: 0.1,
			max_samples_per_subject: 2,
			..SpotDirective::default()
		};
		let mut candidate = ProbeCandidate::new(subject, low, 1.0, 2.0, false, 9);
		candidate.merge(ProbeCandidate::new(subject, high, 1.0, 2.0, true, 9));
		assert_eq!(candidate.rank.directive_priority, 3);
		assert_eq!(candidate.rank.max_samples, 2);
		assert!(candidate.rank.known);
		assert_eq!(candidate.respot_interval_secs, 0.1);
		Ok(())
	}

	#[test]
	fn explicit_hint_reaches_visibility_without_a_broadphase_collider() -> anyhow::Result<()> {
		let mut app = observe_app();
		let subject = app
			.world_mut()
			.spawn((
				Transform::from_xyz(4.0, 0.9, 0.0),
				SpotSubject::new(InterestLayers::CHARACTER, SpotBounds::capsule(0.4, 0.9)),
			))
			.id();
		let mut user = character_user();
		user.hint(subject, SpottingHint::new(2));
		let observer = app.world_mut().spawn((Transform::default(), user)).id();

		app.update();

		let user = app
			.world()
			.get::<SpottingUser>(observer)
			.ok_or_else(|| anyhow::anyhow!("observer lost SpottingUser"))?;
		assert!(user.contacts.contains_key(&subject));
		Ok(())
	}

	#[test]
	fn far_skips_animated_broadphase_unless_fair() -> anyhow::Result<()> {
		let mut app = observe_app();
		app.insert_resource(SpottingObserveLimits { max_observers_per_tick: 1 });
		let subject = spawn_animated_subject(&mut app, Vec3::X * 4.0);
		let far = app
			.world_mut()
			.spawn((
				Transform::default(),
				character_user(),
				IntelligenceLod { band: IntelligenceBand::Far, skips: 0 },
			))
			.id();
		let near = app
			.world_mut()
			.spawn((
				Transform::from_xyz(0.0, 0.0, 1.0),
				character_user(),
				IntelligenceLod::missing(),
			))
			.id();
		app.world_mut().resource_mut::<IntelligencePriority>().rank.insert(near, 0);
		app.world_mut().resource_mut::<IntelligencePriority>().rank.insert(far, 1);

		app.update();

		anyhow::ensure!(app
			.world()
			.get::<SpottingUser>(near)
			.is_some_and(|user| user.contacts.contains_key(&subject)));
		anyhow::ensure!(app
			.world()
			.get::<SpottingUser>(far)
			.is_some_and(|user| !user.contacts.contains_key(&subject)));
		Ok(())
	}

	#[test]
	fn far_broadphase_still_runs_at_fairness_cap() -> anyhow::Result<()> {
		let mut app = observe_app();
		app.insert_resource(SpottingObserveLimits { max_observers_per_tick: 1 });
		let subject = spawn_animated_subject(&mut app, Vec3::X * 4.0);
		let far = app
			.world_mut()
			.spawn((
				Transform::default(),
				character_user(),
				IntelligenceLod {
					band: IntelligenceBand::Far,
					skips: IntelligenceLod::FAIRNESS_CAP,
				},
			))
			.id();

		app.update();

		anyhow::ensure!(app
			.world()
			.get::<SpottingUser>(far)
			.is_some_and(|user| user.contacts.contains_key(&subject)));
		anyhow::ensure!(app.world().get::<IntelligenceLod>(far).is_some_and(|lod| lod.skips == 0));
		Ok(())
	}

	#[test]
	fn forget_stale_still_runs_on_far_we_do_not_discover() -> anyhow::Result<()> {
		let mut app = observe_app();
		app.insert_resource(SpottingObserveLimits { max_observers_per_tick: 1 });
		let stale = spawn_subject(&mut app, Vec3::X * 3.0);
		let keep = spawn_subject(&mut app, Vec3::X * 5.0);
		let mut far_user = character_user();
		far_user.contacts.insert(
			stale,
			SpottedContact::new(stale, Vec3::ZERO, Vec3::ZERO, Vec3::ZERO, None, -4.0, 0.1),
		);
		let mut near_user = character_user();
		near_user.contacts.insert(
			keep,
			SpottedContact::new(keep, Vec3::ZERO, Vec3::ZERO, Vec3::ZERO, None, 0.0, 0.1),
		);
		let far = app
			.world_mut()
			.spawn((
				Transform::default(),
				far_user,
				IntelligenceLod { band: IntelligenceBand::Far, skips: 0 },
			))
			.id();
		let near = app
			.world_mut()
			.spawn((Transform::from_xyz(0.0, 0.0, 1.0), near_user, IntelligenceLod::missing()))
			.id();
		app.world_mut().resource_mut::<IntelligencePriority>().rank.insert(near, 0);
		app.world_mut().resource_mut::<IntelligencePriority>().rank.insert(far, 1);

		app.update();

		anyhow::ensure!(app
			.world()
			.get::<SpottingUser>(far)
			.is_some_and(|user| !user.contacts.contains_key(&stale)));
		anyhow::ensure!(app
			.world()
			.get::<SpottingUser>(near)
			.is_some_and(|user| user.contacts.contains_key(&keep)));
		Ok(())
	}

	#[test]
	fn far_discovery_skip_stretches_the_interval() -> anyhow::Result<()> {
		let mut app = observe_app();
		let far = app
			.world_mut()
			.spawn((
				Transform::default(),
				character_user(),
				IntelligenceLod { band: IntelligenceBand::Far, skips: 3 },
			))
			.id();

		app.update();

		let interval = SpotDirective::new(InterestLayers::CHARACTER, 20.0).discovery_interval_secs;
		let next = app
			.world()
			.get::<SpottingUser>(far)
			.ok_or_else(|| anyhow::anyhow!("far lost SpottingUser"))?
			.next_discovery_at;
		anyhow::ensure!(next >= interval * IntelligenceBand::Far.interval_scale() - 1e-3);
		anyhow::ensure!(app.world().get::<IntelligenceLod>(far).is_some_and(|lod| lod.skips == 3));
		Ok(())
	}
}
