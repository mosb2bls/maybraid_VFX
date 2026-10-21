//! Token-scoped, composable capture. Releasing one owner cannot release another.

use bevy::prelude::*;
use maybraid_input::PadInputPolicy;

use crate::actor::ActorTarget;
use crate::source::InputSource;
use crate::ui::UiTarget;

/// Why a scope claimed a source.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CaptureKind {
	MenuNav,
	TextEntry,
	Overlay,
}

impl CaptureKind {
	/// Focused UI owns **B** as Back. Character collect skips stance.
	pub const fn owns_back(self) -> bool {
		true
	}

	/// Console text clears analog / face buttons. Menu and overlay keep the pad
	/// so nav and keypads still work.
	pub const fn suppresses_pad(self) -> bool {
		matches!(self, Self::TextEntry)
	}

	/// Overlay and text sit above menu nav when they are the top claim.
	pub const fn precedes_menu_nav(self) -> bool {
		matches!(self, Self::Overlay | Self::TextEntry)
	}
}

/// Opaque handle for one capture layer. Only this token (or its owner) releases it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CaptureToken(u64);

impl CaptureToken {
	pub const fn from_raw(raw: u64) -> Self {
		Self(raw)
	}

	pub const fn raw(self) -> u64 {
		self.0
	}
}

/// Who holds a capture or suspension. Distinct owners cannot release each other.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ScopeOwner {
	Actor(ActorTarget),
	Ui(UiTarget),
}

impl ScopeOwner {
	pub const fn actor(target: ActorTarget) -> Self {
		Self::Actor(target)
	}

	pub const fn ui(target: UiTarget) -> Self {
		Self::Ui(target)
	}

	/// Session pause / gameplay freeze of local-pad delivery.
	pub fn pause() -> Self {
		Self::ui(UiTarget::new(Entity::from_bits(u64::MAX)))
	}

	/// World map sheet overlay.
	pub fn map_overlay() -> Self {
		Self::ui(UiTarget::new(Entity::from_bits(u64::MAX - 1)))
	}

	/// Command console text entry.
	pub fn console() -> Self {
		Self::ui(UiTarget::new(Entity::from_bits(u64::MAX - 2)))
	}

	/// Text-menu lock without a keypad.
	pub fn text_lock() -> Self {
		Self::ui(UiTarget::new(Entity::from_bits(u64::MAX - 3)))
	}

	/// Exclusive nav is claimed but the HudMenu has not spawned yet.
	pub fn awaiting_nav() -> Self {
		Self::ui(UiTarget::new(Entity::from_bits(u64::MAX - 4)))
	}

	/// Locked first-spawn / death respawn map picker.
	pub fn spawn_picker() -> Self {
		Self::ui(UiTarget::new(Entity::from_bits(u64::MAX - 5)))
	}

	pub const fn actor_entity(self) -> Option<Entity> {
		match self {
			Self::Actor(target) => Some(target.entity()),
			Self::Ui(_) => None,
		}
	}

	pub const fn ui_entity(self) -> Option<Entity> {
		match self {
			Self::Ui(target) => Some(target.entity()),
			Self::Actor(_) => None,
		}
	}

	pub const fn ui_target(self) -> Option<UiTarget> {
		match self {
			Self::Ui(target) => Some(target),
			Self::Actor(_) => None,
		}
	}

	pub const fn actor_target(self) -> Option<ActorTarget> {
		match self {
			Self::Actor(target) => Some(target),
			Self::Ui(_) => None,
		}
	}
}

/// One stack frame: a source claimed by a scoped owner.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CaptureLayer {
	token: CaptureToken,
	source: InputSource,
	owner: ScopeOwner,
	kind: CaptureKind,
}

impl CaptureLayer {
	pub const fn token(self) -> CaptureToken {
		self.token
	}

	pub const fn source(self) -> InputSource {
		self.source
	}

	pub const fn owner(self) -> ScopeOwner {
		self.owner
	}

	pub const fn ui_owner(self) -> Option<UiTarget> {
		self.owner.ui_target()
	}

	pub const fn kind(self) -> CaptureKind {
		self.kind
	}

	pub fn blocks_actor(self, source: InputSource) -> bool {
		self.source == source
	}

	pub const fn owns_back(self) -> bool {
		self.kind.owns_back()
	}

	pub const fn suppresses_pad(self) -> bool {
		self.kind.suppresses_pad()
	}
}

/// Nested claims. The top layer wins; tokens and owners release independently.
#[derive(Clone, Debug, Default)]
pub struct CaptureStack {
	layers: Vec<CaptureLayer>,
}

impl CaptureStack {
	pub fn is_empty(&self) -> bool {
		self.layers.is_empty()
	}

	pub fn len(&self) -> usize {
		self.layers.len()
	}

	pub fn top(&self) -> Option<CaptureLayer> {
		self.layers.last().copied()
	}

	pub fn top_of_kind(&self, kind: CaptureKind) -> Option<CaptureLayer> {
		self.layers.iter().rev().copied().find(|layer| layer.kind() == kind)
	}

	pub fn layer(&self, token: CaptureToken) -> Option<CaptureLayer> {
		self.layers.iter().copied().find(|layer| layer.token() == token)
	}

	pub fn find(
		&self,
		source: InputSource,
		owner: ScopeOwner,
		kind: CaptureKind,
	) -> Option<CaptureLayer> {
		self.layers.iter().rev().copied().find(|layer| {
			layer.source() == source && layer.owner() == owner && layer.kind() == kind
		})
	}

	pub fn blocks_actor(&self, source: InputSource) -> bool {
		self.layers.iter().any(|layer| layer.blocks_actor(source))
	}

	pub fn owns_back(&self, source: InputSource) -> bool {
		self.layers.iter().any(|layer| layer.source() == source && layer.owns_back())
	}

	pub fn suppresses_pad(&self, source: InputSource) -> bool {
		self.layers
			.iter()
			.any(|layer| layer.source() == source && layer.suppresses_pad())
	}

	/// Single derived pad policy. Hosts must not write lanes of their own.
	pub fn pad_policy(&self) -> PadInputPolicy {
		PadInputPolicy::new(!self.layers.is_empty(), self.suppresses_pad(InputSource::local_pad()))
	}

	pub(crate) fn push(&mut self, layer: CaptureLayer) {
		self.layers.push(layer);
	}

	/// Update an existing token's owner in place. Stack order is unchanged.
	pub(crate) fn update_owner(&mut self, token: CaptureToken, owner: ScopeOwner) -> bool {
		if let Some(layer) = self.layers.iter_mut().find(|layer| layer.token() == token) {
			layer.owner = owner;
			true
		} else {
			false
		}
	}

	/// Release exactly this token. Other owners are left alone.
	pub fn release(&mut self, token: CaptureToken) -> Option<CaptureLayer> {
		let index = self.layers.iter().position(|layer| layer.token() == token)?;
		Some(self.layers.remove(index))
	}

	/// Release every layer held by `owner`. Other owners stay.
	pub fn release_owner(&mut self, owner: ScopeOwner) -> Vec<CaptureLayer> {
		let mut released = Vec::new();
		self.layers.retain(|layer| {
			if layer.owner() == owner {
				released.push(*layer);
				false
			} else {
				true
			}
		});
		released
	}

	pub(crate) fn new_layer(
		token: CaptureToken,
		source: InputSource,
		owner: ScopeOwner,
		kind: CaptureKind,
	) -> CaptureLayer {
		CaptureLayer { token, source, owner, kind }
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn owner(bits: u64) -> ScopeOwner {
		ScopeOwner::ui(UiTarget::new(Entity::from_bits(bits)))
	}

	fn stack_with(layers: &[(u64, u64, CaptureKind)]) -> CaptureStack {
		let mut stack = CaptureStack::default();
		for (token, bits, kind) in layers {
			stack.push(CaptureStack::new_layer(
				CaptureToken::from_raw(*token),
				InputSource::local_pad(),
				owner(*bits),
				*kind,
			));
		}
		stack
	}

	#[test]
	fn nested_captures_release_independently() {
		let mut stack = stack_with(&[(1, 1, CaptureKind::MenuNav), (2, 2, CaptureKind::Overlay)]);
		assert_eq!(stack.len(), 2);
		assert_eq!(
			stack.top().and_then(CaptureLayer::ui_owner),
			Some(UiTarget::new(Entity::from_bits(2)))
		);
		assert!(stack.blocks_actor(InputSource::local_pad()));
		assert!(stack.owns_back(InputSource::local_pad()));
		assert!(!stack.suppresses_pad(InputSource::local_pad()));

		assert!(stack.release(CaptureToken::from_raw(2)).is_some());
		assert_eq!(
			stack.top_of_kind(CaptureKind::MenuNav).and_then(|layer| layer.ui_owner()),
			Some(UiTarget::new(Entity::from_bits(1)))
		);
		assert_eq!(stack.len(), 1);
		assert!(stack.blocks_actor(InputSource::local_pad()));
	}

	#[test]
	fn releasing_one_owner_cannot_release_another() {
		let mut stack = stack_with(&[(1, 1, CaptureKind::MenuNav), (2, 2, CaptureKind::Overlay)]);
		let released = stack.release_owner(owner(1));
		assert_eq!(released.len(), 1);
		assert_eq!(released[0].owner(), owner(1));
		assert_eq!(stack.len(), 1);
		assert_eq!(stack.top().map(CaptureLayer::owner), Some(owner(2)));
		assert!(stack.release(CaptureToken::from_raw(1)).is_none());
	}

	#[test]
	fn updating_an_owner_keeps_its_position() {
		let mut stack = stack_with(&[(1, 1, CaptureKind::MenuNav), (2, 2, CaptureKind::Overlay)]);
		assert!(stack.update_owner(CaptureToken::from_raw(1), owner(3)));
		assert_eq!(stack.len(), 2);
		assert_eq!(stack.top().map(CaptureLayer::kind), Some(CaptureKind::Overlay));
		assert_eq!(
			stack.top_of_kind(CaptureKind::MenuNav).map(CaptureLayer::owner),
			Some(owner(3))
		);
	}

	#[test]
	fn text_entry_is_the_console_pad_policy() {
		let mut stack = CaptureStack::default();
		assert_eq!(stack.pad_policy(), PadInputPolicy::gameplay());
		stack.push(CaptureStack::new_layer(
			CaptureToken::from_raw(1),
			InputSource::local_pad(),
			ScopeOwner::console(),
			CaptureKind::TextEntry,
		));
		assert!(stack.suppresses_pad(InputSource::local_pad()));
		assert_eq!(stack.pad_policy(), PadInputPolicy::console());
		stack.push(CaptureStack::new_layer(
			CaptureToken::from_raw(2),
			InputSource::local_pad(),
			owner(4),
			CaptureKind::MenuNav,
		));
		assert_eq!(stack.pad_policy(), PadInputPolicy::console());
	}

	#[test]
	fn well_known_owners_are_distinct() {
		assert_ne!(ScopeOwner::pause(), ScopeOwner::map_overlay());
		assert_ne!(ScopeOwner::console(), ScopeOwner::text_lock());
		assert_ne!(ScopeOwner::awaiting_nav(), ScopeOwner::pause());
		assert_ne!(ScopeOwner::spawn_picker(), ScopeOwner::map_overlay());
		assert_ne!(ScopeOwner::spawn_picker(), ScopeOwner::awaiting_nav());
	}
}
