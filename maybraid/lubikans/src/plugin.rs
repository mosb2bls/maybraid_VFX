//! Gameplay plugin: register Self, attach ledgers, and honor owner lifetime.

use bevy::prelude::*;
use player::Player;

use crate::activation::{
	LubikanActivationDenied, LubikanActivationReport, LubikanActivationRequest,
	LubikanActivationSession,
};
use crate::catalog::{LubikanCatalog, LubikanSelfConfig};
use crate::change::{LubikanChargeChanged, LubikanChargeReason};
use crate::ledger::{LubikanAccountBinding, LubikanLedger, LubikanOwner};
use crate::lifetime::{LubikanOwnerLifetime, LubikanOwnerLifetimeKind};
use crate::{LubikanActivationRefused, LubikanInterfacePlugin, LubikanSystems};

/// Cached ledger that survives player-body replacement and session restart.
#[derive(Resource, Default, Debug)]
pub struct LubikanSessionStore {
	cached: Option<(Entity, LubikanLedger)>,
	pending_replace: bool,
}

impl LubikanSessionStore {
	pub fn cached_ledger(&self) -> Option<&LubikanLedger> {
		self.cached.as_ref().map(|(_, ledger)| ledger)
	}
}

/// Registers Self and keeps per-player charge independent of the HUD.
///
/// This plugin is the only place that creates or restores a player's ledger,
/// and the sole owner of [`LubikanActivationRequest`]s: it spends once, emits
/// [`LubikanActivationSession`] or [`LubikanActivationRefused`], and applies
/// [`LubikanActivationReport`]s to the captured account. Consumers order after
/// [`LubikanSystems::Owner`] (PreUpdate). Does not depend on Bikanwarda.
pub struct LubikansPlugin;

impl Plugin for LubikansPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<LubikanInterfacePlugin>() {
			app.add_plugins(LubikanInterfacePlugin);
		}
		app.init_resource::<LubikanSelfConfig>().init_resource::<LubikanSessionStore>();
		if app.world().get_resource::<LubikanCatalog>().is_none() {
			let config = *app.world().resource::<LubikanSelfConfig>();
			app.insert_resource(LubikanCatalog::with_self(&config));
		}
		app.configure_sets(PreUpdate, LubikanSystems::Owner)
			.configure_sets(Update, LubikanSystems::Activate)
			.add_systems(
				PreUpdate,
				(
					cache_live_ledgers,
					remember_removed_owners,
					attach_new_owners,
					apply_lifetime_notices,
					sync_account_bindings,
				)
					.chain()
					.in_set(LubikanSystems::Owner),
			)
			.add_systems(
				Update,
				(handle_activation_requests, apply_activation_reports)
					.chain()
					.in_set(LubikanSystems::Activate),
			);
		crate::award::configure(app);
	}
}

type LiveLubikanOwner = Or<(With<Player>, With<LubikanOwner>)>;

fn cache_live_ledgers(
	owners: Query<(Entity, &LubikanLedger), LiveLubikanOwner>,
	mut store: ResMut<LubikanSessionStore>,
) {
	for (entity, ledger) in &owners {
		store.cached = Some((entity, ledger.clone()));
	}
}

fn remember_removed_owners(
	mut players: RemovedComponents<Player>,
	mut owners: RemovedComponents<LubikanOwner>,
	mut store: ResMut<LubikanSessionStore>,
	mut commands: Commands,
) {
	for entity in players.read().chain(owners.read()) {
		if store.cached.as_ref().is_some_and(|(id, _)| *id == entity) {
			store.pending_replace = true;
			commands
				.entity(entity)
				.try_remove::<(LubikanLedger, LubikanOwner, LubikanAccountBinding)>();
		}
	}
}

fn attach_new_owners(
	added_players: Query<Entity, Added<Player>>,
	added_owners: Query<Entity, Added<LubikanOwner>>,
	existing: Query<(), With<LubikanLedger>>,
	mut store: ResMut<LubikanSessionStore>,
	config: Res<LubikanSelfConfig>,
	mut commands: Commands,
	mut lives: MessageWriter<LubikanOwnerLifetime>,
) {
	let mut added: Vec<Entity> = added_players.iter().collect();
	for entity in &added_owners {
		if !added.contains(&entity) {
			added.push(entity);
		}
	}
	for entity in added {
		if existing.contains(entity) {
			continue;
		}
		let previous = store.cached.as_ref().map(|(id, _)| *id).filter(|_| store.pending_replace);
		let replacing = store.pending_replace;
		let mut ledger = if replacing {
			store.pending_replace = false;
			store
				.cached
				.take()
				.map(|(_, ledger)| ledger)
				.unwrap_or_else(|| LubikanLedger::with_self(&config))
		} else {
			LubikanLedger::with_self(&config)
		};
		let kind = if replacing {
			ledger.apply_lifetime(config.lifetime.on_body_replace);
			LubikanOwnerLifetimeKind::ReplaceBody
		} else {
			LubikanOwnerLifetimeKind::Equip
		};
		let binding = LubikanAccountBinding::from_selection(ledger.selection(), entity);
		commands.entity(entity).insert((LubikanOwner, ledger, binding));
		lives.write(LubikanOwnerLifetime::new(previous, Some(entity), kind));
	}
}

fn apply_lifetime_notices(
	mut notices: MessageReader<LubikanOwnerLifetime>,
	mut store: ResMut<LubikanSessionStore>,
	mut ledgers: Query<&mut LubikanLedger>,
	config: Res<LubikanSelfConfig>,
) {
	for notice in notices.read() {
		if notice.kind != LubikanOwnerLifetimeKind::RestartSession {
			continue;
		}
		let persistence = config.lifetime.on_session_restart;
		if let Some((_, ledger)) = store.cached.as_mut() {
			ledger.apply_lifetime(persistence);
		}
		store.pending_replace = store.cached.is_some();
		for mut ledger in &mut ledgers {
			ledger.apply_lifetime(persistence);
		}
	}
}

fn sync_account_bindings(
	mut owners: Query<
		(Entity, &LubikanLedger, Option<&mut LubikanAccountBinding>),
		With<LubikanOwner>,
	>,
	mut commands: Commands,
) {
	for (entity, ledger, binding) in &mut owners {
		let wanted = LubikanAccountBinding::from_selection(ledger.selection(), entity);
		match binding {
			Some(mut binding) if *binding != wanted => *binding = wanted,
			None => {
				commands.entity(entity).insert(wanted);
			}
			Some(_) => {}
		}
	}
}

fn handle_activation_requests(
	mut requests: MessageReader<LubikanActivationRequest>,
	mut ledgers: Query<&mut LubikanLedger>,
	catalog: Res<LubikanCatalog>,
	mut accepted: MessageWriter<LubikanActivationSession>,
	mut refused: MessageWriter<LubikanActivationRefused>,
	mut changes: MessageWriter<LubikanChargeChanged>,
) {
	for request in requests.read() {
		let Ok(mut ledger) = ledgers.get_mut(request.owner) else {
			refused.write(LubikanActivationRefused {
				owner: request.owner,
				lubikan: request.lubikan,
				reason: LubikanActivationDenied::UnknownAccount,
			});
			continue;
		};
		match ledger.try_activate(&catalog, request.lubikan) {
			Ok(grant) => {
				accepted.write(grant.session(request.owner));
				changes.write(LubikanChargeChanged::new(
					request.owner,
					grant.account,
					grant.spent,
					LubikanChargeReason::ACTIVATION,
					None,
				));
			}
			Err(reason) => {
				refused.write(LubikanActivationRefused {
					owner: request.owner,
					lubikan: request.lubikan,
					reason,
				});
			}
		}
	}
}

fn apply_activation_reports(
	mut reports: MessageReader<LubikanActivationReport>,
	mut ledgers: Query<&mut LubikanLedger>,
) {
	for report in reports.read() {
		for mut ledger in &mut ledgers {
			if ledger.finish_activation(report.id, report.outcome) {
				break;
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::activation::LubikanActivationPhase;
	use crate::identity::{LubikanAccountId, LubikanDefinition, LubikanDefinitionId};
	use crate::lifetime::LubikanPersistence;
	use crate::{LubikanActivationReport, LubikanChargeChanged, LubikanChargeReason};
	use anyhow::{anyhow, Result};

	fn gameplay() -> App {
		let mut app = App::new();
		app.add_plugins(LubikansPlugin);
		app
	}

	fn spawn_player(app: &mut App) -> Entity {
		app.world_mut().spawn(Player).id()
	}

	fn ledger(app: &App, owner: Entity) -> Result<&LubikanLedger> {
		app.world()
			.get::<LubikanLedger>(owner)
			.ok_or_else(|| anyhow!("missing ledger on {owner}"))
	}

	#[test]
	fn normal_game_registers_self_without_hud() -> Result<()> {
		let mut app = gameplay();
		let owner = spawn_player(&mut app);
		app.update();
		let catalog = app.world().resource::<LubikanCatalog>();
		if !catalog.self_definition().is_self() {
			return Err(anyhow!("catalog must register Self"));
		}
		let ledger = ledger(&app, owner)?;
		assert!(ledger.selection().is_self());
		assert_eq!(ledger.equipped().id(), LubikanAccountId::SELF);
		assert_eq!(ledger.equipped().charge().current(), 0.0);
		assert!(app.world().get::<LubikanOwner>(owner).is_some());
		assert!(app
			.world()
			.get::<LubikanAccountBinding>(owner)
			.is_some_and(|binding| binding.is_self()));
		Ok(())
	}

	#[test]
	fn charge_mutations_clamp_and_block_activation() -> Result<()> {
		let mut app = gameplay();
		let owner = spawn_player(&mut app);
		app.update();
		let cost = app.world().resource::<LubikanCatalog>().self_definition().activation_cost;
		{
			let mut ledger = app
				.world_mut()
				.get_mut::<LubikanLedger>(owner)
				.ok_or_else(|| anyhow!("ledger"))?;
			let high = ledger.apply_to_equipped(10_000.0);
			if !high.clamped_capacity {
				return Err(anyhow!("capacity must clamp"));
			}
			let low = ledger.apply_to_equipped(-10_000.0);
			if !low.clamped_floor {
				return Err(anyhow!("floor must clamp"));
			}
		}
		app.world_mut().write_message(LubikanActivationRequest::equipped(owner));
		app.update();
		if ledger(&app, owner)?.equipped().activation().phase != LubikanActivationPhase::Idle {
			return Err(anyhow!("empty charge cannot activate"));
		}
		{
			let mut ledger = app
				.world_mut()
				.get_mut::<LubikanLedger>(owner)
				.ok_or_else(|| anyhow!("ledger"))?;
			ledger.apply_to_equipped(cost);
		}
		app.world_mut().write_message(LubikanActivationRequest::equipped(owner));
		app.update();
		assert_eq!(
			ledger(&app, owner)?.equipped().activation().phase,
			LubikanActivationPhase::Active
		);
		assert_eq!(ledger(&app, owner)?.equipped().charge().current(), 0.0);
		Ok(())
	}

	#[test]
	fn selection_keeps_accounts_distinct() -> Result<()> {
		let mut app = gameplay();
		let owner = spawn_player(&mut app);
		app.update();
		let extra = LubikanDefinition::new(LubikanDefinitionId::from_raw(3), 40.0, 5.0);
		app.world_mut().resource_mut::<LubikanCatalog>().register(extra);
		let other = {
			let mut ledger = app
				.world_mut()
				.get_mut::<LubikanLedger>(owner)
				.ok_or_else(|| anyhow!("ledger"))?;
			ledger.apply_to(LubikanAccountId::SELF, 11.0);
			let other = ledger.ensure_owned(extra);
			ledger.apply_to(other, 7.0);
			if !ledger.select(Some(other)) {
				return Err(anyhow!("select other"));
			}
			other
		};
		app.update();
		let ledger = ledger(&app, owner)?;
		assert_eq!(ledger.equipped().id(), other);
		assert_eq!(
			ledger.account(LubikanAccountId::SELF).map(|account| account.charge().current()),
			Some(11.0)
		);
		assert_eq!(ledger.account(other).map(|account| account.charge().current()), Some(7.0));
		assert!(app
			.world()
			.get::<LubikanAccountBinding>(owner)
			.is_some_and(|binding| binding.account == other));
		Ok(())
	}

	#[test]
	fn body_replace_retains_charge_by_default() -> Result<()> {
		let mut app = gameplay();
		let first = spawn_player(&mut app);
		app.update();
		{
			let mut ledger = app
				.world_mut()
				.get_mut::<LubikanLedger>(first)
				.ok_or_else(|| anyhow!("ledger"))?;
			ledger.apply_to_equipped(18.0);
		}
		app.update();
		app.world_mut().entity_mut(first).remove::<Player>();
		app.update();
		let second = spawn_player(&mut app);
		app.update();
		assert_eq!(ledger(&app, second)?.equipped().charge().current(), 18.0);
		assert!(app.world().get::<LubikanLedger>(first).is_none());
		Ok(())
	}

	#[test]
	fn session_restart_resets_charge_and_keeps_self() -> Result<()> {
		let mut app = gameplay();
		let owner = spawn_player(&mut app);
		app.update();
		let extra = LubikanDefinition::new(LubikanDefinitionId::from_raw(8), 40.0, 2.0);
		let extra_id = {
			let mut ledger = app
				.world_mut()
				.get_mut::<LubikanLedger>(owner)
				.ok_or_else(|| anyhow!("ledger"))?;
			let extra_id = ledger.ensure_owned(extra);
			ledger.apply_to(LubikanAccountId::SELF, 11.0);
			ledger.apply_to(extra_id, 20.0);
			if !ledger.select(Some(extra_id)) {
				return Err(anyhow!("select extra before restart"));
			}
			extra_id
		};
		app.update();
		assert_eq!(ledger(&app, owner)?.equipped().id(), extra_id);
		assert_eq!(ledger(&app, owner)?.equipped().charge().current(), 20.0);
		app.world_mut().write_message(LubikanOwnerLifetime::new(
			None,
			None,
			LubikanOwnerLifetimeKind::RestartSession,
		));
		app.update();
		let ledger = ledger(&app, owner)?;
		assert!(ledger.contains(LubikanAccountId::SELF));
		assert!(ledger.contains(extra_id));
		assert_eq!(ledger.equipped().charge().current(), 0.0);
		assert_eq!(
			ledger.account(LubikanAccountId::SELF).map(|account| account.charge().current()),
			Some(0.0)
		);
		assert_eq!(
			app.world().resource::<LubikanSelfConfig>().lifetime.on_session_restart,
			LubikanPersistence::ResetCharge
		);
		Ok(())
	}

	#[test]
	fn marked_owner_without_player_still_gets_self() -> Result<()> {
		let mut app = gameplay();
		let owner = app.world_mut().spawn(LubikanOwner).id();
		app.update();
		assert_eq!(ledger(&app, owner)?.equipped().id(), LubikanAccountId::SELF);
		Ok(())
	}

	#[test]
	fn replacing_a_marked_owner_restores_charge() -> Result<()> {
		let mut app = gameplay();
		let first = app.world_mut().spawn(LubikanOwner).id();
		app.update();
		{
			let mut ledger = app
				.world_mut()
				.get_mut::<LubikanLedger>(first)
				.ok_or_else(|| anyhow!("ledger"))?;
			ledger.apply_to_equipped(14.0);
		}
		app.update();
		app.world_mut().entity_mut(first).remove::<LubikanOwner>();
		app.update();
		let second = app.world_mut().spawn(LubikanOwner).id();
		app.update();
		assert_eq!(ledger(&app, second)?.equipped().charge().current(), 14.0);
		Ok(())
	}

	#[test]
	fn pause_preserves_charge_activation_and_selection() -> Result<()> {
		let mut app = gameplay();
		let owner = spawn_player(&mut app);
		app.update();
		let extra = LubikanDefinition::new(LubikanDefinitionId::from_raw(4), 40.0, 5.0);
		app.world_mut().resource_mut::<LubikanCatalog>().register(extra);
		let cost = app.world().resource::<LubikanCatalog>().self_definition().activation_cost;
		let other = {
			let mut ledger = app
				.world_mut()
				.get_mut::<LubikanLedger>(owner)
				.ok_or_else(|| anyhow!("ledger"))?;
			ledger.apply_to(LubikanAccountId::SELF, cost);
			let other = ledger.ensure_owned(extra);
			ledger.apply_to(other, 8.0);
			other
		};
		app.world_mut().write_message(LubikanActivationRequest::equipped(owner));
		app.update();
		{
			let mut ledger = app
				.world_mut()
				.get_mut::<LubikanLedger>(owner)
				.ok_or_else(|| anyhow!("ledger"))?;
			if !ledger.select(Some(other)) {
				return Err(anyhow!("switch after activate"));
			}
		}
		app.update();
		app.update();
		let snapshot = ledger(&app, owner)?;
		assert_eq!(
			snapshot
				.account(LubikanAccountId::SELF)
				.map(|account| account.activation().phase),
			Some(LubikanActivationPhase::Active)
		);
		assert_eq!(
			snapshot
				.account(LubikanAccountId::SELF)
				.map(|account| account.charge().current()),
			Some(0.0)
		);
		assert_eq!(snapshot.equipped().id(), other);
		assert_eq!(snapshot.account(other).map(|account| account.charge().current()), Some(8.0));
		assert_eq!(
			snapshot.account(other).map(|account| account.activation().phase),
			Some(LubikanActivationPhase::Idle)
		);
		Ok(())
	}

	#[test]
	fn activation_handoff_spends_once_and_completes_the_captured_account() -> Result<()> {
		let mut app = gameplay();
		let owner = spawn_player(&mut app);
		app.update();
		let extra = LubikanDefinition::new(LubikanDefinitionId::from_raw(5), 40.0, 5.0);
		app.world_mut().resource_mut::<LubikanCatalog>().register(extra);
		let cost = app.world().resource::<LubikanCatalog>().self_definition().activation_cost;
		let other = {
			let mut ledger = app
				.world_mut()
				.get_mut::<LubikanLedger>(owner)
				.ok_or_else(|| anyhow!("ledger"))?;
			ledger.apply_to(LubikanAccountId::SELF, cost);
			let other = ledger.ensure_owned(extra);
			ledger.apply_to(other, 6.0);
			other
		};
		app.world_mut().write_message(LubikanActivationRequest::equipped(owner));
		app.update();
		let session = app
			.world_mut()
			.resource_mut::<Messages<LubikanActivationSession>>()
			.drain()
			.next()
			.ok_or_else(|| anyhow!("accepted session"))?;
		assert_eq!(session.owner, owner);
		assert_eq!(session.account, LubikanAccountId::SELF);
		let spend = app
			.world_mut()
			.resource_mut::<Messages<LubikanChargeChanged>>()
			.drain()
			.find(|change| change.reason == LubikanChargeReason::ACTIVATION)
			.ok_or_else(|| anyhow!("activation spend"))?;
		assert_eq!(spend.lubikan, LubikanAccountId::SELF);
		assert_eq!(spend.amount, -cost);
		{
			let mut ledger = app
				.world_mut()
				.get_mut::<LubikanLedger>(owner)
				.ok_or_else(|| anyhow!("ledger"))?;
			if !ledger.select(Some(other)) {
				return Err(anyhow!("switch after activate"));
			}
		}
		app.world_mut().write_message(LubikanActivationReport::completed(session.id));
		app.update();
		let ledger = ledger(&app, owner)?;
		assert_eq!(
			ledger.account(LubikanAccountId::SELF).map(|account| account.activation().phase),
			Some(LubikanActivationPhase::Completed)
		);
		assert_eq!(
			ledger.account(other).map(|account| account.activation().phase),
			Some(LubikanActivationPhase::Idle)
		);
		assert_eq!(ledger.account(other).map(|account| account.charge().current()), Some(6.0));
		Ok(())
	}

	#[test]
	fn denial_emits_refused_and_does_not_spend() -> Result<()> {
		let mut app = gameplay();
		let owner = spawn_player(&mut app);
		app.update();
		app.world_mut().write_message(LubikanActivationRequest::equipped(owner));
		app.update();
		let refused = app
			.world_mut()
			.resource_mut::<Messages<LubikanActivationRefused>>()
			.drain()
			.next()
			.ok_or_else(|| anyhow!("refused"))?;
		assert_eq!(refused.reason, LubikanActivationDenied::InsufficientCharge);
		assert_eq!(ledger(&app, owner)?.equipped().charge().current(), 0.0);
		assert_eq!(
			ledger(&app, owner)?.equipped().activation().phase,
			LubikanActivationPhase::Idle
		);
		Ok(())
	}
}
