//! Token-scoped, composable capture. Releasing one owner cannot release another.

use bevy::prelude::*;

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

	pub(crate) fn push(&mut self, layer: CaptureLayer) {
		self.layers.push(layer);
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

		assert!(stack.release(CaptureToken::from_raw(2)).is_some());
		assert_eq!(
			stack.top_of_kind(CaptureKind::MenuNav).and_then(|layer| layer.ui_owner()),
			Some(UiTarget::new(Entity::from_bits(1)))
		);
		assert_eq!(stack.len(), 1);
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
}
