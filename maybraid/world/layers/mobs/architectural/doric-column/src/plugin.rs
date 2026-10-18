//! Registers placeholder meshes, cover colliders, and optional terrain snap.

use bevy::prelude::*;

use crate::placeholder::DoricColumnPlaceholderPlugin;
use crate::spawn::attach_cover_colliders;

/// Column kit runtime (placeholders, cover colliders).
pub struct DoricColumnPlugin;

impl Plugin for DoricColumnPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins(DoricColumnPlaceholderPlugin)
			.add_systems(Update, attach_cover_colliders);
	}
}
