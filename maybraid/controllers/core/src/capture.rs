//! Scoped, composable UI capture. Nested layers release independently.

use bevy::prelude::*;

use crate::source::InputSource;
use crate::ui::UiTarget;

/// Why a UI owner claimed a source.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CaptureKind {
	MenuNav,
	TextEntry,
	Overlay,
}

/// One stack frame: a source claimed by a UI owner.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CaptureLayer {
	source: InputSource,
	owner: UiTarget,
	kind: CaptureKind,
}

impl CaptureLayer {
	pub const fn new(source: InputSource, owner: UiTarget, kind: CaptureKind) -> Self {
		Self { source, owner, kind }
	}

	pub const fn source(self) -> InputSource {
		self.source
	}

	pub const fn owner(self) -> UiTarget {
		self.owner
	}

	pub const fn kind(self) -> CaptureKind {
		self.kind
	}

	pub fn blocks_actor(self, source: InputSource) -> bool {
		self.source == source
	}
}

/// Nested UI claims. The top layer wins; popping an owner leaves the rest.
#[derive(Resource, Clone, Debug, Default)]
pub struct CaptureStack {
	layers: Vec<CaptureLayer>,
}

impl CaptureStack {
	pub fn push(&mut self, layer: CaptureLayer) {
		self.layers.retain(|existing| {
			existing.kind() != layer.kind() || existing.owner() != layer.owner()
		});
		self.layers.push(layer);
	}

	pub fn replace_kind(&mut self, kind: CaptureKind, layer: Option<CaptureLayer>) {
		self.clear_kind(kind);
		if let Some(layer) = layer {
			debug_assert_eq!(layer.kind(), kind);
			self.layers.push(layer);
		}
	}

	pub fn pop_owner(&mut self, owner: UiTarget) -> Option<CaptureLayer> {
		let index = self.layers.iter().rposition(|layer| layer.owner() == owner)?;
		Some(self.layers.remove(index))
	}

	pub fn clear_kind(&mut self, kind: CaptureKind) {
		self.layers.retain(|layer| layer.kind() != kind);
	}

	pub fn top(&self) -> Option<CaptureLayer> {
		self.layers.last().copied()
	}

	pub fn top_of_kind(&self, kind: CaptureKind) -> Option<CaptureLayer> {
		self.layers.iter().rev().copied().find(|layer| layer.kind() == kind)
	}

	pub fn blocks_actor(&self, source: InputSource) -> bool {
		self.layers.iter().any(|layer| layer.blocks_actor(source))
	}

	pub fn is_empty(&self) -> bool {
		self.layers.is_empty()
	}

	pub fn len(&self) -> usize {
		self.layers.len()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn layer(bits: u64, kind: CaptureKind) -> CaptureLayer {
		CaptureLayer::new(InputSource::local_pad(), UiTarget::new(Entity::from_bits(bits)), kind)
	}

	#[test]
	fn nested_captures_release_independently() {
		let mut stack = CaptureStack::default();
		stack.push(layer(1, CaptureKind::MenuNav));
		stack.push(layer(2, CaptureKind::Overlay));
		assert_eq!(stack.len(), 2);
		assert_eq!(stack.top().map(CaptureLayer::owner), Some(UiTarget::new(Entity::from_bits(2))));
		assert!(stack.blocks_actor(InputSource::local_pad()));

		assert!(stack.pop_owner(UiTarget::new(Entity::from_bits(2))).is_some());
		assert_eq!(
			stack.top_of_kind(CaptureKind::MenuNav).map(|layer| layer.owner().entity()),
			Some(Entity::from_bits(1))
		);
		assert_eq!(stack.len(), 1);
	}

	#[test]
	fn replace_kind_swaps_the_menu_owner() {
		let mut stack = CaptureStack::default();
		stack.replace_kind(CaptureKind::MenuNav, Some(layer(1, CaptureKind::MenuNav)));
		stack.replace_kind(CaptureKind::MenuNav, Some(layer(2, CaptureKind::MenuNav)));
		assert_eq!(stack.len(), 1);
		assert_eq!(stack.top().map(|layer| layer.owner().entity()), Some(Entity::from_bits(2)));
		stack.replace_kind(CaptureKind::MenuNav, None);
		assert!(stack.is_empty());
	}
}
