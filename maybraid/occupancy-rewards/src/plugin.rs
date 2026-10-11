//! Bevy plugin: resolve occupancy from transforms and emit awards / requests.

use bevy::prelude::*;
use lubikans::{LubikanInterfacePlugin, LubikanLedger, LubikanSystems};

use crate::charge::{adapt_award, OccupancyChargeRequest};
use crate::evaluate::step_occupancy;
use crate::identity::OccupancyIdentity;
use crate::metadata::CellMetadataStore;
use crate::occupancy::{OccupancyAward, OccupancyEvent, OccupancySession};
use crate::rule::RewardRuleSet;

#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum OccupancyRewardsSystems {
	Evaluate,
	Award,
}

/// Global evaluation gate. Identity still updates so leaving while paused is
/// recorded; timers do not advance.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct OccupancyRewardsPaused(pub bool);

/// Per-actor occupancy session. `enabled` is a local pause that still tracks
/// enter/exit.
#[derive(Component, Clone, Debug)]
pub struct OccupancyRewardsUser {
	pub enabled: bool,
	pub session: OccupancySession,
}

impl Default for OccupancyRewardsUser {
	fn default() -> Self {
		Self { enabled: true, session: OccupancySession::default() }
	}
}

impl OccupancyRewardsUser {
	pub fn new() -> Self {
		Self::default()
	}
}

/// Occupancy tracking and the Self demonstration rule. Cadence-neutral: the
/// application may attach [`OccupancyRewardsSystems::Evaluate`] to a timer.
pub struct OccupancyRewardsPlugin {
	pub rules: RewardRuleSet,
}

impl Default for OccupancyRewardsPlugin {
	fn default() -> Self {
		Self { rules: RewardRuleSet::demonstration() }
	}
}

impl Plugin for OccupancyRewardsPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<LubikanInterfacePlugin>() {
			app.add_plugins(LubikanInterfacePlugin);
		}
		app.init_resource::<CellMetadataStore>()
			.init_resource::<OccupancyRewardsPaused>()
			.insert_resource(self.rules.clone())
			.add_message::<OccupancyEvent>()
			.add_message::<OccupancyAward>()
			.add_message::<OccupancyChargeRequest>()
			.configure_sets(
				Update,
				(
					OccupancyRewardsSystems::Evaluate.before(LubikanSystems::Award),
					OccupancyRewardsSystems::Award
						.after(OccupancyRewardsSystems::Evaluate)
						.before(LubikanSystems::Award),
				),
			)
			.add_systems(
				Update,
				evaluate_occupancy_rewards.in_set(OccupancyRewardsSystems::Evaluate),
			);
	}
}

#[allow(clippy::too_many_arguments)]
pub fn evaluate_occupancy_rewards(
	time: Res<Time>,
	paused: Res<OccupancyRewardsPaused>,
	store: Res<CellMetadataStore>,
	rules: Res<RewardRuleSet>,
	mut users: Query<(Entity, &GlobalTransform, &mut OccupancyRewardsUser, Option<&LubikanLedger>)>,
	mut events: MessageWriter<OccupancyEvent>,
	mut awards: MessageWriter<OccupancyAward>,
	mut charges: MessageWriter<OccupancyChargeRequest>,
) {
	let dt = time.delta();
	for (entity, transform, mut user, ledger) in &mut users {
		let identity = OccupancyIdentity::resolve(transform.translation(), store.as_ref());
		let locally_paused = paused.0 || !user.enabled;
		let stepped = step_occupancy(
			entity,
			&mut user.session,
			identity,
			store.as_ref(),
			rules.as_ref(),
			dt,
			locally_paused,
		);
		let selection = ledger.map(LubikanLedger::selection).unwrap_or_default();
		for event in stepped.events {
			events.write(event);
		}
		for award in stepped.awards {
			charges.write(adapt_award(&award, selection));
			awards.write(award);
		}
	}
}

#[cfg(test)]
mod tests {
	use std::time::Duration;

	use bevy::ecs::system::RunSystemOnce;
	use bevy::prelude::*;
	use lubikans::fixtures::LubikanFixture;
	use lubikans::{LubikanAccountId, LubikanChargeReason};

	use super::*;
	use crate::identity::{GroveLayerId, OccupancyIdentity, SemanticCellKey, VegetationLayer};
	use crate::metadata::{CellMetadata, GroveInfo};
	use crate::rule::RewardRule;
	use crate::tag::{ConstructionTag, GroveKindTag};

	fn app_with(store: CellMetadataStore, rules: RewardRuleSet) -> App {
		let mut app = App::new();
		app.add_plugins(OccupancyRewardsPlugin { rules }).insert_resource(store);
		let mut time = Time::<()>::default();
		time.advance_by(Duration::from_secs(5));
		app.insert_resource(time);
		app
	}

	fn publish_demo(store: &mut CellMetadataStore, position: Vec3) {
		let identity = OccupancyIdentity::from_position(position);
		store.publish(
			CellMetadata::new(SemanticCellKey::GroveLayer(GroveLayerId {
				tile: identity.grove,
				layer: VegetationLayer::UpperCanopy,
			}))
			.with_grove(GroveInfo {
				layer: VegetationLayer::UpperCanopy,
				kind: Some(GroveKindTag::named("TradeWinds")),
			})
			.with_construction(ConstructionTag::SOPES_BANYAN),
		);
	}

	fn drain_requests(world: &mut World) -> Vec<OccupancyChargeRequest> {
		world.resource_mut::<Messages<OccupancyChargeRequest>>().drain().collect()
	}

	#[test]
	fn plugin_emits_self_mutation_request_on_interval() -> anyhow::Result<()> {
		let mut store = CellMetadataStore::default();
		publish_demo(&mut store, Vec3::ZERO);
		let mut app = app_with(store, RewardRuleSet::new([RewardRule::self_demonstration()]));
		let fixture = LubikanFixture::spawn_in(&mut app);
		app.world_mut().entity_mut(fixture.owner).insert((
			GlobalTransform::from_translation(Vec3::ZERO),
			OccupancyRewardsUser::default(),
		));
		app.world_mut()
			.run_system_once(evaluate_occupancy_rewards)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		let requests = drain_requests(app.world_mut());
		let request = requests
			.first()
			.copied()
			.ok_or_else(|| anyhow::anyhow!("expected a charge-mutation request"))?;
		assert_eq!(request.lubikan, LubikanAccountId::SELF);
		assert_eq!(request.requested, 1.0);
		assert_eq!(request.reason, LubikanChargeReason::OCCUPANCY);
		assert!(request.origin.is_some());
		let mut ledger = fixture
			.ledger_mut(app.world_mut())
			.ok_or_else(|| anyhow::anyhow!("expected Self ledger"))?;
		let mutation = request
			.apply_to_ledger(&mut ledger)
			.ok_or_else(|| anyhow::anyhow!("Self account must accept the apply"))?;
		assert_eq!(mutation.requested, 1.0);
		assert_eq!(mutation.applied, 1.0);
		let changed: Vec<_> = app
			.world_mut()
			.resource_mut::<Messages<lubikans::LubikanChargeChanged>>()
			.drain()
			.collect();
		assert!(changed.is_empty());
		Ok(())
	}

	#[test]
	fn disabled_user_does_not_award() -> anyhow::Result<()> {
		let mut store = CellMetadataStore::default();
		publish_demo(&mut store, Vec3::ZERO);
		let mut app = app_with(store, RewardRuleSet::new([RewardRule::self_demonstration()]));
		app.world_mut().spawn((
			GlobalTransform::from_translation(Vec3::ZERO),
			OccupancyRewardsUser { enabled: false, session: OccupancySession::default() },
		));
		app.world_mut()
			.run_system_once(evaluate_occupancy_rewards)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert!(drain_requests(app.world_mut()).is_empty());
		Ok(())
	}

	#[test]
	fn two_actors_keep_independent_timers() -> anyhow::Result<()> {
		let mut store = CellMetadataStore::default();
		publish_demo(&mut store, Vec3::ZERO);
		let mut time = Time::<()>::default();
		time.advance_by(Duration::from_secs(1));
		let mut app = App::new();
		app.add_plugins(OccupancyRewardsPlugin {
			rules: RewardRuleSet::new([RewardRule::self_always(Duration::from_secs(1), 1.0)]),
		})
		.insert_resource(store)
		.insert_resource(time);
		app.world_mut().spawn((
			GlobalTransform::from_translation(Vec3::ZERO),
			OccupancyRewardsUser::default(),
		));
		app.world_mut().spawn((
			GlobalTransform::from_translation(Vec3::new(1.0, 0.0, 1.0)),
			OccupancyRewardsUser::default(),
		));
		app.world_mut()
			.run_system_once(evaluate_occupancy_rewards)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert_eq!(drain_requests(app.world_mut()).len(), 2);
		Ok(())
	}
}
