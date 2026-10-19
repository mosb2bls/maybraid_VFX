//! Maybraid HUD mark, as a sized [`ImageNode`].

pub mod spinning;

pub use spinning::{spin_icons, SpinningIcon};

use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value, Scene};

use crate::theme::{MAYBRAID_LOGO, TEXT_YELLOW};

/// Static HUD icon (hints, equipped-tile marks). Loading uses [`SpinningIcon`].
#[derive(Component, Debug, Clone, Copy)]
pub struct Icon {
	pub size: f32,
	pub color: Color,
}

impl Default for Icon {
	fn default() -> Self {
		Self { size: crate::theme::HINT_ICON_SIZE, color: TEXT_YELLOW }
	}
}

impl Icon {
	pub fn maybraid(size: f32, color: Color) -> Self {
		Self { size, color }
	}

	pub fn scene(self) -> impl Scene + 'static {
		self.scene_with_visibility(Visibility::Inherited)
	}

	pub fn scene_with_visibility(self, visibility: Visibility) -> impl Scene + 'static {
		let size = self.size;
		let color = self.color;
		bsn! {
			template_value(self)
			template_value(visibility)
			ImageNode {
				image: MAYBRAID_LOGO,
				color: color,
			}
			Node {
				width: px(size),
				height: px(size),
				flex_shrink: 0.0,
			}
			Pickable::IGNORE
		}
	}

	/// Imperative spawn for sinks that walk a dynamic tree.
	pub fn spawn(
		self,
		parent: &mut ChildSpawnerCommands,
		image: Handle<Image>,
		visibility: Visibility,
	) {
		parent.spawn((
			self,
			visibility,
			ImageNode { image, color: self.color, ..default() },
			Node {
				width: Val::Px(self.size),
				height: Val::Px(self.size),
				flex_shrink: 0.0,
				..default()
			},
			Pickable::IGNORE,
		));
	}
}
