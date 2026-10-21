//! Scoped, composable UI capture. Nested layers release independently.
//! Distinct from install / remove and from simulation suspension.

use bevy::prelude::*;
use maybraid_input::PadInputPolicy;

use crate::source::InputSource;
use crate::token::CapabilityToken;

/// Why a UI owner claimed a source.
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
}

/// One stack frame: a source claimed by an owner token.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CaptureLayer {
	source: InputSource,
	token: CapabilityToken,
	kind: CaptureKind,
}

impl CaptureLayer {
	pub const fn new(source: InputSource, token: CapabilityToken, kind: CaptureKind) -> Self {
		Self { source, token, kind }
	}

	/// Console / command-text host. Distinct token so a menu can nest on top.
	pub fn console(source: InputSource) -> Self {
		Self::new(source, CapabilityToken::CONSOLE, CaptureKind::TextEntry)
	}

	/// Text-menu lock without a keypad. Distinct from the command console.
	pub fn text_lock(source: InputSource) -> Self {
		Self::new(source, CapabilityToken::TEXT_LOCK, CaptureKind::TextEntry)
	}

	/// World map sheet. Distinct token so a pause menu can nest on top.
	pub fn map_sheet(source: InputSource) -> Self {
		Self::new(source, CapabilityToken::MAP, CaptureKind::Overlay)
	}

	/// Exclusive nav is claimed but the HudMenu has not spawned yet.
	pub fn awaiting(source: InputSource) -> Self {
		Self::new(source, CapabilityToken::AWAITING, CaptureKind::Overlay)
	}

	pub const fn source(self) -> InputSource {
		self.source
	}

	pub const fn token(self) -> CapabilityToken {
		self.token
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

/// Nested UI claims. The top layer wins; popping a token leaves the rest.
#[derive(Resource, Clone, Debug, Default)]
pub struct CaptureStack {
	layers: Vec<CaptureLayer>,
}

impl CaptureStack {
	pub fn push(&mut self, layer: CaptureLayer) {
		self.layers.retain(|existing| {
			existing.kind() != layer.kind() || existing.token() != layer.token()
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

	/// Install or release one token without touching other layers.
	pub fn set_layer(&mut self, layer: CaptureLayer, present: bool) {
		self.pop_token(layer.token());
		if present {
			self.push(layer);
		}
	}

	pub fn pop_token(&mut self, token: CapabilityToken) -> Option<CaptureLayer> {
		let index = self.layers.iter().rposition(|layer| layer.token() == token)?;
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

	pub fn owns_back(&self, source: InputSource) -> bool {
		self.layers.iter().any(|layer| layer.source() == source && layer.owns_back())
	}

	pub fn suppresses_pad(&self, source: InputSource) -> bool {
		self.layers
			.iter()
			.any(|layer| layer.source() == source && layer.suppresses_pad())
	}

	pub fn keyboard_is_menu(&self) -> bool {
		!self.layers.is_empty()
	}

	/// Single derived pad policy. Hosts must not write lanes of their own.
	pub fn pad_policy(&self) -> PadInputPolicy {
		PadInputPolicy::new(self.keyboard_is_menu(), self.suppresses_pad(InputSource::local_pad()))
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
		CaptureLayer::new(InputSource::local_pad(), CapabilityToken::from_bits(bits), kind)
	}

	#[test]
	fn nested_captures_release_independently() {
		let mut stack = CaptureStack::default();
		stack.push(layer(1, CaptureKind::MenuNav));
		stack.push(layer(2, CaptureKind::Overlay));
		assert_eq!(stack.len(), 2);
		assert_eq!(stack.top().map(CaptureLayer::token), Some(CapabilityToken::from_bits(2)));
		assert!(stack.blocks_actor(InputSource::local_pad()));
		assert!(stack.owns_back(InputSource::local_pad()));
		assert!(!stack.suppresses_pad(InputSource::local_pad()));

		assert!(stack.pop_token(CapabilityToken::from_bits(2)).is_some());
		assert_eq!(
			stack.top_of_kind(CaptureKind::MenuNav).map(CaptureLayer::token),
			Some(CapabilityToken::from_bits(1))
		);
		assert_eq!(stack.len(), 1);
		assert!(stack.blocks_actor(InputSource::local_pad()));
	}

	#[test]
	fn nested_captures_released_out_of_order() {
		let mut stack = CaptureStack::default();
		stack.push(layer(1, CaptureKind::MenuNav));
		stack.push(layer(2, CaptureKind::Overlay));
		stack.push(layer(3, CaptureKind::TextEntry));
		assert!(stack.pop_token(CapabilityToken::from_bits(1)).is_some());
		assert_eq!(stack.len(), 2);
		assert!(stack.top_of_kind(CaptureKind::MenuNav).is_none());
		assert!(stack.suppresses_pad(InputSource::local_pad()));
		assert_eq!(stack.top().map(CaptureLayer::token), Some(CapabilityToken::from_bits(3)));
	}

	#[test]
	fn text_and_menu_nest_and_pop_without_last_closer_wins() {
		let mut stack = CaptureStack::default();
		stack.set_layer(CaptureLayer::console(InputSource::local_pad()), true);
		stack.push(layer(1, CaptureKind::MenuNav));
		assert_eq!(stack.len(), 2);
		assert!(stack.suppresses_pad(InputSource::local_pad()));
		assert_eq!(stack.pad_policy(), PadInputPolicy::console());

		stack.pop_token(CapabilityToken::from_bits(1));
		assert!(stack.suppresses_pad(InputSource::local_pad()));
		assert_eq!(stack.len(), 1);

		stack.set_layer(CaptureLayer::console(InputSource::local_pad()), false);
		assert!(stack.is_empty());
		assert_eq!(stack.pad_policy(), PadInputPolicy::gameplay());
	}

	#[test]
	fn replace_kind_swaps_the_menu_owner() {
		let mut stack = CaptureStack::default();
		stack.replace_kind(CaptureKind::MenuNav, Some(layer(1, CaptureKind::MenuNav)));
		stack.replace_kind(CaptureKind::MenuNav, Some(layer(2, CaptureKind::MenuNav)));
		assert_eq!(stack.len(), 1);
		assert_eq!(stack.top().map(CaptureLayer::token), Some(CapabilityToken::from_bits(2)));
		stack.replace_kind(CaptureKind::MenuNav, None);
		assert!(stack.is_empty());
	}

	#[test]
	fn map_overlay_blocks_the_actor_without_suppressing_the_pad() {
		let mut stack = CaptureStack::default();
		stack.set_layer(CaptureLayer::map_sheet(InputSource::local_pad()), true);
		assert!(stack.blocks_actor(InputSource::local_pad()));
		assert!(!stack.suppresses_pad(InputSource::local_pad()));
		assert_eq!(stack.pad_policy(), PadInputPolicy::menu());
	}
}
