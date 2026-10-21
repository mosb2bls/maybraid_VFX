//! Sole routing owner. Controllers read assignments; they do not rediscover actors.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use crate::actor::ActorTarget;
use crate::capture::{CaptureKind, CaptureLayer, CaptureStack, CaptureToken, ScopeOwner};
use crate::source::InputSource;
use crate::ui::UiTarget;

/// Opaque handle for one suspension. Only this token (or its owner) resumes it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SuspensionToken(u64);

impl SuspensionToken {
	pub const fn from_raw(raw: u64) -> Self {
		Self(raw)
	}

	pub const fn raw(self) -> u64 {
		self.0
	}
}

/// Explicit source → actor / UI binding. Loss of either side clears that delivery.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SourceAssignment {
	actor: Option<ActorTarget>,
	ui: Option<UiTarget>,
}

impl SourceAssignment {
	pub const fn empty() -> Self {
		Self { actor: None, ui: None }
	}

	pub const fn actor(self) -> Option<ActorTarget> {
		self.actor
	}

	pub const fn ui(self) -> Option<UiTarget> {
		self.ui
	}

	pub const fn is_empty(self) -> bool {
		self.actor.is_none() && self.ui.is_none()
	}
}

/// Source lifecycle. Distinct from "no actor assigned yet".
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SourceConnection {
	/// Live source. Empty assignment means the host may apply lowest-ID fallback.
	#[default]
	Connected,
	/// Explicit disconnect. Host fallback must not assign; only [`InputRouter::connect`].
	Disconnected,
}

/// Why the router emitted a lifecycle event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RoutingKind {
	Assigned,
	Reassigned,
	Disconnected,
	Removed,
	/// First capture that blocked actor delivery.
	Captured,
	/// Last blocking capture released; actor delivery restored.
	Uncaptured,
	Suspended,
	Resumed,
}

impl RoutingKind {
	/// Held-input and gesture effects for this kind.
	pub const fn effects(self) -> (HoldEffect, GestureEffect) {
		match self {
			Self::Assigned | Self::Resumed | Self::Uncaptured => {
				(HoldEffect::None, GestureEffect::None)
			}
			Self::Reassigned | Self::Disconnected | Self::Removed | Self::Captured => {
				(HoldEffect::ReleaseNeutral, GestureEffect::Cancel)
			}
			Self::Suspended => (HoldEffect::Freeze, GestureEffect::Freeze),
		}
	}
}

/// What listeners must do with buttons / axes that are currently held.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HoldEffect {
	None,
	/// Release to the control's neutral (reassignment, disconnect, removal).
	ReleaseNeutral,
	/// Keep the held value; do not start new motion (suspension).
	Freeze,
}

/// What listeners must do with in-flight gesture histories.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GestureEffect {
	None,
	/// Drop partial flicks / bumper holds (reassignment, disconnect, removal).
	Cancel,
	/// Keep history but do not complete against a new session (suspension).
	Freeze,
}

/// One routing lifecycle event with its hold / gesture contract.
///
/// Published as a Bevy [`Message`] so each consumer's [`MessageReader`] cursor
/// sees every event exactly once, regardless of schedule stage.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct RoutingEvent {
	source: InputSource,
	kind: RoutingKind,
	hold: HoldEffect,
	gesture: GestureEffect,
}

impl RoutingEvent {
	pub const fn source(self) -> InputSource {
		self.source
	}

	pub const fn kind(self) -> RoutingKind {
		self.kind
	}

	pub const fn hold(self) -> HoldEffect {
		self.hold
	}

	pub const fn gesture(self) -> GestureEffect {
		self.gesture
	}
}

/// One owner's freeze of a source. Simulation may continue; delivery does not.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Suspension {
	token: SuspensionToken,
	source: InputSource,
	owner: ScopeOwner,
}

impl Suspension {
	pub const fn token(self) -> SuspensionToken {
		self.token
	}

	pub const fn source(self) -> InputSource {
		self.source
	}

	pub const fn owner(self) -> ScopeOwner {
		self.owner
	}
}

#[derive(Clone, Debug)]
struct SourceBinding {
	assignment: SourceAssignment,
	connection: SourceConnection,
	suspensions: Vec<Suspension>,
}

impl Default for SourceBinding {
	fn default() -> Self {
		Self {
			assignment: SourceAssignment::empty(),
			connection: SourceConnection::Connected,
			suspensions: Vec::new(),
		}
	}
}

/// Sole owner of source assignment, capture, and suspension.
///
/// Controllers read [`Self::actor`] / [`Self::delivered_actor`]. They must not
/// query players to rediscover a fallback. Hosts call
/// [`Self::assign_lowest_id_actor`] once at the assignment boundary.
#[derive(Resource, Clone, Debug, Default)]
pub struct InputRouter {
	next_token: u64,
	local: SourceBinding,
	captures: CaptureStack,
	events: Vec<RoutingEvent>,
}

impl InputRouter {
	pub fn assignment(&self, source: InputSource) -> SourceAssignment {
		self.binding(source).assignment
	}

	pub fn actor(&self, source: InputSource) -> Option<ActorTarget> {
		self.assignment(source).actor()
	}

	pub fn ui(&self, source: InputSource) -> Option<UiTarget> {
		self.assignment(source).ui()
	}

	pub fn local_actor(&self) -> Option<ActorTarget> {
		self.actor(InputSource::local_pad())
	}

	/// Local-pad actor delivery: assigned, connected, not captured, not suspended.
	pub fn delivered_local_actor(&self) -> Option<ActorTarget> {
		self.delivered_actor(InputSource::local_pad())
	}

	pub fn connection(&self, source: InputSource) -> SourceConnection {
		self.binding(source).connection
	}

	pub fn connected(&self, source: InputSource) -> bool {
		self.connection(source) == SourceConnection::Connected
	}

	/// Connected and waiting for the host's first (or next) actor.
	pub fn awaiting_assignment(&self, source: InputSource) -> bool {
		self.connected(source) && self.actor(source).is_none()
	}

	pub fn captures(&self) -> &CaptureStack {
		&self.captures
	}

	pub fn capture(&self, token: CaptureToken) -> Option<CaptureLayer> {
		self.captures.layer(token)
	}

	pub fn top_capture(&self) -> Option<CaptureLayer> {
		self.captures.top()
	}

	pub fn top_capture_of_kind(&self, kind: CaptureKind) -> Option<CaptureLayer> {
		self.captures.top_of_kind(kind)
	}

	pub fn blocks_actor(&self, source: InputSource) -> bool {
		self.captures.blocks_actor(source)
	}

	pub fn is_suspended(&self, source: InputSource) -> bool {
		!self.binding(source).suspensions.is_empty()
	}

	/// Whether `owner` currently holds a suspension on `source`.
	pub fn has_suspension(&self, source: InputSource, owner: ScopeOwner) -> bool {
		self.binding(source).suspensions.iter().any(|layer| layer.owner() == owner)
	}

	/// Derived pad policy from the capture stack. One writer.
	pub fn pad_policy(&self) -> maybraid_input::PadInputPolicy {
		self.captures.pad_policy()
	}

	/// Session pause of local-pad delivery. Other owners stay frozen.
	pub fn set_paused(&mut self, paused: bool) {
		if paused {
			self.suspend(InputSource::local_pad(), ScopeOwner::pause());
		} else {
			self.resume_owner(ScopeOwner::pause());
		}
	}

	/// Actor delivery: assigned, connected, not captured, not suspended.
	pub fn delivered_actor(&self, source: InputSource) -> Option<ActorTarget> {
		if !self.connected(source) || self.blocks_actor(source) || self.is_suspended(source) {
			return None;
		}
		self.actor(source)
	}

	/// UI delivery: capture owner if present, otherwise the assigned UI.
	pub fn delivered_ui(&self, source: InputSource) -> Option<UiTarget> {
		if !self.connected(source) || self.is_suspended(source) {
			return None;
		}
		if let Some(owner) = self.captures.top().and_then(CaptureLayer::ui_owner) {
			return Some(owner);
		}
		self.ui(source)
	}

	pub fn assign_actor(&mut self, source: InputSource, actor: ActorTarget) -> Option<ActorTarget> {
		if !self.connected(source) {
			return self.actor(source);
		}
		let previous = self.actor(source);
		if previous == Some(actor) {
			return previous;
		}
		self.binding_mut(source).assignment.actor = Some(actor);
		self.emit(
			source,
			if previous.is_some() { RoutingKind::Reassigned } else { RoutingKind::Assigned },
		);
		previous
	}

	pub fn assign_ui(&mut self, source: InputSource, ui: UiTarget) -> Option<UiTarget> {
		if !self.connected(source) {
			return self.ui(source);
		}
		let previous = self.ui(source);
		if previous == Some(ui) {
			return previous;
		}
		self.binding_mut(source).assignment.ui = Some(ui);
		self.emit(
			source,
			if previous.is_some() { RoutingKind::Reassigned } else { RoutingKind::Assigned },
		);
		previous
	}

	/// Host-only. If `source` already has an actor, keep it (no rediscovery).
	/// Otherwise assign the lowest-ID candidate once, only while connected.
	/// Disconnect stays disconnected until [`Self::connect`].
	pub fn assign_lowest_id_actor(
		&mut self,
		source: InputSource,
		candidates: impl IntoIterator<Item = Entity>,
	) -> Option<ActorTarget> {
		if !self.connected(source) {
			return None;
		}
		if let Some(actor) = self.actor(source) {
			return Some(actor);
		}
		let actor = candidates
			.into_iter()
			.min_by_key(|entity| entity.to_bits())
			.map(ActorTarget::new)?;
		self.assign_actor(source, actor);
		Some(actor)
	}

	/// Host-only. Assignment loss (despawn / leave) clears actor delivery.
	pub fn retain_assigned_actor(
		&mut self,
		source: InputSource,
		live: impl IntoIterator<Item = Entity>,
	) {
		let Some(actor) = self.actor(source) else {
			return;
		};
		if live.into_iter().any(|entity| actor.matches(entity)) {
			return;
		}
		self.clear_actor(source);
	}

	pub fn clear_actor(&mut self, source: InputSource) -> Option<ActorTarget> {
		let previous = self.actor(source)?;
		self.binding_mut(source).assignment.actor = None;
		self.emit(source, RoutingKind::Removed);
		Some(previous)
	}

	pub fn clear_ui(&mut self, source: InputSource) -> Option<UiTarget> {
		let previous = self.ui(source)?;
		self.binding_mut(source).assignment.ui = None;
		self.emit(source, RoutingKind::Removed);
		Some(previous)
	}

	pub fn clear_assignment(&mut self, source: InputSource) -> SourceAssignment {
		let previous = self.assignment(source);
		if previous.is_empty() {
			return previous;
		}
		self.binding_mut(source).assignment = SourceAssignment::empty();
		self.emit(source, RoutingKind::Removed);
		previous
	}

	pub fn disconnect(&mut self, source: InputSource) -> bool {
		let binding = self.binding_mut(source);
		if binding.connection == SourceConnection::Disconnected && binding.assignment.is_empty() {
			return false;
		}
		binding.connection = SourceConnection::Disconnected;
		binding.assignment = SourceAssignment::empty();
		self.emit(source, RoutingKind::Disconnected);
		true
	}

	/// Source-lifecycle reconnect. Does not assign; host fallback may run after.
	pub fn connect(&mut self, source: InputSource) -> bool {
		let binding = self.binding_mut(source);
		if binding.connection == SourceConnection::Connected {
			return false;
		}
		binding.connection = SourceConnection::Connected;
		true
	}

	pub fn acquire_capture(
		&mut self,
		source: InputSource,
		owner: ScopeOwner,
		kind: CaptureKind,
	) -> CaptureToken {
		if let Some(layer) = self.captures.find(source, owner, kind) {
			return layer.token();
		}
		let before = self.delivered_actor(source);
		let token = self.push_capture(source, owner, kind);
		self.emit_capture_transition(source, before);
		token
	}

	pub fn release_capture(&mut self, token: CaptureToken) -> Option<CaptureLayer> {
		let source = self.captures.layer(token).map(CaptureLayer::source)?;
		let before = self.delivered_actor(source);
		let layer = self.captures.release(token)?;
		self.emit_capture_transition(source, before);
		Some(layer)
	}

	pub fn release_owner(&mut self, owner: ScopeOwner) -> Vec<CaptureLayer> {
		let source = InputSource::local_pad();
		let before = self.delivered_actor(source);
		let released = self.captures.release_owner(owner);
		self.emit_capture_transition(source, before);
		released
	}

	/// Replace `token`'s capture without releasing any other owner.
	///
	/// Updating the same token's owner keeps its stack position. A new claim
	/// is appended.
	pub fn replace_owned_capture(
		&mut self,
		token: &mut Option<CaptureToken>,
		source: InputSource,
		owner: Option<ScopeOwner>,
		kind: CaptureKind,
	) -> Option<CaptureToken> {
		if let Some(owner) = owner {
			if let Some(current) = *token {
				if let Some(layer) = self.capture(current) {
					if layer.source() == source && layer.kind() == kind {
						if layer.owner() != owner {
							self.captures.update_owner(current, owner);
						}
						return Some(current);
					}
				}
			}
			let before = self.delivered_actor(source);
			if let Some(current) = token.take() {
				self.captures.release(current);
			}
			let next = if let Some(layer) = self.captures.find(source, owner, kind) {
				layer.token()
			} else {
				self.push_capture(source, owner, kind)
			};
			*token = Some(next);
			self.emit_capture_transition(source, before);
			Some(next)
		} else {
			let before = self.delivered_actor(source);
			if let Some(current) = token.take() {
				self.captures.release(current);
			}
			self.emit_capture_transition(source, before);
			None
		}
	}

	pub fn suspend(&mut self, source: InputSource, owner: ScopeOwner) -> SuspensionToken {
		if let Some(existing) = self
			.binding(source)
			.suspensions
			.iter()
			.copied()
			.find(|layer| layer.owner() == owner)
		{
			return existing.token();
		}
		let token = SuspensionToken::from_raw(self.mint());
		self.binding_mut(source).suspensions.push(Suspension { token, source, owner });
		self.emit(source, RoutingKind::Suspended);
		token
	}

	pub fn resume(&mut self, token: SuspensionToken) -> Option<Suspension> {
		for source in [InputSource::local_pad()] {
			let binding = self.binding_mut(source);
			let Some(index) = binding.suspensions.iter().position(|layer| layer.token() == token)
			else {
				continue;
			};
			let layer = binding.suspensions.remove(index);
			self.emit(source, RoutingKind::Resumed);
			return Some(layer);
		}
		None
	}

	/// Resume only this owner's suspensions. Other owners stay frozen.
	pub fn resume_owner(&mut self, owner: ScopeOwner) -> Vec<Suspension> {
		let mut resumed = Vec::new();
		for source in [InputSource::local_pad()] {
			let binding = self.binding_mut(source);
			let mut keep = Vec::new();
			for layer in binding.suspensions.drain(..) {
				if layer.owner() == owner {
					resumed.push(layer);
				} else {
					keep.push(layer);
				}
			}
			binding.suspensions = keep;
			if !resumed.is_empty() {
				self.emit(source, RoutingKind::Resumed);
			}
		}
		resumed
	}

	/// Unpublished buffer. Production consumers use [`MessageReader<RoutingEvent>`].
	pub fn take_events(&mut self) -> Vec<RoutingEvent> {
		std::mem::take(&mut self.events)
	}

	pub fn publish_events(&mut self, writer: &mut MessageWriter<RoutingEvent>) {
		for event in self.take_events() {
			writer.write(event);
		}
	}

	fn push_capture(
		&mut self,
		source: InputSource,
		owner: ScopeOwner,
		kind: CaptureKind,
	) -> CaptureToken {
		let token = CaptureToken::from_raw(self.mint());
		self.captures.push(CaptureStack::new_layer(token, source, owner, kind));
		token
	}

	fn emit_capture_transition(&mut self, source: InputSource, before: Option<ActorTarget>) {
		let after = self.delivered_actor(source);
		if before == after {
			return;
		}
		if before.is_some() && after.is_none() {
			self.emit(source, RoutingKind::Captured);
		} else if before.is_none() && after.is_some() {
			self.emit(source, RoutingKind::Uncaptured);
		}
	}

	fn emit(&mut self, source: InputSource, kind: RoutingKind) {
		let (hold, gesture) = kind.effects();
		self.events.push(RoutingEvent { source, kind, hold, gesture });
	}

	fn mint(&mut self) -> u64 {
		self.next_token += 1;
		self.next_token
	}

	fn binding(&self, source: InputSource) -> &SourceBinding {
		match source {
			InputSource::LocalPad => &self.local,
		}
	}

	fn binding_mut(&mut self, source: InputSource) -> &mut SourceBinding {
		match source {
			InputSource::LocalPad => &mut self.local,
		}
	}
}

/// Mutate [`InputRouter`] and publish lifecycle events as Bevy messages.
///
/// Drop flushes so later stages in the same frame see this system's events.
#[derive(SystemParam)]
pub struct InputRouting<'w> {
	router: ResMut<'w, InputRouter>,
	events: MessageWriter<'w, RoutingEvent>,
}

impl InputRouting<'_> {
	pub fn publish(&mut self) {
		self.router.publish_events(&mut self.events);
	}
}

impl std::ops::Deref for InputRouting<'_> {
	type Target = InputRouter;

	fn deref(&self) -> &InputRouter {
		&self.router
	}
}

impl std::ops::DerefMut for InputRouting<'_> {
	fn deref_mut(&mut self) -> &mut InputRouter {
		&mut self.router
	}
}

impl Drop for InputRouting<'_> {
	fn drop(&mut self) {
		self.publish();
	}
}

/// Safety net for callers that still mutate [`InputRouter`] without [`InputRouting`].
pub fn publish_routing_events(
	mut router: ResMut<InputRouter>,
	mut writer: MessageWriter<RoutingEvent>,
) {
	router.publish_events(&mut writer);
}

#[cfg(test)]
mod tests {
	use super::*;

	fn actor(bits: u64) -> ActorTarget {
		ActorTarget::new(Entity::from_bits(bits))
	}

	fn ui(bits: u64) -> UiTarget {
		UiTarget::new(Entity::from_bits(bits))
	}

	fn last_event(router: &mut InputRouter) -> RoutingEvent {
		router.take_events().pop().expect("routing event")
	}

	fn assert_release_cancel(event: RoutingEvent, kind: RoutingKind) {
		assert_eq!(event.kind(), kind);
		assert_eq!(event.hold(), HoldEffect::ReleaseNeutral);
		assert_eq!(event.gesture(), GestureEffect::Cancel);
	}

	#[test]
	fn assignment_loss_clears_delivery() {
		let mut router = InputRouter::default();
		let source = InputSource::local_pad();
		router.assign_actor(source, actor(4));
		assert_eq!(router.delivered_actor(source), Some(actor(4)));
		router.clear_actor(source);
		assert_eq!(router.actor(source), None);
		assert_eq!(router.delivered_actor(source), None);
		assert_release_cancel(last_event(&mut router), RoutingKind::Removed);
	}

	#[test]
	fn reassignment_releases_holds_and_cancels_gestures() {
		let mut router = InputRouter::default();
		let source = InputSource::local_pad();
		router.assign_actor(source, actor(1));
		router.take_events();
		router.assign_actor(source, actor(2));
		assert_eq!(router.actor(source), Some(actor(2)));
		assert_release_cancel(last_event(&mut router), RoutingKind::Reassigned);
	}

	#[test]
	fn disconnect_clears_delivery_and_cancels() {
		let mut router = InputRouter::default();
		let source = InputSource::local_pad();
		router.assign_actor(source, actor(3));
		router.assign_ui(source, ui(8));
		router.take_events();
		assert!(router.disconnect(source));
		assert_eq!(router.connection(source), SourceConnection::Disconnected);
		assert!(!router.connected(source));
		assert!(!router.awaiting_assignment(source));
		assert!(router.assignment(source).is_empty());
		assert_eq!(router.assign_lowest_id_actor(source, [Entity::from_bits(3)]), None);
		assert_eq!(router.assign_actor(source, actor(4)), None);
		assert_eq!(router.actor(source), None);
		assert_eq!(router.delivered_actor(source), None);
		assert_eq!(router.delivered_ui(source), None);
		assert_release_cancel(last_event(&mut router), RoutingKind::Disconnected);
	}

	#[test]
	fn suspension_freezes_holds_and_gestures() {
		let mut router = InputRouter::default();
		let source = InputSource::local_pad();
		router.assign_actor(source, actor(5));
		router.take_events();
		let owner = ScopeOwner::actor(actor(5));
		let token = router.suspend(source, owner);
		assert!(router.is_suspended(source));
		assert_eq!(router.delivered_actor(source), None);
		let event = last_event(&mut router);
		assert_eq!(event.kind(), RoutingKind::Suspended);
		assert_eq!(event.hold(), HoldEffect::Freeze);
		assert_eq!(event.gesture(), GestureEffect::Freeze);

		assert_eq!(router.resume(token).map(Suspension::owner), Some(owner));
		assert!(!router.is_suspended(source));
		assert_eq!(router.delivered_actor(source), Some(actor(5)));
		assert_eq!(last_event(&mut router).kind(), RoutingKind::Resumed);
	}

	#[test]
	fn releasing_one_suspension_owner_cannot_resume_another() {
		let mut router = InputRouter::default();
		let source = InputSource::local_pad();
		let first = router.suspend(source, ScopeOwner::actor(actor(1)));
		let second = router.suspend(source, ScopeOwner::ui(ui(2)));
		let resumed = router.resume_owner(ScopeOwner::actor(actor(1)));
		assert_eq!(resumed.len(), 1);
		assert_eq!(resumed[0].token(), first);
		assert!(router.is_suspended(source));
		assert_eq!(router.resume(second).map(Suspension::token), Some(second));
		assert!(!router.is_suspended(source));
	}

	#[test]
	fn capture_blocks_actor_delivery_without_clearing_assignment() {
		let mut router = InputRouter::default();
		let source = InputSource::local_pad();
		router.assign_actor(source, actor(7));
		router.take_events();
		let token = router.acquire_capture(source, ScopeOwner::ui(ui(9)), CaptureKind::MenuNav);
		assert_eq!(router.actor(source), Some(actor(7)));
		assert_eq!(router.delivered_actor(source), None);
		assert_eq!(router.delivered_ui(source), Some(ui(9)));
		assert_release_cancel(last_event(&mut router), RoutingKind::Captured);
		router.release_capture(token);
		assert_eq!(router.delivered_actor(source), Some(actor(7)));
		assert_eq!(last_event(&mut router).kind(), RoutingKind::Uncaptured);
	}

	#[test]
	fn nested_captures_emit_only_on_effective_delivery_change() {
		let mut router = InputRouter::default();
		let source = InputSource::local_pad();
		router.assign_actor(source, actor(7));
		router.take_events();
		let first = router.acquire_capture(source, ScopeOwner::ui(ui(1)), CaptureKind::MenuNav);
		assert_eq!(router.take_events().len(), 1);
		let second = router.acquire_capture(source, ScopeOwner::ui(ui(2)), CaptureKind::Overlay);
		assert!(router.take_events().is_empty());
		router.release_capture(second);
		assert!(router.take_events().is_empty());
		router.release_capture(first);
		assert_eq!(last_event(&mut router).kind(), RoutingKind::Uncaptured);
	}

	#[test]
	fn connect_is_the_only_reconnect() {
		let mut router = InputRouter::default();
		let source = InputSource::local_pad();
		assert!(router.awaiting_assignment(source));
		assert_eq!(router.assign_lowest_id_actor(source, [Entity::from_bits(2)]), Some(actor(2)));
		router.take_events();
		router.disconnect(source);
		router.take_events();
		assert_eq!(router.assign_lowest_id_actor(source, [Entity::from_bits(2)]), None);
		assert!(router.connect(source));
		assert!(router.awaiting_assignment(source));
		assert_eq!(router.assign_lowest_id_actor(source, [Entity::from_bits(2)]), Some(actor(2)));
	}

	#[test]
	fn replace_owned_capture_does_not_release_another_owner() {
		let mut router = InputRouter::default();
		let source = InputSource::local_pad();
		let overlay = router.acquire_capture(source, ScopeOwner::ui(ui(2)), CaptureKind::Overlay);
		let mut menu = None;
		router.replace_owned_capture(
			&mut menu,
			source,
			Some(ScopeOwner::ui(ui(1))),
			CaptureKind::MenuNav,
		);
		router.replace_owned_capture(
			&mut menu,
			source,
			Some(ScopeOwner::ui(ui(3))),
			CaptureKind::MenuNav,
		);
		assert_eq!(router.captures().len(), 2);
		assert_eq!(router.capture(overlay).map(CaptureLayer::owner), Some(ScopeOwner::ui(ui(2))));
		assert_eq!(
			router
				.top_capture_of_kind(CaptureKind::MenuNav)
				.and_then(CaptureLayer::ui_owner),
			Some(ui(3))
		);
		router.replace_owned_capture(&mut menu, source, None, CaptureKind::MenuNav);
		assert_eq!(router.capture(overlay).map(CaptureLayer::kind), Some(CaptureKind::Overlay));
		assert!(router.top_capture_of_kind(CaptureKind::MenuNav).is_none());
	}

	#[test]
	fn replace_owned_capture_updates_owner_in_place() {
		let mut router = InputRouter::default();
		let source = InputSource::local_pad();
		let mut menu = None;
		router.replace_owned_capture(
			&mut menu,
			source,
			Some(ScopeOwner::ui(ui(1))),
			CaptureKind::MenuNav,
		);
		let overlay = router.acquire_capture(source, ScopeOwner::ui(ui(2)), CaptureKind::Overlay);
		router.replace_owned_capture(
			&mut menu,
			source,
			Some(ScopeOwner::ui(ui(3))),
			CaptureKind::MenuNav,
		);
		assert_eq!(router.captures().len(), 2);
		assert_eq!(router.top_capture().map(CaptureLayer::kind), Some(CaptureKind::Overlay));
		assert_eq!(
			router
				.top_capture_of_kind(CaptureKind::MenuNav)
				.and_then(CaptureLayer::ui_owner),
			Some(ui(3))
		);
		assert_eq!(router.capture(overlay).map(CaptureLayer::kind), Some(CaptureKind::Overlay));
	}

	#[test]
	fn pause_suspends_delivery_without_clearing_assignment() {
		let mut router = InputRouter::default();
		let source = InputSource::local_pad();
		router.assign_actor(source, actor(5));
		router.take_events();
		router.set_paused(true);
		assert!(router.has_suspension(source, ScopeOwner::pause()));
		assert!(router.is_suspended(source));
		assert_eq!(router.actor(source), Some(actor(5)));
		assert_eq!(router.delivered_actor(source), None);
		router.set_paused(false);
		assert!(!router.is_suspended(source));
		assert_eq!(router.delivered_actor(source), Some(actor(5)));
	}

	#[test]
	fn lowest_id_actor_is_resolved_once() {
		let mut router = InputRouter::default();
		let source = InputSource::local_pad();
		let low = Entity::from_bits(1);
		let high = Entity::from_bits(9);
		assert_eq!(router.assign_lowest_id_actor(source, [high]), Some(actor(9)));
		assert_eq!(router.assign_lowest_id_actor(source, [low, high]), Some(actor(9)));
		router.retain_assigned_actor(source, [high]);
		assert_eq!(router.actor(source), Some(actor(9)));
		router.retain_assigned_actor(source, [low]);
		assert_eq!(router.actor(source), None);
		assert_eq!(router.assign_lowest_id_actor(source, [low, high]), Some(actor(1)));
	}
}
