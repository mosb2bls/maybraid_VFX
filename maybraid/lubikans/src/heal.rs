//! Self activation consumer: regenerate HP from an accepted session.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use damage::{
	DamageActorId, DamageApplied, DamageHistory, DamageHistoryPolicy, DamageSystems, Died, Downed,
	Health,
};

use crate::activation::{
	LubikanActivationFinished, LubikanActivationOutcome, LubikanActivationPhase,
	LubikanActivationReport, LubikanActivationSession, LubikanActivationSessionId,
};
use crate::identity::{LubikanAccountId, LubikanDefinitionId};
use crate::ledger::{LubikanLedger, LubikanOwner};
use crate::lifetime::{LubikanOwnerLifetime, LubikanOwnerLifetimeKind};
use crate::plugin::LubikanSessionStore;
use crate::LubikanSystems;

/// Default Self heal rate: 5% of maximum health per game-time second.
pub const DEFAULT_SELF_HEAL_RATE_FRAC: f32 = 0.05;

/// Default maximum Self heal length. At the default rate this is a full
/// 0 → 100% restore.
pub const DEFAULT_SELF_HEAL_DURATION_SECS: f32 = 20.0;

/// Healing ceiling while the session has not latched applied damage.
pub const DEFAULT_SELF_HEAL_CAP_HEALTHY: f32 = 1.0;

/// Healing ceiling after the session latches positive applied HP.
pub const DEFAULT_SELF_HEAL_CAP_DAMAGED: f32 = 0.7;

/// Configurable Self heal rate, duration, and ceilings.
///
/// Damage during the session lowers the remainder's ceiling. It does not
/// cancel the event, and the cap never reduces current health.
#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub struct LubikanSelfHealConfig {
	pub rate_frac_per_sec: f32,
	pub duration_secs: f32,
	pub cap_healthy: f32,
	pub cap_damaged: f32,
}

impl Default for LubikanSelfHealConfig {
	fn default() -> Self {
		Self {
			rate_frac_per_sec: DEFAULT_SELF_HEAL_RATE_FRAC,
			duration_secs: DEFAULT_SELF_HEAL_DURATION_SECS,
			cap_healthy: DEFAULT_SELF_HEAL_CAP_HEALTHY,
			cap_damaged: DEFAULT_SELF_HEAL_CAP_DAMAGED,
		}
	}
}

impl LubikanSelfHealConfig {
	pub fn rate_frac_per_sec(self) -> f32 {
		if self.rate_frac_per_sec.is_finite() {
			self.rate_frac_per_sec.max(0.0)
		} else {
			0.0
		}
	}

	/// `None` means the session runs until the active cap (no time limit).
	pub fn duration_secs(self) -> Option<f32> {
		if self.duration_secs.is_finite() && self.duration_secs > 0.0 {
			Some(self.duration_secs)
		} else {
			None
		}
	}

	pub fn cap_fraction(self, damaged: bool) -> f32 {
		let raw = if damaged { self.cap_damaged } else { self.cap_healthy };
		if raw.is_finite() {
			raw.clamp(0.0, 1.0)
		} else {
			0.0
		}
	}

	pub fn cap_hp(self, max_hp: f32, damaged: bool) -> f32 {
		let max = if max_hp.is_finite() { max_hp.max(0.0) } else { 0.0 };
		max * self.cap_fraction(damaged)
	}

	pub fn hp_per_sec(self, max_hp: f32) -> f32 {
		let max = if max_hp.is_finite() { max_hp.max(0.0) } else { 0.0 };
		max * self.rate_frac_per_sec()
	}

	pub fn remaining_hp(self, health: Health, damaged: bool) -> f32 {
		let cap = self.cap_hp(health.max, damaged);
		let current = if health.current.is_finite() { health.current.max(0.0) } else { 0.0 };
		(cap - current).max(0.0)
	}
}

/// Pause / menu gate. World copies `WorldGameplayEnabled` onto this; when
/// `false`, healing freezes without cancelling the session.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct LubikanHealingEnabled(pub bool);

impl Default for LubikanHealingEnabled {
	fn default() -> Self {
		Self(true)
	}
}

/// HUD-readable Self healing channel.
///
/// Presentation reads `active`, `remaining`, and `rate`. It does not mutate
/// charge or start sessions. `owner` is `None` when no session is live.
#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub struct LubikanSelfHeal {
	pub active: bool,
	pub remaining: f32,
	pub rate: f32,
	pub owner: Option<Entity>,
	pub account: Option<LubikanAccountId>,
	pub session: Option<LubikanActivationSessionId>,
	pub damaged: bool,
	pub cap_fraction: f32,
}

impl Default for LubikanSelfHeal {
	fn default() -> Self {
		Self {
			active: false,
			remaining: 0.0,
			rate: 0.0,
			owner: None,
			account: None,
			session: None,
			damaged: false,
			cap_fraction: DEFAULT_SELF_HEAL_CAP_HEALTHY,
		}
	}
}

impl LubikanSelfHeal {
	pub fn idle() -> Self {
		Self::default()
	}

	pub fn is_active(self) -> bool {
		self.active
	}
}

/// Live channel on the session owner. Removed when the session closes so
/// HUD cannot keep a stale owner.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct LubikanSelfHealChannel {
	pub active: bool,
	pub remaining: f32,
	pub rate: f32,
	pub account: LubikanAccountId,
	pub session: LubikanActivationSessionId,
	pub damaged: bool,
	pub cap_fraction: f32,
}

#[derive(Resource, Debug, Default)]
struct LubikanSelfHealRuntime {
	live: Option<LiveSelfHeal>,
}

#[derive(SystemParam)]
struct SelfHealClose<'w, 's> {
	runtime: ResMut<'w, LubikanSelfHealRuntime>,
	presentation: ResMut<'w, LubikanSelfHeal>,
	reports: MessageWriter<'w, LubikanActivationReport>,
	finished: MessageWriter<'w, LubikanActivationFinished>,
	ledgers: Query<'w, 's, &'static mut LubikanLedger>,
	store: ResMut<'w, LubikanSessionStore>,
	commands: Commands<'w, 's>,
}

impl SelfHealClose<'_, '_> {
	fn complete(&mut self, session: LubikanActivationSession) {
		self.close(session, LubikanActivationOutcome::Completed);
	}

	fn cancel(&mut self, session: LubikanActivationSession) {
		self.close(session, LubikanActivationOutcome::Cancelled);
	}

	fn drop_without_report(&mut self, owner: Entity) {
		clear_heal(&mut self.commands, &mut self.presentation, Some(owner));
		self.runtime.live = None;
	}

	fn close(&mut self, session: LubikanActivationSession, outcome: LubikanActivationOutcome) {
		let mut applied = false;
		for mut ledger in self.ledgers.iter_mut() {
			if apply_close(&mut ledger, session, outcome) {
				applied = true;
				break;
			}
		}
		if !applied {
			if let Some(cached) = self.store.cached_ledger_mut() {
				apply_close(cached, session, outcome);
			}
		}
		self.reports.write(LubikanActivationReport { id: session.id, outcome });
		self.finished.write(LubikanActivationFinished::from_session(session, outcome));
		clear_heal(&mut self.commands, &mut self.presentation, Some(session.owner));
		self.runtime.live = None;
	}
}

#[derive(Clone, Copy, Debug)]
struct LiveSelfHeal {
	session: LubikanActivationSession,
	started_at: f32,
	elapsed: f32,
	damaged: bool,
}

impl LiveSelfHeal {
	fn latch_damage(&mut self) {
		self.damaged = true;
	}
}

pub(crate) fn configure(app: &mut App) {
	if !app.is_plugin_added::<bevy::time::TimePlugin>() {
		app.add_plugins(bevy::time::TimePlugin);
	}
	if app.world().get_resource::<Messages<DamageApplied>>().is_none() {
		app.add_message::<DamageApplied>();
	}
	if app.world().get_resource::<Messages<Died>>().is_none() {
		app.add_message::<Died>();
	}
	app.init_resource::<LubikanSelfHealConfig>()
		.init_resource::<LubikanHealingEnabled>()
		.init_resource::<LubikanSelfHeal>()
		.init_resource::<LubikanSelfHealRuntime>()
		.init_resource::<DamageHistory>()
		.init_resource::<DamageHistoryPolicy>()
		.configure_sets(
			PostUpdate,
			LubikanSystems::Heal
				.after(DamageSystems::History)
				.after(LubikanSystems::Award),
		)
		.add_systems(
			PostUpdate,
			(
				accept_self_sessions,
				latch_self_heal_damage,
				cancel_self_heal,
				tick_self_heal,
			)
				.chain()
				.in_set(LubikanSystems::Heal),
		);
}

fn accept_self_sessions(
	mut sessions: MessageReader<LubikanActivationSession>,
	time: Res<Time>,
	config: Res<LubikanSelfHealConfig>,
	healths: Query<&Health>,
	mut runtime: ResMut<LubikanSelfHealRuntime>,
	mut presentation: ResMut<LubikanSelfHeal>,
	mut commands: Commands,
) {
	for session in sessions.read() {
		if session.definition != LubikanDefinitionId::SELF {
			continue;
		}
		if runtime.live.is_some() {
			continue;
		}
		let live = LiveSelfHeal {
			session: *session,
			started_at: time.elapsed_secs(),
			elapsed: 0.0,
			damaged: false,
		};
		publish_heal(
			&mut commands,
			&mut presentation,
			&config,
			healths.get(session.owner).ok().copied(),
			&live,
		);
		runtime.live = Some(live);
	}
}

fn cancel_self_heal(
	mut close: SelfHealClose,
	owners: Query<(), With<LubikanOwner>>,
	healths: Query<&Health>,
	downed: Query<(), With<Downed>>,
	mut deaths: MessageReader<Died>,
	mut lives: MessageReader<LubikanOwnerLifetime>,
) {
	let Some(live) = close.runtime.live else {
		return;
	};
	match session_phase(close.ledgers.iter(), &close.store, live.session.id) {
		Some(LubikanActivationPhase::Active) => {}
		Some(LubikanActivationPhase::Completed) => {
			close.drop_without_report(live.session.owner);
			return;
		}
		Some(_) | None => {
			close.cancel(live.session);
			return;
		}
	}
	if should_cancel(live.session, &owners, &healths, &downed, &mut deaths, &mut lives) {
		close.cancel(live.session);
	}
}

fn tick_self_heal(
	time: Res<Time>,
	enabled: Res<LubikanHealingEnabled>,
	config: Res<LubikanSelfHealConfig>,
	mut close: SelfHealClose,
	mut healths: Query<&mut Health>,
) {
	let Some(live) = close.runtime.live else {
		return;
	};
	if !enabled.0 {
		if let Ok(health) = healths.get(live.session.owner) {
			publish_heal(
				&mut close.commands,
				&mut close.presentation,
				&config,
				Some(*health),
				&live,
			);
		}
		return;
	}

	let mut live = live;
	live.elapsed += time.delta_secs().max(0.0);
	let timed_out = config.duration_secs().is_some_and(|limit| live.elapsed >= limit);
	let finished = match healths.get_mut(live.session.owner) {
		Ok(mut health) => {
			if health.is_dead() {
				close.cancel(live.session);
				return;
			}
			let rate = config.hp_per_sec(health.max);
			let amount = rate * time.delta_secs().max(0.0);
			let cap = config.cap_hp(health.max, live.damaged);
			health.heal_toward(amount, cap);
			let remaining = config.remaining_hp(*health, live.damaged);
			publish_heal(
				&mut close.commands,
				&mut close.presentation,
				&config,
				Some(*health),
				&live,
			);
			remaining <= 0.0 || timed_out
		}
		Err(_) => timed_out,
	};
	if finished {
		close.complete(live.session);
		return;
	}
	close.runtime.live = Some(live);
}

fn latch_self_heal_damage(
	mut applied: MessageReader<DamageApplied>,
	history: Res<DamageHistory>,
	config: Res<LubikanSelfHealConfig>,
	healths: Query<&Health>,
	mut runtime: ResMut<LubikanSelfHealRuntime>,
	mut presentation: ResMut<LubikanSelfHeal>,
	mut commands: Commands,
) {
	// Always drain so pre-session hits cannot latch a later activation.
	let hits: Vec<DamageApplied> = applied.read().copied().collect();
	let Some(mut live) = runtime.live else {
		return;
	};
	if live.damaged {
		return;
	}
	let owner = live.session.owner;
	let from_events = hits.iter().any(|event| event.target == owner && !event.is_zero_effect());
	let from_history =
		history.received_positive_damage_since(DamageActorId::from_entity(owner), live.started_at);
	if !(from_events || from_history) {
		return;
	}
	live.latch_damage();
	publish_heal(
		&mut commands,
		&mut presentation,
		&config,
		healths.get(owner).ok().copied(),
		&live,
	);
	runtime.live = Some(live);
}

fn should_cancel(
	session: LubikanActivationSession,
	owners: &Query<(), With<LubikanOwner>>,
	healths: &Query<&Health>,
	downed: &Query<(), With<Downed>>,
	deaths: &mut MessageReader<Died>,
	lives: &mut MessageReader<LubikanOwnerLifetime>,
) -> bool {
	if owners.get(session.owner).is_err() {
		return true;
	}
	if downed.get(session.owner).is_ok() {
		return true;
	}
	if healths.get(session.owner).is_ok_and(|health| health.is_dead()) {
		return true;
	}
	if deaths.read().any(|death| death.entity == session.owner) {
		return true;
	}
	lives.read().any(|notice| match notice.kind {
		LubikanOwnerLifetimeKind::RestartSession => true,
		LubikanOwnerLifetimeKind::ReplaceBody | LubikanOwnerLifetimeKind::Unequip => {
			notice.previous == Some(session.owner)
		}
		LubikanOwnerLifetimeKind::Equip => false,
	})
}

fn session_phase<'a>(
	ledgers: impl IntoIterator<Item = &'a LubikanLedger>,
	store: &LubikanSessionStore,
	id: LubikanActivationSessionId,
) -> Option<LubikanActivationPhase> {
	ledgers
		.into_iter()
		.find_map(|ledger| ledger_session_phase(ledger, id))
		.or_else(|| store.cached_ledger().and_then(|ledger| ledger_session_phase(ledger, id)))
}

fn ledger_session_phase(
	ledger: &LubikanLedger,
	id: LubikanActivationSessionId,
) -> Option<LubikanActivationPhase> {
	ledger.iter().find_map(|account| {
		(account.activation().session == Some(id)).then_some(account.activation().phase)
	})
}

/// Close the captured account. A natural complete reports through
/// [`LubikanActivationFinished`] and returns the account to Idle in this
/// frame so a later activate in the same life can spend again.
fn apply_close(
	ledger: &mut LubikanLedger,
	session: LubikanActivationSession,
	outcome: LubikanActivationOutcome,
) -> bool {
	if !ledger.finish_activation(session.id, outcome) {
		return false;
	}
	if outcome == LubikanActivationOutcome::Completed {
		ledger.reset_activation(session.account);
	}
	true
}

fn publish_heal(
	commands: &mut Commands,
	presentation: &mut LubikanSelfHeal,
	config: &LubikanSelfHealConfig,
	health: Option<Health>,
	live: &LiveSelfHeal,
) {
	let remaining = health.map(|health| config.remaining_hp(health, live.damaged)).unwrap_or(0.0);
	let rate = health.map(|health| config.hp_per_sec(health.max)).unwrap_or(0.0);
	let cap_fraction = config.cap_fraction(live.damaged);
	*presentation = LubikanSelfHeal {
		active: true,
		remaining,
		rate,
		owner: Some(live.session.owner),
		account: Some(live.session.account),
		session: Some(live.session.id),
		damaged: live.damaged,
		cap_fraction,
	};
	commands.entity(live.session.owner).try_insert(LubikanSelfHealChannel {
		active: true,
		remaining,
		rate,
		account: live.session.account,
		session: live.session.id,
		damaged: live.damaged,
		cap_fraction,
	});
}

fn clear_heal(commands: &mut Commands, presentation: &mut LubikanSelfHeal, owner: Option<Entity>) {
	if let Some(owner) = owner {
		commands.entity(owner).try_remove::<LubikanSelfHealChannel>();
	}
	*presentation = LubikanSelfHeal::idle();
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::activation::{LubikanActivationDenied, LubikanActivationPhase};
	use crate::identity::{LubikanAccountId, LubikanDefinition, LubikanDefinitionId};
	use crate::plugin::LubikansPlugin;
	use crate::{
		LubikanActivationFinished, LubikanActivationRefused, LubikanActivationRequest,
		LubikanChargeChanged, LubikanChargeReason, LubikanOwnerLifetime,
	};
	use anyhow::{anyhow, Result};
	use bevy::time::{TimeUpdateStrategy, Virtual};
	use damage::{DamagePlugin, DamageRecord, Hit};
	use std::time::Duration;

	const DT: f32 = 1.0;

	fn heal_app() -> App {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(DT)))
			.add_plugins(LubikansPlugin);
		app.world_mut()
			.resource_mut::<Time<Virtual>>()
			.set_max_delta(Duration::from_secs(10));
		app
	}

	fn spawn_owner(app: &mut App, current: f32, max: f32) -> Entity {
		let owner = app.world_mut().spawn((LubikanOwner, Health { current, max })).id();
		app.update();
		owner
	}

	fn fund_and_activate(app: &mut App, owner: Entity) -> Result<LubikanActivationSession> {
		let cost = app
			.world()
			.resource::<crate::LubikanCatalog>()
			.self_definition()
			.activation_cost;
		{
			let mut ledger = app
				.world_mut()
				.get_mut::<LubikanLedger>(owner)
				.ok_or_else(|| anyhow!("ledger"))?;
			ledger.apply_to_equipped(cost);
		}
		app.world_mut().write_message(LubikanActivationRequest::equipped(owner));
		app.update();
		app.world_mut()
			.resource_mut::<Messages<LubikanActivationSession>>()
			.drain()
			.find(|session| session.definition == LubikanDefinitionId::SELF)
			.ok_or_else(|| anyhow!("self session"))
	}

	fn health(app: &App, owner: Entity) -> Result<Health> {
		app.world().get::<Health>(owner).copied().ok_or_else(|| anyhow!("health"))
	}

	fn phase(
		app: &App,
		owner: Entity,
		account: LubikanAccountId,
	) -> Result<LubikanActivationPhase> {
		app.world()
			.get::<LubikanLedger>(owner)
			.and_then(|ledger| ledger.account(account))
			.map(|account| account.activation().phase)
			.ok_or_else(|| anyhow!("activation phase"))
	}

	fn charge(app: &App, owner: Entity, account: LubikanAccountId) -> Result<f32> {
		app.world()
			.get::<LubikanLedger>(owner)
			.and_then(|ledger| ledger.account(account))
			.map(|account| account.charge().current())
			.ok_or_else(|| anyhow!("charge"))
	}

	fn applied(target: Entity, amount: f32, lost: f32, remaining: f32) -> DamageApplied {
		DamageApplied { target, source: None, amount, applied: lost, remaining, point: Vec3::ZERO }
	}

	fn hit(target: Entity, amount: f32) -> Hit {
		Hit { target, source: None, amount, point: Vec3::ZERO, allow_self: false }
	}

	fn hit_app() -> App {
		let mut app = heal_app();
		if !app.is_plugin_added::<DamagePlugin>() {
			app.add_plugins(DamagePlugin);
		}
		app
	}

	fn finished(app: &mut App) -> Vec<LubikanActivationFinished> {
		app.world_mut()
			.resource_mut::<Messages<LubikanActivationFinished>>()
			.drain()
			.collect()
	}

	#[test]
	fn insufficient_charge_is_refused_with_no_spend_or_heal() -> Result<()> {
		let mut app = heal_app();
		let owner = spawn_owner(&mut app, 50.0, 100.0);
		app.world_mut().write_message(LubikanActivationRequest::equipped(owner));
		app.update();
		let refused = app
			.world_mut()
			.resource_mut::<Messages<LubikanActivationRefused>>()
			.drain()
			.next()
			.ok_or_else(|| anyhow!("refused"))?;
		assert_eq!(refused.reason, LubikanActivationDenied::InsufficientCharge);
		assert_eq!(charge(&app, owner, LubikanAccountId::SELF)?, 0.0);
		assert_eq!(phase(&app, owner, LubikanAccountId::SELF)?, LubikanActivationPhase::Idle);
		assert!(!app.world().resource::<LubikanSelfHeal>().active);
		assert_eq!(health(&app, owner)?.current, 50.0);
		Ok(())
	}

	#[test]
	fn granted_session_spends_exactly_once_and_starts_heal() -> Result<()> {
		let mut app = heal_app();
		let owner = spawn_owner(&mut app, 40.0, 100.0);
		let cost = app
			.world()
			.resource::<crate::LubikanCatalog>()
			.self_definition()
			.activation_cost;
		let session = fund_and_activate(&mut app, owner)?;
		assert_eq!(session.account, LubikanAccountId::SELF);
		let spends: Vec<_> = app
			.world_mut()
			.resource_mut::<Messages<LubikanChargeChanged>>()
			.drain()
			.filter(|change| change.reason == LubikanChargeReason::ACTIVATION)
			.collect();
		assert_eq!(spends.len(), 1);
		assert_eq!(spends[0].amount, -cost);
		assert_eq!(charge(&app, owner, LubikanAccountId::SELF)?, 0.0);
		assert_eq!(phase(&app, owner, LubikanAccountId::SELF)?, LubikanActivationPhase::Active);
		let heal = *app.world().resource::<LubikanSelfHeal>();
		assert!(heal.active);
		assert_eq!(heal.owner, Some(owner));
		assert_eq!(heal.rate, 5.0);
		assert!((heal.remaining - 55.0).abs() < 1e-4);
		assert_eq!(health(&app, owner)?.current, 45.0);
		Ok(())
	}

	#[test]
	fn healing_amount_and_timing_follow_config() -> Result<()> {
		let mut app = heal_app();
		app.insert_resource(LubikanSelfHealConfig {
			rate_frac_per_sec: 0.10,
			duration_secs: 20.0,
			cap_healthy: 1.0,
			cap_damaged: 0.7,
		});
		let owner = spawn_owner(&mut app, 20.0, 50.0);
		fund_and_activate(&mut app, owner)?;
		assert!((health(&app, owner)?.current - 25.0).abs() < 1e-4);
		app.update();
		assert!((health(&app, owner)?.current - 30.0).abs() < 1e-4);
		assert!((app.world().resource::<LubikanSelfHeal>().rate - 5.0).abs() < 1e-4);
		Ok(())
	}

	#[test]
	fn undamaged_healing_stops_at_full_and_never_exceeds_max() -> Result<()> {
		let mut app = heal_app();
		let owner = spawn_owner(&mut app, 90.0, 100.0);
		fund_and_activate(&mut app, owner)?;
		assert_eq!(health(&app, owner)?.current, 95.0);
		app.update();
		assert_eq!(health(&app, owner)?.current, 100.0);
		assert!(!app.world().resource::<LubikanSelfHeal>().active);
		assert_eq!(phase(&app, owner, LubikanAccountId::SELF)?, LubikanActivationPhase::Idle);
		let pulse = finished(&mut app);
		assert_eq!(pulse.len(), 1);
		assert_eq!(pulse[0].account, LubikanAccountId::SELF);
		assert_eq!(pulse[0].outcome, LubikanActivationOutcome::Completed);
		app.update();
		assert_eq!(health(&app, owner)?.current, 100.0);
		assert_eq!(phase(&app, owner, LubikanAccountId::SELF)?, LubikanActivationPhase::Idle);
		Ok(())
	}

	#[test]
	fn damaged_healing_caps_at_seventy_and_starting_above_loses_nothing() -> Result<()> {
		let mut app = heal_app();
		let owner = spawn_owner(&mut app, 50.0, 100.0);
		fund_and_activate(&mut app, owner)?;
		app.world_mut().write_message(applied(owner, 80.0, 10.0, 45.0));
		app.update();
		assert!(app.world().resource::<LubikanSelfHeal>().damaged);
		assert_eq!(app.world().resource::<LubikanSelfHeal>().cap_fraction, 0.7);
		let after_latch = health(&app, owner)?.current;
		assert!(after_latch <= 70.0);
		for _ in 0..10 {
			app.update();
		}
		assert_eq!(health(&app, owner)?.current, 70.0);
		assert!(!app.world().resource::<LubikanSelfHeal>().active);

		let high = spawn_owner(&mut app, 85.0, 100.0);
		fund_and_activate(&mut app, high)?;
		app.world_mut().resource_mut::<LubikanHealingEnabled>().0 = false;
		app.world_mut().write_message(applied(high, 40.0, 5.0, 85.0));
		app.update();
		let paused = health(&app, high)?.current;
		assert!(paused > 70.0);
		assert!(app.world().resource::<LubikanSelfHeal>().damaged);
		app.world_mut().resource_mut::<LubikanHealingEnabled>().0 = true;
		app.update();
		assert_eq!(health(&app, high)?.current, paused);
		assert!(!app.world().resource::<LubikanSelfHeal>().active);
		Ok(())
	}

	#[test]
	fn select_switch_completes_the_original_account() -> Result<()> {
		let mut app = heal_app();
		let owner = spawn_owner(&mut app, 90.0, 100.0);
		let extra = LubikanDefinition::new(LubikanDefinitionId::from_raw(3), 40.0, 5.0);
		app.world_mut().resource_mut::<crate::LubikanCatalog>().register(extra);
		let other = {
			let mut ledger = app
				.world_mut()
				.get_mut::<LubikanLedger>(owner)
				.ok_or_else(|| anyhow!("ledger"))?;
			let other = ledger.ensure_owned(extra);
			ledger.apply_to(other, 9.0);
			other
		};
		fund_and_activate(&mut app, owner)?;
		{
			let mut ledger = app
				.world_mut()
				.get_mut::<LubikanLedger>(owner)
				.ok_or_else(|| anyhow!("ledger"))?;
			if !ledger.select(Some(other)) {
				return Err(anyhow!("switch"));
			}
		}
		app.update();
		assert_eq!(phase(&app, owner, LubikanAccountId::SELF)?, LubikanActivationPhase::Idle);
		assert_eq!(phase(&app, owner, other)?, LubikanActivationPhase::Idle);
		assert_eq!(charge(&app, owner, other)?, 9.0);
		assert_eq!(health(&app, owner)?.current, 100.0);
		let pulse = finished(&mut app);
		assert_eq!(pulse.len(), 1);
		assert_eq!(pulse[0].account, LubikanAccountId::SELF);
		assert_eq!(pulse[0].outcome, LubikanActivationOutcome::Completed);
		Ok(())
	}

	#[test]
	fn pause_freezes_healing_without_resetting_the_session() -> Result<()> {
		let mut app = heal_app();
		let owner = spawn_owner(&mut app, 40.0, 100.0);
		fund_and_activate(&mut app, owner)?;
		assert_eq!(health(&app, owner)?.current, 45.0);
		app.world_mut().resource_mut::<LubikanHealingEnabled>().0 = false;
		app.update();
		app.update();
		assert_eq!(health(&app, owner)?.current, 45.0);
		assert_eq!(phase(&app, owner, LubikanAccountId::SELF)?, LubikanActivationPhase::Active);
		assert!(app.world().resource::<LubikanSelfHeal>().active);
		app.world_mut().resource_mut::<LubikanHealingEnabled>().0 = true;
		app.update();
		assert_eq!(health(&app, owner)?.current, 50.0);
		assert_eq!(phase(&app, owner, LubikanAccountId::SELF)?, LubikanActivationPhase::Active);
		Ok(())
	}

	#[test]
	fn death_replacement_and_session_end_cancel_cleanly() -> Result<()> {
		let mut app = heal_app();
		let owner = spawn_owner(&mut app, 40.0, 100.0);
		fund_and_activate(&mut app, owner)?;
		app.world_mut()
			.write_message(Died { entity: owner, source: None, point: Vec3::ZERO });
		app.update();
		assert!(!app.world().resource::<LubikanSelfHeal>().active);
		assert_eq!(app.world().resource::<LubikanSelfHeal>().owner, None);
		assert!(app.world().get::<LubikanSelfHealChannel>(owner).is_none());
		assert_eq!(phase(&app, owner, LubikanAccountId::SELF)?, LubikanActivationPhase::Idle);

		let first = spawn_owner(&mut app, 30.0, 100.0);
		fund_and_activate(&mut app, first)?;
		app.world_mut().entity_mut(first).remove::<LubikanOwner>();
		app.update();
		assert!(!app.world().resource::<LubikanSelfHeal>().active);
		assert!(app.world().get::<LubikanSelfHealChannel>(first).is_none());
		let second =
			app.world_mut().spawn((LubikanOwner, Health { current: 30.0, max: 100.0 })).id();
		app.update();
		assert_eq!(phase(&app, second, LubikanAccountId::SELF)?, LubikanActivationPhase::Idle);
		assert!(!app.world().resource::<LubikanSelfHeal>().active);

		let session_owner = spawn_owner(&mut app, 20.0, 100.0);
		fund_and_activate(&mut app, session_owner)?;
		app.world_mut().write_message(LubikanOwnerLifetime::new(
			None,
			None,
			LubikanOwnerLifetimeKind::RestartSession,
		));
		app.update();
		assert!(!app.world().resource::<LubikanSelfHeal>().active);
		assert_eq!(app.world().resource::<LubikanSelfHeal>().owner, None);
		assert!(app.world().get::<LubikanSelfHealChannel>(session_owner).is_none());
		Ok(())
	}

	#[test]
	fn applied_hp_latch_survives_history_window_expiry() -> Result<()> {
		let mut app = heal_app();
		let owner = spawn_owner(&mut app, 40.0, 100.0);
		fund_and_activate(&mut app, owner)?;
		app.world_mut().write_message(applied(owner, 50.0, 0.0, 45.0));
		app.update();
		assert!(!app.world().resource::<LubikanSelfHeal>().damaged);
		app.world_mut().write_message(applied(owner, 80.0, 12.0, 33.0));
		app.update();
		assert!(app.world().resource::<LubikanSelfHeal>().damaged);
		app.world_mut().resource_mut::<DamageHistory>().clear();
		app.world_mut().resource_mut::<DamageHistory>().push(DamageRecord {
			at: 0.0,
			recipient: DamageActorId::from_entity(owner),
			requested: 80.0,
			applied: 0.0,
			remaining: 33.0,
			point: Vec3::ZERO,
			source: None,
			owner: None,
			credit: None,
		});
		for _ in 0..8 {
			app.update();
		}
		assert!(
			app.world().resource::<LubikanSelfHeal>().damaged
				|| !app.world().resource::<LubikanSelfHeal>().active
		);
		assert_eq!(health(&app, owner)?.current, 70.0);
		Ok(())
	}

	#[test]
	fn pre_activation_hit_does_not_latch_the_ceiling() -> Result<()> {
		let mut app = hit_app();
		let owner = spawn_owner(&mut app, 40.0, 100.0);
		app.world_mut().write_message(hit(owner, 10.0));
		app.update();
		assert_eq!(health(&app, owner)?.current, 30.0);
		fund_and_activate(&mut app, owner)?;
		assert!(!app.world().resource::<LubikanSelfHeal>().damaged);
		for _ in 0..20 {
			app.update();
		}
		assert!(!app.world().resource::<LubikanSelfHeal>().active);
		assert_eq!(health(&app, owner)?.current, 100.0);
		Ok(())
	}

	#[test]
	fn hit_during_healing_latches_seventy() -> Result<()> {
		let mut app = hit_app();
		let owner = spawn_owner(&mut app, 40.0, 100.0);
		fund_and_activate(&mut app, owner)?;
		assert_eq!(health(&app, owner)?.current, 45.0);
		app.world_mut().write_message(hit(owner, 5.0));
		app.update();
		assert!(app.world().resource::<LubikanSelfHeal>().damaged);
		assert_eq!(app.world().resource::<LubikanSelfHeal>().cap_fraction, 0.7);
		for _ in 0..12 {
			app.update();
		}
		assert_eq!(health(&app, owner)?.current, 70.0);
		Ok(())
	}

	#[test]
	fn hit_near_completion_does_not_subtract_existing_hp() -> Result<()> {
		let mut app = hit_app();
		let owner = spawn_owner(&mut app, 90.0, 100.0);
		fund_and_activate(&mut app, owner)?;
		assert_eq!(health(&app, owner)?.current, 95.0);
		app.world_mut().write_message(hit(owner, 5.0));
		app.update();
		assert_eq!(health(&app, owner)?.current, 90.0);
		assert!(!app.world().resource::<LubikanSelfHeal>().active);
		let pulse = finished(&mut app);
		assert_eq!(pulse.len(), 1);
		assert_eq!(pulse[0].outcome, LubikanActivationOutcome::Completed);
		app.update();
		assert_eq!(health(&app, owner)?.current, 90.0);
		Ok(())
	}

	#[test]
	fn pause_freezes_healing_on_the_same_update() -> Result<()> {
		let mut app = heal_app();
		let owner = spawn_owner(&mut app, 40.0, 100.0);
		fund_and_activate(&mut app, owner)?;
		assert_eq!(health(&app, owner)?.current, 45.0);
		app.world_mut().resource_mut::<LubikanHealingEnabled>().0 = false;
		app.update();
		assert_eq!(health(&app, owner)?.current, 45.0);
		assert_eq!(phase(&app, owner, LubikanAccountId::SELF)?, LubikanActivationPhase::Active);
		app.world_mut().resource_mut::<LubikanHealingEnabled>().0 = true;
		app.update();
		assert_eq!(health(&app, owner)?.current, 50.0);
		Ok(())
	}

	#[test]
	fn natural_finish_idles_and_a_second_activate_spends_and_heals() -> Result<()> {
		let mut app = heal_app();
		let owner = spawn_owner(&mut app, 90.0, 100.0);
		let cost = app
			.world()
			.resource::<crate::LubikanCatalog>()
			.self_definition()
			.activation_cost;
		let first = fund_and_activate(&mut app, owner)?;
		assert_eq!(health(&app, owner)?.current, 95.0);
		assert_eq!(phase(&app, owner, LubikanAccountId::SELF)?, LubikanActivationPhase::Active);
		app.update();
		assert_eq!(health(&app, owner)?.current, 100.0);
		assert_eq!(phase(&app, owner, LubikanAccountId::SELF)?, LubikanActivationPhase::Idle);
		assert!(!app.world().resource::<LubikanSelfHeal>().active);
		let pulse = finished(&mut app);
		assert_eq!(pulse.len(), 1);
		assert_eq!(pulse[0].id, first.id);
		assert_eq!(pulse[0].account, LubikanAccountId::SELF);
		assert_eq!(pulse[0].owner, owner);
		assert_eq!(pulse[0].outcome, LubikanActivationOutcome::Completed);

		{
			let mut health =
				app.world_mut().get_mut::<Health>(owner).ok_or_else(|| anyhow!("health"))?;
			health.take_damage(40.0);
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
		let second = app
			.world_mut()
			.resource_mut::<Messages<LubikanActivationSession>>()
			.drain()
			.find(|session| session.definition == LubikanDefinitionId::SELF)
			.ok_or_else(|| anyhow!("second session"))?;
		assert_ne!(second.id, first.id);
		assert_eq!(second.account, LubikanAccountId::SELF);
		let spends: Vec<_> = app
			.world_mut()
			.resource_mut::<Messages<LubikanChargeChanged>>()
			.drain()
			.filter(|change| change.reason == LubikanChargeReason::ACTIVATION)
			.collect();
		assert_eq!(spends.len(), 1);
		assert_eq!(spends[0].amount, -cost);
		assert_eq!(charge(&app, owner, LubikanAccountId::SELF)?, 0.0);
		assert_eq!(phase(&app, owner, LubikanAccountId::SELF)?, LubikanActivationPhase::Active);
		assert_eq!(health(&app, owner)?.current, 65.0);
		assert!(app.world().resource::<LubikanSelfHeal>().active);
		Ok(())
	}
}
