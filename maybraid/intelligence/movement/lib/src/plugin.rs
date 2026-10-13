//! Generic plugin over a [`MovementIntelligenceSurface`] [`SystemParam`].

use std::marker::PhantomData;

use bevy::ecs::system::{StaticSystemParam, SystemParam};
use bevy::prelude::*;
use player::MoveWish;

use intelligence_lod::{due_by_rank, IntelligenceBand, IntelligenceLod, IntelligencePriority};

use crate::ability::MovementSheet;
use crate::configure_movement_intelligence_sets;
use crate::location::MovementLocation;
use crate::step::MovementDrive;
use crate::surface::{MovementIntelligenceLimits, MovementIntelligenceSurface, WalkProbeBudget};
use crate::user::{MovementDriveResult, MovementIntelligence, ReplanMovement};
use crate::MovementIntelligenceSystems;

/// Registers replan + drive for surface `S` and interaction / ability types `I`, `A`.
pub struct MovementIntelligencePlugin<S, I = crate::MovementStep, A = crate::MovementAbility>
where
	S: SystemParam + 'static,
{
	_marker: PhantomData<fn() -> (S, I, A)>,
}

impl<S, I, A> Default for MovementIntelligencePlugin<S, I, A>
where
	S: SystemParam + 'static,
{
	fn default() -> Self {
		Self { _marker: PhantomData }
	}
}

impl<S, I, A> Plugin for MovementIntelligencePlugin<S, I, A>
where
	S: SystemParam + 'static,
	for<'w, 's> S::Item<'w, 's>: MovementIntelligenceSurface<I, A>,
	I: MovementDrive + Clone + Send + Sync + 'static,
	A: MovementSheet + Send + Sync + 'static,
{
	fn build(&self, app: &mut App) {
		configure_movement_intelligence_sets(app);
		app.init_resource::<MovementIntelligenceLimits>()
			.init_resource::<IntelligencePriority>()
			.add_systems(
				Update,
				replan_movement::<S, I, A>.in_set(MovementIntelligenceSystems::Replan),
			)
			.add_systems(Update, drive_movement::<I, A>.in_set(MovementIntelligenceSystems::Drive));
	}
}

pub fn replan_movement<S, I, A>(
	surface: StaticSystemParam<S>,
	limits: Res<MovementIntelligenceLimits>,
	priority: Res<IntelligencePriority>,
	mut movers: Query<
		(Entity, &Transform, &mut MovementIntelligence<I, A>, Option<&mut IntelligenceLod>),
		With<ReplanMovement>,
	>,
	mut commands: Commands,
) where
	S: SystemParam + 'static,
	for<'w, 's> S::Item<'w, 's>: MovementIntelligenceSurface<I, A>,
	I: Send + Sync + 'static,
	A: MovementSheet + Send + Sync + 'static,
{
	let mut surface = surface.into_inner();
	let mut remaining = limits.max_replans_per_frame;
	let mut probes = WalkProbeBudget::new(limits.max_walk_probes_per_frame);
	let mut due: Vec<Entity> = movers.iter_mut().map(|(entity, ..)| entity).collect();
	due_by_rank(&mut due, &priority);
	let fair = due.iter().copied().find(|entity| {
		movers.get_mut(*entity).is_ok_and(|(_, _, _, lod)| {
			lod.as_deref().is_some_and(|lod| {
				lod.band != IntelligenceBand::Near && lod.skips >= IntelligenceLod::FAIRNESS_CAP
			})
		})
	});

	for entity in due {
		if remaining == 0 || probes.is_exhausted() {
			break;
		}
		let Ok((_, transform, mut brain, mut lod)) = movers.get_mut(entity) else {
			continue;
		};
		let band = IntelligenceLod::band_or_near(lod.as_deref());
		if band == IntelligenceBand::Far && Some(entity) != fair {
			continue;
		}
		remaining -= 1;
		let from = MovementLocation::new(transform.translation, brain.ability.agent_radius());
		let exclude = [entity];
		let objective = brain.objective;
		let budget = brain
			.ability
			.candidate_budget()
			.clamp_to(limits.max_budget)
			.lod_for(transform.translation, objective)
			.clamp_viewer(band);
		let candidates = surface.recommend_candidates_budgeted(
			from,
			&exclude,
			&brain.ability,
			objective,
			budget,
			&mut probes,
		);
		if probes.is_starved() && candidates.is_empty() {
			continue;
		}
		if let Some(candidate) = brain.pick_best_candidate(candidates) {
			brain.adopt_plan(candidate.steps);
		} else {
			brain.note_replan_failed();
		}
		if let Some(lod) = lod.as_deref_mut() {
			if lod.skips != 0 {
				lod.skips = 0;
			}
		}
		commands.entity(entity).remove::<ReplanMovement>();
	}
}

pub fn drive_movement<I, A>(
	time: Res<Time>,
	mut movers: Query<(Entity, &Transform, &mut MovementIntelligence<I, A>, &mut MoveWish)>,
	mut commands: Commands,
) where
	I: MovementDrive + Send + Sync + 'static,
	A: Send + Sync + 'static,
{
	let dt = time.delta_secs();
	for (entity, transform, mut brain, mut wish) in &mut movers {
		match brain.drive(dt, transform.translation) {
			MovementDriveResult::Wish(dir) => {
				// Motor unstick may override this wish later this frame.
				wish.set_if_neq(MoveWish(dir));
			}
			MovementDriveResult::Stuck { wish: dir } => {
				wish.set_if_neq(MoveWish(dir));
				if dir.length_squared() < 1e-6 {
					commands.entity(entity).insert(ReplanMovement);
				}
			}
			MovementDriveResult::Hold => {
				wish.set_if_neq(MoveWish(Vec3::ZERO));
			}
			MovementDriveResult::Retry => {
				wish.set_if_neq(MoveWish(Vec3::ZERO));
				commands.entity(entity).insert(ReplanMovement);
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use bevy::ecs::system::SystemParam;

	use intelligence_lod::{IntelligenceBand, IntelligenceLod, IntelligencePriority};

	use crate::objective::MovementObjective;
	use crate::surface::CandidateBudget;
	use crate::{MovementAbility, MovementStep};

	use super::*;

	#[derive(Resource, Default)]
	struct ReplanCalls(usize);

	#[derive(SystemParam)]
	struct CountingSurface<'w> {
		calls: ResMut<'w, ReplanCalls>,
	}

	impl<I, A> MovementIntelligenceSurface<I, A> for CountingSurface<'_> {
		fn recommend_candidates(
			&mut self,
			_from: MovementLocation,
			_exclude: &[Entity],
			_ability: &A,
			_objective: MovementObjective,
			_budget: CandidateBudget,
		) -> Vec<crate::MovementCandidate<I>> {
			self.calls.0 += 1;
			Vec::new()
		}

		fn recommend_candidates_budgeted(
			&mut self,
			from: MovementLocation,
			exclude: &[Entity],
			ability: &A,
			objective: MovementObjective,
			budget: CandidateBudget,
			probes: &mut WalkProbeBudget,
		) -> Vec<crate::MovementCandidate<I>> {
			for _ in 0..budget.max_candidates.max(1) {
				if !probes.take() {
					break;
				}
			}
			self.recommend_candidates(from, exclude, ability, objective, budget)
		}
	}

	fn spawn_pending(world: &mut World, count: usize) {
		let goal = MovementObjective::Reach(MovementLocation::new(Vec3::X * 4.0, 0.5));
		for index in 0..count {
			world.spawn((
				Transform::from_xyz(index as f32, 0.0, 0.0),
				MovementIntelligence::<MovementStep, MovementAbility>::new(goal),
				ReplanMovement,
			));
		}
	}

	fn pending_replans(world: &mut World) -> usize {
		world.query::<&ReplanMovement>().iter(world).count()
	}

	#[test]
	fn replan_drains_a_frame_budget_and_leaves_the_rest() -> anyhow::Result<()> {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.insert_resource(MovementIntelligenceLimits { max_replans_per_frame: 2, ..default() })
			.init_resource::<IntelligencePriority>()
			.init_resource::<ReplanCalls>()
			.add_systems(Update, replan_movement::<CountingSurface, MovementStep, MovementAbility>);
		spawn_pending(app.world_mut(), 5);

		app.update();
		anyhow::ensure!(app.world().resource::<ReplanCalls>().0 == 2);
		anyhow::ensure!(pending_replans(app.world_mut()) == 3);

		app.update();
		anyhow::ensure!(app.world().resource::<ReplanCalls>().0 == 4);
		anyhow::ensure!(pending_replans(app.world_mut()) == 1);
		Ok(())
	}

	#[test]
	fn walk_probe_cap_stops_starting_new_replans() -> anyhow::Result<()> {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.insert_resource(MovementIntelligenceLimits {
				max_replans_per_frame: 8,
				max_walk_probes_per_frame: 2,
				..default()
			})
			.init_resource::<IntelligencePriority>()
			.init_resource::<ReplanCalls>()
			.add_systems(Update, replan_movement::<CountingSurface, MovementStep, MovementAbility>);
		spawn_pending(app.world_mut(), 5);

		app.update();
		anyhow::ensure!(app.world().resource::<ReplanCalls>().0 == 2);
		anyhow::ensure!(pending_replans(app.world_mut()) == 3);
		Ok(())
	}

	#[test]
	fn starved_vantage_keeps_the_marker() -> anyhow::Result<()> {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.insert_resource(MovementIntelligenceLimits {
				max_replans_per_frame: 8,
				max_walk_probes_per_frame: 3,
				..default()
			})
			.init_resource::<IntelligencePriority>()
			.init_resource::<ReplanCalls>()
			.add_systems(Update, replan_movement::<CountingSurface, MovementStep, MovementAbility>);
		let goal = MovementObjective::VantageOn {
			location: MovementLocation::new(Vec3::X * 4.0, 1.0),
			hide_weight: 1.0,
			sightline_weight: 1.0,
		};
		app.world_mut().spawn((
			Transform::default(),
			MovementIntelligence::<MovementStep, MovementAbility>::new(goal),
			ReplanMovement,
		));

		app.update();
		anyhow::ensure!(app.world().resource::<ReplanCalls>().0 == 1);
		anyhow::ensure!(pending_replans(app.world_mut()) == 1);
		Ok(())
	}

	fn vantage_on(at: Vec3) -> MovementObjective {
		MovementObjective::VantageOn {
			location: MovementLocation::new(at, 1.0),
			hide_weight: 1.0,
			sightline_weight: 1.0,
		}
	}

	#[test]
	fn far_vantage_does_not_take_the_drain_before_near() -> anyhow::Result<()> {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.insert_resource(MovementIntelligenceLimits { max_replans_per_frame: 1, ..default() })
			.init_resource::<IntelligencePriority>()
			.init_resource::<ReplanCalls>()
			.add_systems(Update, replan_movement::<CountingSurface, MovementStep, MovementAbility>);
		let far = app
			.world_mut()
			.spawn((
				Transform::from_xyz(40.0, 0.0, 0.0),
				MovementIntelligence::<MovementStep, MovementAbility>::new(vantage_on(
					Vec3::X * 80.0,
				)),
				IntelligenceLod { band: IntelligenceBand::Far, skips: 0 },
				ReplanMovement,
			))
			.id();
		let near = app
			.world_mut()
			.spawn((
				Transform::default(),
				MovementIntelligence::<MovementStep, MovementAbility>::new(vantage_on(
					Vec3::X * 4.0,
				)),
				IntelligenceLod::missing(),
				ReplanMovement,
			))
			.id();
		app.world_mut().resource_mut::<IntelligencePriority>().rank.insert(near, 0);
		app.world_mut().resource_mut::<IntelligencePriority>().rank.insert(far, 1);

		app.update();
		anyhow::ensure!(app.world().resource::<ReplanCalls>().0 == 1);
		anyhow::ensure!(app.world().get::<ReplanMovement>(near).is_none());
		anyhow::ensure!(app.world().get::<ReplanMovement>(far).is_some());
		Ok(())
	}

	#[test]
	fn far_reach_still_completes_at_fairness_cap() -> anyhow::Result<()> {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.insert_resource(MovementIntelligenceLimits { max_replans_per_frame: 1, ..default() })
			.init_resource::<IntelligencePriority>()
			.init_resource::<ReplanCalls>()
			.add_systems(Update, replan_movement::<CountingSurface, MovementStep, MovementAbility>);
		let far = app
			.world_mut()
			.spawn((
				Transform::from_xyz(40.0, 0.0, 0.0),
				MovementIntelligence::<MovementStep, MovementAbility>::new(
					MovementObjective::Reach(MovementLocation::new(Vec3::X * 80.0, 0.5)),
				),
				IntelligenceLod {
					band: IntelligenceBand::Far,
					skips: IntelligenceLod::FAIRNESS_CAP,
				},
				ReplanMovement,
			))
			.id();

		app.update();
		anyhow::ensure!(app.world().resource::<ReplanCalls>().0 == 1);
		anyhow::ensure!(app.world().get::<ReplanMovement>(far).is_none());
		anyhow::ensure!(app.world().get::<IntelligenceLod>(far).is_some_and(|lod| lod.skips == 0));
		Ok(())
	}

	#[test]
	fn settled_hold_does_not_rewrite_wish() -> anyhow::Result<()> {
		use bevy::ecs::system::RunSystemOnce;

		let mut world = World::new();
		world.init_resource::<Time>();
		let entity = world
			.spawn((
				Transform::default(),
				MovementIntelligence::<MovementStep, MovementAbility>::new(
					MovementObjective::Reach(MovementLocation::new(Vec3::ZERO, 0.5)),
				),
				MoveWish(Vec3::ZERO),
			))
			.id();
		anyhow::ensure!(world
			.run_system_once(drive_movement::<MovementStep, MovementAbility>)
			.is_ok());
		let before = world
			.entity(entity)
			.get_change_ticks::<MoveWish>()
			.ok_or_else(|| anyhow::anyhow!("missing wish"))?
			.changed
			.get();
		anyhow::ensure!(world
			.run_system_once(drive_movement::<MovementStep, MovementAbility>)
			.is_ok());
		let after = world
			.entity(entity)
			.get_change_ticks::<MoveWish>()
			.ok_or_else(|| anyhow::anyhow!("missing wish"))?
			.changed
			.get();
		anyhow::ensure!(before == after);
		Ok(())
	}
}
