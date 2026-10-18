//! Hanabi helpers: SPRITE_INDEX mapped to particle lifetime, clamped to the last frame.

use bevy_hanabi::prelude::{Attribute, ExprWriter, SetAttributeModifier, WriterExpr};
use bevy_hanabi::ScalarType;

use crate::assets::FlipbookAsset;

/// `min(age / lifetime * frames, last_frame)` so the last puff stays visible.
pub fn sprite_index_over_lifetime(writer: &ExprWriter, flipbook: &FlipbookAsset) -> WriterExpr {
	let frames = writer.lit(flipbook.frame_count as f32);
	let last = writer.lit(flipbook.last_frame() as f32);
	let t = writer
		.attr(Attribute::AGE)
		.div(writer.attr(Attribute::LIFETIME).max(writer.lit(1e-3)));
	t.mul(frames).min(last)
}

pub fn update_sprite_index(writer: &ExprWriter, flipbook: &FlipbookAsset) -> SetAttributeModifier {
	SetAttributeModifier::new(
		Attribute::SPRITE_INDEX,
		sprite_index_over_lifetime(writer, flipbook).cast(ScalarType::Int).expr(),
	)
}
