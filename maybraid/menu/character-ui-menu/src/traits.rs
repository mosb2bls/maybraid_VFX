//! Option traits and thumbnail requests for typed menu primitives.
//!
//! Interaction path: menus lower typed state into [`crate::MenuNode`] trees
//! with host events embedded in leaves; the renderer forwards widget presses
//! back as those events.

use crate::{IdentifiedAsset, ThumbnailCamera};

/// Types with a fixed list of selectable variants.
pub trait ListValues: Copy + PartialEq + 'static {
	fn values() -> &'static [Self];
}

/// Stable string id for persistence and renderer keys.
pub trait StringIdentified {
	fn id(&self) -> &'static str;
}

/// Human-readable option label.
pub trait LabelOption {
	fn label(&self) -> &'static str;
}

/// Color swatch option contract.
pub trait SwatchOption: LabelOption {
	fn color_hex(&self) -> &'static str;
}

/// Asset-backed option contract.
pub trait AssetOption: LabelOption {
	fn asset(&self) -> IdentifiedAsset;
}

/// Controls whether asset pickers show thumbnail previews.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AssetThumbnailDisplay {
	#[default]
	None,
	Inline,
	HoverPreview,
}

/// Request to render a cached asset thumbnail.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ThumbnailRequest {
	pub path: &'static str,
	pub color: [u8; 4],
	pub camera: ThumbnailCamera,
}

impl ThumbnailRequest {
	pub const fn new(path: &'static str, color: [u8; 4], camera: ThumbnailCamera) -> Self {
		Self { path, color, camera }
	}
}
