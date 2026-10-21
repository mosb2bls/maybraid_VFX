//! Scoped, composable UI capture. Nested layers release independently.
//! Distinct from install / remove and from simulation suspension.
//!
//! Destination is stored on the layer — not reconstructed from token bits.
//! Updating a claim keeps its stack position unless the owner requests focus.

use bevy::prelude::*;
use maybraid_input::PadInputPolicy;

use crate::actor::ActorTarget;
use crate::source::InputSource;
use crate::token::CapabilityToken;
use crate::ui::UiTarget;

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

	/// Overlay and text sit above menu nav when they are the top claim.
	pub const fn precedes_menu_nav(self) -> bool {
		matches!(self, Self::Overlay | Self::TextEntry)
	}
}

/// One stack frame: a source claimed by an owner token, with an optional UI
/// destination that is not derived from the token.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CaptureLayer {
	source: InputSource,
	token: CapabilityToken,
	kind: CaptureKind,
	destination: Option<UiTarget>,
}

impl CaptureLayer {
	pub const fn new(source: InputSource, token: CapabilityToken, kind: CaptureKind) -> Self {
		Self { source, token, kind, destination: None }
	}

	/// Menu-nav claim whose delivery target is stored beside the owner token.
	pub const fn to_ui(
		source: InputSource,
		token: CapabilityToken,
		destination: UiTarget,
		kind: CaptureKind,
	) -> Self {
		Self { source, token, kind, destination: Some(destination) }
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

	/// UI delivery target. Never reconstructed from [`CapabilityToken`] bits.
	pub const fn destination(self) -> Option<UiTarget> {
		self.destination
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
	/// Acquire or update a claim. An existing token keeps its position.
	pub fn push(&mut self, layer: CaptureLayer) {
		self.acquire(layer);
	}

	/// Update an existing token in place, or append if it is new.
	pub fn acquire(&mut self, layer: CaptureLayer) {
		if let Some(existing) =
			self.layers.iter_mut().find(|existing| existing.token() == layer.token())
		{
			*existing = layer;
			return;
		}
		self.layers.push(layer);
	}

	/// Move this token to the top. Use only when the owner requests focus.
	pub fn focus(&mut self, layer: CaptureLayer) {
		self.layers.retain(|existing| existing.token() != layer.token());
		self.layers.push(layer);
	}

	/// Replace the first layer of `kind` in place, or append if none exists.
	pub fn replace_kind(&mut self, kind: CaptureKind, layer: Option<CaptureLayer>) {
		match layer {
			Some(layer) => {
				debug_assert_eq!(layer.kind(), kind);
				let mut found = false;
				self.layers.retain_mut(|existing| {
					if existing.kind() != kind {
						return true;
					}
					if found {
						return false;
					}
					*existing = layer;
					found = true;
					true
				});
				if !found {
					self.layers.push(layer);
				}
			}
			None => self.clear_kind(kind),
		}
	}

	/// Install or release one token without restacking other layers.
	pub fn set_layer(&mut self, layer: CaptureLayer, present: bool) {
		if present {
			self.acquire(layer);
		} else {
			self.pop_token(layer.token());
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

	/// Actor delivery: the assignment only when no capture claims this source.
	pub fn delivered_actor(
		&self,
		source: InputSource,
		assigned: Option<ActorTarget>,
	) -> Option<ActorTarget> {
		if self.blocks_actor(source) { None } else { assigned }
	}

	/// UI delivery: the top claim's stored destination, if it has one.
	pub fn delivered_ui(&self) -> Option<UiTarget> {
		self.top().and_then(CaptureLayer::destination)
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

	fn menu(bits: u64) -> CaptureLayer {
		let destination = UiTarget::new(Entity::from_bits(bits));
		CaptureLayer::to_ui(
			InputSource::local_pad(),
			CapabilityToken::from_bits(bits),
			destination,
			CaptureKind::MenuNav,
		)
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

	#[test]
	fn updating_a_claim_keeps_its_position() {
		let mut stack = CaptureStack::default();
		stack.push(menu(1));
		stack.push(layer(2, CaptureKind::Overlay));
		stack.replace_kind(CaptureKind::MenuNav, Some(menu(3)));
		assert_eq!(stack.len(), 2);
		assert_eq!(stack.top().map(CaptureLayer::kind), Some(CaptureKind::Overlay));
		assert_eq!(
			stack.top_of_kind(CaptureKind::MenuNav).and_then(CaptureLayer::destination),
			Some(UiTarget::new(Entity::from_bits(3)))
		);
		assert_eq!(stack.delivered_ui(), None);
	}

	#[test]
	fn focus_moves_a_claim_to_the_top() {
		let mut stack = CaptureStack::default();
		stack.push(menu(1));
		stack.push(layer(2, CaptureKind::Overlay));
		stack.focus(menu(1));
		assert_eq!(stack.top().map(CaptureLayer::kind), Some(CaptureKind::MenuNav));
		assert_eq!(stack.delivered_ui(), Some(UiTarget::new(Entity::from_bits(1))));
	}

	#[test]
	fn delivered_actor_is_none_when_captured() {
		let actor = ActorTarget::new(Entity::from_bits(9));
		let mut stack = CaptureStack::default();
		assert_eq!(stack.delivered_actor(InputSource::local_pad(), Some(actor)), Some(actor));
		stack.push(layer(1, CaptureKind::MenuNav));
		assert_eq!(stack.delivered_actor(InputSource::local_pad(), Some(actor)), None);
	}

	#[test]
	fn destination_is_not_reconstructed_from_token_bits() {
		let destination = UiTarget::new(Entity::from_bits(11));
		let layer = CaptureLayer::to_ui(
			InputSource::local_pad(),
			CapabilityToken::CONSOLE,
			destination,
			CaptureKind::MenuNav,
		);
		assert_eq!(layer.token(), CapabilityToken::CONSOLE);
		assert_eq!(layer.destination(), Some(destination));
		assert_ne!(layer.token().bits(), destination.entity().to_bits());
	}
}
