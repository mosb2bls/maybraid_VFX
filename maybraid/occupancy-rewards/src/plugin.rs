//! Bevy plugin: resolve occupancy from transforms and emit awards / requests.

use bevy::prelude::*;
use lubikans::{
	LubikanChargeRequest, LubikanInterfacePlugin, LubikanLedger, LubikanOwner, LubikanSystems,
};

use crate::charge::adapt_award;
use crate::evaluate::step_occupancy;
use crate::identity::OccupancyIdentity;
use crate::metadata::CellMetadataStore;
use crate::occupancy::{OccupancyAward, OccupancyEvent, OccupancySession};
use crate::rule::{OccupancyDemoConfig, RewardRuleSet};

/// Occupancy evaluation. Runs in PostUpdate after [`LubikanSystems::Owner`]
/// (PreUpdate) and before [`LubikanSystems::Award`].
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum OccupancyRewardsSystems {
	Evaluate,
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

	/// Local pause plus a closed session. Retiring bodies and ended play
	/// sessions use this so leftover timers cannot credit after teardown.
	pub fn close(&mut self) {
		self.enabled = false;
		self.session.close();
	}
}

/// Occupancy tracking and the Self demonstration rule. Cadence-neutral: the
/// application may attach [`OccupancyRewardsSystems::Evaluate`] to a timer.
pub struct OccupancyRewardsPlugin {
	pub rules: RewardRuleSet,
}

impl Default for OccupancyRewardsPlugin {
	fn default() -> Self {
		Self { rules: OccupancyDemoConfig::from_env().rule_set() }
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
			.configure_sets(
				PostUpdate,
				OccupancyRewardsSystems::Evaluate.before(LubikanSystems::Award),
			)
			.add_systems(
				PostUpdate,
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
	mut users: Query<
		(Entity, &GlobalTransform, &mut OccupancyRewardsUser, &LubikanLedger),
		With<LubikanOwner>,
	>,
	mut events: MessageWriter<OccupancyEvent>,
	mut awards: MessageWriter<OccupancyAward>,
	mut charges: MessageWriter<LubikanChargeRequest>,
) {
	let dt = time.delta();
	for (entity, transform, mut user, ledger) in &mut users {
		if user.session.is_closed() {
			continue;
		}
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
		let selection = ledger.selection();
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
	use lubikans::{
		LubikanAccountId, LubikanChargeReason, LubikanChargeRequest, LubikanOwner, LubikansPlugin,
	};

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

	fn drain_requests(world: &mut World) -> Vec<LubikanChargeRequest> {
		world.resource_mut::<Messages<LubikanChargeRequest>>().drain().collect()
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
		assert_eq!(request.lubikan, Some(LubikanAccountId::SELF));
		assert_eq!(request.requested, 1.0);
		assert_eq!(request.reason, LubikanChargeReason::OCCUPANCY);
		assert!(request.origin.is_some());
		let changed: Vec<_> = app
			.world_mut()
			.resource_mut::<Messages<lubikans::LubikanChargeChanged>>()
			.drain()
			.collect();
		assert!(changed.is_empty(), "Award owns LubikanChargeChanged");
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
		for at in [Vec3::ZERO, Vec3::new(1.0, 0.0, 1.0)] {
			let fixture = LubikanFixture::spawn_in(&mut app);
			app.world_mut()
				.entity_mut(fixture.owner)
				.insert((GlobalTransform::from_translation(at), OccupancyRewardsUser::default()));
		}
		app.world_mut()
			.run_system_once(evaluate_occupancy_rewards)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert_eq!(drain_requests(app.world_mut()).len(), 2);
		Ok(())
	}

	#[test]
	fn award_applies_occupancy_request_with_points_earned() -> anyhow::Result<()> {
		let mut store = CellMetadataStore::default();
		publish_demo(&mut store, Vec3::ZERO);
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.add_plugins(LubikansPlugin)
			.add_plugins(OccupancyRewardsPlugin {
				rules: RewardRuleSet::new([RewardRule::self_demonstration()]),
			})
			.insert_resource(store);
		let owner = app
			.world_mut()
			.spawn((
				LubikanOwner,
				GlobalTransform::from_translation(Vec3::ZERO),
				OccupancyRewardsUser::default(),
			))
			.id();
		app.update();
		let _ = app
			.world_mut()
			.resource_mut::<Messages<lubikans::LubikanChargeChanged>>()
			.drain()
			.count();
		let mut time = Time::<()>::default();
		time.advance_by(Duration::from_secs(5));
		app.insert_resource(time);
		app.world_mut()
			.run_system_once(evaluate_occupancy_rewards)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		app.world_mut().resource_mut::<OccupancyRewardsPaused>().0 = true;
		app.update();
		app.world_mut().resource_mut::<OccupancyRewardsPaused>().0 = false;
		let changed: Vec<_> = app
			.world_mut()
			.resource_mut::<Messages<lubikans::LubikanChargeChanged>>()
			.drain()
			.filter(|change| change.reason == LubikanChargeReason::OCCUPANCY)
			.collect();
		anyhow::ensure!(changed.len() == 1, "Award should apply the occupancy request once");
		assert_eq!(changed[0].recipient, owner);
		assert_eq!(changed[0].lubikan, LubikanAccountId::SELF);
		assert_eq!(changed[0].amount, 1.0);
		assert_eq!(changed[0].points_earned, 1.0);
		Ok(())
	}

	#[test]
	fn closed_user_does_not_award() -> anyhow::Result<()> {
		let mut store = CellMetadataStore::default();
		publish_demo(&mut store, Vec3::ZERO);
		let mut app = app_with(store, RewardRuleSet::new([RewardRule::self_demonstration()]));
		let fixture = LubikanFixture::spawn_in(&mut app);
		app.world_mut().entity_mut(fixture.owner).insert((
			GlobalTransform::from_translation(Vec3::ZERO),
			OccupancyRewardsUser::default(),
		));
		app.world_mut()
			.get_mut::<OccupancyRewardsUser>(fixture.owner)
			.ok_or_else(|| anyhow::anyhow!("user"))?
			.close();
		app.world_mut()
			.run_system_once(evaluate_occupancy_rewards)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert!(drain_requests(app.world_mut()).is_empty());
		Ok(())
	}

	#[test]
	fn missing_ledger_is_skipped_that_frame() -> anyhow::Result<()> {
		let mut store = CellMetadataStore::default();
		publish_demo(&mut store, Vec3::ZERO);
		let mut app = app_with(
			store,
			RewardRuleSet::new([RewardRule::self_always(Duration::from_secs(1), 1.0)]),
		);
		app.world_mut().spawn((
			GlobalTransform::from_translation(Vec3::ZERO),
			OccupancyRewardsUser::default(),
		));
		app.world_mut()
			.run_system_once(evaluate_occupancy_rewards)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert!(drain_requests(app.world_mut()).is_empty());
		Ok(())
	}
}
